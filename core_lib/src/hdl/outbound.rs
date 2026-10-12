use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use bytes::Bytes;
use hmac::{Hmac, Mac};
use libaes::{Cipher, AES_256_KEY_LEN};
use p256::ecdh::diffie_hellman;
use p256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
use p256::{EncodedPoint, PublicKey};
use prost::Message;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::broadcast::error::TryRecvError;
use tokio::sync::broadcast::{Receiver, Sender};
use ts_rs::TS;

use super::info::{InternalFileInfo, TransferMetadata};
use super::{InnerState, State, TextPayloadType};
use crate::channel::{ChannelAction, ChannelDirection, ChannelMessage};
use crate::file_reader::read_file_chunk;
use crate::location_nearby_connections::bandwidth_upgrade_negotiation_frame::upgrade_path_info::Medium;
use crate::location_nearby_connections::connection_response_frame::ResponseStatus;
use crate::location_nearby_connections::payload_transfer_frame::{
    payload_header, PacketType, PayloadChunk, PayloadHeader,
};
use crate::location_nearby_connections::{KeepAliveFrame, OfflineFrame, PayloadTransferFrame};
use crate::protocol::{checked_payload_buffer_size, SANE_FRAME_LENGTH, SANITY_DURATION};
use crate::securegcm::ukey2_alert::AlertType;
use crate::securegcm::ukey2_client_init::CipherCommitment;
use crate::securegcm::{
    ukey2_message, DeviceToDeviceMessage, GcmMetadata, Type, Ukey2Alert, Ukey2ClientFinished,
    Ukey2ClientInit, Ukey2HandshakeCipher, Ukey2Message, Ukey2ServerInit,
};
use crate::securemessage::{
    EcP256PublicKey, EncScheme, GenericPublicKey, Header, HeaderAndBody, PublicKeyType,
    SecureMessage, SigScheme,
};
use crate::sharing_nearby::{
    file_metadata, paired_key_result_frame, text_metadata, FileMetadata, IntroductionFrame,
    TextMetadata,
};
use crate::utils::{
    encode_point, gen_ecdsa_keypair, gen_random, hkdf_extract_expand, normalize_p256_coordinate,
    stream_read_exact, to_four_digit_string, DeviceType, RemoteDeviceInfo,
};
use crate::{location_nearby_connections, sharing_nearby};

type HmacSha256 = Hmac<Sha256>;

pub const MANAGED_EPHEMERAL_FILE_PREFIX: &str = "rquickshare-clipboard-";
const MAX_OUTBOUND_FOLDER_DEPTH: usize = 64;

#[derive(Debug)]
pub struct ManagedEphemeralFile {
    path: PathBuf,
}

impl ManagedEphemeralFile {
    pub fn try_from_path(path: impl Into<PathBuf>) -> Result<Self, anyhow::Error> {
        let path = path.into();
        let temp_dir = std::env::temp_dir();
        if path.parent() != Some(temp_dir.as_path()) {
            return Err(anyhow!(
                "Managed ephemeral file must be a direct child of the system temp directory"
            ));
        }

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("Managed ephemeral file name is not valid UTF-8"))?;
        let token = name
            .strip_prefix(MANAGED_EPHEMERAL_FILE_PREFIX)
            .and_then(|name| name.strip_suffix(".png"))
            .ok_or_else(|| anyhow!("Path is not an rQuickShare managed clipboard image"))?;

        let mut parts = token.split('-');
        let valid_token = (0..3).all(|_| {
            parts.next().is_some_and(|part| {
                !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
            })
        }) && parts.next().is_none();

        if !valid_token {
            return Err(anyhow!("Managed clipboard image name has an invalid token"));
        }

        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn remove(&self) -> Result<(), anyhow::Error> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(anyhow!(
                "Could not remove managed ephemeral file {}: {error}",
                self.path.display()
            )),
        }
    }
}

fn is_cancel_request(message: &ChannelMessage, transfer_id: &str) -> bool {
    message.direction == ChannelDirection::FrontToLib
        && message.id == transfer_id
        && message.action == Some(ChannelAction::CancelTransfer)
}

struct PreparedOutboundFiles {
    metadata: Vec<FileMetadata>,
    files: HashMap<i64, InternalFileInfo>,
    total_bytes: u64,
}

#[derive(Debug)]
struct PreparedOutboundText {
    metadata: TextMetadata,
    payload_id: i64,
    bytes: Vec<u8>,
    payload_type: TextPayloadType,
}

fn classify_outbound_text(text: &str) -> text_metadata::Type {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();

    if lower.starts_with("https://") || lower.starts_with("http://") {
        return text_metadata::Type::Url;
    }

    let phone_chars = trimmed
        .chars()
        .filter(|character| !character.is_whitespace() && !matches!(character, '-' | '(' | ')'))
        .collect::<String>();
    let phone_digits = phone_chars
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    if phone_digits >= 7
        && phone_chars
            .chars()
            .all(|character| character.is_ascii_digit() || character == '+')
    {
        return text_metadata::Type::PhoneNumber;
    }

    text_metadata::Type::Text
}

fn prepare_outbound_text(text: &str) -> Result<PreparedOutboundText, anyhow::Error> {
    if text.is_empty() {
        return Err(anyhow!("Cannot share empty text"));
    }

    let bytes = text.as_bytes().to_vec();
    let size = checked_payload_buffer_size(
        i64::try_from(bytes.len()).map_err(|_| anyhow!("Text payload is too large"))?,
    )?;
    let payload_id = rand::rng().random::<i64>();
    let metadata_type = classify_outbound_text(text);
    let payload_type = if metadata_type == text_metadata::Type::Url {
        TextPayloadType::Url
    } else {
        TextPayloadType::Text
    };
    let title = text.chars().take(64).collect::<String>();

    Ok(PreparedOutboundText {
        metadata: TextMetadata {
            text_title: Some(title),
            r#type: Some(metadata_type.into()),
            payload_id: Some(payload_id),
            size: Some(size as i64),
            id: Some(rand::rng().random::<i64>()),
            ..Default::default()
        },
        payload_id,
        bytes,
        payload_type,
    })
}

fn byte_payload_frames(payload_id: i64, body: &[u8]) -> Result<Vec<OfflineFrame>, anyhow::Error> {
    const CHUNK_SIZE: usize = 256 * 1024;

    let total_size = i64::try_from(body.len()).map_err(|_| anyhow!("Byte payload is too large"))?;
    checked_payload_buffer_size(total_size)?;

    let header = PayloadHeader {
        id: Some(payload_id),
        r#type: Some(payload_header::PayloadType::Bytes.into()),
        total_size: Some(total_size),
        is_sensitive: Some(false),
        ..Default::default()
    };

    let mut frames = Vec::with_capacity(body.len().div_ceil(CHUNK_SIZE) + 1);
    for (index, chunk) in body.chunks(CHUNK_SIZE).enumerate() {
        let offset = index
            .checked_mul(CHUNK_SIZE)
            .and_then(|value| i64::try_from(value).ok())
            .ok_or_else(|| anyhow!("Byte payload offset overflow"))?;

        frames.push(OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
                ),
                payload_transfer: Some(PayloadTransferFrame {
                    packet_type: Some(PacketType::Data.into()),
                    payload_header: Some(header.clone()),
                    payload_chunk: Some(PayloadChunk {
                        offset: Some(offset),
                        flags: Some(0),
                        body: Some(chunk.to_vec()),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        });
    }

    frames.push(OfflineFrame {
        version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
        v1: Some(location_nearby_connections::V1Frame {
            r#type: Some(location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into()),
            payload_transfer: Some(PayloadTransferFrame {
                packet_type: Some(PacketType::Data.into()),
                payload_header: Some(header),
                payload_chunk: Some(PayloadChunk {
                    offset: Some(total_size),
                    flags: Some(1),
                    body: Some(Vec::new()),
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
    });

    Ok(frames)
}

#[derive(Debug)]
struct OutboundFileCandidate {
    path: PathBuf,
    parent_folder: Option<String>,
}

fn validate_outbound_path_component(component: &OsStr) -> Result<String, anyhow::Error> {
    let value = component
        .to_str()
        .ok_or_else(|| anyhow!("Folder path component is not valid UTF-8"))?;

    if value.is_empty()
        || value.len() > 255
        || value
            .chars()
            .any(|character| character == '\\' || character == '\0' || character.is_control())
    {
        return Err(anyhow!("Folder path component is unsafe: {value:?}"));
    }

    Ok(value.to_owned())
}

fn collect_directory_files(
    root: &Path,
    current: &Path,
    root_name: &str,
    depth: usize,
    output: &mut Vec<OutboundFileCandidate>,
) -> Result<(), anyhow::Error> {
    if depth > MAX_OUTBOUND_FOLDER_DEPTH {
        return Err(anyhow!(
            "Outbound folder exceeds maximum nesting depth of {MAX_OUTBOUND_FOLDER_DEPTH}"
        ));
    }

    let mut entries = std::fs::read_dir(current)?.collect::<Result<Vec<_>, std::io::Error>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_symlink() {
            warn!(
                "Skipping symbolic link in outbound folder: {}",
                path.display()
            );
            continue;
        }

        if file_type.is_dir() {
            collect_directory_files(root, &path, root_name, depth + 1, output)?;
            continue;
        }

        if !file_type.is_file() {
            warn!("Skipping non-regular outbound path: {}", path.display());
            continue;
        }

        let relative_parent = path
            .parent()
            .ok_or_else(|| anyhow!("Outbound file has no parent: {}", path.display()))?
            .strip_prefix(root)
            .map_err(|_| anyhow!("Outbound file escaped selected folder: {}", path.display()))?;

        let mut parent_segments = vec![root_name.to_owned()];
        for component in relative_parent.components() {
            match component {
                std::path::Component::Normal(value) => {
                    parent_segments.push(validate_outbound_path_component(value)?);
                }
                _ => {
                    return Err(anyhow!(
                        "Outbound relative folder contains unsafe components: {}",
                        relative_parent.display()
                    ));
                }
            }
        }

        output.push(OutboundFileCandidate {
            path,
            parent_folder: Some(parent_segments.join("/")),
        });
    }

    Ok(())
}

fn collect_outbound_file_candidates(
    inputs: &[String],
) -> Result<Vec<OutboundFileCandidate>, anyhow::Error> {
    let mut output = Vec::new();
    let mut reserved_root_names = HashSet::new();

    for input in inputs {
        let path = Path::new(input);
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                warn!("Outbound path does not exist: {input}");
                continue;
            }
            Err(error) => {
                return Err(anyhow!("Failed to inspect outbound path {input}: {error}"));
            }
        };

        if metadata.file_type().is_symlink() {
            warn!("Skipping symbolic link selected for outbound transfer: {input}");
            continue;
        }

        if metadata.is_file() {
            output.push(OutboundFileCandidate {
                path: path.to_path_buf(),
                parent_folder: None,
            });
            continue;
        }

        if metadata.is_dir() {
            let root_name = path
                .file_name()
                .ok_or_else(|| anyhow!("Selected folder has no root name: {input}"))
                .and_then(validate_outbound_path_component)?;

            let mut chosen_root = root_name.clone();
            let mut counter = 1_u64;
            while !reserved_root_names.insert(chosen_root.clone()) {
                chosen_root = format!("{counter}_{root_name}");
                validate_outbound_path_component(OsStr::new(&chosen_root))?;
                counter = counter
                    .checked_add(1)
                    .ok_or_else(|| anyhow!("Outbound folder root suffix overflow"))?;
            }

            collect_directory_files(path, path, &chosen_root, 0, &mut output)?;
            continue;
        }

        warn!("Skipping non-file outbound path: {input}");
    }

    Ok(output)
}

fn prepare_outbound_files(files: &[String]) -> Result<PreparedOutboundFiles, anyhow::Error> {
    let candidates = collect_outbound_file_candidates(files)?;
    if candidates.is_empty() {
        return Err(anyhow!(
            "Outbound selection contains no shareable regular files"
        ));
    }

    let opened = candidates.into_iter().map(|candidate| {
        let file = File::open(&candidate.path)
            .map_err(|error| anyhow!("Failed to open outbound file: {error}"))?;
        Ok((file, candidate.path, candidate.parent_folder))
    });
    prepare_open_file_metadata(opened)
}

#[cfg(target_os = "linux")]
fn prepare_opened_outbound_files(
    selection: &crate::file_source::OpenedSelection,
) -> Result<PreparedOutboundFiles, anyhow::Error> {
    let opened = selection.files().iter().map(|source| {
        let (parent, name) = match source.relative_name().rsplit_once('/') {
            Some((parent, name)) => (Some(parent.to_owned()), name),
            None => (None, source.relative_name()),
        };
        let file = source.clone_handle()?;
        if file.metadata()?.len() != source.size() {
            return Err(anyhow!("Source size changed after acquisition"));
        }
        Ok((file, PathBuf::from(name), parent))
    });
    prepare_open_file_metadata(opened)
}

fn prepare_open_file_metadata(
    opened: impl Iterator<Item = Result<(File, PathBuf, Option<String>), anyhow::Error>>,
) -> Result<PreparedOutboundFiles, anyhow::Error> {
    let mut file_metadata = Vec::new();
    let mut transferred_files = HashMap::new();
    let mut total_to_send = 0_u64;

    for source in opened {
        let (file, path, parent_folder) = source?;
        let path = path.as_path();
        let file_path = path.display().to_string();

        let metadata = file
            .metadata()
            .map_err(|error| anyhow!("Failed to get metadata for {file_path}: {error}"))?;

        if !metadata.is_file() {
            return Err(anyhow!(
                "Outbound candidate is no longer a regular file: {file_path}"
            ));
        }

        let mime_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        let attachment_type = if mime_type.starts_with("image/") {
            file_metadata::Type::Image
        } else if mime_type.starts_with("video/") {
            file_metadata::Type::Video
        } else if mime_type.starts_with("audio/") {
            file_metadata::Type::Audio
        } else if path.extension().is_some_and(|extension| extension == "apk") {
            file_metadata::Type::App
        } else {
            file_metadata::Type::Unknown
        };

        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("Failed to get file_name for {file_path}"))?
            .to_str()
            .ok_or_else(|| anyhow!("File name is not valid UTF-8: {file_path}"))?
            .to_owned();

        let payload_id = loop {
            let candidate_id = rand::rng().random::<i64>();
            if !transferred_files.contains_key(&candidate_id) {
                break candidate_id;
            }
        };

        let file_size = i64::try_from(metadata.size())
            .map_err(|_| anyhow!("File is too large to represent in the protocol: {file_path}"))?;

        let protocol_metadata = FileMetadata {
            payload_id: Some(payload_id),
            name: Some(file_name),
            size: Some(file_size),
            mime_type: Some(mime_type),
            r#type: Some(attachment_type.into()),
            parent_folder: parent_folder.clone(),
            ..Default::default()
        };

        transferred_files.insert(
            payload_id,
            InternalFileInfo {
                payload_id,
                file_url: path.to_path_buf(),
                parent_folder,
                bytes_transferred: 0,
                total_size: file_size,
                file: Some(file),
            },
        );
        file_metadata.push(protocol_metadata);

        total_to_send = total_to_send
            .checked_add(metadata.size())
            .ok_or_else(|| anyhow!("Total outbound transfer size overflow"))?;
    }

    Ok(PreparedOutboundFiles {
        metadata: file_metadata,
        files: transferred_files,
        total_bytes: total_to_send,
    })
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub enum OutboundPayload {
    Files(Vec<String>),
    EphemeralFiles(Vec<String>),
    Text(String),
    /// Native-only resources cannot be constructed through the legacy JSON boundary.
    #[cfg(target_os = "linux")]
    #[serde(skip)]
    #[ts(skip)]
    OpenedFiles(std::sync::Arc<crate::file_source::OpenedSelection>),
}

#[derive(Debug)]
pub struct OutboundRequest {
    endpoint_id: [u8; 4],
    socket: TcpStream,
    pub state: InnerState,
    sender: Sender<ChannelMessage>,
    receiver: Receiver<ChannelMessage>,
    payload: OutboundPayload,
    text_payload_id: Option<i64>,
    cleanup_files: Vec<ManagedEphemeralFile>,
}

impl Drop for OutboundRequest {
    fn drop(&mut self) {
        for file in self.cleanup_files.drain(..) {
            match file.remove() {
                Ok(()) => debug!(
                    "Removed managed ephemeral outbound file: {}",
                    file.path().display()
                ),
                Err(error) => warn!("{error}"),
            }
        }
    }
}

impl OutboundRequest {
    pub fn new(
        endpoint_id: [u8; 4],
        socket: TcpStream,
        id: String,
        sender: Sender<ChannelMessage>,
        payload: OutboundPayload,
        rdi: RemoteDeviceInfo,
    ) -> Result<Self, anyhow::Error> {
        let receiver = sender.subscribe();
        let (files, text_payload, cleanup_files) = match &payload {
            OutboundPayload::Files(files) => (Some(files.clone()), None, Vec::new()),
            OutboundPayload::EphemeralFiles(files) => {
                let cleanup_files = files
                    .iter()
                    .cloned()
                    .map(ManagedEphemeralFile::try_from_path)
                    .collect::<Result<Vec<_>, _>>()?;
                (Some(files.clone()), None, cleanup_files)
            }
            OutboundPayload::Text(text) => (None, Some(text.clone()), Vec::new()),
            #[cfg(target_os = "linux")]
            OutboundPayload::OpenedFiles(selection) => (
                Some(
                    selection
                        .files()
                        .iter()
                        .map(|file| file.relative_name().to_owned())
                        .collect(),
                ),
                None,
                Vec::new(),
            ),
        };

        Ok(Self {
            endpoint_id,
            socket,
            state: InnerState {
                id,
                server_seq: 0,
                client_seq: 0,
                state: State::Initial,
                encryption_done: true,
                transfer_metadata: Some(TransferMetadata {
                    id: String::from(""),
                    source: Some(rdi),
                    files,
                    text_payload,
                    ..Default::default()
                }),
                ..Default::default()
            },
            sender,
            receiver,
            payload,
            text_payload_id: None,
            cleanup_files,
        })
    }

    fn take_pending_cancel_request(&mut self) -> Result<bool, anyhow::Error> {
        loop {
            match self.receiver.try_recv() {
                Ok(message) => {
                    if is_cancel_request(&message, &self.state.id) {
                        debug!("outbound: received cancellation while sending");
                        return Ok(true);
                    }
                }
                Err(TryRecvError::Empty) => return Ok(false),
                Err(TryRecvError::Lagged(skipped)) => {
                    warn!("outbound: control channel lagged by {skipped} messages");
                }
                Err(TryRecvError::Closed) => {
                    return Err(anyhow!("Outbound control channel is closed"));
                }
            }
        }
    }

    pub async fn handle(&mut self) -> Result<(), anyhow::Error> {
        // Buffer for the 4-byte length
        let mut length_buf = [0u8; 4];

        tokio::select! {
            i = self.receiver.recv() => {
                match i {
                    Ok(channel_msg) => {
                        if channel_msg.direction == ChannelDirection::LibToFront {
                            return Ok(());
                        }

                        if channel_msg.id != self.state.id {
                            return Ok(());
                        }

                        debug!("outbound: got: {:?}", channel_msg);
                        match channel_msg.action {
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
                            _ => {}
                        }
                    }
                    Err(e) => {
                        error!("inbound: channel error: {}", e);
                    }
                }
            },
            h = stream_read_exact(&mut self.socket, &mut length_buf) => {
                h?;

                self._handle(length_buf).await?
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
            State::SentUkeyClientInit => {
                debug!("Handling State::SentUkeyClientInit frame");
                let msg = Ukey2Message::decode(&*frame_data)?;
                self.update_state(
                    |e| {
                        e.server_init_data = Some(frame_data);
                    },
                    false,
                )
                .await;
                self.process_ukey2_server_init(&msg).await?;

                // Advance current state
                self.update_state(
                    |e: &mut InnerState| {
                        e.state = State::SentUkeyClientFinish;
                        e.encryption_done = true;
                    },
                    false,
                )
                .await;
            }
            State::SentUkeyClientFinish => {
                debug!("Handling State::SentUkeyClientFinish frame");
                let frame = location_nearby_connections::OfflineFrame::decode(&*frame_data)?;
                self.process_connection_response(&frame).await?;

                // Advance current state
                self.update_state(
                    |e: &mut InnerState| {
                        e.state = State::SentPairedKeyEncryption;
                        e.server_init_data = Some(frame_data);
                        e.encryption_done = true;
                    },
                    false,
                )
                .await;
            }
            _ => {
                debug!("Handling SecureMessage frame");
                let smsg = SecureMessage::decode(&*frame_data)?;
                self.decrypt_and_process_secure_message(&smsg).await?;
            }
        }

        Ok(())
    }

    pub async fn send_connection_request(&mut self) -> Result<(), anyhow::Error> {
        let request = location_nearby_connections::OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::ConnectionRequest.into(),
                ),
                connection_request: Some(location_nearby_connections::ConnectionRequestFrame {
                    endpoint_id: Some(String::from_utf8_lossy(&self.endpoint_id).to_string()),
                    endpoint_name: Some(sys_metrics::host::get_hostname()?.into()),
                    endpoint_info: Some(
                        RemoteDeviceInfo {
                            name: sys_metrics::host::get_hostname()?,
                            device_type: DeviceType::Laptop,
                        }
                        .serialize(),
                    ),
                    mediums: vec![Medium::WifiLan.into()],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        self.send_frame(request.encode_to_vec()).await?;

        Ok(())
    }

    pub async fn send_ukey2_client_init(&mut self) -> Result<(), anyhow::Error> {
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

        let finish_frame = Ukey2Message {
            message_type: Some(ukey2_message::Type::ClientFinish.into()),
            message_data: Some(
                Ukey2ClientFinished {
                    public_key: Some(pkey.encode_to_vec()),
                }
                .encode_to_vec(),
            ),
        };

        let sha512 = Sha512::digest(finish_frame.encode_to_vec());
        let frame = Ukey2Message {
            message_type: Some(ukey2_message::Type::ClientInit.into()),
            message_data: Some(
                Ukey2ClientInit {
                    version: Some(1),
                    random: Some(gen_random(32)),
                    next_protocol: Some(String::from("AES_256_CBC-HMAC_SHA256")),
                    cipher_commitments: vec![CipherCommitment {
                        handshake_cipher: Some(Ukey2HandshakeCipher::P256Sha512.into()),
                        commitment: Some(sha512.to_vec()),
                    }],
                }
                .encode_to_vec(),
            ),
        };

        self.send_frame(frame.encode_to_vec()).await?;

        self.update_state(
            |e| {
                e.state = State::SentUkeyClientInit;
                e.private_key = Some(secret_key);
                e.public_key = Some(public_key);
                e.client_init_msg_data = Some(frame.encode_to_vec());
                e.ukey_client_finish_msg_data = Some(finish_frame.encode_to_vec());
            },
            false,
        )
        .await;

        Ok(())
    }

    async fn process_ukey2_server_init(&mut self, msg: &Ukey2Message) -> Result<(), anyhow::Error> {
        if msg.message_type() != ukey2_message::Type::ServerInit {
            self.send_ukey2_alert(AlertType::BadMessageType).await?;
            return Err(anyhow!(
                "UKey2: message_type({:?}) != ServerInit",
                msg.message_type
            ));
        }

        let server_init = match Ukey2ServerInit::decode(msg.message_data()) {
            Ok(uk2si) => uk2si,
            Err(e) => {
                return Err(anyhow!("UKey2: Ukey2ClientFinished::decode: {}", e));
            }
        };

        if server_init.version() != 1 {
            self.send_ukey2_alert(AlertType::BadVersion).await?;
            return Err(anyhow!("UKey2: server_init.version != 1"));
        }

        if server_init.random().len() != 32 {
            self.send_ukey2_alert(AlertType::BadRandom).await?;
            return Err(anyhow!("UKey2: server_init.random.len != 32"));
        }

        if server_init.handshake_cipher() != Ukey2HandshakeCipher::P256Sha512 {
            self.send_ukey2_alert(AlertType::BadHandshakeCipher).await?;
            return Err(anyhow!("UKey2: handshake_cipher != P256Sha512"));
        }

        let server_public_key = match GenericPublicKey::decode(server_init.public_key()) {
            Ok(spk) => spk,
            Err(e) => {
                return Err(anyhow!("UKey2: GenericPublicKey::decode: {}", e));
            }
        };

        self.finalize_key_exchange(server_public_key).await?;
        let finish_msg = self
            .state
            .ukey_client_finish_msg_data
            .clone()
            .ok_or_else(|| anyhow!("Missing UKEY2 client finish data"))?;
        self.send_frame(finish_msg).await?;

        let frame = location_nearby_connections::OfflineFrame {
            version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
            v1: Some(location_nearby_connections::V1Frame {
                r#type: Some(
                    location_nearby_connections::v1_frame::FrameType::ConnectionResponse.into(),
                ),
                connection_response: Some(location_nearby_connections::ConnectionResponseFrame {
					response: Some(location_nearby_connections::connection_response_frame::ResponseStatus::Accept.into()),
					os_info: Some(location_nearby_connections::OsInfo {
						r#type: Some(location_nearby_connections::os_info::OsType::Linux.into())
					}),
					..Default::default()
				}),
                ..Default::default()
            }),
        };

        self.send_frame(frame.encode_to_vec()).await?;

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

        let connection_response = v1_frame
            .connection_response
            .as_ref()
            .ok_or_else(|| anyhow!("Missing connection response"))?;

        if connection_response.response() != ResponseStatus::Accept {
            return Err(anyhow!("Connection rejected by third party"));
        }

        let paired_encryption = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::PairedKeyEncryption.into()),
                paired_key_encryption: Some(sharing_nearby::PairedKeyEncryptionFrame {
                    secret_id_hash: Some(gen_random(6)),
                    signed_data: Some(gen_random(72)),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        self.send_encrypted_frame(&paired_encryption).await?;

        Ok(())
    }

    async fn decrypt_and_process_secure_message(
        &mut self,
        smsg: &SecureMessage,
    ) -> Result<(), anyhow::Error> {
        let recv_hmac_key = self
            .state
            .recv_hmac_key
            .as_ref()
            .ok_or_else(|| anyhow!("Missing receive HMAC key"))?;
        let mut hmac = HmacSha256::new_from_slice(recv_hmac_key)?;
        hmac.update(&smsg.header_and_body);
        if !hmac
            .finalize()
            .into_bytes()
            .as_slice()
            .eq(smsg.signature.as_slice())
        {
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

        let offline = location_nearby_connections::OfflineFrame::decode(d2d_msg.message())?;
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

                        let declared_size = checked_payload_buffer_size(header.total_size())?;

                        self.state
                            .payload_buffers
                            .entry(payload_id)
                            .or_insert_with(|| Vec::with_capacity(declared_size));

                        // Get the current length of the buffer, if it exists, without holding a mutable borrow.
                        let buffer_len = self
                            .state
                            .payload_buffers
                            .get(&payload_id)
                            .ok_or_else(|| anyhow!("Missing payload buffer for {payload_id}"))?
                            .len();
                        if chunk.offset() != buffer_len as i64 {
                            self.state.payload_buffers.remove(&payload_id);
                            return Err(anyhow!(
                                "Unexpected chunk offset: {}, expected: {}",
                                chunk.offset(),
                                buffer_len
                            ));
                        }

                        let body_len = chunk.body().len();
                        let new_len = buffer_len
                            .checked_add(body_len)
                            .ok_or_else(|| anyhow!("Byte payload size overflow"))?;
                        if new_len > declared_size {
                            self.state.payload_buffers.remove(&payload_id);
                            return Err(anyhow!(
                                "Byte payload exceeds declared size: {new_len} > {declared_size}"
                            ));
                        }

                        let buffer = self
                            .state
                            .payload_buffers
                            .get_mut(&payload_id)
                            .ok_or_else(|| anyhow!("Missing payload buffer for {payload_id}"))?;
                        if let Some(body) = &chunk.body {
                            buffer.extend(body);
                        }

                        if (chunk.flags() & 1) == 1 {
                            debug!("Chunk flags & 1 == 1 ?? End of data ??");

                            let innner_frame = sharing_nearby::Frame::decode(buffer.as_slice())?;
                            self.process_transfer_setup(&innner_frame).await?;
                        }
                    }
                    payload_header::PayloadType::File => {
                        error!("Unhandled PayloadType::File: {:?}", header.r#type())
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
            State::SentPairedKeyEncryption => {
                debug!("Processing State::SentPairedKeyEncryption");
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
                        e.state = State::SentIntroduction;
                    },
                    true,
                )
                .await;
            }
            State::SentIntroduction => {
                debug!("Processing State::SentIntroduction");
                self.process_consent(v1_frame).await?;
            }
            State::SendingFiles => {}
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
        if v1_frame.paired_key_encryption.is_none() {
            return Err(anyhow!("Missing required fields"));
        }

        let paired_result = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::PairedKeyResult.into()),
                paired_key_result: Some(sharing_nearby::PairedKeyResultFrame {
                    status: Some(paired_key_result_frame::Status::Unable.into()),
                }),
                ..Default::default()
            }),
        };

        self.send_encrypted_frame(&paired_result).await?;

        Ok(())
    }

    async fn process_paired_key_result(
        &mut self,
        v1_frame: &sharing_nearby::V1Frame,
    ) -> Result<(), anyhow::Error> {
        if v1_frame.paired_key_result.is_none() {
            return Err(anyhow!("Missing required fields"));
        }

        let prepared_files = match &self.payload {
            OutboundPayload::Files(files) | OutboundPayload::EphemeralFiles(files) => {
                Some(prepare_outbound_files(files)?)
            }
            #[cfg(target_os = "linux")]
            OutboundPayload::OpenedFiles(selection) => {
                Some(prepare_opened_outbound_files(selection)?)
            }
            OutboundPayload::Text(_) => None,
        };

        let introduction = match prepared_files {
            Some(prepared) => {
                let PreparedOutboundFiles {
                    metadata: file_metadata,
                    files: transferred_files,
                    total_bytes: total_to_send,
                } = prepared;

                self.update_state(
                    |state| {
                        if let Some(metadata) = state.transfer_metadata.as_mut() {
                            metadata.total_bytes = total_to_send;
                        }
                        state.transferred_files = transferred_files;
                    },
                    false,
                )
                .await;

                IntroductionFrame {
                    file_metadata,
                    use_case: Some(
                        sharing_nearby::introduction_frame::SharingUseCase::NearbyShare.into(),
                    ),
                    ..Default::default()
                }
            }
            None => {
                let OutboundPayload::Text(text) = &self.payload else {
                    return Err(anyhow!("Missing prepared outbound files"));
                };
                let prepared = prepare_outbound_text(text)?;
                self.text_payload_id = Some(prepared.payload_id);
                let payload_type = prepared.payload_type.clone();
                let total_bytes = prepared.bytes.len() as u64;
                self.update_state(
                    |state| {
                        if let Some(metadata) = state.transfer_metadata.as_mut() {
                            metadata.total_bytes = total_bytes;
                            metadata.text_type = Some(payload_type);
                            metadata.text_description = prepared.metadata.text_title.clone();
                        }
                    },
                    false,
                )
                .await;

                IntroductionFrame {
                    text_metadata: vec![prepared.metadata],
                    use_case: Some(
                        sharing_nearby::introduction_frame::SharingUseCase::RemoteCopy.into(),
                    ),
                    ..Default::default()
                }
            }
        };

        let frame = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::Introduction.into()),
                introduction: Some(introduction),
                ..Default::default()
            }),
        };

        self.send_encrypted_frame(&frame).await?;

        Ok(())
    }

    async fn process_consent(
        &mut self,
        v1_frame: &sharing_nearby::V1Frame,
    ) -> Result<(), anyhow::Error> {
        if v1_frame.r#type() != sharing_nearby::v1_frame::FrameType::Response {
            return Err(anyhow!("Expected consent response frame"));
        }

        let connection_response = v1_frame
            .connection_response
            .as_ref()
            .ok_or_else(|| anyhow!("Missing consent response"))?;
        let consent_status = connection_response.status();

        match consent_status {
            sharing_nearby::connection_response_frame::Status::Accept => {
                info!("State is now State::SendingFiles");
                self.update_state(
                    |e| {
                        e.state = State::SendingFiles;
                    },
                    true,
                )
                .await;

                if let OutboundPayload::Text(text) = self.payload.clone() {
                    if self.take_pending_cancel_request()? {
                        self.update_state(|state| state.state = State::Cancelled, true)
                            .await;
                        self.disconnection().await?;
                        return Err(anyhow!(crate::errors::AppError::NotAnError));
                    }

                    let payload_id = self
                        .text_payload_id
                        .ok_or_else(|| anyhow!("Missing outbound text payload id"))?;
                    let frames = byte_payload_frames(payload_id, text.as_bytes())?;
                    for frame in frames {
                        self.encrypt_and_send(&frame).await?;
                    }

                    let total_bytes = text.len() as u64;
                    self.update_state(
                        |state| {
                            if let Some(metadata) = state.transfer_metadata.as_mut() {
                                metadata.ack_bytes = total_bytes;
                            }
                            state.state = State::Finished;
                        },
                        true,
                    )
                    .await;
                    self.disconnection().await?;
                    return Ok(());
                }

                let ids: Vec<i64> = self.state.transferred_files.keys().cloned().collect();
                info!("We are sending: {:?}", ids);
                let mut ids_iter = ids.into_iter();
                // Loop through all files
                loop {
                    let current = match ids_iter.next() {
                        Some(i) => i,
                        None => {
                            info!("All files have been transferred");
                            self.update_state(
                                |e| {
                                    e.state = State::Finished;
                                },
                                true,
                            )
                            .await;
                            self.disconnection().await?;
                            // Breaking instead of NotAnError to allow peacefull termination
                            break;
                        }
                    };

                    // Loop until we reached end of file
                    loop {
                        if self.take_pending_cancel_request()? {
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

                        // Workaround to limit scope of the immutable borrow on self
                        let (curr_state, buffer, bytes_read) = {
                            let curr_state = match self.state.transferred_files.get(&current) {
                                Some(s) => s,
                                None => break,
                            };

                            info!("> Currently sending {:?}", curr_state.file_url);
                            if curr_state.bytes_transferred == curr_state.total_size {
                                debug!("File {current} finished");
                                self.update_state(
                                    |e| {
                                        e.transferred_files.remove(&current);
                                    },
                                    false,
                                )
                                .await;
                                break;
                            }

                            let mut buffer = vec![0u8; 512 * 1024];
                            let file = curr_state
                                .file
                                .as_ref()
                                .ok_or_else(|| anyhow!("Outbound file handle is missing"))?;
                            let total_size = u64::try_from(curr_state.total_size)
                                .map_err(|_| anyhow!("Negative outbound file size"))?;
                            let offset = u64::try_from(curr_state.bytes_transferred)
                                .map_err(|_| anyhow!("Negative outbound file offset"))?;
                            let bytes_read =
                                read_file_chunk(file, total_size, offset, &mut buffer)?;

                            (
                                InternalFileInfo {
                                    payload_id: curr_state.payload_id,
                                    file_url: curr_state.file_url.clone(),
                                    parent_folder: curr_state.parent_folder.clone(),
                                    bytes_transferred: curr_state.bytes_transferred,
                                    total_size: curr_state.total_size,
                                    file: None,
                                },
                                buffer,
                                bytes_read,
                            )
                        };

                        let sending_buffer = buffer[..bytes_read].to_vec();
                        info!(
                            "> File ready: {bytes_read} bytes && {} && left to send: {} with current offset: {}",
                            sending_buffer.len(),
                            curr_state.total_size - curr_state.bytes_transferred,
							curr_state.bytes_transferred
                        );

                        let payload_header = PayloadHeader {
                            id: Some(current),
                            r#type: Some(payload_header::PayloadType::File.into()),
                            total_size: Some(curr_state.total_size),
                            is_sensitive: Some(false),
                            file_name: curr_state
                                .file_url
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned()),
                            parent_folder: curr_state.parent_folder.clone(),
                        };

                        let wrapper = location_nearby_connections::OfflineFrame {
							version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
							v1: Some(location_nearby_connections::V1Frame {
								r#type: Some(
									location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
								),
								payload_transfer: Some(PayloadTransferFrame {
									packet_type: Some(PacketType::Data.into()),
									payload_chunk: Some(PayloadChunk {
										offset: Some(curr_state.bytes_transferred),
										flags: Some(0),
										body: Some(buffer[..bytes_read].to_vec()),
									}),
									payload_header: Some(payload_header.clone()),
									..Default::default()
								}),
								..Default::default()
							}),
						};

                        self.encrypt_and_send(&wrapper).await?;
                        self.update_state(
                            |e| {
                                if let Some(mu) = e.transferred_files.get_mut(&current) {
                                    mu.bytes_transferred += bytes_read as i64;
                                }

                                if let Some(tmd) = e.transfer_metadata.as_mut() {
                                    tmd.ack_bytes += bytes_read as u64;
                                }
                            },
                            true,
                        )
                        .await;

                        // If we just sent the last bytes of the file, mark it as finished
                        if curr_state.bytes_transferred + bytes_read as i64 == curr_state.total_size
                        {
                            debug!(
                                "File {current} finished, curr offset: {} over total: {}",
                                curr_state.bytes_transferred + bytes_read as i64,
                                curr_state.total_size
                            );

                            let wrapper = location_nearby_connections::OfflineFrame {
								version: Some(location_nearby_connections::offline_frame::Version::V1.into()),
								v1: Some(location_nearby_connections::V1Frame {
									r#type: Some(
										location_nearby_connections::v1_frame::FrameType::PayloadTransfer.into(),
									),
									payload_transfer: Some(PayloadTransferFrame {
										packet_type: Some(PacketType::Data.into()),
										payload_chunk: Some(PayloadChunk {
											offset: Some(curr_state.total_size),
											flags: Some(1), // lastChunk
											body: Some(vec![]),
										}),
										payload_header: Some(payload_header),
										..Default::default()
									}),
									..Default::default()
								}),
							};

                            self.encrypt_and_send(&wrapper).await?;
                            break;
                        }
                    }
                }
            }
            sharing_nearby::connection_response_frame::Status::Reject
            | sharing_nearby::connection_response_frame::Status::NotEnoughSpace
            | sharing_nearby::connection_response_frame::Status::UnsupportedAttachmentType
            | sharing_nearby::connection_response_frame::Status::TimedOut => {
                warn!("Cannot process: consent denied: {:?}", consent_status);
                self.update_state(
                    |e| {
                        e.state = State::Disconnected;
                    },
                    true,
                )
                .await;
                self.disconnection().await?;
                return Err(anyhow!(crate::errors::AppError::NotAnError));
            }
            sharing_nearby::connection_response_frame::Status::Unknown => {
                error!("Unknown consent type: aborting");
                self.update_state(
                    |e| {
                        e.state = State::Disconnected;
                    },
                    true,
                )
                .await;
                self.disconnection().await?;
                return Err(anyhow!(crate::errors::AppError::NotAnError));
            }
        }

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

        let auth_string = hkdf_extract_expand(auth_label, &derived_secret, &ukey_info, 32)?;
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
                e.decrypt_key = Some(server_key);
                e.recv_hmac_key = Some(server_hmac_key);
                e.encrypt_key = Some(client_key);
                e.send_hmac_key = Some(client_hmac_key);
                e.pin_code = Some(to_four_digit_string(&auth_string));
                e.encryption_done = true;

                if let Some(ref mut tm) = e.transfer_metadata {
                    tm.pin_code = Some(to_four_digit_string(&auth_string));
                }
            },
            true,
        )
        .await;

        info!("Pin code: {:?}", self.state.pin_code);

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

        if !inform {
            return;
        }

        let _ = self.sender.send(ChannelMessage {
            id: self.state.id.clone(),
            direction: ChannelDirection::LibToFront,
            rtype: Some(crate::channel::TransferType::Outbound),
            state: Some(self.state.state.clone()),
            meta: self.state.transfer_metadata.clone(),
            ..Default::default()
        });
        // Add a small sleep timer to allow the Tokio runtime to have
        // some spare time to process channel's message. Otherwise it
        // get spammed by new requests. Currently set to 10 micro secs.
        tokio::time::sleep(SANITY_DURATION).await;
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[cfg(target_os = "linux")]
    mod opened_sources {
        use std::io::{Seek, SeekFrom};
        use std::sync::Arc;

        use super::*;
        use crate::file_source::{OpenedFile, OpenedSelection, SelectionLimits};

        fn selection(path: &Path, name: &str) -> OpenedSelection {
            OpenedSelection::from_files(
                vec![OpenedFile::adopt(File::open(path).unwrap(), name.to_owned()).unwrap()],
                SelectionLimits::default(),
            )
            .unwrap()
        }

        async fn native_request(selected: OpenedSelection) -> (OutboundRequest, TcpStream) {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let peer = TcpStream::connect(listener.local_addr().unwrap())
                .await
                .unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            let (sender, _) = tokio::sync::broadcast::channel(16);
            let mut request = OutboundRequest::new(
                [1, 2, 3, 4],
                socket,
                "native-test".into(),
                sender,
                OutboundPayload::OpenedFiles(Arc::new(selected)),
                RemoteDeviceInfo {
                    name: "Test phone".into(),
                    device_type: DeviceType::Phone,
                },
            )
            .unwrap();
            // Established-session fixture: real framing/crypto with explicit test keys.
            request.state.encryption_done = true;
            request.state.encrypt_key = Some(vec![0x11; 32]);
            request.state.send_hmac_key = Some(vec![0x22; 32]);
            (request, peer)
        }

        async fn read_native_frame(socket: &mut TcpStream, sequence: i32) -> OfflineFrame {
            use tokio::io::AsyncReadExt;
            let mut length = [0; 4];
            socket.read_exact(&mut length).await.unwrap();
            let length = u32::from_be_bytes(length) as usize;
            assert!((1..=SANE_FRAME_LENGTH as usize).contains(&length));
            let mut bytes = vec![0; length];
            socket.read_exact(&mut bytes).await.unwrap();
            let secure = SecureMessage::decode(bytes.as_slice()).unwrap();
            let mut hmac = HmacSha256::new_from_slice(&[0x22; 32]).unwrap();
            hmac.update(&secure.header_and_body);
            hmac.verify_slice(&secure.signature).unwrap();
            let body = HeaderAndBody::decode(secure.header_and_body.as_slice()).unwrap();
            assert_eq!(body.header.encryption_scheme, EncScheme::Aes256Cbc as i32);
            assert_eq!(body.header.signature_scheme, SigScheme::HmacSha256 as i32);
            let mut cipher = Cipher::new_256(&[0x11; 32]);
            cipher.set_auto_padding(true);
            let plaintext = cipher.cbc_decrypt(body.header.iv(), &body.body);
            let message = DeviceToDeviceMessage::decode(plaintext.as_slice()).unwrap();
            assert_eq!(message.sequence_number(), sequence);
            OfflineFrame::decode(message.message()).unwrap()
        }

        fn paired_result() -> sharing_nearby::V1Frame {
            sharing_nearby::V1Frame {
                paired_key_result: Some(sharing_nearby::PairedKeyResultFrame::default()),
                ..Default::default()
            }
        }

        #[tokio::test]
        async fn given_native_selection_when_introduced_and_accepted_then_exact_file_frames_complete(
        ) {
            // Arrange.
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("source");
            std::fs::write(&path, b"abc").unwrap();
            let (mut request, mut peer) =
                native_request(selection(&path, "folder/display.txt")).await;
            std::fs::remove_file(&path).unwrap();
            assert!(request.cleanup_files.is_empty());
            assert_eq!(
                request
                    .state
                    .transfer_metadata
                    .as_ref()
                    .unwrap()
                    .files
                    .as_ref()
                    .unwrap(),
                &["folder/display.txt"]
            );
            // Act.
            request
                .process_paired_key_result(&paired_result())
                .await
                .unwrap();
            let intro = read_native_frame(&mut peer, 1).await;
            let body = intro
                .v1
                .unwrap()
                .payload_transfer
                .unwrap()
                .payload_chunk
                .unwrap()
                .body
                .unwrap();
            let introduction = sharing_nearby::Frame::decode(body.as_slice())
                .unwrap()
                .v1
                .unwrap()
                .introduction
                .unwrap();
            let intro_end = read_native_frame(&mut peer, 2).await;
            assert_eq!(
                intro_end
                    .v1
                    .unwrap()
                    .payload_transfer
                    .unwrap()
                    .payload_chunk
                    .unwrap()
                    .flags(),
                1
            );
            request
                .process_consent(&consent_frame(
                    sharing_nearby::connection_response_frame::Status::Accept,
                ))
                .await
                .unwrap();
            let data = read_native_frame(&mut peer, 3)
                .await
                .v1
                .unwrap()
                .payload_transfer
                .unwrap();
            let end = read_native_frame(&mut peer, 4)
                .await
                .v1
                .unwrap()
                .payload_transfer
                .unwrap();
            let disconnect = read_native_frame(&mut peer, 5).await.v1.unwrap();
            // Assert.
            assert_eq!(introduction.file_metadata.len(), 1);
            let metadata = &introduction.file_metadata[0];
            assert_eq!(metadata.name(), "display.txt");
            assert_eq!(metadata.parent_folder.as_deref(), Some("folder"));
            assert_eq!(metadata.size(), 3);
            let header = data.payload_header.unwrap();
            assert_eq!(header.id(), metadata.payload_id());
            assert_eq!(header.total_size(), 3);
            assert_eq!(header.file_name.as_deref(), Some("display.txt"));
            assert_eq!(header.parent_folder.as_deref(), Some("folder"));
            let chunk = data.payload_chunk.unwrap();
            assert_eq!(chunk.offset(), 0);
            assert_eq!(chunk.body(), b"abc");
            assert_eq!(end.payload_chunk.as_ref().unwrap().flags(), 1);
            assert_eq!(end.payload_chunk.as_ref().unwrap().offset(), 3);
            assert_eq!(
                disconnect.r#type(),
                location_nearby_connections::v1_frame::FrameType::Disconnection
            );
            assert_eq!(request.state.state, State::Finished);
            assert_eq!(
                request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
                3
            );
        }

        #[tokio::test]
        async fn given_prepared_source_changes_when_consent_arrives_then_no_false_completion_occurs(
        ) {
            // Arrange / Act / Assert.
            for bytes in [b"a".as_slice(), b"abcdef".as_slice()] {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("source");
                std::fs::write(&path, b"abc").unwrap();
                let (mut request, mut peer) = native_request(selection(&path, "display")).await;
                request
                    .process_paired_key_result(&paired_result())
                    .await
                    .unwrap();
                read_native_frame(&mut peer, 1).await;
                read_native_frame(&mut peer, 2).await;
                std::fs::write(&path, bytes).unwrap();
                let result = request
                    .process_consent(&consent_frame(
                        sharing_nearby::connection_response_frame::Status::Accept,
                    ))
                    .await;
                assert!(result
                    .unwrap_err()
                    .to_string()
                    .contains("Source size changed"));
                assert_ne!(request.state.state, State::Finished);
                assert_eq!(
                    request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
                    0
                );
                assert_eq!(std::fs::read(path).unwrap(), bytes);
            }
        }

        #[tokio::test]
        async fn given_native_transfer_cancelled_before_read_when_accepted_then_original_is_preserved(
        ) {
            // Arrange.
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("source");
            std::fs::write(&path, b"abc").unwrap();
            let (mut request, mut peer) = native_request(selection(&path, "display")).await;
            request
                .process_paired_key_result(&paired_result())
                .await
                .unwrap();
            read_native_frame(&mut peer, 1).await;
            read_native_frame(&mut peer, 2).await;
            request
                .sender
                .send(ChannelMessage {
                    id: "native-test".into(),
                    direction: ChannelDirection::FrontToLib,
                    action: Some(ChannelAction::CancelTransfer),
                    ..Default::default()
                })
                .unwrap();
            // Act.
            let result = request
                .process_consent(&consent_frame(
                    sharing_nearby::connection_response_frame::Status::Accept,
                ))
                .await;
            // Assert.
            assert!(result.is_err());
            assert_eq!(request.state.state, State::Cancelled);
            assert_eq!(
                request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
                0
            );
            assert_eq!(
                read_native_frame(&mut peer, 3).await.v1.unwrap().r#type(),
                location_nearby_connections::v1_frame::FrameType::Disconnection
            );
            drop(request);
            assert_eq!(std::fs::read(path).unwrap(), b"abc");
        }

        #[test]
        fn given_native_files_when_prepared_then_protocol_names_types_sizes_and_parents_match() {
            // Arrange.
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("source");
            std::fs::write(&path, b"abc").unwrap();
            let inputs = [
                ("folder/photo.png", file_metadata::Type::Image),
                ("video.mp4", file_metadata::Type::Video),
                ("audio.mp3", file_metadata::Type::Audio),
                ("client.apk", file_metadata::Type::App),
                ("notes.txt", file_metadata::Type::Unknown),
            ];
            let selected = OpenedSelection::from_files(
                inputs
                    .iter()
                    .map(|(name, _)| {
                        OpenedFile::adopt(File::open(&path).unwrap(), (*name).to_owned()).unwrap()
                    })
                    .collect(),
                SelectionLimits::default(),
            )
            .unwrap();
            // Act.
            let prepared = prepare_opened_outbound_files(&selected).unwrap();
            // Assert.
            assert_eq!(prepared.total_bytes, 15);
            assert_eq!(prepared.metadata.len(), 5);
            assert_eq!(prepared.files.len(), 5);
            for (metadata, (relative, kind)) in prepared.metadata.iter().zip(inputs) {
                assert_eq!(metadata.name(), relative.rsplit('/').next().unwrap());
                assert_eq!(metadata.size(), 3);
                assert_eq!(metadata.r#type(), kind);
                assert_eq!(
                    metadata.parent_folder.as_deref(),
                    relative.rsplit_once('/').map(|(parent, _)| parent)
                );
                let file = &prepared.files[&metadata.payload_id()];
                assert_eq!(file.total_size, 3);
                assert_eq!(file.bytes_transferred, 0);
                assert_eq!(file.parent_folder, metadata.parent_folder);
            }
        }

        #[test]
        fn given_unlinked_native_source_when_prepared_then_no_path_reopen_occurs() {
            // Arrange.
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("source");
            std::fs::write(&path, b"original").unwrap();
            let selected = selection(&path, "folder/display.txt");
            std::fs::remove_file(&path).unwrap();
            // Act.
            let prepared = prepare_opened_outbound_files(&selected).unwrap();
            let info = prepared.files.values().next().unwrap();
            let mut bytes = [0; 8];
            read_file_chunk(info.file.as_ref().unwrap(), 8, 0, &mut bytes).unwrap();
            // Assert.
            assert_eq!(&bytes, b"original");
            assert_eq!(info.file_url, Path::new("display.txt"));
            assert_eq!(info.parent_folder.as_deref(), Some("folder"));
        }

        #[test]
        fn given_native_source_size_changed_when_prepared_then_transfer_is_rejected() {
            // Arrange / Act / Assert.
            for bytes in [b"a".as_slice(), b"abcdef".as_slice()] {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("source");
                std::fs::write(&path, b"abc").unwrap();
                let selected = selection(&path, "display");
                std::fs::write(path, bytes).unwrap();
                assert!(prepare_opened_outbound_files(&selected).is_err());
            }
        }

        #[test]
        fn given_native_payload_when_serialized_or_deserialized_then_json_cannot_carry_handles() {
            // Arrange.
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("source");
            std::fs::write(&path, b"abc").unwrap();
            let payload = OutboundPayload::OpenedFiles(Arc::new(selection(&path, "display")));
            // Act / Assert.
            assert!(serde_json::to_string(&payload).is_err());
            for json in [
                r#"{"OpenedFiles":[]}"#,
                r#"{"OpenedFiles":{}}"#,
                r#"{"OpenedFiles":null}"#,
            ] {
                assert!(serde_json::from_str::<OutboundPayload>(json).is_err());
            }
            let declaration = OutboundPayload::decl();
            assert!(!declaration.contains("OpenedFiles"));
            for legacy in ["Files", "EphemeralFiles", "Text"] {
                assert!(declaration.contains(legacy));
            }
        }

        #[test]
        fn given_legacy_payloads_when_round_tripped_then_existing_json_contract_is_preserved() {
            // Arrange / Act / Assert.
            for json in [
                r#"{"Files":["source"]}"#,
                r#"{"EphemeralFiles":["source"]}"#,
                r#"{"Text":"hello"}"#,
            ] {
                let payload: OutboundPayload = serde_json::from_str(json).unwrap();
                assert_eq!(serde_json::to_string(&payload).unwrap(), json);
            }
        }

        #[test]
        fn given_native_payload_when_dropped_then_original_source_has_no_cleanup_capability() {
            // Arrange.
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("source");
            std::fs::write(&path, b"abc").unwrap();
            let payload = OutboundPayload::OpenedFiles(Arc::new(selection(&path, "display")));
            // Act.
            drop(payload);
            // Assert.
            assert_eq!(std::fs::read(path).unwrap(), b"abc");
        }

        #[test]
        fn given_prepared_native_handles_when_shared_position_changes_then_readers_stay_independent(
        ) {
            // Arrange.
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("source");
            std::fs::write(&path, b"abcdef").unwrap();
            let selected = selection(&path, "display");
            let first = prepare_opened_outbound_files(&selected).unwrap();
            let second = prepare_opened_outbound_files(&selected).unwrap();
            let a = first.files.values().next().unwrap().file.as_ref().unwrap();
            let b = second.files.values().next().unwrap().file.as_ref().unwrap();
            // Act.
            let mut other = a.try_clone().unwrap();
            other.seek(SeekFrom::Start(5)).unwrap();
            let mut bytes = [0; 2];
            read_file_chunk(b, 6, 0, &mut bytes).unwrap();
            // Assert.
            assert_eq!(&bytes, b"ab");
            assert_eq!(other.stream_position().unwrap(), 5);
        }
    }

    #[test]
    fn outbound_file_preparation_classifies_files_and_skips_missing_paths() {
        let root = std::env::temp_dir().join(format!(
            "rquickshare-outbound-test-{}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&root).unwrap();

        let image = root.join("photo.png");
        let app = root.join("client.apk");
        let text = root.join("notes.txt");
        let missing = root.join("missing.bin");

        std::fs::write(&image, [1_u8, 2, 3]).unwrap();
        std::fs::write(&app, [4_u8, 5]).unwrap();
        std::fs::write(&text, [6_u8]).unwrap();

        let inputs = vec![
            image.to_string_lossy().into_owned(),
            app.to_string_lossy().into_owned(),
            text.to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
        ];

        let prepared = prepare_outbound_files(&inputs).unwrap();
        let metadata = prepared.metadata;
        let transferred = prepared.files;

        assert_eq!(metadata.len(), 3);
        assert_eq!(transferred.len(), 3);
        assert_eq!(prepared.total_bytes, 6);

        let by_name = metadata
            .iter()
            .map(|item| (item.name().to_owned(), item))
            .collect::<HashMap<_, _>>();

        assert_eq!(by_name["photo.png"].r#type(), file_metadata::Type::Image);
        assert_eq!(by_name["client.apk"].r#type(), file_metadata::Type::App);
        assert_eq!(by_name["notes.txt"].r#type(), file_metadata::Type::Unknown);

        let payload_ids = metadata
            .iter()
            .map(FileMetadata::payload_id)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(payload_ids.len(), 3);

        for item in metadata {
            let internal = &transferred[&item.payload_id()];
            assert_eq!(internal.total_size, item.size());
            assert!(internal.file.is_some());
            assert_eq!(
                internal.file_url.file_name().unwrap().to_string_lossy(),
                item.name()
            );
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn outbound_file_preparation_classifies_audio_and_video() {
        let root = std::env::temp_dir().join(format!(
            "rquickshare-outbound-media-test-{}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&root).unwrap();

        let video = root.join("clip.mp4");
        let audio = root.join("sound.mp3");
        std::fs::write(&video, [1_u8]).unwrap();
        std::fs::write(&audio, [2_u8]).unwrap();

        let prepared = prepare_outbound_files(&[
            video.to_string_lossy().into_owned(),
            audio.to_string_lossy().into_owned(),
        ])
        .unwrap();

        let by_name = prepared
            .metadata
            .iter()
            .map(|item| (item.name().to_owned(), item.r#type()))
            .collect::<HashMap<_, _>>();

        assert_eq!(by_name["clip.mp4"], file_metadata::Type::Video);
        assert_eq!(by_name["sound.mp3"], file_metadata::Type::Audio);
        assert_eq!(prepared.total_bytes, 2);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn outbound_file_preparation_rejects_empty_input() {
        assert!(prepare_outbound_files(&[]).is_err());
    }

    #[test]
    fn outbound_folder_preparation_preserves_selected_root_and_nested_parents() {
        let base = std::env::temp_dir().join(format!(
            "rquickshare-outbound-folder-test-{}",
            rand::random::<u64>()
        ));
        let root = base.join("Trip");
        let nested = root.join("photos");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("readme.txt"), [1_u8]).unwrap();
        std::fs::write(nested.join("sunset.jpg"), [2_u8, 3]).unwrap();

        let prepared = prepare_outbound_files(&[root.to_string_lossy().into_owned()]).unwrap();
        assert_eq!(prepared.metadata.len(), 2);
        assert_eq!(prepared.total_bytes, 3);

        let by_name = prepared
            .metadata
            .iter()
            .map(|metadata| (metadata.name().to_owned(), metadata))
            .collect::<HashMap<_, _>>();

        assert_eq!(by_name["readme.txt"].parent_folder(), "Trip");
        assert_eq!(by_name["sunset.jpg"].parent_folder(), "Trip/photos");

        for metadata in &prepared.metadata {
            assert_eq!(
                prepared.files[&metadata.payload_id()]
                    .parent_folder
                    .as_deref(),
                metadata.parent_folder.as_deref()
            );
        }

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn outbound_duplicate_selected_root_names_are_kept_separate() {
        let base = std::env::temp_dir().join(format!(
            "rquickshare-outbound-duplicate-root-test-{}",
            rand::random::<u64>()
        ));
        let first = base.join("one").join("Trip");
        let second = base.join("two").join("Trip");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("one.txt"), [1_u8]).unwrap();
        std::fs::write(second.join("two.txt"), [2_u8]).unwrap();

        let prepared = prepare_outbound_files(&[
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
        ])
        .unwrap();

        let parents = prepared
            .metadata
            .iter()
            .map(|metadata| metadata.parent_folder().to_owned())
            .collect::<HashSet<_>>();
        assert_eq!(
            parents,
            HashSet::from(["Trip".to_owned(), "1_Trip".to_owned()])
        );

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn outbound_folder_preparation_rejects_empty_folder() {
        let base = std::env::temp_dir().join(format!(
            "rquickshare-outbound-empty-folder-test-{}",
            rand::random::<u64>()
        ));
        let root = base.join("Empty");
        std::fs::create_dir_all(&root).unwrap();

        assert!(prepare_outbound_files(&[root.to_string_lossy().into_owned()]).is_err());

        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn outbound_folder_preparation_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;

        let base = std::env::temp_dir().join(format!(
            "rquickshare-outbound-symlink-test-{}",
            rand::random::<u64>()
        ));
        let root = base.join("Share");
        std::fs::create_dir_all(&root).unwrap();
        let outside = base.join("secret.txt");
        std::fs::write(&outside, [9_u8]).unwrap();
        std::fs::write(root.join("public.txt"), [1_u8]).unwrap();
        symlink(&outside, root.join("linked-secret.txt")).unwrap();

        let prepared = prepare_outbound_files(&[root.to_string_lossy().into_owned()]).unwrap();

        assert_eq!(prepared.metadata.len(), 1);
        assert_eq!(prepared.metadata[0].name(), "public.txt");
        assert_eq!(prepared.metadata[0].parent_folder(), "Share");

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn outbound_text_classification_covers_common_quick_share_types() {
        assert_eq!(
            classify_outbound_text("https://example.com/path"),
            text_metadata::Type::Url
        );
        assert_eq!(
            classify_outbound_text("+351 912 345 678"),
            text_metadata::Type::PhoneNumber
        );
        assert_eq!(
            classify_outbound_text("hello from Linux"),
            text_metadata::Type::Text
        );
    }

    #[test]
    fn outbound_text_classification_handles_boundary_shapes() {
        for (value, expected) in [
            ("  HTTPS://example.com/path  ", text_metadata::Type::Url),
            ("http://example.com", text_metadata::Type::Url),
            ("+351 (912) 345-678", text_metadata::Type::PhoneNumber),
            ("123456", text_metadata::Type::Text),
            ("Rua do Souto 10, Braga", text_metadata::Type::Text),
        ] {
            assert_eq!(classify_outbound_text(value), expected, "{value}");
        }
    }

    #[test]
    fn outbound_text_preparation_enforces_title_and_payload_boundaries() {
        let utf8 = "é".repeat(65);
        let prepared = prepare_outbound_text(&utf8).unwrap();
        assert_eq!(
            prepared.metadata.text_title().chars().count(),
            64,
            "title must be truncated by Unicode scalar value rather than byte"
        );
        assert_eq!(prepared.metadata.size(), utf8.len() as i64);

        let max = "a".repeat(SANE_FRAME_LENGTH as usize);
        assert!(prepare_outbound_text(&max).is_ok());

        let too_large = "a".repeat(SANE_FRAME_LENGTH as usize + 1);
        assert!(prepare_outbound_text(&too_large).is_err());
    }

    #[test]
    fn byte_payload_frames_cover_empty_and_chunk_boundaries() {
        const CHUNK_SIZE: usize = 256 * 1024;

        for (length, expected_frames) in [
            (0_usize, 1_usize),
            (1, 2),
            (CHUNK_SIZE, 2),
            (CHUNK_SIZE + 1, 3),
        ] {
            let body = vec![0x5a; length];
            let frames = byte_payload_frames(91, &body).unwrap();
            assert_eq!(frames.len(), expected_frames, "length={length}");

            let terminal = frames
                .last()
                .unwrap()
                .v1
                .as_ref()
                .unwrap()
                .payload_transfer
                .as_ref()
                .unwrap()
                .payload_chunk
                .as_ref()
                .unwrap();
            assert_eq!(terminal.offset(), length as i64);
            assert_eq!(terminal.flags(), 1);
            assert!(terminal.body().is_empty());
        }

        let oversized = vec![0_u8; SANE_FRAME_LENGTH as usize + 1];
        assert!(byte_payload_frames(91, &oversized).is_err());
    }

    #[test]
    fn outbound_text_preparation_rejects_empty_and_preserves_utf8_size() {
        assert!(prepare_outbound_text("").is_err());

        let prepared = prepare_outbound_text("olá 🌍").unwrap();
        assert_eq!(prepared.metadata.size(), "olá 🌍".len() as i64);
        assert_eq!(prepared.metadata.payload_id(), prepared.payload_id);
        assert_eq!(prepared.bytes, "olá 🌍".as_bytes());
        assert_eq!(prepared.payload_type, TextPayloadType::Text);
    }

    #[test]
    fn byte_payload_frames_are_chunked_and_terminated() {
        let body = vec![7_u8; 600 * 1024];
        let frames = byte_payload_frames(77, &body).unwrap();

        assert_eq!(frames.len(), 4);

        let mut offsets = Vec::new();
        let mut reconstructed = Vec::new();
        for frame in &frames[..frames.len() - 1] {
            let transfer = frame
                .v1
                .as_ref()
                .unwrap()
                .payload_transfer
                .as_ref()
                .unwrap();
            let header = transfer.payload_header.as_ref().unwrap();
            let chunk = transfer.payload_chunk.as_ref().unwrap();

            assert_eq!(header.id(), 77);
            assert_eq!(header.r#type(), payload_header::PayloadType::Bytes);
            assert_eq!(header.total_size(), body.len() as i64);
            assert_eq!(chunk.flags(), 0);
            offsets.push(chunk.offset());
            reconstructed.extend_from_slice(chunk.body());
        }

        assert_eq!(offsets, vec![0, 256 * 1024, 512 * 1024]);
        assert_eq!(reconstructed, body);

        let final_transfer = frames
            .last()
            .unwrap()
            .v1
            .as_ref()
            .unwrap()
            .payload_transfer
            .as_ref()
            .unwrap();
        let final_chunk = final_transfer.payload_chunk.as_ref().unwrap();
        assert_eq!(final_chunk.offset(), body.len() as i64);
        assert_eq!(final_chunk.flags(), 1);
        assert!(final_chunk.body().is_empty());
    }

    #[test]
    fn managed_ephemeral_file_accepts_only_owned_temp_names() {
        let valid = std::env::temp_dir().join(format!(
            "{MANAGED_EPHEMERAL_FILE_PREFIX}{}-{}-0.png",
            std::process::id(),
            rand::random::<u64>()
        ));
        assert!(ManagedEphemeralFile::try_from_path(&valid).is_ok());

        for invalid in [
            std::env::temp_dir().join("other-app.png"),
            std::env::temp_dir().join("rquickshare-clipboard-user-file.png"),
            std::env::temp_dir().join("rquickshare-clipboard-1-2-0.jpg"),
            std::env::temp_dir()
                .join("nested")
                .join("rquickshare-clipboard-1-2-0.png"),
            PathBuf::from("/etc/passwd"),
        ] {
            assert!(
                ManagedEphemeralFile::try_from_path(&invalid).is_err(),
                "unmanaged path unexpectedly accepted: {}",
                invalid.display()
            );
        }
    }

    #[test]
    fn managed_ephemeral_file_removal_is_idempotent() {
        let path = std::env::temp_dir().join(format!(
            "{MANAGED_EPHEMERAL_FILE_PREFIX}{}-{}-0.png",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::write(&path, [1_u8, 2, 3]).unwrap();

        let managed = ManagedEphemeralFile::try_from_path(&path).unwrap();
        managed.remove().unwrap();
        assert!(!path.exists());
        managed.remove().unwrap();
    }

    #[tokio::test]
    async fn ephemeral_request_rejects_unmanaged_paths() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let _peer = tokio::net::TcpStream::connect(address).await.unwrap();
        let (socket, _) = listener.accept().await.unwrap();
        let (sender, _receiver) = tokio::sync::broadcast::channel(16);

        let result = OutboundRequest::new(
            [1, 2, 3, 4],
            socket,
            "unmanaged-ephemeral-transfer".to_owned(),
            sender,
            OutboundPayload::EphemeralFiles(vec!["/etc/passwd".to_owned()]),
            RemoteDeviceInfo {
                name: "Test phone".to_owned(),
                device_type: DeviceType::Phone,
            },
        );

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn normal_files_never_acquire_cleanup_capability() {
        let path = std::env::temp_dir().join(format!(
            "{MANAGED_EPHEMERAL_FILE_PREFIX}{}-{}-0.png",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::write(&path, [9_u8]).unwrap();

        let (request, _peer) = test_request(vec![path.to_string_lossy().into_owned()]).await;
        assert!(request.cleanup_files.is_empty());
        drop(request);
        assert!(path.exists());

        std::fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn ephemeral_files_are_removed_when_request_is_dropped() {
        let image = std::env::temp_dir().join(format!(
            "{MANAGED_EPHEMERAL_FILE_PREFIX}{}-{}-0.png",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::write(&image, [1_u8, 2, 3, 4]).unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let _peer = tokio::net::TcpStream::connect(address).await.unwrap();
        let (socket, _) = listener.accept().await.unwrap();
        let (sender, _receiver) = tokio::sync::broadcast::channel(16);

        let request = OutboundRequest::new(
            [1, 2, 3, 4],
            socket,
            "ephemeral-transfer".to_owned(),
            sender,
            OutboundPayload::EphemeralFiles(vec![image.to_string_lossy().into_owned()]),
            RemoteDeviceInfo {
                name: "Test phone".to_owned(),
                device_type: DeviceType::Phone,
            },
        )
        .unwrap();

        assert!(image.exists());
        drop(request);
        assert!(!image.exists());
    }

    async fn test_request(files: Vec<String>) -> (OutboundRequest, tokio::net::TcpStream) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::net::TcpStream::connect(address).await.unwrap();
        let (socket, _) = listener.accept().await.unwrap();
        let (sender, _receiver) = tokio::sync::broadcast::channel(16);

        let mut request = OutboundRequest::new(
            [1, 2, 3, 4],
            socket,
            "test-outbound".to_owned(),
            sender,
            OutboundPayload::Files(files),
            RemoteDeviceInfo {
                name: "Test phone".to_owned(),
                device_type: DeviceType::Phone,
            },
        )
        .unwrap();
        request.state.encryption_done = false;
        (request, peer)
    }

    fn consent_frame(
        status: sharing_nearby::connection_response_frame::Status,
    ) -> sharing_nearby::V1Frame {
        sharing_nearby::V1Frame {
            r#type: Some(sharing_nearby::v1_frame::FrameType::Response.into()),
            connection_response: Some(sharing_nearby::ConnectionResponseFrame {
                status: Some(status.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    async fn read_offline_frame(
        socket: &mut tokio::net::TcpStream,
    ) -> location_nearby_connections::OfflineFrame {
        use tokio::io::AsyncReadExt;

        let mut length = [0_u8; 4];
        socket.read_exact(&mut length).await.unwrap();
        let length = u32::from_be_bytes(length) as usize;
        let mut payload = vec![0_u8; length];
        socket.read_exact(&mut payload).await.unwrap();
        location_nearby_connections::OfflineFrame::decode(payload.as_slice()).unwrap()
    }

    #[tokio::test]
    async fn accepted_empty_transfer_finishes_and_disconnects() {
        let (mut request, mut peer) = test_request(vec![]).await;
        let frame = consent_frame(sharing_nearby::connection_response_frame::Status::Accept);

        request.process_consent(&frame).await.unwrap();

        assert_eq!(request.state.state, State::Finished);
        let disconnect = read_offline_frame(&mut peer).await;
        assert_eq!(
            disconnect.v1.unwrap().r#type(),
            location_nearby_connections::v1_frame::FrameType::Disconnection
        );
    }

    #[tokio::test]
    async fn denied_consent_disconnects_cleanly() {
        for status in [
            sharing_nearby::connection_response_frame::Status::Reject,
            sharing_nearby::connection_response_frame::Status::NotEnoughSpace,
            sharing_nearby::connection_response_frame::Status::UnsupportedAttachmentType,
            sharing_nearby::connection_response_frame::Status::TimedOut,
        ] {
            let (mut request, mut peer) = test_request(vec![]).await;
            let result = request.process_consent(&consent_frame(status)).await;

            assert!(result.is_err());
            assert_eq!(request.state.state, State::Disconnected);

            let disconnect = read_offline_frame(&mut peer).await;
            assert_eq!(
                disconnect.v1.unwrap().r#type(),
                location_nearby_connections::v1_frame::FrameType::Disconnection
            );
        }
    }

    #[tokio::test]
    async fn malformed_consent_response_is_rejected() {
        let (mut request, _peer) = test_request(vec![]).await;

        let wrong_type = sharing_nearby::V1Frame {
            r#type: Some(sharing_nearby::v1_frame::FrameType::Introduction.into()),
            ..Default::default()
        };
        assert!(request.process_consent(&wrong_type).await.is_err());
        assert_eq!(request.state.state, State::Initial);

        let missing_response = sharing_nearby::V1Frame {
            r#type: Some(sharing_nearby::v1_frame::FrameType::Response.into()),
            ..Default::default()
        };
        assert!(request.process_consent(&missing_response).await.is_err());
        assert_eq!(request.state.state, State::Initial);
    }

    #[test]
    fn test_is_cancel_request_matches_only_current_frontend_transfer() {
        let cancel = ChannelMessage {
            id: "transfer-1".to_owned(),
            direction: ChannelDirection::FrontToLib,
            action: Some(ChannelAction::CancelTransfer),
            ..Default::default()
        };
        assert!(is_cancel_request(&cancel, "transfer-1"));
        assert!(!is_cancel_request(&cancel, "transfer-2"));

        let lib_message = ChannelMessage {
            direction: ChannelDirection::LibToFront,
            ..cancel.clone()
        };
        assert!(!is_cancel_request(&lib_message, "transfer-1"));

        let accept = ChannelMessage {
            action: Some(ChannelAction::AcceptTransfer),
            ..cancel
        };
        assert!(!is_cancel_request(&accept, "transfer-1"));
    }
}
