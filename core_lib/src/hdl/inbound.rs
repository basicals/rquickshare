use std::collections::{HashMap, HashSet};
use std::os::unix::fs::FileExt;
use std::path::{Component, Path};
use std::time::{Duration, Instant};

use anyhow::anyhow;
use bytes::Bytes;
use hmac::{Hmac, Mac};
use libaes::{Cipher, AES_256_KEY_LEN};
use p256::ecdh::diffie_hellman;
use p256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
use p256::elliptic_curve::zeroize::{Zeroize, Zeroizing};
use p256::{EncodedPoint, PublicKey};
use prost::Message;
use rand::Rng;
use sha2::{Digest, Sha256, Sha512};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast::{Receiver, Sender};

use super::{InnerState, State};
use crate::channel::{ChannelAction, ChannelDirection, ChannelMessage};
use crate::hdl::info::{InternalFileInfo, TransferMetadata};
use crate::hdl::{TextPayloadInfo, TextPayloadType};
use crate::location_nearby_connections::payload_transfer_frame::{
    payload_header, PacketType, PayloadChunk, PayloadHeader,
};
use crate::location_nearby_connections::{KeepAliveFrame, OfflineFrame, PayloadTransferFrame};
use crate::protocol::{checked_payload_buffer_size, SANE_FRAME_LENGTH};
use crate::securegcm::ukey2_alert::AlertType;
use crate::securegcm::{
    ukey2_message, DeviceToDeviceMessage, GcmMetadata, Type, Ukey2Alert, Ukey2ClientFinished,
    Ukey2ClientInit, Ukey2HandshakeCipher, Ukey2Message, Ukey2ServerInit,
};
use crate::securemessage::{
    EcP256PublicKey, EncScheme, GenericPublicKey, Header, HeaderAndBody, PublicKeyType,
    SecureMessage, SigScheme,
};
use crate::sharing_nearby::{text_metadata, wifi_credentials_metadata::SecurityType};
use crate::utils::{
    encode_point, gen_ecdsa_keypair, gen_random, hkdf_extract_expand, normalize_p256_coordinate,
    stream_read_exact, to_four_digit_string, DeviceType, RemoteDeviceInfo,
};
use crate::{location_nearby_connections, sharing_nearby};

type HmacSha256 = Hmac<Sha256>;

const MAX_RECEIVED_FILENAME_BYTES: usize = 255;
const MAX_RECEIVED_PARENT_FOLDER_BYTES: usize = 4096;
const MAX_RECEIVED_FOLDER_DEPTH: usize = 64;

fn validate_received_path_component(value: &str, kind: &str) -> Result<(), anyhow::Error> {
    if value.is_empty() {
        return Err(anyhow!("Received {kind} is empty"));
    }

    if value.len() > MAX_RECEIVED_FILENAME_BYTES {
        return Err(anyhow!(
            "Received {kind} is too long: {} bytes",
            value.len()
        ));
    }

    if value == "."
        || value == ".."
        || value
            .chars()
            .any(|c| c == '/' || c == '\\' || c == '\0' || c.is_control())
    {
        return Err(anyhow!("Received {kind} contains unsafe characters"));
    }

    Ok(())
}

fn validate_received_file_name(name: &str) -> Result<(), anyhow::Error> {
    validate_received_path_component(name, "file name")?;

    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(anyhow!(
            "Received file name is not a single safe path component"
        )),
    }
}

fn normalize_received_parent_folder(value: &str) -> Result<Vec<String>, anyhow::Error> {
    if value.is_empty() {
        return Ok(Vec::new());
    }

    if value.len() > MAX_RECEIVED_PARENT_FOLDER_BYTES {
        return Err(anyhow!(
            "Received parent folder is too long: {} bytes",
            value.len()
        ));
    }

    let segments = value
        .split(['/', '\\'])
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if segments.first().is_some_and(|segment| {
        let bytes = segment.as_bytes();
        bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
    }) {
        return Err(anyhow!("Received parent folder must be relative"));
    }

    if segments.len() > MAX_RECEIVED_FOLDER_DEPTH {
        return Err(anyhow!(
            "Received parent folder exceeds maximum depth of {MAX_RECEIVED_FOLDER_DEPTH}"
        ));
    }

    for segment in &segments {
        validate_received_path_component(segment, "folder component")?;
    }

    Ok(segments)
}

fn path_is_occupied(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn unique_child_path(
    parent: &Path,
    name: &str,
    reserved: &HashSet<std::path::PathBuf>,
) -> Result<std::path::PathBuf, anyhow::Error> {
    let direct = parent.join(name);
    if !path_is_occupied(&direct) && !reserved.contains(&direct) {
        return Ok(direct);
    }

    let mut counter = 1_u64;
    loop {
        let candidate = parent.join(format!("{counter}_{name}"));
        if !path_is_occupied(&candidate) && !reserved.contains(&candidate) {
            return Ok(candidate);
        }
        counter = counter
            .checked_add(1)
            .ok_or_else(|| anyhow!("Destination suffix overflow"))?;
    }
}

fn wire_relative_path(path: &Path) -> Result<String, anyhow::Error> {
    let mut segments = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let value = value
                    .to_str()
                    .ok_or_else(|| anyhow!("Destination path is not valid UTF-8"))?;
                validate_received_path_component(value, "destination component")?;
                segments.push(value.to_owned());
            }
            _ => return Err(anyhow!("Destination path is not safely relative")),
        }
    }

    Ok(segments.join("/"))
}

struct PreparedInboundFiles {
    files: Vec<(i64, InternalFileInfo)>,
    names: Vec<String>,
    total_bytes: u64,
}

fn prepare_inbound_files_at(
    download_dir: &Path,
    files: &[sharing_nearby::FileMetadata],
    existing_payload_ids: &HashSet<i64>,
) -> Result<PreparedInboundFiles, anyhow::Error> {
    let mut prepared = Vec::with_capacity(files.len());
    let mut display_paths = Vec::with_capacity(files.len());
    let mut total_bytes = 0_u64;
    let mut reserved_destinations = HashSet::new();
    let mut root_aliases = HashMap::<String, String>::new();
    let mut payload_ids = existing_payload_ids.clone();

    for file in files {
        let file_name = file.name();
        validate_received_file_name(file_name)?;

        if file.size() < 0 {
            return Err(anyhow!(
                "Invalid negative file size for {file_name}: {}",
                file.size()
            ));
        }

        let payload_id = file.payload_id();
        if !payload_ids.insert(payload_id) {
            return Err(anyhow!("Duplicate file payload id: {payload_id}"));
        }

        let parent_segments = normalize_received_parent_folder(file.parent_folder())?;

        let (destination, normalized_parent) = if parent_segments.is_empty() {
            (
                unique_child_path(download_dir, file_name, &reserved_destinations)?,
                None,
            )
        } else {
            let original_root = parent_segments
                .first()
                .ok_or_else(|| anyhow!("Missing parent-folder root"))?
                .clone();

            let chosen_root = if let Some(chosen) = root_aliases.get(&original_root) {
                chosen.clone()
            } else {
                let chosen_path =
                    unique_child_path(download_dir, &original_root, &reserved_destinations)?;
                let chosen = chosen_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| anyhow!("Resolved root folder name is not valid UTF-8"))?
                    .to_owned();
                reserved_destinations.insert(chosen_path);
                root_aliases.insert(original_root.clone(), chosen.clone());
                chosen
            };

            let mut remapped_parent = Vec::with_capacity(parent_segments.len());
            remapped_parent.push(chosen_root);
            remapped_parent.extend(parent_segments.iter().skip(1).cloned());

            let parent_dir = remapped_parent
                .iter()
                .fold(download_dir.to_path_buf(), |path, segment| {
                    path.join(segment)
                });
            let destination = unique_child_path(&parent_dir, file_name, &reserved_destinations)?;

            (destination, Some(remapped_parent.join("/")))
        };

        reserved_destinations.insert(destination.clone());

        let relative = destination
            .strip_prefix(download_dir)
            .map_err(|_| anyhow!("Prepared destination escaped download directory"))?;
        display_paths.push(wire_relative_path(relative)?);

        let info = InternalFileInfo {
            payload_id,
            file_url: destination,
            parent_folder: normalized_parent,
            bytes_transferred: 0,
            total_size: file.size(),
            file: None,
        };

        total_bytes = total_bytes
            .checked_add(info.total_size as u64)
            .ok_or_else(|| anyhow!("Total transfer size overflow"))?;

        prepared.push((payload_id, info));
    }

    Ok(PreparedInboundFiles {
        files: prepared,
        names: display_paths,
        total_bytes,
    })
}

#[cfg(any(test, not(target_os = "linux")))]
pub(crate) fn materialize_safe_destination_at(
    download_dir: &Path,
    destination: &Path,
) -> Result<std::path::PathBuf, anyhow::Error> {
    let relative = destination
        .strip_prefix(download_dir)
        .map_err(|_| anyhow!("Destination escaped the configured download directory"))?;
    let file_name = relative
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Destination file name is invalid"))?;
    validate_received_file_name(file_name)?;

    let canonical_root = download_dir
        .canonicalize()
        .map_err(|error| anyhow!("Unable to resolve download directory: {error}"))?;
    if !canonical_root.is_dir() {
        return Err(anyhow!("Download directory is not a directory"));
    }

    let mut safe_parent = canonical_root.clone();
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            let Component::Normal(component) = component else {
                return Err(anyhow!("Destination parent contains unsafe components"));
            };
            let component = component
                .to_str()
                .ok_or_else(|| anyhow!("Destination parent is not valid UTF-8"))?;
            validate_received_path_component(component, "destination folder component")?;

            safe_parent.push(component);
            match std::fs::symlink_metadata(&safe_parent) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() || !metadata.is_dir() {
                        return Err(anyhow!(
                            "Destination parent is not a safe directory: {}",
                            safe_parent.display()
                        ));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    std::fs::create_dir(&safe_parent)?;
                }
                Err(error) => return Err(error.into()),
            }

            let canonical_parent = safe_parent.canonicalize()?;
            if !canonical_parent.starts_with(&canonical_root) {
                return Err(anyhow!("Destination parent escaped download directory"));
            }
            safe_parent = canonical_parent;
        }
    }

    Ok(safe_parent.join(file_name))
}

fn parse_wifi_password_payload(buffer: &[u8]) -> Result<String, anyhow::Error> {
    if buffer.len() < 4 {
        return Err(anyhow!(
            "Wi-Fi password payload is too short: {} bytes",
            buffer.len()
        ));
    }

    if buffer[0] != 0x0A {
        return Err(anyhow!(
            "Unexpected Wi-Fi password payload prefix: 0x{:02x}",
            buffer[0]
        ));
    }

    let password_len = usize::from(buffer[1]);
    let password_end = 2_usize
        .checked_add(password_len)
        .ok_or_else(|| anyhow!("Wi-Fi password length overflow"))?;
    let trailer_end = password_end
        .checked_add(2)
        .ok_or_else(|| anyhow!("Wi-Fi password trailer overflow"))?;

    if trailer_end > buffer.len() {
        return Err(anyhow!(
            "Wi-Fi password payload declares {password_len} bytes but only {} payload bytes are available",
            buffer.len().saturating_sub(4)
        ));
    }

    if buffer[password_end] != 0x10 {
        return Err(anyhow!(
            "Unexpected Wi-Fi password trailer marker: 0x{:02x}",
            buffer[password_end]
        ));
    }

    let password = std::str::from_utf8(&buffer[2..password_end])?;
    Ok(password.to_owned())
}

#[cfg(all(feature = "experimental", target_os = "linux"))]
async fn read_plain_frame_from<R: AsyncRead + Unpin>(
    stream: &mut R,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut length = [0_u8; 4];
    stream_read_exact(stream, &mut length).await?;
    let frame_len = u32::from_be_bytes(length) as usize;
    if frame_len == 0 || frame_len > SANE_FRAME_LENGTH as usize {
        return Err(anyhow!("Invalid plaintext frame length: {frame_len}"));
    }

    let mut frame = vec![0_u8; frame_len];
    stream_read_exact(stream, &mut frame).await?;
    Ok(frame)
}

#[cfg(all(feature = "experimental", target_os = "linux"))]
async fn send_plain_frame_on<W: AsyncWrite + Unpin>(
    stream: &mut W,
    data: &[u8],
) -> Result<(), anyhow::Error> {
    let frame_len =
        u32::try_from(data.len()).map_err(|_| anyhow!("Plaintext frame is too large"))?;
    if data.is_empty() || data.len() > SANE_FRAME_LENGTH as usize {
        return Err(anyhow!("Invalid plaintext frame length: {}", data.len()));
    }

    stream.write_all(&frame_len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BwuClientIntroductionIdentity {
    pub endpoint_id: String,
    pub last_endpoint_id: Option<String>,
}

impl BwuClientIntroductionIdentity {
    fn matches_expected(&self, expected_endpoint_id: &str) -> bool {
        self.endpoint_id == expected_endpoint_id
            || self.last_endpoint_id.as_deref() == Some(expected_endpoint_id)
    }
}

fn validate_client_introduction(
    frame_data: &[u8],
) -> Result<BwuClientIntroductionIdentity, anyhow::Error> {
    use location_nearby_connections::bandwidth_upgrade_negotiation_frame::EventType;
    use location_nearby_connections::v1_frame::FrameType;

    let frame = OfflineFrame::decode(frame_data)?;
    let v1 = frame
        .v1
        .as_ref()
        .ok_or_else(|| anyhow!("Bandwidth-upgrade introduction has no v1 frame"))?;
    if v1.r#type() != FrameType::BandwidthUpgradeNegotiation {
        return Err(anyhow!(
            "Expected bandwidth-upgrade introduction, got {:?}",
            v1.r#type()
        ));
    }

    let negotiation = v1
        .bandwidth_upgrade_negotiation
        .as_ref()
        .ok_or_else(|| anyhow!("Missing bandwidth-upgrade negotiation payload"))?;
    if negotiation.event_type() != EventType::ClientIntroduction {
        return Err(anyhow!(
            "Expected CLIENT_INTRODUCTION, got {:?}",
            negotiation.event_type()
        ));
    }

    let introduction = negotiation
        .client_introduction
        .as_ref()
        .ok_or_else(|| anyhow!("CLIENT_INTRODUCTION has no identity payload"))?;
    let endpoint_id = introduction.endpoint_id().to_owned();
    if endpoint_id.is_empty() {
        return Err(anyhow!("CLIENT_INTRODUCTION has no endpoint id"));
    }
    let last_endpoint_id = introduction
        .last_endpoint_id
        .as_ref()
        .filter(|id| !id.is_empty())
        .cloned();

    Ok(BwuClientIntroductionIdentity {
        endpoint_id,
        last_endpoint_id,
    })
}

/// Inspect a non-consuming TCP peek buffer for a complete bandwidth-upgrade
/// CLIENT_INTRODUCTION. Returns None for incomplete or ordinary Quick Share
/// frames so the primary listener can continue with the normal inbound path.
pub fn peek_client_introduction(
    buffer: &[u8],
) -> Result<Option<BwuClientIntroductionIdentity>, anyhow::Error> {
    if buffer.len() < 4 {
        return Ok(None);
    }

    let frame_len = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
    if frame_len == 0 || frame_len > SANE_FRAME_LENGTH as usize {
        return Ok(None);
    }

    let total = 4_usize
        .checked_add(frame_len)
        .ok_or_else(|| anyhow!("CLIENT_INTRODUCTION frame length overflow"))?;
    if buffer.len() < total {
        return Ok(None);
    }

    Ok(validate_client_introduction(&buffer[4..total]).ok())
}

#[derive(Debug)]
pub struct InboundRequest<S = TcpStream> {
    socket: S,
    frame_header: super::frame_header::FrameHeader,
    pub state: InnerState,
    sender: Sender<ChannelMessage>,
    receiver: Option<Receiver<ChannelMessage>>,
    session: Option<crate::sessions::SessionLease>,
    authentication: crate::peer_authentication::PeerAuthentication,
    previews: crate::preview_payload::PreviewIds,
    bandwidth_upgrade_enabled: bool,
    bwu_pending: bool,
    peer_endpoint_id: Option<String>,
    receive_directory: crate::receive::Directory,
    receive_root: Option<crate::receive::Root>,
    #[cfg(target_os = "linux")]
    pending_files: Option<crate::received_files::PendingFiles>,
    #[cfg(target_os = "linux")]
    publications: std::collections::HashMap<i64, crate::receive::Publication>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> InboundRequest<S> {
    pub fn new(socket: S, id: String, sender: Sender<ChannelMessage>) -> Self {
        let receiver = sender.subscribe();

        Self {
            socket,
            frame_header: super::frame_header::FrameHeader::default(),
            state: InnerState {
                id,
                server_seq: 0,
                client_seq: 0,
                state: State::Initial,
                encryption_done: true,
                ..Default::default()
            },
            sender,
            receiver: Some(receiver),
            session: None,
            authentication: crate::peer_authentication::PeerAuthentication::anonymous(
                crate::peer_authentication::Role::Receiver,
            ),
            previews: crate::preview_payload::PreviewIds::default(),
            bandwidth_upgrade_enabled: false,
            bwu_pending: false,
            peer_endpoint_id: None,
            receive_directory: crate::receive::ReceivePolicy::new(None).snapshot(),
            receive_root: None,
            #[cfg(target_os = "linux")]
            pending_files: None,
            #[cfg(target_os = "linux")]
            publications: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn with_receive_directory(mut self, directory: crate::receive::Directory) -> Self {
        self.receive_directory = directory;
        self
    }

    pub fn enable_bandwidth_upgrade(&mut self) {
        self.bandwidth_upgrade_enabled = true;
    }

    pub(crate) fn attach_session(mut self, session: crate::sessions::SessionLease) -> Self {
        self.state.id = session.event_id();
        self.receiver = None;
        self.session = Some(session);
        self
    }

    pub(crate) fn cancellation(&self) -> tokio_util::sync::CancellationToken {
        self.session
            .as_ref()
            .map(|session| session.cancellation())
            .unwrap_or_default()
    }

    pub(crate) fn record_termination(&mut self, state: State) {
        if !crate::sessions::terminal(&self.state.state) {
            self.state.state = state;
            self.finish_received();
            if let Some(session) = self.session.as_mut() {
                session.observe_state(&self.state);
            }
        }
    }

    fn finish_received(&mut self) {
        #[cfg(target_os = "linux")]
        if crate::sessions::terminal(&self.state.state) {
            // Unlinked incomplete members disappear on close; completed links remain.
            for file in self.state.transferred_files.values_mut() {
                file.file.take();
            }
            self.publications.clear();
            if let Some(pending) = self.pending_files.take() {
                if let Some(session) = self.session.as_mut() {
                    session.completed_files = if self.state.state == State::Finished {
                        pending.finish()
                    } else {
                        pending.finish_partial()
                    };
                }
            }
        }
    }

    pub fn take_bwu_pending(&mut self) -> bool {
        std::mem::take(&mut self.bwu_pending)
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    fn bandwidth_upgrade_frame(
        event_type: location_nearby_connections::bandwidth_upgrade_negotiation_frame::EventType,
        upgrade_path_info: Option<
            location_nearby_connections::bandwidth_upgrade_negotiation_frame::UpgradePathInfo,
        >,
    ) -> OfflineFrame {
        use location_nearby_connections::BandwidthUpgradeNegotiationFrame;

        OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::BandwidthUpgradeNegotiation
                        .into(),
                ),
                bandwidth_upgrade_negotiation: Some(BandwidthUpgradeNegotiationFrame {
                    event_type: Some(event_type.into()),
                    upgrade_path_info,
                    client_introduction: None,
                    client_introduction_ack: None,
                }),
                ..Default::default()
            }),
        }
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    async fn send_upgrade_path_available(&mut self, port: u16) -> Result<(), anyhow::Error> {
        use location_nearby_connections::bandwidth_upgrade_negotiation_frame::{
            upgrade_path_info::{Medium, WifiLanSocket},
            EventType, UpgradePathInfo,
        };

        let ip = crate::utils::local_lan_ipv4()
            .ok_or_else(|| anyhow!("No suitable LAN IPv4 address for bandwidth upgrade"))?;
        info!(
            "BWU: offering WIFI_LAN at {}.{}.{}.{}:{port}",
            ip[0], ip[1], ip[2], ip[3]
        );

        let frame = Self::bandwidth_upgrade_frame(
            EventType::UpgradePathAvailable,
            Some(UpgradePathInfo {
                medium: Some(Medium::WifiLan.into()),
                wifi_lan_socket: Some(WifiLanSocket {
                    ip_address: Some(ip.to_vec()),
                    wifi_port: Some(i32::from(port)),
                }),
                supports_client_introduction_ack: Some(true),
                ..Default::default()
            }),
        );
        self.encrypt_and_send(&frame).await
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    fn client_introduction_ack_frame() -> OfflineFrame {
        use location_nearby_connections::bandwidth_upgrade_negotiation_frame::{
            ClientIntroductionAck, EventType,
        };
        use location_nearby_connections::BandwidthUpgradeNegotiationFrame;

        OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::BandwidthUpgradeNegotiation
                        .into(),
                ),
                bandwidth_upgrade_negotiation: Some(BandwidthUpgradeNegotiationFrame {
                    event_type: Some(EventType::ClientIntroductionAck.into()),
                    client_introduction_ack: Some(ClientIntroductionAck {}),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        }
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    async fn read_encrypted_offline_frame(&mut self) -> Result<OfflineFrame, anyhow::Error> {
        let frame_data = read_plain_frame_from(&mut self.socket).await?;
        let secure_message = SecureMessage::decode(frame_data.as_slice())?;
        self.decrypt_secure_message(&secure_message).await
    }

    pub async fn handle(&mut self) -> Result<(), anyhow::Error> {
        tokio::select! {
            command = crate::sessions::consent_command(&mut self.session) => {
                if let Some(command) = command {
                    let answer = self.session.as_mut().and_then(|session| session.apply(command, &self.state.state));
                    if let Some(accept) = answer { self.apply_consent(accept).await?; }
                }
            },
            i = crate::sessions::legacy_message(&mut self.receiver) => {
                match i {
                    Ok(channel_msg) => {
                        if channel_msg.direction == ChannelDirection::LibToFront {
                            return Ok(());
                        }

                        if channel_msg.id != self.state.id {
                            return Ok(());
                        }

                        debug!("inbound: got: {:?}", channel_msg);
                        match channel_msg.action {
                            Some(ChannelAction::AcceptTransfer) => {
                                self.apply_consent(true).await?;
                            },
                            Some(ChannelAction::RejectTransfer) => {
                                self.apply_consent(false).await?;
                            },
                            Some(ChannelAction::CancelTransfer) => {
                                self.update_state(
                                    |e| {
                                        e.state = State::Cancelled;
                                    },
                                    true,
                                ).await;
                                self.disconnection().await?;
                                return Err(anyhow!(crate::errors::AppError::NotAnError));
                            },
                            None => {
                                trace!("inbound: nothing to do")
                            },
                        }
                    }
                    Err(e) => {
                        error!("inbound: channel error: {}", e);
                    }
                }
            },
            header = self.frame_header.read(&mut self.socket) => {
                self._handle(header?).await?
            }
        }

        Ok(())
    }

    pub async fn _handle(&mut self, length_buf: [u8; 4]) -> Result<(), anyhow::Error> {
        let msg_length = u32::from_be_bytes(length_buf) as usize;
        // Ensure the message length is not unreasonably big to avoid allocation attacks
        if msg_length > SANE_FRAME_LENGTH as usize {
            error!("Message length too big");
            return Err(anyhow!("value"));
        }

        // Allocate buffer for the actual message and read it
        let mut frame_data = vec![0u8; msg_length];
        stream_read_exact(&mut self.socket, &mut frame_data).await?;

        let current_state = &self.state;
        // Now determine what will be the request type based on current state
        match current_state.state {
            State::Initial => {
                debug!("Handling State::Initial frame");
                let frame = location_nearby_connections::OfflineFrame::decode(&*frame_data)?;
                let rdi = self.process_connection_request(&frame)?;
                info!("Remote device introduction received");

                // Advance current state
                self.update_state(
                    |e: &mut InnerState| {
                        e.state = State::ReceivedConnectionRequest;
                        e.remote_device_info = Some(rdi);
                    },
                    false,
                )
                .await;
            }
            State::ReceivedConnectionRequest => {
                debug!("Handling State::ReceivedConnectionRequest frame");
                let msg = Ukey2Message::decode(&*frame_data)?;
                self.process_ukey2_client_init(&msg).await?;

                self.update_state(
                    |e: &mut InnerState| {
                        e.state = State::SentUkeyServerInit;
                        e.client_init_msg_data = Some(frame_data);
                    },
                    false,
                )
                .await;
            }
            State::SentUkeyServerInit => {
                debug!("Handling State::SentUkeyServerInit frame");
                let msg = Ukey2Message::decode(&*frame_data)?;
                self.process_ukey2_client_finish(&msg, &frame_data).await?;

                self.update_state(
                    |e: &mut InnerState| {
                        e.state = State::ReceivedUkeyClientFinish;
                    },
                    false,
                )
                .await;
            }
            State::ReceivedUkeyClientFinish => {
                debug!("Handling State::ReceivedUkeyClientFinish frame");
                let frame = location_nearby_connections::OfflineFrame::decode(&*frame_data)?;
                self.process_connection_response(&frame).await?;

                self.update_state(
                    |e: &mut InnerState| {
                        e.state = State::SentConnectionResponse;
                    },
                    false,
                )
                .await;

                if self.bandwidth_upgrade_enabled {
                    self.bwu_pending = true;
                }
            }
            _ => {
                debug!("Handling SecureMessage frame");
                let smsg = SecureMessage::decode(&*frame_data)?;
                self.decrypt_and_process_secure_message(&smsg).await?;
            }
        }

        Ok(())
    }

    fn process_connection_request(
        &mut self,
        frame: &location_nearby_connections::OfflineFrame,
    ) -> Result<RemoteDeviceInfo, anyhow::Error> {
        let v1_frame = frame
            .v1
            .as_ref()
            .ok_or_else(|| anyhow!("Missing required fields"))?;

        if v1_frame.r#type() != location_nearby_connections::v1_frame::FrameType::ConnectionRequest
        {
            return Err(anyhow!(format!(
                "Unexpected frame type: {:?}",
                v1_frame.r#type()
            )));
        }

        let connection_request = v1_frame
            .connection_request
            .as_ref()
            .ok_or_else(|| anyhow!("Missing required fields"))?;

        let peer_endpoint_id = connection_request.endpoint_id();
        if peer_endpoint_id.is_empty() {
            return Err(anyhow!("Connection request has no endpoint id"));
        }
        self.peer_endpoint_id = Some(peer_endpoint_id.to_owned());

        let endpoint_info = connection_request
            .endpoint_info
            .as_ref()
            .ok_or_else(|| anyhow!("Missing endpoint info"))?;

        // Check if endpoint info length is greater than 17
        if endpoint_info.len() <= 17 {
            return Err(anyhow!("Endpoint info too short"));
        }

        let device_name_length = endpoint_info[17] as usize;
        // Validate length including device name
        if endpoint_info.len() < device_name_length + 18 {
            return Err(anyhow!(
                "Endpoint info too short to contain the device name"
            ));
        }

        // Extract and validate device name based on length
        let device_name = std::str::from_utf8(&endpoint_info[18..(18 + device_name_length)])
            .map_err(|_| anyhow!("Device name is not valid UTF-8"))?;

        // Parsing the device type
        let raw_device_type = (endpoint_info[0] & 7) >> 1_usize;

        Ok(RemoteDeviceInfo {
            name: device_name.to_string(),
            device_type: DeviceType::from_raw_value(raw_device_type),
        })
    }

    async fn process_ukey2_client_init(&mut self, msg: &Ukey2Message) -> Result<(), anyhow::Error> {
        if msg.message_type() != ukey2_message::Type::ClientInit {
            self.send_ukey2_alert(AlertType::BadMessageType).await?;
            return Err(anyhow!(
                "UKey2: message_type({:?}) != ClientInit",
                msg.message_type
            ));
        }

        let client_init = match Ukey2ClientInit::decode(msg.message_data()) {
            Ok(uk2ci) => uk2ci,
            Err(e) => {
                self.send_ukey2_alert(AlertType::BadMessageData).await?;
                return Err(anyhow!("UKey2: Ukey2ClientInit::decode: {}", e));
            }
        };

        if client_init.version() != 1 {
            self.send_ukey2_alert(AlertType::BadVersion).await?;
            return Err(anyhow!("UKey2: client_init.version != 1"));
        }

        if client_init.random().len() != 32 {
            self.send_ukey2_alert(AlertType::BadRandom).await?;
            return Err(anyhow!("UKey2: client_init.random.len != 32"));
        }

        // Searching for preferred cipher commitment
        let mut found = false;
        for commitment in &client_init.cipher_commitments {
            trace!("CipherCommitment: {:?}", commitment.handshake_cipher());
            if Ukey2HandshakeCipher::P256Sha512 == commitment.handshake_cipher() {
                found = true;
                self.update_state(
                    |e| {
                        e.cipher_commitment = Some(commitment.clone());
                    },
                    false,
                )
                .await;
                break;
            }
        }

        if !found {
            self.send_ukey2_alert(AlertType::BadHandshakeCipher).await?;
            return Err(anyhow!("UKey2: badHandshakeCipher"));
        }

        if client_init.next_protocol() != "AES_256_CBC-HMAC_SHA256" {
            self.send_ukey2_alert(AlertType::BadNextProtocol).await?;
            return Err(anyhow!(
                "UKey2: badNextProtocol: {}",
                client_init.next_protocol()
            ));
        }

        let (secret_key, public_key) = gen_ecdsa_keypair();

        let encoded_point = public_key.to_encoded_point(false);
        let x = encoded_point
            .x()
            .ok_or_else(|| anyhow!("Generated P-256 point has no X coordinate"))?;
        let y = encoded_point
            .y()
            .ok_or_else(|| anyhow!("Generated P-256 point has no Y coordinate"))?;

        let pkey = GenericPublicKey {
            r#type: PublicKeyType::EcP256.into(),
            ec_p256_public_key: Some(EcP256PublicKey {
                x: encode_point(Bytes::from(x.to_vec()))?,
                y: encode_point(Bytes::from(y.to_vec()))?,
            }),
            ..Default::default()
        };

        let server_init = Ukey2ServerInit {
            version: Some(1),
            random: Some(rand::rng().random::<[u8; 32]>().to_vec()),
            handshake_cipher: Some(Ukey2HandshakeCipher::P256Sha512.into()),
            public_key: Some(pkey.encode_to_vec()),
        };

        let server_init_msg = Ukey2Message {
            message_type: Some(ukey2_message::Type::ServerInit.into()),
            message_data: Some(server_init.encode_to_vec()),
        };

        let server_init_data = server_init_msg.encode_to_vec();
        self.update_state(
            |e| {
                e.private_key = Some(secret_key);
                e.public_key = Some(public_key);
                e.server_init_data = Some(server_init_data.clone());
            },
            false,
        )
        .await;

        self.send_frame(server_init_data).await?;

        Ok(())
    }

    async fn process_ukey2_client_finish(
        &mut self,
        msg: &Ukey2Message,
        frame_data: &Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        if msg.message_type() != ukey2_message::Type::ClientFinish {
            self.send_ukey2_alert(AlertType::BadMessageType).await?;
            return Err(anyhow!(
                "UKey2: message_type({:?}) != ClientFinish",
                msg.message_type
            ));
        }

        let sha512 = Sha512::digest(frame_data);
        let cipher_commitment = self
            .state
            .cipher_commitment
            .as_ref()
            .ok_or_else(|| anyhow!("Missing cipher commitment"))?;
        if cipher_commitment.commitment() != &sha512[..] {
            error!("cipher_commitment isn't equals to sha512(frame_data)");
            return Err(anyhow!("UKey2: cipher_commitment != sha512"));
        }

        let client_finish = match Ukey2ClientFinished::decode(msg.message_data()) {
            Ok(uk2cf) => uk2cf,
            Err(e) => {
                return Err(anyhow!("UKey2: Ukey2ClientFinished::decode: {}", e));
            }
        };

        if client_finish.public_key.is_none() {
            return Err(anyhow!("UKey2: client_finish.public_key None"));
        }

        let client_public_key = match GenericPublicKey::decode(client_finish.public_key()) {
            Ok(cpk) => cpk,
            Err(e) => {
                return Err(anyhow!("UKey2: GenericPublicKey::decode: {}", e));
            }
        };

        self.finalize_key_exchange(client_public_key).await?;

        Ok(())
    }

    async fn process_connection_response(
        &mut self,
        frame: &location_nearby_connections::OfflineFrame,
    ) -> Result<(), anyhow::Error> {
        let v1_frame = frame
            .v1
            .as_ref()
            .ok_or_else(|| anyhow!("Missing required fields"))?;

        if v1_frame.r#type() != location_nearby_connections::v1_frame::FrameType::ConnectionResponse
        {
            return Err(anyhow!(format!(
                "Unexpected frame type: {:?}",
                v1_frame.r#type()
            )));
        }

        let response = location_nearby_connections::OfflineFrame {
			version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
			v1: Some(location_nearby_connections::V1Frame {
				r#type: Some(location_nearby_connections::v1_frame::FrameType::ConnectionResponse.into()),
				connection_response: Some(location_nearby_connections::ConnectionResponseFrame {
					response: Some(location_nearby_connections::connection_response_frame::ResponseStatus::Accept.into()),
					os_info: Some(location_nearby_connections::OsInfo {
						r#type: Some(location_nearby_connections::os_info::OsType::Linux.into())
					}),
					..Default::default()
				}),
				..Default::default()
			})
		};

        self.send_frame(response.encode_to_vec()).await?;

        let paired_encryption = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::PairedKeyEncryption.into()),
                paired_key_encryption: Some(self.authentication.outgoing_frame()?),
                ..Default::default()
            }),
        };

        self.send_encrypted_frame(&paired_encryption).await?;

        Ok(())
    }

    async fn decrypt_secure_message(
        &mut self,
        smsg: &SecureMessage,
    ) -> Result<OfflineFrame, anyhow::Error> {
        let recv_hmac_key = self
            .state
            .recv_hmac_key
            .as_ref()
            .ok_or_else(|| anyhow!("Missing receive HMAC key"))?;
        let mut hmac = HmacSha256::new_from_slice(recv_hmac_key)?;
        hmac.update(&smsg.header_and_body);
        if !hmac.finalize().into_bytes()[..].eq(smsg.signature.as_slice()) {
            return Err(anyhow!("hmac!=signature"));
        }

        let header_and_body = HeaderAndBody::decode(&*smsg.header_and_body)?;

        let msg_data = header_and_body.body;
        let key = self
            .state
            .decrypt_key
            .as_ref()
            .ok_or_else(|| anyhow!("Missing decrypt key"))?;

        let key_bytes: &[u8; AES_256_KEY_LEN] = key
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Invalid decrypt key length: {}", key.len()))?;
        let mut cipher = Cipher::new_256(key_bytes);
        cipher.set_auto_padding(true);
        let decrypted = cipher.cbc_decrypt(header_and_body.header.iv(), &msg_data);

        let d2d_msg = DeviceToDeviceMessage::decode(&*decrypted)?;

        let seq = self.get_client_seq_inc().await;
        if d2d_msg.sequence_number() != seq {
            return Err(anyhow!(
                "Error d2d_msg.sequence_number invalid ({} vs {})",
                d2d_msg.sequence_number(),
                seq
            ));
        }

        Ok(location_nearby_connections::OfflineFrame::decode(
            d2d_msg.message(),
        )?)
    }

    async fn decrypt_and_process_secure_message(
        &mut self,
        smsg: &SecureMessage,
    ) -> Result<(), anyhow::Error> {
        let offline = self.decrypt_secure_message(smsg).await?;
        self.process_offline_frame(offline).await
    }

    async fn process_offline_frame(&mut self, offline: OfflineFrame) -> Result<(), anyhow::Error> {
        let v1_frame = offline
            .v1
            .as_ref()
            .ok_or_else(|| anyhow!("Missing required fields"))?;
        match v1_frame.r#type() {
            location_nearby_connections::v1_frame::FrameType::PayloadTransfer => {
                trace!("Received FrameType::PayloadTransfer");
                let payload_transfer = v1_frame
                    .payload_transfer
                    .as_ref()
                    .ok_or_else(|| anyhow!("Missing required fields"))?;

                let header = payload_transfer
                    .payload_header
                    .as_ref()
                    .ok_or_else(|| anyhow!("Missing required fields"))?;
                let chunk = payload_transfer
                    .payload_chunk
                    .as_ref()
                    .ok_or_else(|| anyhow!("Missing required fields"))?;

                match header.r#type() {
                    payload_header::PayloadType::Bytes => {
                        info!("Processing PayloadType::Bytes");
                        let payload_id = header.id();
                        if self.previews.contains(payload_id) {
                            // Optional opaque data must never be interpreted as sharing control.
                            // Use the same reservations/framing checks; release completion immediately.
                            let _ = self.state.byte_payloads.push(header, chunk)?;
                            return Ok(());
                        }

                        let declared_size = checked_payload_buffer_size(header.total_size())?;
                        if self.session.is_some()
                            && self.state.text_payload.as_ref().is_some_and(|text| {
                                text.get_i64_value() == payload_id
                                    && !matches!(text, TextPayloadInfo::Wifi { .. })
                            })
                        {
                            let metadata = self
                                .state
                                .transfer_metadata
                                .as_ref()
                                .ok_or_else(|| anyhow!("Missing text metadata"))?;
                            if header.id.is_none()
                                || header.total_size.is_none()
                                || self.state.state != State::ReceivingFiles
                                || declared_size as u64 != metadata.total_bytes
                                || declared_size > crate::sessions::text::MAX_TEXT_BYTES
                            {
                                return Err(anyhow!(
                                    "Native text payload does not match accepted metadata"
                                ));
                            }
                        }

                        if let Some(buffer) = self.state.byte_payloads.push(header, chunk)? {
                            let matching_text_payload =
                                self.state.text_payload.clone().filter(|text_payload| {
                                    text_payload.get_i64_value() == payload_id
                                });

                            if let Some(text_payload) = matching_text_payload {
                                if self.session.is_some()
                                    && !matches!(text_payload, TextPayloadInfo::Wifi { .. })
                                {
                                    if buffer.len() != declared_size {
                                        return Err(anyhow!("Native text payload is incomplete"));
                                    }
                                    let body = std::str::from_utf8(&buffer)
                                        .map_err(|_| anyhow!("Native text encoding is invalid"))?;
                                    if body.contains('\0') {
                                        return Err(anyhow!(
                                            "Native text contains unsupported NUL"
                                        ));
                                    }
                                }
                                info!("Transfer finished");

                                match text_payload {
                                    TextPayloadInfo::Url(_) => {
                                        let payload = std::str::from_utf8(&buffer)?.to_owned();
                                        self.update_state(
                                            |e| {
                                                if let Some(tmd) = e.transfer_metadata.as_mut() {
                                                    tmd.text_payload = Some(payload);
                                                    tmd.text_type = Some(TextPayloadType::Url);
                                                    tmd.ack_bytes = buffer.len() as u64;
                                                }
                                            },
                                            false,
                                        )
                                        .await;
                                    }
                                    TextPayloadInfo::Text(_) => {
                                        let payload = std::str::from_utf8(&buffer)?.to_owned();
                                        self.update_state(
                                            |e| {
                                                if let Some(tmd) = e.transfer_metadata.as_mut() {
                                                    tmd.text_payload = Some(payload);
                                                    tmd.text_type = Some(TextPayloadType::Text);
                                                    tmd.ack_bytes = buffer.len() as u64;
                                                }
                                            },
                                            false,
                                        )
                                        .await;
                                    }
                                    TextPayloadInfo::Wifi {
                                        ssid,
                                        security_type,
                                        ..
                                    } => {
                                        let password = match security_type {
                                            SecurityType::Open => String::new(),
                                            SecurityType::WpaPsk
                                            | SecurityType::Wep
                                            | SecurityType::Sae => {
                                                parse_wifi_password_payload(&buffer)?
                                            }
                                            SecurityType::UnknownSecurityType => {
                                                return Err(anyhow!(
                                                    "Unsupported Wi-Fi credential security type"
                                                ));
                                            }
                                        };

                                        self.update_state(
                                            |e| {
                                                if let Some(tmd) = e.transfer_metadata.as_mut() {
                                                    tmd.text_payload =
                                                        Some(format!("{ssid}: {password}"));
                                                    tmd.text_type = Some(TextPayloadType::Wifi);
                                                }
                                            },
                                            false,
                                        )
                                        .await;
                                    }
                                }

                                self.update_state(
                                    |e| {
                                        e.state = State::Finished;
                                    },
                                    true,
                                )
                                .await;
                                self.disconnection().await?;
                                return Err(anyhow!(crate::errors::AppError::NotAnError));
                            }

                            let inner_frame = sharing_nearby::Frame::decode(buffer.as_slice())?;
                            self.process_transfer_setup(&inner_frame).await?;
                        }
                    }
                    payload_header::PayloadType::File => {
                        info!("Processing PayloadType::File");
                        let payload_id = header.id();

                        if self.state.state != State::ReceivingFiles {
                            return Err(anyhow!(
                                "File payload received before transfer acceptance"
                            ));
                        }

                        let file_internal = self
                            .state
                            .transferred_files
                            .get_mut(&payload_id)
                            .ok_or_else(|| {
                                anyhow!("File payload ID ({}) is not known", payload_id)
                            })?;

                        let current_offset = file_internal.bytes_transferred;
                        if chunk.offset() != current_offset {
                            return Err(anyhow!(
                                "Invalid offset into file {}, expected {}",
                                chunk.offset(),
                                current_offset
                            ));
                        }

                        let chunk_size = i64::try_from(chunk.body().len())
                            .map_err(|_| anyhow!("File chunk is too large"))?;
                        let new_offset = current_offset
                            .checked_add(chunk_size)
                            .ok_or_else(|| anyhow!("File offset overflow"))?;
                        if new_offset > file_internal.total_size {
                            return Err(anyhow!(
                                "Transferred file size exceeds previously specified value: {new_offset} vs {}",
                                file_internal.total_size
                            ));
                        }

                        // A final chunk may carry the last bytes as well as its
                        // EOF flag. Reject a premature EOF before writing anything.
                        let is_final_chunk = (chunk.flags() & 1) == 1;
                        if is_final_chunk && new_offset != file_internal.total_size {
                            return Err(anyhow!("File payload ended before its declared size"));
                        }

                        let file = file_internal.file.as_ref().ok_or_else(|| {
                            anyhow!("File payload received before destination file was opened")
                        })?;
                        if !chunk.body().is_empty() {
                            file.write_all_at(chunk.body(), current_offset as u64)?;
                            file_internal.bytes_transferred = new_offset;

                            self.update_state(
                                |e| {
                                    if let Some(tmd) = e.transfer_metadata.as_mut() {
                                        tmd.ack_bytes += chunk_size as u64;
                                    }
                                },
                                true,
                            )
                            .await;
                        }

                        if is_final_chunk {
                            #[cfg(target_os = "linux")]
                            let file_internal = self
                                .state
                                .transferred_files
                                .get(&payload_id)
                                .ok_or_else(|| anyhow!("Missing final file metadata"))?;
                            #[cfg(target_os = "linux")]
                            let file = file_internal.file.as_ref().ok_or_else(|| {
                                anyhow!("File payload received before destination file was opened")
                            })?;
                            #[cfg(target_os = "linux")]
                            if let Some(publication) = self.publications.get(&payload_id) {
                                publication
                                    .publish(file, file_internal.total_size as u64)
                                    .map_err(|_| anyhow!("Cannot finish received file safely"))?;
                            }
                            #[cfg(target_os = "linux")]
                            if let Some(pending) = self.pending_files.as_mut() {
                                pending
                                    .record(
                                        payload_id,
                                        &file_internal.file_url,
                                        file,
                                        file_internal.total_size as u64,
                                    )
                                    .map_err(|_| {
                                        anyhow!("Cannot record completed received file")
                                    })?;
                            }
                            #[cfg(target_os = "linux")]
                            if let Some(publication) = self.publications.remove(&payload_id) {
                                publication
                                    .sync()
                                    .map_err(|_| anyhow!("Received file directory sync failed"))?;
                            }
                            self.state.transferred_files.remove(&payload_id);
                            if self.state.transferred_files.is_empty() {
                                info!("Transfer finished");
                                self.update_state(
                                    |e| {
                                        e.state = State::Finished;
                                    },
                                    true,
                                )
                                .await;
                                self.disconnection().await?;
                                return Err(anyhow!(crate::errors::AppError::NotAnError));
                            }
                        }
                    }
                    payload_header::PayloadType::Stream => {
                        error!("Unhandled PayloadType::Stream: {:?}", header.r#type())
                    }
                    payload_header::PayloadType::UnknownPayloadType => {
                        error!(
                            "Invalid PayloadType::UnknownPayloadType: {:?}",
                            header.r#type()
                        )
                    }
                }
            }
            location_nearby_connections::v1_frame::FrameType::KeepAlive => {
                trace!("Sending keepalive");
                self.send_keepalive(true).await?;
            }
            location_nearby_connections::v1_frame::FrameType::BandwidthUpgradeRetry => {
                debug!("BWU: peer requested bandwidth-upgrade retry; continuing current transport");
            }
            location_nearby_connections::v1_frame::FrameType::BandwidthUpgradeNegotiation => {
                let event = v1_frame
                    .bandwidth_upgrade_negotiation
                    .as_ref()
                    .map(|frame| frame.event_type());
                debug!("BWU: peer negotiation event on current transport: {event:?}");
            }
            _ => {
                error!("Unhandled offline frame encrypted: {:?}", offline);
            }
        }

        Ok(())
    }

    async fn process_transfer_setup(
        &mut self,
        frame: &sharing_nearby::Frame,
    ) -> Result<(), anyhow::Error> {
        let v1_frame = frame
            .v1
            .as_ref()
            .ok_or_else(|| anyhow!("Missing required fields"))?;

        if v1_frame.r#type() == sharing_nearby::v1_frame::FrameType::Cancel {
            info!("Transfer canceled");
            self.update_state(
                |e| {
                    e.state = State::Cancelled;
                },
                true,
            )
            .await;
            self.disconnection().await?;
            return Err(anyhow!(crate::errors::AppError::NotAnError));
        }

        match self.state.state {
            State::SentConnectionResponse => {
                debug!("Processing State::SentConnectionResponse");
                self.process_paired_key_encryption_frame(v1_frame).await?;
                self.update_state(
                    |e| {
                        e.state = State::SentPairedKeyResult;
                    },
                    false,
                )
                .await;
            }
            State::SentPairedKeyResult => {
                debug!("Processing State::SentPairedKeyResult");
                self.process_paired_key_result(v1_frame).await?;
                self.update_state(
                    |e| {
                        e.state = State::ReceivedPairedKeyResult;
                    },
                    false,
                )
                .await;
            }
            State::ReceivedPairedKeyResult => {
                debug!("Processing State::ReceivedPairedKeyResult");
                if v1_frame.introduction.is_some() {
                    self.process_introduction(v1_frame).await?;
                } else {
                    debug!(
                        "Ignoring interleaved sharing frame {:?} while waiting for Introduction",
                        v1_frame.r#type()
                    );
                }
            }
            _ => {
                info!(
                    "Unhandled connection state in process_transfer_setup: {:?}",
                    self.state.state
                );
            }
        }

        Ok(())
    }

    async fn process_paired_key_encryption_frame(
        &mut self,
        v1_frame: &sharing_nearby::V1Frame,
    ) -> Result<(), anyhow::Error> {
        let proof = v1_frame
            .paired_key_encryption
            .as_ref()
            .ok_or_else(|| anyhow!("Missing paired-key proof"))?;
        let outcome = self.authentication.verify(proof);

        let paired_result = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::PairedKeyResult.into()),
                paired_key_result: Some(sharing_nearby::PairedKeyResultFrame {
                    status: Some(outcome.wire()),
                }),
                ..Default::default()
            }),
        };

        self.send_encrypted_frame(&paired_result).await?;
        if outcome == crate::peer_authentication::Outcome::Fail {
            return Err(crate::peer_authentication::AuthenticationError.into());
        }

        Ok(())
    }

    async fn process_paired_key_result(
        &mut self,
        v1_frame: &sharing_nearby::V1Frame,
    ) -> Result<(), anyhow::Error> {
        let result = v1_frame
            .paired_key_result
            .as_ref()
            .ok_or_else(|| anyhow!("Missing paired-key result"))?;
        self.authentication.finish(result.status)?;

        Ok(())
    }

    async fn process_introduction(
        &mut self,
        v1_frame: &sharing_nearby::V1Frame,
    ) -> Result<(), anyhow::Error> {
        let introduction = v1_frame
            .introduction
            .as_ref()
            .ok_or_else(|| anyhow!("Missing required fields"))?;
        if self.session.is_some() && !introduction.text_metadata.is_empty() {
            let meta = introduction
                .text_metadata
                .first()
                .ok_or_else(|| anyhow!("Missing text attachment"))?;
            if introduction.text_metadata.len() != 1
                || !introduction.file_metadata.is_empty()
                || !introduction.wifi_credentials_metadata.is_empty()
                || meta.payload_id.is_none()
                || meta.size.is_none_or(|size| {
                    size < 0 || size as u64 > crate::sessions::text::MAX_TEXT_BYTES as u64
                })
                || !matches!(
                    meta.r#type(),
                    text_metadata::Type::Text
                        | text_metadata::Type::Url
                        | text_metadata::Type::Address
                        | text_metadata::Type::PhoneNumber
                )
            {
                return Err(anyhow!("Unsupported native text attachment"));
            }
        }

        let previews = crate::preview_payload::PreviewIds::from_introduction(introduction)?;

        // No need to inform the channel here, we'll do it anyway with files info
        self.update_state(
            |e| {
                e.state = State::WaitingForUserConsent;
            },
            false,
        )
        .await;

        if !introduction.file_metadata.is_empty() && introduction.text_metadata.is_empty() {
            trace!("process_introduction: handling file_metadata");
            let existing_payload_ids = self
                .state
                .transferred_files
                .keys()
                .copied()
                .collect::<HashSet<_>>();
            if self.receive_root.is_none() {
                self.receive_root = Some(
                    self.receive_directory
                        .pin()
                        .map_err(|_| anyhow!("Receive directory is unavailable"))?,
                );
            }
            let root = self
                .receive_root
                .as_ref()
                .ok_or_else(|| anyhow!("Receive directory is unavailable"))?;
            #[cfg(target_os = "linux")]
            if let Some(session) = &self.session {
                self.pending_files = crate::received_files::PendingFiles::new(
                    root.clone(),
                    introduction
                        .file_metadata
                        .iter()
                        .map(|file| file.payload_id())
                        .collect(),
                    session.file_budget.clone(),
                );
                if self.pending_files.is_none() {
                    return Err(anyhow!("Received file tracking capacity is unavailable"));
                }
            }
            let planning = root.planning_path();
            let PreparedInboundFiles {
                files: mut prepared_files,
                names: files_name,
                total_bytes,
            } = prepare_inbound_files_at(
                &planning,
                &introduction.file_metadata,
                &existing_payload_ids,
            )?;
            for (_, info) in &mut prepared_files {
                let relative = info
                    .file_url
                    .strip_prefix(&planning)
                    .map_err(|_| anyhow!("Invalid receive destination"))?;
                info.file_url = root.display().join(relative);
            }

            for ((payload_id, info), _file_name) in
                prepared_files.into_iter().zip(files_name.iter())
            {
                debug!("Inbound file destination prepared");
                self.state.transferred_files.insert(payload_id, info);
            }

            let metadata = TransferMetadata {
                id: self.state.id.clone(),
                destination: Some(root.display().to_string_lossy().into_owned()),
                source: self.state.remote_device_info.clone(),
                files: Some(files_name),
                pin_code: self.state.pin_code.clone(),
                text_description: None,
                total_bytes,
                ..Default::default()
            };

            info!("Requesting user consent for incoming transfer");
            self.update_state(
                |e| {
                    e.transfer_metadata = Some(metadata);
                },
                true,
            )
            .await;
        } else if introduction.text_metadata.len() == 1 {
            trace!("process_introduction: handling text_metadata");
            let meta = introduction
                .text_metadata
                .first()
                .ok_or_else(|| anyhow!("Missing text metadata"))?;

            match meta.r#type() {
                text_metadata::Type::Url => {
                    let metadata = TransferMetadata {
                        id: self.state.id.clone(),
                        destination: None,
                        source: self.state.remote_device_info.clone(),
                        files: None,
                        pin_code: self.state.pin_code.clone(),
                        text_description: meta.text_title.clone(),
                        text_type: Some(TextPayloadType::Url),
                        total_bytes: meta.size().max(0) as u64,
                        ..Default::default()
                    };

                    info!("Requesting user consent for incoming transfer");
                    self.update_state(
                        |e| {
                            e.text_payload = Some(TextPayloadInfo::Url(meta.payload_id()));
                            e.transfer_metadata = Some(metadata);
                        },
                        true,
                    )
                    .await;
                }
                text_metadata::Type::PhoneNumber
                | text_metadata::Type::Address
                | text_metadata::Type::Text => {
                    let metadata = TransferMetadata {
                        id: self.state.id.clone(),
                        destination: None,
                        source: self.state.remote_device_info.clone(),
                        files: None,
                        pin_code: self.state.pin_code.clone(),
                        text_description: meta.text_title.clone(),
                        text_type: Some(TextPayloadType::Text),
                        total_bytes: meta.size().max(0) as u64,
                        ..Default::default()
                    };

                    info!("Requesting user consent for incoming transfer");
                    self.update_state(
                        |e| {
                            e.text_payload = Some(TextPayloadInfo::Text(meta.payload_id()));
                            e.transfer_metadata = Some(metadata);
                        },
                        true,
                    )
                    .await;
                }
                text_metadata::Type::Unknown => {
                    return self.reject_transfer(Some(
                        sharing_nearby::connection_response_frame::Status::UnsupportedAttachmentType,
                    )).await;
                }
            }
        } else if introduction.wifi_credentials_metadata.len() == 1 {
            trace!("process_introduction: handling wifi_credentials_metadata");
            let meta = introduction
                .wifi_credentials_metadata
                .first()
                .ok_or_else(|| anyhow!("Missing Wi-Fi credential metadata"))?;

            let metadata = TransferMetadata {
                id: self.state.id.clone(),
                destination: None,
                source: self.state.remote_device_info.clone(),
                files: None,
                pin_code: self.state.pin_code.clone(),
                text_description: meta.ssid.clone(),
                ..Default::default()
            };

            self.update_state(
                |e| {
                    e.text_payload = Some(TextPayloadInfo::Wifi {
                        payload_id: meta.payload_id(),
                        ssid: meta.ssid().to_owned(),
                        security_type: meta.security_type(),
                    });
                    e.transfer_metadata = Some(metadata);
                },
                true,
            )
            .await;
        } else {
            return self
                .reject_transfer(Some(
                    sharing_nearby::connection_response_frame::Status::UnsupportedAttachmentType,
                ))
                .await;
        }

        self.previews = previews;
        Ok(())
    }

    async fn disconnection(&mut self) -> Result<(), anyhow::Error> {
        let frame = location_nearby_connections::OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::Disconnection.into(),
                ),
                disconnection: Some(location_nearby_connections::DisconnectionFrame {
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        if self.state.encryption_done {
            self.encrypt_and_send(&frame).await
        } else {
            self.send_frame(frame.encode_to_vec()).await
        }
    }

    async fn accept_transfer(&mut self) -> Result<(), anyhow::Error> {
        let ids: Vec<i64> = self.state.transferred_files.keys().cloned().collect();
        #[cfg(target_os = "linux")]
        let mut prepared = Vec::with_capacity(ids.len());
        for id in ids {
            let mfi = self
                .state
                .transferred_files
                .get_mut(&id)
                .ok_or_else(|| anyhow!("Missing transfer metadata for payload {id}"))?;

            let root = self
                .receive_root
                .as_ref()
                .ok_or_else(|| anyhow!("Receive directory is unavailable"))?;
            #[cfg(not(target_os = "linux"))]
            {
                let file = root
                    .create_file(&mfi.file_url)
                    .map_err(|_| anyhow!("Cannot create received file safely"))?;
                mfi.file = Some(file);
            }
            #[cfg(target_os = "linux")]
            {
                let (file, publication) = root
                    .stage_file(&mfi.file_url)
                    .map_err(|_| anyhow!("Receive filesystem cannot prepare a safe file"))?;
                prepared.push((id, file, publication));
            }
        }
        #[cfg(target_os = "linux")]
        for (id, file, publication) in prepared {
            self.state
                .transferred_files
                .get_mut(&id)
                .expect("prepared file")
                .file = Some(file);
            self.publications.insert(id, publication);
        }

        let frame = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::Response.into()),
                connection_response: Some(sharing_nearby::ConnectionResponseFrame {
                    status: Some(sharing_nearby::connection_response_frame::Status::Accept.into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        self.send_encrypted_frame(&frame).await?;

        self.update_state(
            |e| {
                e.state = State::ReceivingFiles;
            },
            true,
        )
        .await;

        Ok(())
    }

    async fn apply_consent(&mut self, accept: bool) -> Result<(), anyhow::Error> {
        if accept {
            return self.accept_transfer().await;
        }
        self.update_state(|state| state.state = State::Rejected, true)
            .await;
        self.reject_transfer(Some(
            sharing_nearby::connection_response_frame::Status::Reject,
        ))
        .await?;
        Err(anyhow!(crate::errors::AppError::NotAnError))
    }

    async fn reject_transfer(
        &mut self,
        reason: Option<sharing_nearby::connection_response_frame::Status>,
    ) -> Result<(), anyhow::Error> {
        let sreason = if let Some(r) = reason {
            r
        } else {
            sharing_nearby::connection_response_frame::Status::Reject
        };

        let frame = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::Response.into()),
                connection_response: Some(sharing_nearby::ConnectionResponseFrame {
                    status: Some(sreason.into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        self.send_encrypted_frame(&frame).await?;

        Ok(())
    }

    async fn finalize_key_exchange(
        &mut self,
        raw_peer_key: GenericPublicKey,
    ) -> Result<(), anyhow::Error> {
        let peer_p256_key = raw_peer_key
            .ec_p256_public_key
            .ok_or_else(|| anyhow!("Missing required fields"))?;

        let x = normalize_p256_coordinate(&peer_p256_key.x)?;
        let y = normalize_p256_coordinate(&peer_p256_key.y)?;

        let mut bytes = Vec::with_capacity(65);
        bytes.push(0x04);
        bytes.extend_from_slice(&x);
        bytes.extend_from_slice(&y);

        let encoded_point = EncodedPoint::from_bytes(bytes)?;
        let peer_key = Option::<PublicKey>::from(PublicKey::from_encoded_point(&encoded_point))
            .ok_or_else(|| anyhow!("Invalid peer P-256 public key"))?;
        let priv_key = self
            .state
            .private_key
            .as_ref()
            .ok_or_else(|| anyhow!("Missing local private key"))?;

        let dhs = diffie_hellman(priv_key.to_nonzero_scalar(), peer_key.as_affine());
        let derived_secret = Sha256::digest(dhs.raw_secret_bytes());

        let client_init_msg_data = self
            .state
            .client_init_msg_data
            .as_ref()
            .ok_or_else(|| anyhow!("Missing UKEY2 client init data"))?;
        let server_init_data = self
            .state
            .server_init_data
            .as_ref()
            .ok_or_else(|| anyhow!("Missing UKEY2 server init data"))?;

        let mut ukey_info: Vec<u8> = vec![];
        ukey_info.extend_from_slice(client_init_msg_data);
        ukey_info.extend_from_slice(server_init_data);

        let auth_label = "UKEY2 v1 auth".as_bytes();
        let next_label = "UKEY2 v1 next".as_bytes();

        let mut auth_string = Zeroizing::new(hkdf_extract_expand(
            auth_label,
            &derived_secret,
            &ukey_info,
            32,
        )?);
        self.authentication.bind_token(&auth_string)?;
        let next_secret = hkdf_extract_expand(next_label, &derived_secret, &ukey_info, 32)?;

        let salt_hex = "82AA55A0D397F88346CA1CEE8D3909B95F13FA7DEB1D4AB38376B8256DA85510";
        let salt =
            hex::decode(salt_hex).map_err(|e| anyhow!("Failed to decode salt_hex: {}", e))?;

        let d2d_client = hkdf_extract_expand(&salt, &next_secret, "client".as_bytes(), 32)?;
        let d2d_server = hkdf_extract_expand(&salt, &next_secret, "server".as_bytes(), 32)?;

        let key_salt_hex = "BF9D2A53C63616D75DB0A7165B91C1EF73E537F2427405FA23610A4BE657642E";
        let key_salt = hex::decode(key_salt_hex)
            .map_err(|e| anyhow!("Failed to decode key_salt_hex: {}", e))?;

        let client_key = hkdf_extract_expand(&key_salt, &d2d_client, "ENC:2".as_bytes(), 32)?;
        let client_hmac_key = hkdf_extract_expand(&key_salt, &d2d_client, "SIG:1".as_bytes(), 32)?;
        let server_key = hkdf_extract_expand(&key_salt, &d2d_server, "ENC:2".as_bytes(), 32)?;
        let server_hmac_key = hkdf_extract_expand(&key_salt, &d2d_server, "SIG:1".as_bytes(), 32)?;

        self.update_state(
            |e| {
                e.decrypt_key = Some(client_key);
                e.recv_hmac_key = Some(client_hmac_key);
                e.encrypt_key = Some(server_key);
                e.send_hmac_key = Some(server_hmac_key);
                e.pin_code = Some(to_four_digit_string(&auth_string));
                e.encryption_done = true;
            },
            false,
        )
        .await;

        auth_string.zeroize();
        debug!("Pairing verification code prepared");

        Ok(())
    }

    async fn send_ukey2_alert(&mut self, atype: AlertType) -> Result<(), anyhow::Error> {
        let alert = Ukey2Alert {
            r#type: Some(atype.into()),
            error_message: None,
        };

        let data = Ukey2Message {
            message_type: Some(atype.into()),
            message_data: Some(alert.encode_to_vec()),
        };

        self.send_frame(data.encode_to_vec()).await
    }

    async fn send_encrypted_frame(
        &mut self,
        frame: &sharing_nearby::Frame,
    ) -> Result<(), anyhow::Error> {
        let frame_data = frame.encode_to_vec();
        let body_size = frame_data.len();

        let payload_header = PayloadHeader {
            id: Some(rand::rng().random_range(i64::MIN..i64::MAX)),
            r#type: Some(payload_header::PayloadType::Bytes.into()),
            total_size: Some(body_size as i64),
            is_sensitive: Some(false),
            ..Default::default()
        };

        let transfer = PayloadTransferFrame {
            packet_type: Some(PacketType::Data.into()),
            payload_chunk: Some(PayloadChunk {
                offset: Some(0),
                flags: Some(0),
                body: Some(frame_data),
            }),
            payload_header: Some(payload_header.clone()),
            ..Default::default()
        };

        let wrapper = location_nearby_connections::OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
                ),
                payload_transfer: Some(transfer),
                ..Default::default()
            }),
        };

        // Encrypt and send offline
        self.encrypt_and_send(&wrapper).await?;

        // Send lastChunk
        let transfer = PayloadTransferFrame {
            packet_type: Some(PacketType::Data.into()),
            payload_chunk: Some(PayloadChunk {
                offset: Some(body_size as i64),
                flags: Some(1), // lastChunk
                body: Some(vec![]),
            }),
            payload_header: Some(payload_header),
            ..Default::default()
        };

        let wrapper = location_nearby_connections::OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
                ),
                payload_transfer: Some(transfer),
                ..Default::default()
            }),
        };

        // Encrypt and send offline
        self.encrypt_and_send(&wrapper).await?;

        Ok(())
    }

    async fn encrypt_and_send(&mut self, frame: &OfflineFrame) -> Result<(), anyhow::Error> {
        let d2d_msg = DeviceToDeviceMessage {
            sequence_number: Some(self.get_server_seq_inc().await),
            message: Some(frame.encode_to_vec()),
        };

        let key = self
            .state
            .encrypt_key
            .as_ref()
            .ok_or_else(|| anyhow!("Missing encrypt key"))?;
        let msg_data = d2d_msg.encode_to_vec();
        let iv = gen_random(16);

        let key_bytes: &[u8; AES_256_KEY_LEN] = key
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Invalid encrypt key length: {}", key.len()))?;
        let mut cipher = Cipher::new_256(key_bytes);
        cipher.set_auto_padding(true);
        let encrypted = cipher.cbc_encrypt(&iv, &msg_data);

        let hb = HeaderAndBody {
            body: encrypted,
            header: Header {
                encryption_scheme: EncScheme::Aes256Cbc.into(),
                signature_scheme: SigScheme::HmacSha256.into(),
                iv: Some(iv),
                public_metadata: Some(
                    GcmMetadata {
                        r#type: Type::DeviceToDeviceMessage.into(),
                        version: Some(1),
                    }
                    .encode_to_vec(),
                ),
                ..Default::default()
            },
        };

        let send_hmac_key = self
            .state
            .send_hmac_key
            .as_ref()
            .ok_or_else(|| anyhow!("Missing send HMAC key"))?;
        let mut hmac = HmacSha256::new_from_slice(send_hmac_key)?;
        hmac.update(&hb.encode_to_vec());
        let result = hmac.finalize();

        let smsg = SecureMessage {
            header_and_body: hb.encode_to_vec(),
            signature: result.into_bytes().to_vec(),
        };

        self.send_frame(smsg.encode_to_vec()).await?;

        Ok(())
    }

    async fn send_keepalive(&mut self, ack: bool) -> Result<(), anyhow::Error> {
        let ack_frame = location_nearby_connections::OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(location_nearby_connections::v1_frame::FrameType::KeepAlive.into()),
                keep_alive: Some(KeepAliveFrame { ack: Some(ack) }),
                ..Default::default()
            }),
        };

        if self.state.encryption_done {
            self.encrypt_and_send(&ack_frame).await
        } else {
            self.send_frame(ack_frame.encode_to_vec()).await
        }
    }

    async fn send_frame(&mut self, data: Vec<u8>) -> Result<(), anyhow::Error> {
        let length = data.len();

        // Prepare length prefix in big-endian format
        let length_bytes = [
            (length >> 24) as u8,
            (length >> 16) as u8,
            (length >> 8) as u8,
            length as u8,
        ];

        let mut prefixed_length = Vec::with_capacity(length + 4);
        prefixed_length.extend_from_slice(&length_bytes);
        prefixed_length.extend_from_slice(&data);

        self.socket.write_all(&prefixed_length).await?;
        self.socket.flush().await?;

        Ok(())
    }

    async fn get_server_seq_inc(&mut self) -> i32 {
        self.update_state(
            |e| {
                e.server_seq += 1;
            },
            false,
        )
        .await;

        self.state.server_seq
    }

    async fn get_client_seq_inc(&mut self) -> i32 {
        self.update_state(
            |e| {
                e.client_seq += 1;
            },
            false,
        )
        .await;

        self.state.client_seq
    }

    async fn update_state<F>(&mut self, f: F, inform: bool)
    where
        F: FnOnce(&mut InnerState),
    {
        f(&mut self.state);
        self.finish_received();

        if let Some(session) = self.session.as_mut() {
            session.observe_state(&self.state);
        }

        if !inform {
            return;
        }

        trace!("Sending msg into the channel");
        let _ = self.sender.send(ChannelMessage {
            id: self.state.id.clone(),
            direction: ChannelDirection::LibToFront,
            rtype: Some(crate::channel::TransferType::Inbound),
            state: Some(self.state.state.clone()),
            meta: self.state.transfer_metadata.clone(),
            ..Default::default()
        });
        tokio::task::yield_now().await;
    }
}

#[cfg(all(feature = "experimental", target_os = "linux"))]
impl InboundRequest<crate::hdl::MigratableStream> {
    /// Upgrade an established encrypted BLE session to Wi-Fi LAN.
    ///
    /// Failure before the transport swap leaves the existing BLE stream intact,
    /// so callers can continue the session on BLE when an upgrade is unavailable.
    pub async fn do_bandwidth_upgrade(
        &mut self,
        router: &crate::hdl::BwuRouter,
        tcp_port: u16,
    ) -> Result<(), anyhow::Error> {
        use location_nearby_connections::bandwidth_upgrade_negotiation_frame::EventType;
        use location_nearby_connections::v1_frame::FrameType;

        let bwu_started = Instant::now();
        let expected_endpoint_id = self
            .peer_endpoint_id
            .clone()
            .ok_or_else(|| anyhow!("BWU session has no peer endpoint id"))?;
        let socket_receiver = router.register(expected_endpoint_id.clone()).await?;

        if let Err(error) = self.send_upgrade_path_available(tcp_port).await {
            router.cancel(&expected_endpoint_id).await;
            return Err(error);
        }
        info!(
            "BWU timing: upgrade offer sent at {} ms for endpoint {expected_endpoint_id}",
            bwu_started.elapsed().as_millis()
        );

        let mut tcp = match tokio::time::timeout(Duration::from_secs(15), socket_receiver).await {
            Ok(Ok(socket)) => {
                info!(
                    "BWU timing: TCP route established at {} ms for endpoint {expected_endpoint_id}",
                    bwu_started.elapsed().as_millis()
                );
                socket
            }
            Ok(Err(_)) => {
                router.cancel(&expected_endpoint_id).await;
                return Err(anyhow!("BWU TCP route closed before a socket arrived"));
            }
            Err(_) => {
                router.cancel(&expected_endpoint_id).await;
                warn!(
                    "BWU timing: no routed TCP connection after {} ms; continuing on BLE",
                    bwu_started.elapsed().as_millis()
                );
                return Ok(());
            }
        };

        let introduction = read_plain_frame_from(&mut tcp).await?;
        let identity = validate_client_introduction(&introduction)?;
        let expected_endpoint_id = expected_endpoint_id.as_str();
        if !identity.matches_expected(expected_endpoint_id) {
            return Err(anyhow!(
                "BWU CLIENT_INTRODUCTION endpoint mismatch: expected {expected_endpoint_id}, got current={} last={:?}",
                identity.endpoint_id,
                identity.last_endpoint_id
            ));
        }
        if identity.endpoint_id != expected_endpoint_id {
            info!(
                "BWU: accepted dynamic endpoint alias current={} last={:?} for expected {expected_endpoint_id}",
                identity.endpoint_id,
                identity.last_endpoint_id
            );
        } else {
            debug!(
                "BWU: validated CLIENT_INTRODUCTION for endpoint {}",
                identity.endpoint_id
            );
        }
        info!(
            "BWU timing: CLIENT_INTRODUCTION validated at {} ms",
            bwu_started.elapsed().as_millis()
        );

        let ack = Self::client_introduction_ack_frame().encode_to_vec();
        send_plain_frame_on(&mut tcp, &ack).await?;

        self.encrypt_and_send(&Self::bandwidth_upgrade_frame(
            EventType::LastWriteToPriorChannel,
            None,
        ))
        .await?;

        let drain_started = Instant::now();
        for _ in 0..16 {
            let offline = match tokio::time::timeout(
                Duration::from_secs(5),
                self.read_encrypted_offline_frame(),
            )
            .await
            {
                Ok(Ok(frame)) => frame,
                Ok(Err(error)) => return Err(error),
                Err(_) => {
                    warn!(
                        "BWU timing: prior BLE drain timed out after {} ms ({} ms total)",
                        drain_started.elapsed().as_millis(),
                        bwu_started.elapsed().as_millis()
                    );
                    break;
                }
            };

            let Some(v1) = offline.v1.as_ref() else {
                return Err(anyhow!("BWU drain frame has no v1 payload"));
            };

            if v1.r#type() != FrameType::BandwidthUpgradeNegotiation {
                self.process_offline_frame(offline).await?;
                continue;
            }

            let event = v1
                .bandwidth_upgrade_negotiation
                .as_ref()
                .map(|frame| frame.event_type());

            match event {
                Some(EventType::LastWriteToPriorChannel) => {
                    debug!("BWU: peer sent LAST_WRITE; replying SAFE_TO_CLOSE");
                    self.encrypt_and_send(&Self::bandwidth_upgrade_frame(
                        EventType::SafeToClosePriorChannel,
                        None,
                    ))
                    .await?;
                }
                Some(EventType::SafeToClosePriorChannel) => {
                    debug!(
                        "BWU timing: peer marked prior channel safe to close after {} ms drain ({} ms total)",
                        drain_started.elapsed().as_millis(),
                        bwu_started.elapsed().as_millis()
                    );
                    break;
                }
                Some(other) => {
                    debug!("BWU: ignoring drain event {other:?}");
                }
                None => {
                    return Err(anyhow!("BWU negotiation frame has no event type"));
                }
            }
        }

        let disconnection = OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(FrameType::Disconnection.into()),
                disconnection: Some(location_nearby_connections::DisconnectionFrame {
                    request_safe_to_disconnect: Some(false),
                    ack_safe_to_disconnect: Some(false),
                }),
                ..Default::default()
            }),
        };

        // This final prior-channel DISCONNECTION is plaintext by protocol design.
        self.send_frame(disconnection.encode_to_vec()).await?;
        tokio::time::sleep(Duration::from_millis(200)).await;

        self.socket = crate::hdl::MigratableStream::Tcp(tcp);
        info!(
            "BWU timing: migrated inbound session from BLE to Wi-Fi LAN in {} ms total",
            bwu_started.elapsed().as_millis()
        );
        Ok(())
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[tokio::test]
    async fn given_interrupted_frame_header_when_control_is_handled_then_inbound_resumes_exactly() {
        use tokio::io::AsyncWriteExt;

        for native in [false, true] {
            for split in 1..4 {
                // Arrange: a real connection request split inside its length prefix.
                let body = OfflineFrame {
                    v1: Some(crate::location_nearby_connections::V1Frame {
                        r#type: Some(crate::location_nearby_connections::v1_frame::FrameType::ConnectionRequest.into()),
                        connection_request: Some(crate::location_nearby_connections::ConnectionRequestFrame {
                            endpoint_id: Some("TEST".into()),
                            endpoint_info: Some(RemoteDeviceInfo {
                                name: "Fixture phone".into(),
                                device_type: DeviceType::Phone,
                            }.serialize()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                }.encode_to_vec();
                let header = (body.len() as u32).to_be_bytes();
                let (socket, mut peer) = tokio::io::duplex(4096);
                let (sender, _events) = tokio::sync::broadcast::channel(16);
                let mut request =
                    InboundRequest::new(socket, "header-fixture".into(), sender.clone());
                let hub = crate::sessions::SessionHub::new(17).unwrap();
                let mut response = None;
                let control = if native {
                    let mut session = hub
                        .open(
                            crate::channel::TransferType::Inbound,
                            None,
                            tokio_util::sync::CancellationToken::new(),
                        )
                        .unwrap();
                    session.observe(&State::WaitingForUserConsent);
                    let control = session.control();
                    request = request.attach_session(session);
                    Some(control)
                } else {
                    None
                };
                peer.write_all(&header[..split]).await.unwrap();

                // Act: poll until the prefix bytes are consumed, then interrupt it.
                {
                    let handle = request.handle();
                    tokio::pin!(handle);
                    assert!(futures::poll!(handle.as_mut()).is_pending());
                    if let Some(control) = control.as_ref() {
                        // A queued command whose observed consent phase is now stale.
                        response = Some(
                            control
                                .respond(control.snapshot().consent.unwrap(), 1, true)
                                .unwrap(),
                        );
                    } else {
                        sender
                            .send(ChannelMessage {
                                direction: ChannelDirection::LibToFront,
                                ..Default::default()
                            })
                            .unwrap();
                    }
                    handle.await.unwrap();
                }
                if let Some(response) = response {
                    assert_eq!(
                        response.wait().await,
                        Err(crate::sessions::SessionError::Stale)
                    );
                }
                peer.write_all(&header[split..]).await.unwrap();
                peer.write_all(&body).await.unwrap();
                peer.shutdown().await.unwrap();

                // Assert: parse the original frame, with no discarded prefix bytes.
                request.handle().await.unwrap();
                assert_eq!(request.state.state, State::ReceivedConnectionRequest);
                let device = request.state.remote_device_info.as_ref().unwrap();
                assert_eq!(device.name, "Fixture phone");
                assert_eq!(device.device_type, DeviceType::Phone);
            }
        }
    }

    #[tokio::test]
    async fn given_inbound_byte_chunks_when_routed_then_completion_is_exact_and_reservations_are_bounded(
    ) {
        use crate::byte_payload::tests::frame;
        for scenario in ["complete", "short", "budget"] {
            // Arrange: real handler, retained peer and no filesystem side effects.
            let (socket, _peer) = tokio::io::duplex(4096);
            let (sender, _events) = tokio::sync::broadcast::channel(16);
            let mut request = InboundRequest::new(socket, "bytes-fixture".into(), sender);
            request.state.encrypt_key = Some(vec![0x11; 32]);
            request.state.send_hmac_key = Some(vec![0x22; 32]);
            let cancel = sharing_nearby::Frame {
                v1: Some(sharing_nearby::V1Frame {
                    r#type: Some(sharing_nearby::v1_frame::FrameType::Cancel.into()),
                    ..Default::default()
                }),
                ..Default::default()
            }
            .encode_to_vec();
            // Act / Assert
            if scenario == "budget" {
                request
                    .process_offline_frame(frame(1, SANE_FRAME_LENGTH as usize, 0, b"", false))
                    .await
                    .unwrap();
                let error = request
                    .process_offline_frame(frame(2, 1, 0, b"", false))
                    .await
                    .unwrap_err();
                assert_eq!(error.to_string(), "Byte payload reservation limit exceeded");
                assert_eq!(request.state.state, State::Initial);
            } else {
                request
                    .process_offline_frame(frame(1, cancel.len(), 0, &cancel[..1], false))
                    .await
                    .unwrap();
                assert_eq!(request.state.state, State::Initial);
                assert!(!request.state.byte_payloads.is_empty());
                let body = if scenario == "complete" {
                    &cancel[1..]
                } else {
                    &cancel[1..cancel.len() - 1]
                };
                let error = request
                    .process_offline_frame(frame(1, cancel.len(), 1, body, true))
                    .await
                    .unwrap_err();
                if scenario == "complete" {
                    assert!(error.is::<crate::errors::AppError>(), "{error:?}");
                    assert_eq!(request.state.state, State::Cancelled);
                } else {
                    assert_eq!(error.to_string(), "Byte payload is incomplete");
                    assert_eq!(request.state.state, State::Initial);
                }
            }
            assert!(request.state.byte_payloads.is_empty());
            assert!(request.state.transfer_metadata.is_none());
        }
    }

    fn preview_request(
        managed: bool,
        text: bool,
    ) -> (
        TestRequest,
        tokio::io::DuplexStream,
        sharing_nearby::V1Frame,
    ) {
        let mut request = test_request();
        let (socket, peer) = tokio::io::duplex(8192);
        request.socket = socket;
        request.state.encrypt_key = Some(vec![0; 32]);
        request.state.send_hmac_key = Some(vec![0; 32]);
        request.state.remote_device_info = Some(RemoteDeviceInfo {
            name: "phone".into(),
            device_type: crate::utils::DeviceType::Phone,
        });
        request.state.pin_code = Some("1234".into());
        if managed {
            let hub = crate::sessions::SessionHub::new(42).unwrap();
            request.session = Some(
                hub.open(
                    crate::channel::TransferType::Inbound,
                    None,
                    tokio_util::sync::CancellationToken::new(),
                )
                .unwrap(),
            );
        }
        let mut introduction = if text {
            introduction_frame(
                vec![],
                vec![sharing_nearby::TextMetadata {
                    payload_id: Some(101),
                    size: Some(3),
                    r#type: Some(text_metadata::Type::Text.into()),
                    text_title: Some("Text".into()),
                    ..Default::default()
                }],
                vec![],
            )
        } else {
            introduction_frame(vec![file_metadata("example.bin", 101, 3)], vec![], vec![])
        };
        introduction
            .introduction
            .as_mut()
            .unwrap()
            .preview_payload_ids = vec![501, 502];
        (request, peer, introduction)
    }

    #[tokio::test]
    async fn given_announced_preview_bytes_when_fragmented_replayed_or_after_consent_then_no_control_content_or_storage_side_effect_occurs(
    ) {
        use crate::byte_payload::tests::frame;
        let cancel = sharing_nearby::Frame {
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::Cancel.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
        .encode_to_vec();
        for managed in [false, true] {
            for text in [false, true] {
                for body in [
                    b"".as_slice(),
                    b"\x89PNG\r\n\x1a\nopaque",
                    b"\xff\xfe",
                    cancel.as_slice(),
                ] {
                    for separate_final in [false, true] {
                        // Arrange: actual supported Introduction and request-owned receive root.
                        let (mut request, _peer, introduction) = preview_request(managed, text);
                        request.process_introduction(&introduction).await.unwrap();
                        assert_eq!(request.state.state, State::WaitingForUserConsent);
                        assert_eq!(
                            std::fs::read_dir(request._directory.path())
                                .unwrap()
                                .count(),
                            0
                        );
                        // Act: fragmented complete auxiliary payload, including a valid Cancel body.
                        let split = body.len() / 2;
                        request
                            .process_offline_frame(frame(501, body.len(), 0, &body[..split], false))
                            .await
                            .unwrap();
                        assert!(!request.state.byte_payloads.is_empty());
                        request
                            .process_offline_frame(frame(
                                501,
                                body.len(),
                                split,
                                &body[split..],
                                !separate_final,
                            ))
                            .await
                            .unwrap();
                        if separate_final {
                            request
                                .process_offline_frame(frame(
                                    501,
                                    body.len(),
                                    body.len(),
                                    b"",
                                    true,
                                ))
                                .await
                                .unwrap();
                        }
                        // Assert: no implicit acceptance, content completion, progress, file or decoder.
                        assert!(request.state.byte_payloads.is_empty());
                        assert_eq!(request.state.state, State::WaitingForUserConsent);
                        assert_eq!(
                            std::fs::read_dir(request._directory.path())
                                .unwrap()
                                .count(),
                            0
                        );
                        let metadata = request.state.transfer_metadata.as_ref().unwrap();
                        assert_eq!(metadata.total_bytes, 3);
                        assert_eq!(metadata.ack_bytes, 0);
                        assert!(metadata.text_payload.is_none());
                        if let Some(session) = &request.session {
                            let snapshot = session.control().snapshot();
                            assert_eq!(
                                snapshot.details.status,
                                crate::sessions::details::DetailsStatus::Ready
                            );
                            assert!(snapshot.received_text.is_none());
                            assert!(snapshot.received_files.is_none());
                            assert_eq!(snapshot.details.transferred_bytes, 0);
                        }
                        if managed {
                            let control = request.session.as_ref().unwrap().control();
                            let receipt = control
                                .respond(control.snapshot().consent.unwrap(), 1, true)
                                .unwrap();
                            let command = crate::sessions::consent_command(&mut request.session)
                                .await
                                .unwrap();
                            let state = request.state.state.clone();
                            assert_eq!(
                                request.session.as_mut().unwrap().apply(command, &state),
                                Some(true)
                            );
                            receipt.wait().await.unwrap();
                        }
                        request.accept_transfer().await.unwrap();
                        for _ in 0..2 {
                            request
                                .process_offline_frame(frame(501, body.len(), 0, body, true))
                                .await
                                .unwrap();
                        }
                        assert!(request.state.byte_payloads.is_empty());
                        assert_eq!(request.state.state, State::ReceivingFiles);
                        assert_eq!(
                            request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
                            0
                        );
                        assert!(request
                            .state
                            .transfer_metadata
                            .as_ref()
                            .unwrap()
                            .text_payload
                            .is_none());
                        if !text {
                            let file = request.state.transferred_files[&101].file.as_ref().unwrap();
                            assert_eq!(file.metadata().unwrap().len(), 0);
                            assert_eq!(request.state.transferred_files[&101].bytes_transferred, 0);
                        }
                        // An undeclared control payload with the exact same Cancel bytes still works.
                        let error = request
                            .process_offline_frame(frame(999, cancel.len(), 0, &cancel, true))
                            .await
                            .unwrap_err();
                        assert!(error.is::<crate::errors::AppError>());
                        assert_eq!(request.state.state, State::Cancelled);
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn given_bad_preview_announcements_or_unknown_bytes_when_received_then_no_auxiliary_authority_or_consent_is_created(
    ) {
        use crate::byte_payload::tests::frame;
        for managed in [false, true] {
            for case in [
                "collision",
                "count",
                "before",
                "unknown",
                "unsupported",
                "unsupported-text",
            ] {
                // Arrange
                let (mut request, _peer, mut introduction) = preview_request(managed, false);
                // Act / Assert
                match case {
                    "collision" | "count" => {
                        introduction
                            .introduction
                            .as_mut()
                            .unwrap()
                            .preview_payload_ids = if case == "collision" {
                            vec![101]
                        } else {
                            vec![501; 257]
                        };
                        assert!(request.process_introduction(&introduction).await.is_err());
                        assert_eq!(request.state.state, State::Initial);
                        assert!(request.state.transfer_metadata.is_none());
                    }
                    "unsupported" | "unsupported-text" => {
                        if case == "unsupported-text" {
                            introduction
                                .introduction
                                .as_mut()
                                .unwrap()
                                .text_metadata
                                .push(sharing_nearby::TextMetadata {
                                    payload_id: Some(101),
                                    size: Some(3),
                                    r#type: Some(text_metadata::Type::Unknown.into()),
                                    ..Default::default()
                                });
                        }
                        introduction
                            .introduction
                            .as_mut()
                            .unwrap()
                            .file_metadata
                            .clear();
                        assert_eq!(
                            request.process_introduction(&introduction).await.is_err(),
                            managed && case == "unsupported-text"
                        );
                        assert!(request.state.transfer_metadata.is_none());
                    }
                    _ => {
                        if case == "unknown" {
                            request.process_introduction(&introduction).await.unwrap();
                        }
                        let id = if case == "before" { 501 } else { 999 };
                        assert!(request
                            .process_offline_frame(frame(id, 1, 0, &[255], true))
                            .await
                            .is_err());
                        assert_eq!(
                            request.state.state,
                            if case == "before" {
                                State::Initial
                            } else {
                                State::WaitingForUserConsent
                            }
                        );
                    }
                }
                assert!(!request.previews.contains(101));
                if case != "unknown" {
                    assert!(!request.previews.contains(501));
                }
                assert!(request.state.byte_payloads.is_empty());
                assert_eq!(
                    std::fs::read_dir(request._directory.path())
                        .unwrap()
                        .count(),
                    0
                );
            }
        }
    }

    #[tokio::test]
    async fn given_announced_previews_when_framing_or_shared_budgets_are_invalid_then_request_fails_and_pending_ownership_is_released(
    ) {
        use crate::byte_payload::tests::frame;
        for case in [
            "offset",
            "size-change",
            "short",
            "overrun",
            "budget",
            "count",
            "oversize",
            "missing-id",
            "missing-size",
            "missing-offset",
        ] {
            // Arrange
            let (mut request, _peer, mut introduction) = preview_request(true, true);
            introduction
                .introduction
                .as_mut()
                .unwrap()
                .preview_payload_ids = (0..=17)
                .map(|id| if id == 0 { 0 } else { 500 + id })
                .collect();
            request.process_introduction(&introduction).await.unwrap();
            let invalid = match case {
                "budget" => {
                    request
                        .process_offline_frame(frame(
                            501,
                            SANE_FRAME_LENGTH as usize,
                            0,
                            b"",
                            false,
                        ))
                        .await
                        .unwrap();
                    frame(502, 1, 0, b"", false)
                }
                "count" => {
                    for id in 501..517 {
                        request
                            .process_offline_frame(frame(id, 0, 0, b"", false))
                            .await
                            .unwrap();
                    }
                    frame(517, 0, 0, b"", false)
                }
                "oversize" => frame(501, SANE_FRAME_LENGTH as usize + 1, 0, b"", false),
                "missing-id" | "missing-size" | "missing-offset" => {
                    let mut packet =
                        frame(if case == "missing-id" { 0 } else { 501 }, 1, 0, b"a", true);
                    let payload = packet
                        .v1
                        .as_mut()
                        .unwrap()
                        .payload_transfer
                        .as_mut()
                        .unwrap();
                    match case {
                        "missing-id" => payload.payload_header.as_mut().unwrap().id = None,
                        "missing-size" => {
                            payload.payload_header.as_mut().unwrap().total_size = None
                        }
                        _ => payload.payload_chunk.as_mut().unwrap().offset = None,
                    }
                    packet
                }
                _ => {
                    request
                        .process_offline_frame(frame(501, 3, 0, b"a", false))
                        .await
                        .unwrap();
                    match case {
                        "offset" => frame(501, 3, 0, b"bc", true),
                        "size-change" => frame(501, 4, 1, b"bcd", true),
                        "short" => frame(501, 3, 1, b"b", true),
                        _ => frame(501, 3, 1, b"bcd", true),
                    }
                }
            };
            // Act / Assert
            assert!(
                request.process_offline_frame(invalid).await.is_err(),
                "{case}"
            );
            assert!(request.state.byte_payloads.is_empty(), "{case}");
            assert_eq!(request.state.state, State::WaitingForUserConsent);
            assert_eq!(
                request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
                0
            );
            assert!(request
                .state
                .transfer_metadata
                .as_ref()
                .unwrap()
                .text_payload
                .is_none());
            assert_eq!(
                std::fs::read_dir(request._directory.path())
                    .unwrap()
                    .count(),
                0
            );
        }
    }

    #[tokio::test]
    async fn given_paired_key_proofs_and_remote_results_when_receiving_then_actual_crypto_reply_precedes_any_consent(
    ) {
        use crate::peer_authentication::{tests::known_fixture, Role};
        for kind in ["anonymous", "known", "bad-signature", "oversize"] {
            for status in [None, Some(0), Some(1), Some(2), Some(3), Some(4)] {
                // Arrange: real handler/framing/encryption over a controlled external stream.
                let (socket, peer_socket) = tokio::io::duplex(8192);
                let (sender, _) = tokio::sync::broadcast::channel(16);
                let mut request =
                    InboundRequest::new(socket, "auth-fixture".into(), sender.clone());
                let mut peer = InboundRequest::new(peer_socket, "peer-fixture".into(), sender);
                request.state.state = State::SentConnectionResponse;
                request.state.encrypt_key = Some(vec![0x11; 32]);
                request.state.send_hmac_key = Some(vec![0x22; 32]);
                peer.state.decrypt_key = Some(vec![0x11; 32]);
                peer.state.recv_hmac_key = Some(vec![0x22; 32]);
                let (authentication, mut proof) = known_fixture(Role::Receiver);
                if kind != "anonymous" {
                    request.authentication = authentication;
                }
                if kind == "bad-signature" {
                    proof.signed_data = Some(vec![0; 72]);
                }
                if kind == "oversize" {
                    proof.signed_data = Some(vec![0; 73]);
                }
                let frame = sharing_nearby::Frame {
                    v1: Some(sharing_nearby::V1Frame {
                        paired_key_encryption: Some(proof),
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                // Act / Assert
                let accepted = request.process_transfer_setup(&frame).await;
                let rejected_proof = matches!(kind, "bad-signature" | "oversize");
                assert_eq!(accepted.is_err(), rejected_proof);
                let raw = read_plain_frame_from(&mut peer.socket).await.unwrap();
                let secure = SecureMessage::decode(raw.as_slice()).unwrap();
                let offline = peer.decrypt_secure_message(&secure).await.unwrap();
                let body = offline
                    .v1
                    .unwrap()
                    .payload_transfer
                    .unwrap()
                    .payload_chunk
                    .unwrap()
                    .body
                    .unwrap();
                let reply = sharing_nearby::Frame::decode(body.as_slice()).unwrap();
                assert_eq!(
                    reply.v1.unwrap().paired_key_result.unwrap().status,
                    Some(if rejected_proof {
                        2
                    } else if kind == "known" {
                        1
                    } else {
                        3
                    })
                );
                let raw = read_plain_frame_from(&mut peer.socket).await.unwrap();
                let secure = SecureMessage::decode(raw.as_slice()).unwrap();
                let final_chunk = peer.decrypt_secure_message(&secure).await.unwrap();
                assert_eq!(
                    final_chunk
                        .v1
                        .unwrap()
                        .payload_transfer
                        .unwrap()
                        .payload_chunk
                        .unwrap()
                        .flags,
                    Some(1)
                );
                if rejected_proof {
                    assert_eq!(request.state.state, State::SentConnectionResponse);
                } else {
                    assert_eq!(request.state.state, State::SentPairedKeyResult);
                    let result = sharing_nearby::Frame {
                        v1: Some(sharing_nearby::V1Frame {
                            paired_key_result: Some(sharing_nearby::PairedKeyResultFrame {
                                status,
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    };
                    assert_eq!(
                        request.process_transfer_setup(&result).await.is_err(),
                        status == Some(2)
                    );
                    assert_eq!(
                        request.state.state,
                        if status == Some(2) {
                            State::SentPairedKeyResult
                        } else {
                            State::ReceivedPairedKeyResult
                        }
                    );
                }
                assert!(request.state.transferred_files.is_empty());
                assert!(request.receive_root.is_none());
                assert!(request.state.transfer_metadata.is_none());
                let mut next = [0];
                let read = tokio::io::AsyncReadExt::read(&mut peer.socket, &mut next);
                tokio::pin!(read);
                assert!(futures::poll!(read).is_pending());
            }
        }
    }

    #[test]
    fn received_file_name_accepts_single_component() {
        assert!(validate_received_file_name("photo 01.jpg").is_ok());
        assert!(validate_received_file_name("данные.txt").is_ok());
    }

    #[test]
    fn received_file_name_rejects_path_traversal_and_absolute_paths() {
        for value in [
            "../secret",
            "../../secret",
            "/tmp/secret",
            "folder/file.txt",
            "folder\\file.txt",
            ".",
            "..",
        ] {
            assert!(
                validate_received_file_name(value).is_err(),
                "unsafe name unexpectedly accepted: {value}"
            );
        }
    }

    #[test]
    fn received_file_name_rejects_control_chars_and_oversize_components() {
        assert!(validate_received_file_name("evil\nname.txt").is_err());
        assert!(validate_received_file_name(&"a".repeat(MAX_RECEIVED_FILENAME_BYTES + 1)).is_err());
    }

    #[test]
    fn received_file_name_enforces_byte_length_boundary() {
        let exact_ascii = "a".repeat(MAX_RECEIVED_FILENAME_BYTES);
        let exact_multibyte = format!("{}a", "é".repeat(127));
        let oversized_multibyte = "é".repeat(128);

        assert_eq!(exact_ascii.len(), MAX_RECEIVED_FILENAME_BYTES);
        assert_eq!(exact_multibyte.len(), MAX_RECEIVED_FILENAME_BYTES);
        assert!(validate_received_file_name(&exact_ascii).is_ok());
        assert!(validate_received_file_name(&exact_multibyte).is_ok());
        assert!(validate_received_file_name(&oversized_multibyte).is_err());
    }

    #[test]
    fn test_parse_wifi_password_payload_handles_16_byte_password() {
        let password = b"1234567890abcdef";
        let mut payload = vec![0x0A, password.len() as u8];
        payload.extend_from_slice(password);
        payload.extend_from_slice(&[0x10, 0x01]);

        assert_eq!(
            parse_wifi_password_payload(&payload).unwrap(),
            "1234567890abcdef"
        );
    }

    #[test]
    fn test_parse_wifi_password_payload_rejects_malformed_frames() {
        assert!(parse_wifi_password_payload(&[]).is_err());
        assert!(parse_wifi_password_payload(&[0x09, 0, 0x10, 0]).is_err());
        assert!(parse_wifi_password_payload(&[0x0A, 5, b'a', 0x10, 0]).is_err());
        assert!(parse_wifi_password_payload(&[0x0A, 1, b'a', 0x11, 0]).is_err());
        assert!(parse_wifi_password_payload(&[0x0A, 1, 0xff, 0x10, 0]).is_err());
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    #[tokio::test]
    async fn plaintext_bwu_frame_round_trip() {
        let (mut writer, mut reader) = tokio::io::duplex(256);
        let payload = b"bandwidth-upgrade";

        send_plain_frame_on(&mut writer, payload).await.unwrap();
        let received = read_plain_frame_from(&mut reader).await.unwrap();

        assert_eq!(received, payload);
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    #[tokio::test]
    async fn plaintext_bwu_frame_rejects_zero_length() {
        let (mut writer, mut reader) = tokio::io::duplex(16);
        writer.write_all(&0_u32.to_be_bytes()).await.unwrap();
        writer.flush().await.unwrap();

        assert!(read_plain_frame_from(&mut reader).await.is_err());
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    #[tokio::test]
    async fn plaintext_bwu_frame_rejects_oversized_declared_length() {
        let (mut writer, mut reader) = tokio::io::duplex(16);
        writer
            .write_all(&((SANE_FRAME_LENGTH as u32) + 1).to_be_bytes())
            .await
            .unwrap();
        writer.flush().await.unwrap();

        assert!(read_plain_frame_from(&mut reader).await.is_err());
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    #[tokio::test]
    async fn plaintext_bwu_send_rejects_empty_payload() {
        let (mut writer, _reader) = tokio::io::duplex(16);

        assert!(send_plain_frame_on(&mut writer, &[]).await.is_err());
    }

    #[cfg(target_os = "linux")]
    mod receive_reliability {
        include!("inbound/receive_reliability.rs");
    }

    struct TestRequest {
        request: InboundRequest<tokio::io::DuplexStream>,
        _directory: tempfile::TempDir,
    }
    impl std::ops::Deref for TestRequest {
        type Target = InboundRequest<tokio::io::DuplexStream>;
        fn deref(&self) -> &Self::Target {
            &self.request
        }
    }
    impl std::ops::DerefMut for TestRequest {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.request
        }
    }
    fn test_request() -> TestRequest {
        let directory = tempfile::tempdir().unwrap();
        let policy = crate::receive::ReceivePolicy::new(Some(directory.path().to_owned()));
        let (socket, _peer) = tokio::io::duplex(4096);
        let (sender, _receiver) = tokio::sync::broadcast::channel(16);
        TestRequest {
            request: InboundRequest::new(socket, "test-transfer".to_owned(), sender)
                .with_receive_directory(policy.snapshot()),
            _directory: directory,
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn given_admitted_file_request_when_policy_or_root_name_changes_then_accept_uses_original_directory_authority(
    ) {
        use std::os::unix::fs::symlink;
        for (rename, non_utf8) in [(false, false), (true, false), (false, true)] {
            // Arrange
            let base = tempfile::tempdir().unwrap();
            use std::os::unix::ffi::OsStringExt;
            let selected = base.path().join(if non_utf8 {
                std::ffi::OsString::from_vec(vec![b's', 0xff])
            } else {
                "selected".into()
            });
            let other = base.path().join("other");
            let moved = base.path().join("moved");
            std::fs::create_dir(&selected).unwrap();
            std::fs::create_dir(&other).unwrap();
            let policy = crate::receive::ReceivePolicy::new(Some(selected.clone()));
            let (socket, _peer) = tokio::io::duplex(8192);
            let (sender, _events) = tokio::sync::broadcast::channel(16);
            let mut request = InboundRequest::new(socket, "pinned-request".into(), sender)
                .with_receive_directory(policy.snapshot());
            request.state.encrypt_key = Some(vec![0; 32]);
            request.state.send_hmac_key = Some(vec![0; 32]);
            // Act: even a policy update before Introduction cannot change admission.
            policy.set(Some(other.clone()));
            request
                .process_introduction(&introduction_frame(
                    vec![file_metadata_with_parent("file.bin", 101, 3, "Trip/nested")],
                    vec![],
                    vec![],
                ))
                .await
                .unwrap();
            // Assert: planning does not create any received files or directories.
            assert!(std::fs::read_dir(&selected).unwrap().next().is_none());
            assert!(std::fs::read_dir(&other).unwrap().next().is_none());
            assert_eq!(
                request
                    .state
                    .transfer_metadata
                    .as_ref()
                    .unwrap()
                    .destination
                    .as_deref(),
                Some(selected.to_string_lossy().as_ref())
            );
            assert_eq!(
                request
                    .state
                    .transfer_metadata
                    .as_ref()
                    .unwrap()
                    .files
                    .as_ref()
                    .unwrap(),
                &vec!["Trip/nested/file.bin".to_owned()]
            );
            if rename {
                std::fs::rename(&selected, &moved).unwrap();
                symlink(&other, &selected).unwrap();
            }
            request.accept_transfer().await.unwrap();
            let target = if rename { &moved } else { &selected };
            assert!(!target.join("Trip/nested/file.bin").exists());
            assert!(std::fs::read_dir(&other).unwrap().next().is_none());
            assert!(request.state.transferred_files[&101].file.is_some());
            assert_eq!(request.state.state, State::ReceivingFiles);
            request
                .process_offline_frame(received_file_frame(101, 3, 0, b"abc", 0))
                .await
                .unwrap();
            assert!(!target.join("Trip/nested/file.bin").exists());
            let completed = request
                .process_offline_frame(received_file_frame(101, 3, 3, &[], 1))
                .await
                .unwrap_err();
            assert!(completed.is::<crate::errors::AppError>());
            assert_eq!(request.state.state, State::Finished);
            assert_eq!(
                std::fs::read(target.join("Trip/nested/file.bin")).unwrap(),
                b"abc"
            );
            assert!(std::fs::read_dir(&other).unwrap().next().is_none());
        }
    }

    #[test]
    fn bandwidth_upgrade_flags_are_explicit_and_one_shot() {
        let mut request = test_request();

        assert!(!request.bandwidth_upgrade_enabled);
        assert!(!request.take_bwu_pending());

        request.enable_bandwidth_upgrade();
        request.bwu_pending = true;

        assert!(request.bandwidth_upgrade_enabled);
        assert!(request.take_bwu_pending());
        assert!(!request.take_bwu_pending());
    }

    fn introduction_frame(
        files: Vec<sharing_nearby::FileMetadata>,
        text: Vec<sharing_nearby::TextMetadata>,
        wifi: Vec<sharing_nearby::WifiCredentialsMetadata>,
    ) -> sharing_nearby::V1Frame {
        sharing_nearby::V1Frame {
            r#type: Some(sharing_nearby::v1_frame::FrameType::Introduction.into()),
            introduction: Some(sharing_nearby::IntroductionFrame {
                file_metadata: files,
                text_metadata: text,
                wifi_credentials_metadata: wifi,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn file_metadata(name: &str, payload_id: i64, size: i64) -> sharing_nearby::FileMetadata {
        file_metadata_with_parent(name, payload_id, size, "")
    }

    fn file_metadata_with_parent(
        name: &str,
        payload_id: i64,
        size: i64,
        parent_folder: &str,
    ) -> sharing_nearby::FileMetadata {
        sharing_nearby::FileMetadata {
            name: Some(name.to_owned()),
            payload_id: Some(payload_id),
            size: Some(size),
            parent_folder: (!parent_folder.is_empty()).then(|| parent_folder.to_owned()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn introduction_prepares_duplicate_file_names_with_unique_destinations() {
        let mut request = test_request();
        let frame = introduction_frame(
            vec![
                file_metadata("mode-b-duplicate.bin", 101, 5),
                file_metadata("mode-b-duplicate.bin", 102, 7),
            ],
            vec![],
            vec![],
        );

        request.process_introduction(&frame).await.unwrap();

        assert_eq!(request.state.state, State::WaitingForUserConsent);
        assert_eq!(request.state.transferred_files.len(), 2);

        let first = &request.state.transferred_files[&101].file_url;
        let second = &request.state.transferred_files[&102].file_url;
        assert_ne!(first, second);
        assert_eq!(first.parent(), second.parent());

        let metadata = request.state.transfer_metadata.as_ref().unwrap();
        assert_eq!(metadata.total_bytes, 12);
        assert_eq!(
            metadata.files.as_ref().unwrap(),
            &vec![
                "mode-b-duplicate.bin".to_owned(),
                "1_mode-b-duplicate.bin".to_owned()
            ]
        );
    }

    #[tokio::test]
    async fn given_managed_introduction_when_files_are_prepared_and_retired_then_ordered_details_keep_complete_manifest_and_progress(
    ) {
        // Arrange
        let mut request = test_request();
        request.state.remote_device_info = Some(RemoteDeviceInfo {
            name: "phone".into(),
            device_type: crate::utils::DeviceType::Phone,
        });
        request.state.pin_code = Some("0123".into());
        let hub = crate::sessions::SessionHub::new(123).unwrap();
        let mut events = hub.subscribe_fresh().unwrap();
        let session = hub
            .open(
                crate::channel::TransferType::Inbound,
                None,
                tokio_util::sync::CancellationToken::new(),
            )
            .unwrap();
        let control = session.control();
        request.session = Some(session);
        let frame = introduction_frame(
            vec![
                file_metadata("native-complete-a.bin", 101, 5),
                file_metadata("native-complete-b.bin", 102, 7),
            ],
            vec![],
            vec![],
        );
        // Act
        request.process_introduction(&frame).await.unwrap();
        // Assert
        let ready = control.snapshot();
        assert_eq!(
            ready.details.status,
            crate::sessions::details::DetailsStatus::Ready
        );
        assert_eq!(ready.details.pin, "0123");
        assert_eq!(ready.details.files.len(), 2);
        assert!(matches!(
            events.try_recv().unwrap(),
            crate::sessions::SessionEvent::Opened(_)
        ));
        let crate::sessions::SessionEvent::Changed(waiting) = events.try_recv().unwrap() else {
            panic!("Missing waiting state")
        };
        let crate::sessions::SessionEvent::Changed(described) = events.try_recv().unwrap() else {
            panic!("Missing ready details")
        };
        assert_eq!(
            waiting.details.status,
            crate::sessions::details::DetailsStatus::Unavailable
        );
        assert_eq!(waiting.consent, described.consent);
        request
            .update_state(
                |state| {
                    state.state = State::ReceivingFiles;
                    state.transferred_files.remove(&101);
                    state.transfer_metadata.as_mut().unwrap().ack_bytes = 5;
                },
                true,
            )
            .await;
        let progress = control.snapshot();
        assert_eq!(progress.details.files, ready.details.files);
        assert!(std::sync::Arc::ptr_eq(
            &progress.details.files,
            &ready.details.files
        ));
        assert_eq!(progress.details.transferred_bytes, 5);
        assert!(progress.details.pin.is_empty());
        request
            .update_state(
                |state| {
                    state.transferred_files.clear();
                    state.state = State::Finished;
                    state.transfer_metadata.as_mut().unwrap().ack_bytes = 12;
                },
                true,
            )
            .await;
        let finished = control.snapshot();
        assert_eq!(finished.details.files, ready.details.files);
        assert_eq!(
            (
                finished.details.total_bytes,
                finished.details.transferred_bytes
            ),
            (12, 12)
        );
    }

    #[test]
    fn parent_folder_normalization_accepts_both_separators_and_rejects_traversal() {
        assert_eq!(
            normalize_received_parent_folder("Trip/photos").unwrap(),
            vec!["Trip".to_owned(), "photos".to_owned()]
        );
        assert_eq!(
            normalize_received_parent_folder("Trip\\photos").unwrap(),
            vec!["Trip".to_owned(), "photos".to_owned()]
        );

        for invalid in [
            "/absolute",
            "C:\\Windows\\Temp",
            "D:/Downloads",
            "Trip//photos",
            "Trip/../secret",
            "Trip/./photos",
            "Trip/evil\nname",
        ] {
            assert!(
                normalize_received_parent_folder(invalid).is_err(),
                "unsafe parent folder unexpectedly accepted: {invalid:?}"
            );
        }
    }

    #[test]
    fn inbound_folder_preparation_preserves_tree_without_creating_directories() {
        let base = std::env::temp_dir().join(format!(
            "rquickshare-inbound-folder-test-{}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&base).unwrap();

        let files = vec![
            file_metadata_with_parent("readme.txt", 501, 1, "Trip"),
            file_metadata_with_parent("sunset.jpg", 502, 2, "Trip/photos"),
        ];
        let prepared = prepare_inbound_files_at(&base, &files, &HashSet::new()).unwrap();

        assert_eq!(
            prepared.names,
            vec![
                "Trip/readme.txt".to_owned(),
                "Trip/photos/sunset.jpg".to_owned()
            ]
        );
        assert_eq!(prepared.files[0].1.parent_folder.as_deref(), Some("Trip"));
        assert_eq!(
            prepared.files[1].1.parent_folder.as_deref(),
            Some("Trip/photos")
        );
        assert!(!base.join("Trip").exists());

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn inbound_folder_root_collision_is_remapped_consistently() {
        let base = std::env::temp_dir().join(format!(
            "rquickshare-inbound-root-collision-test-{}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(base.join("Trip")).unwrap();

        let files = vec![
            file_metadata_with_parent("one.txt", 601, 1, "Trip"),
            file_metadata_with_parent("two.txt", 602, 1, "Trip/nested"),
        ];
        let prepared = prepare_inbound_files_at(&base, &files, &HashSet::new()).unwrap();

        assert_eq!(
            prepared.names,
            vec![
                "1_Trip/one.txt".to_owned(),
                "1_Trip/nested/two.txt".to_owned()
            ]
        );
        assert!(!base.join("1_Trip").exists());

        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn safe_destination_materialization_rejects_symlink_parent() {
        use std::os::unix::fs::symlink;

        let base = std::env::temp_dir().join(format!(
            "rquickshare-inbound-materialize-test-{}",
            rand::random::<u64>()
        ));
        let outside = std::env::temp_dir().join(format!(
            "rquickshare-inbound-outside-test-{}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, base.join("Trip")).unwrap();

        let destination = base.join("Trip").join("secret.txt");
        assert!(materialize_safe_destination_at(&base, &destination).is_err());

        std::fs::remove_dir_all(base).unwrap();
        std::fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn safe_destination_materialization_creates_parent_tree_only_on_accept_path() {
        let base = std::env::temp_dir().join(format!(
            "rquickshare-inbound-materialize-create-test-{}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let destination = base.join("Trip").join("photos").join("sunset.jpg");

        let safe = materialize_safe_destination_at(&base, &destination).unwrap();

        assert_eq!(safe.file_name().unwrap(), "sunset.jpg");
        assert!(base.join("Trip").join("photos").is_dir());

        std::fs::remove_dir_all(base).unwrap();
    }

    #[tokio::test]
    async fn malformed_file_introduction_is_transactional() {
        let mut duplicate = test_request();
        let duplicate_frame = introduction_frame(
            vec![
                file_metadata("first.bin", 201, 1),
                file_metadata("second.bin", 201, 1),
            ],
            vec![],
            vec![],
        );

        assert!(duplicate
            .process_introduction(&duplicate_frame)
            .await
            .is_err());
        assert!(duplicate.state.transferred_files.is_empty());
        assert!(duplicate.state.transfer_metadata.is_none());

        let mut negative = test_request();
        let negative_frame =
            introduction_frame(vec![file_metadata("negative.bin", 202, -1)], vec![], vec![]);

        assert!(negative
            .process_introduction(&negative_frame)
            .await
            .is_err());
        assert!(negative.state.transferred_files.is_empty());
        assert!(negative.state.transfer_metadata.is_none());
    }

    fn received_file_frame(
        id: i64,
        size: i64,
        offset: i64,
        body: &[u8],
        flags: i32,
    ) -> OfflineFrame {
        OfflineFrame {
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
                ),
                payload_transfer: Some(PayloadTransferFrame {
                    packet_type: Some(PacketType::Data.into()),
                    payload_header: Some(PayloadHeader {
                        id: Some(id),
                        r#type: Some(payload_header::PayloadType::File.into()),
                        total_size: Some(size),
                        ..Default::default()
                    }),
                    payload_chunk: Some(PayloadChunk {
                        offset: Some(offset),
                        flags: Some(flags),
                        body: Some(body.to_vec()),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn given_inbound_file_end_when_content_is_short_or_destination_missing_then_success_is_rejected_without_retiring_file(
    ) {
        // Arrange / Act / Assert: actual frame handler and destination writes, both request modes.
        for managed in [false, true] {
            for (size, body, opened, offset, extra, complete) in [
                (0, b"".as_slice(), true, 0, false, true),
                (1, b"".as_slice(), true, 0, false, false),
                (1, b"a".as_slice(), true, 1, false, true),
                (3, b"a".as_slice(), true, 1, false, false),
                (3, b"abc".as_slice(), true, 3, false, true),
                (3, b"abc".as_slice(), true, 2, false, false),
                (0, b"".as_slice(), false, 0, false, false),
                (1, b"a".as_slice(), true, 1, true, true),
            ] {
                let mut request = test_request();
                let (socket, _peer) = tokio::io::duplex(4096);
                request.socket = socket;
                request.state.encrypt_key = Some(vec![0; 32]);
                request.state.send_hmac_key = Some(vec![0; 32]);
                if managed {
                    let hub = crate::sessions::SessionHub::new(42).unwrap();
                    request.session = Some(
                        hub.open(
                            crate::channel::TransferType::Inbound,
                            None,
                            tokio_util::sync::CancellationToken::new(),
                        )
                        .unwrap(),
                    );
                }
                request.state.state = State::ReceivingFiles;
                request.state.transfer_metadata = Some(TransferMetadata {
                    total_bytes: size as u64,
                    ..Default::default()
                });
                let destination = tempfile::NamedTempFile::new().unwrap();
                request.state.transferred_files.insert(
                    101,
                    InternalFileInfo {
                        payload_id: 101,
                        file_url: destination.path().to_owned(),
                        parent_folder: None,
                        bytes_transferred: 0,
                        total_size: size,
                        file: opened.then(|| destination.as_file().try_clone().unwrap()),
                    },
                );
                if extra {
                    request.state.transferred_files.insert(
                        102,
                        InternalFileInfo {
                            payload_id: 102,
                            file_url: destination.path().to_owned(),
                            parent_folder: None,
                            bytes_transferred: 0,
                            total_size: 0,
                            file: Some(tempfile::tempfile().unwrap()),
                        },
                    );
                }
                if !body.is_empty() {
                    request
                        .process_offline_frame(received_file_frame(101, size, 0, body, 0))
                        .await
                        .unwrap();
                }
                let before_ack = request.state.transfer_metadata.as_ref().unwrap().ack_bytes;
                if opened && offset == body.len() as i64 {
                    request
                        .process_offline_frame(received_file_frame(101, size, offset, &[], 0))
                        .await
                        .unwrap();
                    assert!(request.state.transferred_files.contains_key(&101));
                    // Empty nonfinal chunk is not EOF.
                }
                let result = request
                    .process_offline_frame(received_file_frame(101, size, offset, &[], 1))
                    .await;
                assert_eq!(request.state.transferred_files.contains_key(&101), !complete, "size={size}, body={body:?}, opened={opened}, offset={offset}, managed={managed}");
                assert_eq!(
                    request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
                    before_ack
                );
                assert_eq!(std::fs::read(destination.path()).unwrap(), body);
                if complete && extra {
                    assert!(result.is_ok());
                    assert_eq!(request.state.state, State::ReceivingFiles);
                    let last = request
                        .process_offline_frame(received_file_frame(102, 0, 0, &[], 1))
                        .await
                        .unwrap_err();
                    assert!(last.is::<crate::errors::AppError>());
                    assert_eq!(request.state.state, State::Finished);
                } else {
                    let error = result.unwrap_err();
                    assert_eq!(error.is::<crate::errors::AppError>(), complete);
                    assert_eq!(
                        request.state.state,
                        if complete {
                            State::Finished
                        } else {
                            State::ReceivingFiles
                        }
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn given_nonempty_last_file_chunk_when_managed_or_legacy_then_exact_end_is_required() {
        for managed in [false, true] {
            for (size, body, completes, expected_bytes) in [
                (1_i64, b"a".as_slice(), true, 1_u64),
                (3, b"abc".as_slice(), true, 3),
                (3, b"ab".as_slice(), false, 0),
                (3, b"abcd".as_slice(), false, 0),
            ] {
                // Arrange
                let mut request = test_request();
                let (socket, _peer) = tokio::io::duplex(4096);
                request.socket = socket;
                request.state.encrypt_key = Some(vec![0; 32]);
                request.state.send_hmac_key = Some(vec![0; 32]);
                if managed {
                    let hub = crate::sessions::SessionHub::new(42).unwrap();
                    request.session = Some(
                        hub.open(
                            crate::channel::TransferType::Inbound,
                            None,
                            tokio_util::sync::CancellationToken::new(),
                        )
                        .unwrap(),
                    );
                }
                request.state.state = State::ReceivingFiles;
                request.state.transfer_metadata = Some(TransferMetadata {
                    total_bytes: size as u64,
                    ..Default::default()
                });
                let destination = tempfile::NamedTempFile::new().unwrap();
                request.state.transferred_files.insert(
                    101,
                    InternalFileInfo {
                        payload_id: 101,
                        file_url: destination.path().to_owned(),
                        parent_folder: None,
                        bytes_transferred: 0,
                        total_size: size,
                        file: Some(destination.as_file().try_clone().unwrap()),
                    },
                );

                // Act: final flag and body arrive in one valid or invalid frame.
                let result = request
                    .process_offline_frame(received_file_frame(101, size, 0, body, 1))
                    .await;

                // Assert: strict declared bounds and exactly one terminal state.
                assert_eq!(
                    result
                        .as_ref()
                        .err()
                        .is_some_and(|error| error.is::<crate::errors::AppError>()),
                    completes,
                    "managed={managed}, size={size}, body={body:?}"
                );
                assert_eq!(
                    request.state.state,
                    if completes {
                        State::Finished
                    } else {
                        State::ReceivingFiles
                    }
                );
                assert_eq!(
                    request.state.transferred_files.contains_key(&101),
                    !completes
                );
                assert_eq!(
                    request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
                    expected_bytes
                );
                assert_eq!(
                    std::fs::read(destination.path()).unwrap(),
                    if completes { body } else { b"".as_slice() }
                );
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn given_accepted_managed_files_when_completed_out_of_order_then_private_receipt_uses_actual_collision_names_and_survives_session_close(
    ) {
        // Arrange: real consent, actual destination writes, final observation and EOF handling.
        let mut request = test_request();
        let (socket, _peer) = tokio::io::duplex(8192);
        request.socket = socket;
        request.state.encrypt_key = Some(vec![0; 32]);
        request.state.send_hmac_key = Some(vec![0; 32]);
        request.state.remote_device_info = Some(RemoteDeviceInfo {
            name: "phone".into(),
            device_type: crate::utils::DeviceType::Phone,
        });
        request.state.pin_code = Some("0123".into());
        let hub = crate::sessions::SessionHub::new(42).unwrap();
        let session = hub
            .open(
                crate::channel::TransferType::Inbound,
                None,
                tokio_util::sync::CancellationToken::new(),
            )
            .unwrap();
        let control = session.control();
        request.session = Some(session);
        request
            .process_introduction(&introduction_frame(
                vec![
                    file_metadata("same.txt", 101, 1),
                    file_metadata("same.txt", 102, 2),
                ],
                vec![],
                vec![],
            ))
            .await
            .unwrap();
        assert_eq!(
            control.snapshot().details.status,
            crate::sessions::details::DetailsStatus::Ready
        );
        let consent = control.snapshot().consent.unwrap();
        let receipt = control.respond(consent, 1, true).unwrap();
        let command = crate::sessions::consent_command(&mut request.session)
            .await
            .unwrap();
        let state = request.state.state.clone();
        assert_eq!(
            request.session.as_mut().unwrap().apply(command, &state),
            Some(true)
        );
        receipt.wait().await.unwrap();
        request.accept_transfer().await.unwrap();
        // Act
        request
            .process_offline_frame(received_file_frame(102, 2, 0, b"bc", 0))
            .await
            .unwrap();
        request
            .process_offline_frame(received_file_frame(102, 2, 2, &[], 1))
            .await
            .unwrap();
        assert!(control.snapshot().received_files.is_none());
        request
            .process_offline_frame(received_file_frame(101, 1, 0, b"a", 0))
            .await
            .unwrap();
        let error = request
            .process_offline_frame(received_file_frame(101, 1, 1, &[], 1))
            .await
            .unwrap_err();
        // Assert
        assert!(error.is::<crate::errors::AppError>());
        let snapshot = control.snapshot();
        assert_eq!(snapshot.state, State::Finished);
        let files = snapshot.received_files.as_ref().unwrap();
        assert!(files.valid_for(&snapshot.details));
        assert_eq!(files.len(), 2);
        assert_ne!(
            snapshot.details.files[0].name,
            snapshot.details.files[1].name
        );
        drop(request.session.take());
        use std::io::Read;
        for (index, expected) in [(0, b"bc".as_slice()), (1, b"a".as_slice())] {
            let mut body = Vec::new();
            files.open(index).unwrap().read_to_end(&mut body).unwrap();
            assert_eq!(body, expected);
        }
    }

    #[tokio::test]
    async fn given_managed_text_chunks_when_consent_size_offsets_or_body_are_invalid_then_completion_is_rejected_before_ack(
    ) {
        // Arrange / Act / Assert
        for (receiving, declared, offset, body, expected) in [
            (false, Some(3), 0, b"abc".as_slice(), false),
            (true, None, 0, b"abc".as_slice(), false),
            (true, Some(4), 0, b"abc".as_slice(), false),
            (true, Some(3), 1, b"abc".as_slice(), false),
            (true, Some(3), 0, b"ab".as_slice(), false),
            (true, Some(3), 0, b"abcd".as_slice(), false),
            (true, Some(3), 0, b"a\0c".as_slice(), false),
            (true, Some(3), 0, &[255, 255, 255], false),
            (true, Some(3), 0, b"abc".as_slice(), true),
        ] {
            let mut request = test_request();
            let (socket, _peer) = tokio::io::duplex(4096);
            request.socket = socket;
            request.state.encrypt_key = Some(vec![0; 32]);
            request.state.send_hmac_key = Some(vec![0; 32]);
            let hub = crate::sessions::SessionHub::new(42).unwrap();
            request.session = Some(
                hub.open(
                    crate::channel::TransferType::Inbound,
                    None,
                    tokio_util::sync::CancellationToken::new(),
                )
                .unwrap(),
            );
            request.state.state = if receiving {
                State::ReceivingFiles
            } else {
                State::WaitingForUserConsent
            };
            request.state.text_payload = Some(TextPayloadInfo::Text(1));
            request.state.transfer_metadata = Some(TransferMetadata {
                total_bytes: 3,
                text_type: Some(TextPayloadType::Text),
                ..Default::default()
            });
            let frame = OfflineFrame {
                v1: Some(location_nearby_connections::V1Frame {
                    r#type: Some(
                        location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
                    ),
                    payload_transfer: Some(PayloadTransferFrame {
                        packet_type: Some(PacketType::Data.into()),
                        payload_header: Some(PayloadHeader {
                            id: Some(1),
                            r#type: Some(payload_header::PayloadType::Bytes.into()),
                            total_size: declared,
                            ..Default::default()
                        }),
                        payload_chunk: Some(PayloadChunk {
                            offset: Some(offset),
                            flags: Some(1),
                            body: Some(body.to_vec()),
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let result = request.process_offline_frame(frame).await;
            assert!(result.is_err()); // Successful completion returns the protocol stop marker.
            assert_eq!(request.state.state == State::Finished, expected);
            assert_eq!(
                request
                    .state
                    .transfer_metadata
                    .as_ref()
                    .unwrap()
                    .text_payload
                    .is_some(),
                expected
            );
            if expected {
                let error = result.unwrap_err();
                assert!(error.is::<crate::errors::AppError>(), "{error:?}");
            }
            assert!(request.state.byte_payloads.is_empty());
        }
    }

    #[tokio::test]
    async fn given_managed_text_introduction_when_metadata_is_invalid_then_no_consent_is_exposed() {
        // Arrange / Act / Assert
        for (size, id, ty, count, mixed, expected) in [
            (Some(0), Some(1), Some(1), 1, false, true),
            (Some(131072), Some(1), Some(1), 1, false, true),
            (Some(131073), Some(1), Some(1), 1, false, false),
            (Some(-1), Some(1), Some(1), 1, false, false),
            (None, Some(1), Some(1), 1, false, false),
            (Some(1), None, Some(1), 1, false, false),
            (Some(1), Some(1), None, 1, false, false),
            (Some(1), Some(1), Some(999), 1, false, false),
            (Some(1), Some(1), Some(1), 2, false, false),
            (Some(1), Some(1), Some(1), 1, true, false),
        ] {
            let mut request = test_request();
            let hub = crate::sessions::SessionHub::new(42).unwrap();
            request.session = Some(
                hub.open(
                    crate::channel::TransferType::Inbound,
                    None,
                    tokio_util::sync::CancellationToken::new(),
                )
                .unwrap(),
            );
            request.state.remote_device_info = Some(RemoteDeviceInfo {
                name: "phone".into(),
                device_type: crate::utils::DeviceType::Phone,
            });
            request.state.pin_code = Some("0123".into());
            let text = sharing_nearby::TextMetadata {
                payload_id: id,
                size,
                r#type: ty,
                ..Default::default()
            };
            let frame = introduction_frame(
                if mixed {
                    vec![file_metadata("mixed", 2, 1)]
                } else {
                    vec![]
                },
                vec![text; count],
                vec![],
            );
            assert_eq!(request.process_introduction(&frame).await.is_ok(), expected);
            let snapshot = request.session.as_ref().unwrap().control().snapshot();
            assert_eq!(
                snapshot.details.status == crate::sessions::details::DetailsStatus::Ready,
                expected
            );
            assert!(snapshot.received_text.is_none());
        }
    }

    #[tokio::test]
    async fn non_text_bytes_after_text_introduction_remain_transfer_setup_frames() {
        let mut request = test_request();
        let text_payload_id = 301;
        let setup_payload_id = 999;

        let introduction = introduction_frame(
            vec![],
            vec![sharing_nearby::TextMetadata {
                text_title: Some("Clipboard text".to_owned()),
                r#type: Some(sharing_nearby::text_metadata::Type::Text.into()),
                payload_id: Some(text_payload_id),
                size: Some(12),
                ..Default::default()
            }],
            vec![],
        );
        request.process_introduction(&introduction).await.unwrap();

        let setup = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame::default()),
        }
        .encode_to_vec();

        let first = OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
                ),
                payload_transfer: Some(PayloadTransferFrame {
                    packet_type: Some(PacketType::Data.into()),
                    payload_header: Some(PayloadHeader {
                        id: Some(setup_payload_id),
                        r#type: Some(payload_header::PayloadType::Bytes.into()),
                        total_size: Some(setup.len() as i64),
                        is_sensitive: Some(false),
                        ..Default::default()
                    }),
                    payload_chunk: Some(PayloadChunk {
                        offset: Some(0),
                        flags: Some(0),
                        body: Some(setup.clone()),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        request.process_offline_frame(first).await.unwrap();

        let last = OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
                ),
                payload_transfer: Some(PayloadTransferFrame {
                    packet_type: Some(PacketType::Data.into()),
                    payload_header: Some(PayloadHeader {
                        id: Some(setup_payload_id),
                        r#type: Some(payload_header::PayloadType::Bytes.into()),
                        total_size: Some(setup.len() as i64),
                        is_sensitive: Some(false),
                        ..Default::default()
                    }),
                    payload_chunk: Some(PayloadChunk {
                        offset: Some(setup.len() as i64),
                        flags: Some(1),
                        body: Some(vec![]),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        request.process_offline_frame(last).await.unwrap();

        assert_eq!(request.state.state, State::WaitingForUserConsent);
        assert!(matches!(
            request.state.text_payload,
            Some(TextPayloadInfo::Text(id)) if id == text_payload_id
        ));
        assert!(request.state.byte_payloads.is_empty());
    }

    #[tokio::test]
    async fn introduction_records_url_consent_metadata() {
        let mut request = test_request();
        let frame = introduction_frame(
            vec![],
            vec![sharing_nearby::TextMetadata {
                text_title: Some("Example link".to_owned()),
                r#type: Some(sharing_nearby::text_metadata::Type::Url.into()),
                payload_id: Some(301),
                size: Some(24),
                ..Default::default()
            }],
            vec![],
        );

        request.process_introduction(&frame).await.unwrap();

        assert!(matches!(
            request.state.text_payload,
            Some(TextPayloadInfo::Url(301))
        ));
        let metadata = request.state.transfer_metadata.as_ref().unwrap();
        assert_eq!(metadata.text_description.as_deref(), Some("Example link"));
        assert_eq!(request.state.state, State::WaitingForUserConsent);
    }

    #[tokio::test]
    async fn introduction_records_wifi_consent_metadata() {
        let mut request = test_request();
        let frame = introduction_frame(
            vec![],
            vec![],
            vec![sharing_nearby::WifiCredentialsMetadata {
                ssid: Some("ModeB-WiFi".to_owned()),
                security_type: Some(2),
                payload_id: Some(401),
                ..Default::default()
            }],
        );

        request.process_introduction(&frame).await.unwrap();

        match request.state.text_payload.as_ref().unwrap() {
            TextPayloadInfo::Wifi {
                payload_id,
                ssid,
                security_type,
            } => {
                assert_eq!(*payload_id, 401);
                assert_eq!(ssid, "ModeB-WiFi");
                assert_eq!(*security_type as i32, 2);
            }
            other => panic!("unexpected payload info: {other:?}"),
        }

        let metadata = request.state.transfer_metadata.as_ref().unwrap();
        assert_eq!(metadata.text_description.as_deref(), Some("ModeB-WiFi"));
        assert_eq!(request.state.state, State::WaitingForUserConsent);
    }

    #[tokio::test]
    async fn introduction_preserves_sae_wifi_security_type() {
        let mut request = test_request();
        let frame = introduction_frame(
            vec![],
            vec![],
            vec![sharing_nearby::WifiCredentialsMetadata {
                ssid: Some("ModeB-SAE".to_owned()),
                security_type: Some(
                    sharing_nearby::wifi_credentials_metadata::SecurityType::Sae.into(),
                ),
                payload_id: Some(402),
                ..Default::default()
            }],
        );

        request.process_introduction(&frame).await.unwrap();

        match request.state.text_payload.as_ref().unwrap() {
            TextPayloadInfo::Wifi {
                payload_id,
                ssid,
                security_type,
            } => {
                assert_eq!(*payload_id, 402);
                assert_eq!(ssid, "ModeB-SAE");
                assert_eq!(
                    *security_type,
                    sharing_nearby::wifi_credentials_metadata::SecurityType::Sae
                );
            }
            other => panic!("unexpected payload info: {other:?}"),
        }
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    #[test]
    fn client_introduction_validation_accepts_only_bwu_intro() {
        use location_nearby_connections::bandwidth_upgrade_negotiation_frame::{
            ClientIntroduction, EventType,
        };
        use location_nearby_connections::{
            offline_frame, v1_frame, BandwidthUpgradeNegotiationFrame, V1Frame,
        };

        let frame = OfflineFrame {
            version: Some(offline_frame::Version::V1.into()),
            v1: Some(V1Frame {
                r#type: Some(v1_frame::FrameType::BandwidthUpgradeNegotiation.into()),
                bandwidth_upgrade_negotiation: Some(BandwidthUpgradeNegotiationFrame {
                    event_type: Some(EventType::ClientIntroduction.into()),
                    client_introduction: Some(ClientIntroduction {
                        endpoint_id: Some("peer-1234".to_owned()),
                        supports_disabling_encryption: Some(false),
                        last_endpoint_id: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        assert_eq!(
            validate_client_introduction(&frame.encode_to_vec()).unwrap(),
            BwuClientIntroductionIdentity {
                endpoint_id: "peer-1234".to_owned(),
                last_endpoint_id: None,
            }
        );

        let encoded = frame.encode_to_vec();
        let mut framed = (encoded.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(&encoded);
        assert_eq!(
            peek_client_introduction(&framed).unwrap(),
            Some(BwuClientIntroductionIdentity {
                endpoint_id: "peer-1234".to_owned(),
                last_endpoint_id: None,
            })
        );
        assert_eq!(peek_client_introduction(&framed[..3]).unwrap(), None);

        let mut dynamic_alias = frame.clone();
        dynamic_alias
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .client_introduction
            .as_mut()
            .unwrap()
            .last_endpoint_id = Some("peer-old".to_owned());
        assert_eq!(
            validate_client_introduction(&dynamic_alias.encode_to_vec()).unwrap(),
            BwuClientIntroductionIdentity {
                endpoint_id: "peer-1234".to_owned(),
                last_endpoint_id: Some("peer-old".to_owned()),
            }
        );

        let mut wrong_event = frame.clone();
        wrong_event
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .event_type = Some(EventType::SafeToClosePriorChannel.into());
        assert!(validate_client_introduction(&wrong_event.encode_to_vec()).is_err());

        let mut empty_id = frame;
        empty_id
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .client_introduction
            .as_mut()
            .unwrap()
            .endpoint_id = Some(String::new());
        assert!(validate_client_introduction(&empty_id.encode_to_vec()).is_err());
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    fn valid_bwu_client_intro_fixture() -> OfflineFrame {
        use location_nearby_connections::bandwidth_upgrade_negotiation_frame::{
            ClientIntroduction, EventType,
        };
        use location_nearby_connections::{
            offline_frame, v1_frame, BandwidthUpgradeNegotiationFrame, V1Frame,
        };

        OfflineFrame {
            version: Some(offline_frame::Version::V1.into()),
            v1: Some(V1Frame {
                r#type: Some(v1_frame::FrameType::BandwidthUpgradeNegotiation.into()),
                bandwidth_upgrade_negotiation: Some(BandwidthUpgradeNegotiationFrame {
                    event_type: Some(EventType::ClientIntroduction.into()),
                    client_introduction: Some(ClientIntroduction {
                        endpoint_id: Some("peer-1234".into()),
                        last_endpoint_id: None,
                        supports_disabling_encryption: Some(false),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        }
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    #[test]
    fn given_invalid_bwu_introduction_fields_when_validated_then_no_identity_is_admitted() {
        use location_nearby_connections::bandwidth_upgrade_negotiation_frame::EventType;
        use location_nearby_connections::v1_frame::FrameType;

        // Arrange: genuine encoded protobuf variants, each missing an independent
        // required introduction field or identifying the wrong protocol event.
        let valid = valid_bwu_client_intro_fixture();
        let mut cases: Vec<(&str, OfflineFrame)> = Vec::new();

        let mut no_v1 = valid.clone();
        no_v1.v1 = None;
        cases.push(("missing-v1", no_v1));

        let mut wrong_type = valid.clone();
        wrong_type.v1.as_mut().unwrap().r#type = Some(FrameType::ConnectionRequest.into());
        cases.push(("wrong-frame-type", wrong_type));

        let mut no_negotiation = valid.clone();
        no_negotiation
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation = None;
        cases.push(("missing-negotiation", no_negotiation));

        let mut wrong_event = valid.clone();
        wrong_event
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .event_type = Some(EventType::SafeToClosePriorChannel.into());
        cases.push(("wrong-event", wrong_event));

        let mut missing_event = valid.clone();
        missing_event
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .event_type = None;
        cases.push(("missing-event", missing_event));

        let mut no_intro = valid.clone();
        no_intro
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .client_introduction = None;
        cases.push(("missing-introduction", no_intro));

        let mut no_endpoint = valid.clone();
        no_endpoint
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .client_introduction
            .as_mut()
            .unwrap()
            .endpoint_id = None;
        cases.push(("missing-current-id", no_endpoint));

        let mut empty_endpoint = valid;
        empty_endpoint
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .client_introduction
            .as_mut()
            .unwrap()
            .endpoint_id = Some(String::new());
        cases.push(("empty-current-id", empty_endpoint));

        // Act / Assert: no corrupted or wrong-kind introduction may create
        // a route identity. A valid protobuf shape alone is not sufficient.
        for (name, frame) in cases {
            let bytes = frame.encode_to_vec();
            assert!(
                validate_client_introduction(&bytes).is_err(),
                "unexpected BWU identity from {name}"
            );
        }
        for (name, bytes) in [
            ("empty-payload", vec![]),
            ("invalid-wire-type", vec![0xff]),
            ("truncated-length-varint", vec![0x0a, 0x80]),
        ] {
            assert!(
                validate_client_introduction(&bytes).is_err(),
                "unexpected BWU identity from {name}"
            );
        }
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    #[test]
    fn given_partial_or_malformed_bwu_tcp_prefix_when_peeked_then_only_complete_valid_identity_is_returned(
    ) {
        use location_nearby_connections::v1_frame::FrameType;

        let valid = valid_bwu_client_intro_fixture();
        let payload = valid.encode_to_vec();
        let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(&payload);
        let expected = Some(BwuClientIntroductionIdentity {
            endpoint_id: "peer-1234".into(),
            last_endpoint_id: None,
        });

        // Incomplete TCP framing cannot be mistaken for admission, even
        // when a complete protobuf would follow the remaining bytes.
        for prefix_len in 0..4 {
            assert_eq!(
                peek_client_introduction(&frame[..prefix_len]).unwrap(),
                None,
                "incomplete length prefix at {prefix_len} bytes"
            );
        }
        for announced in [
            0_u32,
            (SANE_FRAME_LENGTH as u32) + 1,
            u32::MAX,
        ] {
            assert_eq!(
                peek_client_introduction(&announced.to_be_bytes()).unwrap(),
                None,
                "invalid announced length {announced}"
            );
        }
        for available in [4, frame.len() - 1] {
            assert_eq!(
                peek_client_introduction(&frame[..available]).unwrap(),
                None,
                "incomplete body with {available} bytes"
            );
        }
        let mut too_long_header = ((payload.len() as u32) + 1).to_be_bytes().to_vec();
        too_long_header.extend_from_slice(&payload);
        assert_eq!(peek_client_introduction(&too_long_header).unwrap(), None);

        let malformed_body = [0xff_u8];
        let mut malformed = (malformed_body.len() as u32).to_be_bytes().to_vec();
        malformed.extend_from_slice(&malformed_body);
        assert_eq!(peek_client_introduction(&malformed).unwrap(), None);

        let mut wrong_kind = valid;
        wrong_kind.v1.as_mut().unwrap().r#type = Some(FrameType::ConnectionRequest.into());
        let wrong_payload = wrong_kind.encode_to_vec();
        let mut wrong_frame = (wrong_payload.len() as u32).to_be_bytes().to_vec();
        wrong_frame.extend_from_slice(&wrong_payload);
        assert_eq!(peek_client_introduction(&wrong_frame).unwrap(), None);

        // A complete first frame must be recognized even when the stream
        // already contains data from a following frame; the peek consumes none.
        assert_eq!(peek_client_introduction(&frame).unwrap(), expected);
        let mut with_trailing = frame.clone();
        with_trailing.extend_from_slice(&[0xff, 0x00, 0x12, 0x34]);
        assert_eq!(peek_client_introduction(&with_trailing).unwrap(), expected);
        assert_eq!(frame, with_trailing[..frame.len()]);
    }

    #[cfg(all(feature = "experimental", target_os = "linux"))]
    #[test]
    fn given_bwu_current_and_last_ids_when_compared_then_only_exact_expected_id_matches() {
        // A matching string is a router lookup predicate, NOT authenticated
        // peer identity. Cryptographic alias authorization remains open.
        for (current, previous, expected, matches) in [
            ("peer-1234", None, "peer-1234", true),
            ("peer-1234", Some("peer-old"), "peer-old", true),
            ("peer-1234", Some("peer-old"), "peer-1234", true),
            ("peer-1234", None, "peer-old", false),
            ("peer-1234", Some("peer-old"), "peer-other", false),
            ("peer-1234", Some("peer-old"), "peer", false),
            ("peer-1234", Some("peer-old"), "PEER-OLD", false),
            ("peer-1234", Some("peer-old"), "", false),
        ] {
            let identity = BwuClientIntroductionIdentity {
                endpoint_id: current.into(),
                last_endpoint_id: previous.map(str::to_owned),
            };
            assert_eq!(
                identity.matches_expected(expected),
                matches,
                "unexpected lookup for current={current} previous={previous:?} expected={expected}"
            );
        }

        let mut frame = valid_bwu_client_intro_fixture();
        let intro = frame
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .client_introduction
            .as_mut()
            .unwrap();
        intro.last_endpoint_id = Some(String::new());
        assert_eq!(
            validate_client_introduction(&frame.encode_to_vec()).unwrap(),
            BwuClientIntroductionIdentity {
                endpoint_id: "peer-1234".into(),
                last_endpoint_id: None,
            }
        );
        frame
            .v1
            .as_mut()
            .unwrap()
            .bandwidth_upgrade_negotiation
            .as_mut()
            .unwrap()
            .client_introduction
            .as_mut()
            .unwrap()
            .last_endpoint_id = Some("peer-old".into());
        let identity = validate_client_introduction(&frame.encode_to_vec()).unwrap();
        assert!(identity.matches_expected("peer-old"));
        assert!(!identity.matches_expected("peer-other"));
    }

}

#[cfg(all(test, feature = "experimental", target_os = "linux"))]
mod managed_migration_tests {
    use super::*;

    #[tokio::test]
    async fn given_managed_ble_request_when_stream_moves_to_tcp_then_session_identity_and_cancellation_survive(
    ) {
        tokio::time::timeout(Duration::from_secs(5), async {
            // Arrange
            let hub = crate::sessions::SessionHub::new(91).unwrap();
            let session = hub
                .open(
                    crate::channel::TransferType::Inbound,
                    None,
                    tokio_util::sync::CancellationToken::new(),
                )
                .unwrap();
            let control = session.control();
            let before = control.snapshot();
            let (sender, _events) = tokio::sync::broadcast::channel(16);
            let (ble, _old_peer) = tokio::io::duplex(4096);
            let mut request = InboundRequest::new(
                crate::hdl::MigratableStream::Ble(ble),
                "old-ble".into(),
                sender,
            )
            .attach_session(session);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let mut peer = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
                .await
                .unwrap();
            let (tcp, _) = listener.accept().await.unwrap();
            // Act: the same stream replacement used by do_bandwidth_upgrade, without changing its wire negotiation.
            request.socket = crate::hdl::MigratableStream::Tcp(tcp);
            let cancellation = request.cancellation();
            let worker = tokio::spawn(async move {
                let result: Option<Result<(), anyhow::Error>> =
                    crate::manager::session::until_either_stops(
                        &tokio_util::sync::CancellationToken::new(),
                        &cancellation,
                        async {
                            loop {
                                request.handle().await?;
                            }
                        },
                    )
                    .await;
                assert!(result.is_none());
                request.record_termination(State::Cancelled);
                (request.state.id.clone(), request.state.state.clone())
            });
            crate::manager::supervision_tests::establish_inbound(&mut peer).await;
            control.cancel();
            let (event_id, state) = worker.await.unwrap();
            // Assert
            assert_eq!(event_id, before.event_id);
            assert_eq!(control.snapshot().id, before.id);
            assert_eq!(state, State::Cancelled);
            assert_eq!(control.snapshot().state, State::Cancelled);
            assert!(control.is_closed());
            assert!(hub.active().unwrap().is_empty());
        })
        .await
        .expect("managed migration deadlocked");
    }
}
