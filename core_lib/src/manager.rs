use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::channel::{ChannelDirection, ChannelMessage, TransferType};
use crate::errors::AppError;
use crate::hdl::info::TransferMetadata;
use crate::hdl::{
    peek_client_introduction, BwuClientIntroductionIdentity, BwuRouter, InboundRequest,
    OutboundPayload, OutboundRequest, State,
};
use crate::utils::RemoteDeviceInfo;

const INNER_NAME: &str = "TcpServer";
const MAX_UI_ERROR_CHARS: usize = 512;
const BWU_PEEK_LIMIT: usize = 8 * 1024;

async fn route_bandwidth_upgrade_if_pending(
    socket: TcpStream,
    router: &BwuRouter,
) -> Result<Option<TcpStream>, anyhow::Error> {
    if !router.has_pending().await {
        return Ok(Some(socket));
    }

    let mut buffer = vec![0_u8; BWU_PEEK_LIMIT];
    let endpoint_id = match tokio::time::timeout(Duration::from_millis(750), async {
        loop {
            let count = socket.peek(&mut buffer).await?;
            if count == 0 {
                return Ok::<Option<BwuClientIntroductionIdentity>, anyhow::Error>(None);
            }

            if count >= 4 {
                let frame_len =
                    u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
                if frame_len == 0 || frame_len > BWU_PEEK_LIMIT.saturating_sub(4) {
                    return Ok(None);
                }

                let total = 4 + frame_len;
                if count >= total {
                    return peek_client_introduction(&buffer[..total]);
                }
            }

            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    {
        Ok(result) => result?,
        Err(_) => None,
    };

    let Some(identity) = endpoint_id else {
        return Ok(Some(socket));
    };

    match router
        .route_with_alias(
            &identity.endpoint_id,
            identity.last_endpoint_id.as_deref(),
            socket,
        )
        .await
    {
        Ok(routed_as) => {
            if routed_as != identity.endpoint_id {
                info!(
                    "{INNER_NAME}: routed dynamic BWU endpoint current={} last={:?} as {routed_as}",
                    identity.endpoint_id, identity.last_endpoint_id
                );
            }
            Ok(None)
        }
        Err(_socket) => {
            warn!(
                "{INNER_NAME}: received BWU CLIENT_INTRODUCTION for unregistered endpoint current={} last={:?}",
                identity.endpoint_id,
                identity.last_endpoint_id
            );
            Ok(None)
        }
    }
}

fn error_for_ui(error: &anyhow::Error) -> String {
    error.to_string().chars().take(MAX_UI_ERROR_CHARS).collect()
}

fn outbound_failure(
    id: &str,
    state: &State,
    meta: Option<TransferMetadata>,
    error: &anyhow::Error,
) -> Option<ChannelMessage> {
    if matches!(
        state,
        State::Finished | State::Cancelled | State::Rejected | State::Disconnected
    ) {
        return None;
    }
    Some(ChannelMessage {
        id: id.to_owned(),
        direction: ChannelDirection::LibToFront,
        rtype: Some(TransferType::Outbound),
        state: Some(State::Disconnected),
        meta,
        error: Some(error_for_ui(error)),
        ..Default::default()
    })
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct SendInfo {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub ob: OutboundPayload,
}

pub struct TcpServer {
    endpoint_id: [u8; 4],
    tcp_listener: TcpListener,
    sender: Sender<ChannelMessage>,
    connect_receiver: Receiver<SendInfo>,
    bwu_router: BwuRouter,
}

impl TcpServer {
    pub fn new(
        endpoint_id: [u8; 4],
        tcp_listener: TcpListener,
        sender: Sender<ChannelMessage>,
        connect_receiver: Receiver<SendInfo>,
        bwu_router: BwuRouter,
    ) -> Result<Self, anyhow::Error> {
        Ok(Self {
            endpoint_id,
            tcp_listener,
            sender,
            connect_receiver,
            bwu_router,
        })
    }

    pub async fn run(&mut self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        info!("{INNER_NAME}: service starting");

        loop {
            let cctk = ctk.clone();

            tokio::select! {
                _ = ctk.cancelled() => {
                    info!("{INNER_NAME}: tracker cancelled, breaking");
                    break;
                }
                Some(i) = self.connect_receiver.recv() => {
                    info!("{INNER_NAME}: outbound request queued for {}", i.addr);
                    self.execute_outbound(cctk, i).await;
                }
                r = self.tcp_listener.accept() => {
                    match r {
                        Ok((socket, remote_addr)) => {
                            info!("{INNER_NAME}: accepted inbound client from {remote_addr}");
                            let esender = self.sender.clone();
                            let csender = self.sender.clone();
                            let bwu_router = self.bwu_router.clone();

                            tokio::spawn(async move {
                                let socket = match route_bandwidth_upgrade_if_pending(
                                    socket,
                                    &bwu_router,
                                )
                                .await
                                {
                                    Ok(Some(socket)) => socket,
                                    Ok(None) => {
                                        info!(
                                            "{INNER_NAME}: routed bandwidth-upgrade client from {remote_addr}"
                                        );
                                        return;
                                    }
                                    Err(error) => {
                                        warn!(
                                            "{INNER_NAME}: failed to classify inbound client {remote_addr}: {error}"
                                        );
                                        return;
                                    }
                                };

                                let mut ir = InboundRequest::new(socket, remote_addr.to_string(), csender);

                                loop {
                                    match ir.handle().await {
                                        Ok(_) => {},
                                        Err(e) => match e.downcast_ref() {
                                            Some(AppError::NotAnError) => {
                                                debug!("{INNER_NAME}: inbound session {remote_addr} completed");
                                                break;
                                            },
                                            None => {
                                                if ir.state.state == State::Initial {
                                                    warn!(
                                                        "{INNER_NAME}: inbound client {remote_addr} failed during initial handshake: {e}"
                                                    );
                                                    break;
                                                }

                                                if ir.state.state != State::Finished {
                                                    let _ = esender.send(ChannelMessage {
                                                        id: remote_addr.to_string(),
                                                        direction: ChannelDirection::LibToFront,
                                                        state: Some(State::Disconnected),
                                                        meta: ir.state.transfer_metadata.clone(),
                                                        error: Some(error_for_ui(&e)),
                                                        ..Default::default()
                                                    });
                                                }
                                                error!(
                                                    "{INNER_NAME}: error while handling inbound client {remote_addr}: {e} ({:?})",
                                                    ir.state.state
                                                );
                                                break;
                                            }
                                        },
                                    }
                                }
                            });
                        },
                        Err(err) => {
                            error!("{INNER_NAME}: error accepting: {err}");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn execute_outbound(&self, ctk: CancellationToken, info: SendInfo) {
        let id = info.id.clone();
        if let Err(error) = self.connect(ctk, info).await {
            error!("{INNER_NAME}: error sending: {error}");
            if let Some(message) = outbound_failure(&id, &State::Initial, None, &error) {
                let _ = self.sender.send(message);
            }
        }
    }

    /// To be called inside a separate task if we want to handle concurrency
    pub async fn connect(&self, ctk: CancellationToken, si: SendInfo) -> Result<(), anyhow::Error> {
        info!("{INNER_NAME}: connecting to outbound peer {}", si.addr);
        let socket = TcpStream::connect(si.addr.clone()).await?;
        info!("{INNER_NAME}: TCP connection established to {}", si.addr);

        let mut or = OutboundRequest::new(
            self.endpoint_id,
            socket,
            si.id,
            self.sender.clone(),
            si.ob,
            RemoteDeviceInfo {
                device_type: crate::DeviceType::Unknown,
                name: si.name,
            },
        )?;

        // Send connection request
        or.send_connection_request().await?;
        debug!("{INNER_NAME}: connection request sent to {}", si.addr);
        // Send UKEY init
        or.send_ukey2_client_init().await?;
        debug!("{INNER_NAME}: UKEY2 client init sent to {}", si.addr);

        loop {
            tokio::select! {
                _ = ctk.cancelled() => {
                    info!("{INNER_NAME}: tracker cancelled, breaking");
                    break;
                },
                r = or.handle() => {
                    if let Err(e) = r {
                        match e.downcast_ref() {
                            Some(AppError::NotAnError) => break,
                            None => {
                                if let Some(message) = outbound_failure(
                                    &or.state.id,
                                    &or.state.state,
                                    or.state.transfer_metadata.clone(),
                                    &e,
                                ) {
                                    let _ = self.sender.send(message);
                                }
                                error!(
                                    "{INNER_NAME}: error while handling outbound peer {}: {e} ({:?})",
                                    si.addr,
                                    or.state.state
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use prost::Message;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;
    use crate::location_nearby_connections::bandwidth_upgrade_negotiation_frame::{
        ClientIntroduction, EventType,
    };
    use crate::location_nearby_connections::{
        offline_frame, v1_frame, BandwidthUpgradeNegotiationFrame, OfflineFrame, V1Frame,
    };

    fn client_introduction(endpoint_id: &str) -> Vec<u8> {
        client_introduction_with_last(endpoint_id, None)
    }

    fn client_introduction_with_last(endpoint_id: &str, last_endpoint_id: Option<&str>) -> Vec<u8> {
        let frame = OfflineFrame {
            version: Some(offline_frame::Version::V1.into()),
            v1: Some(V1Frame {
                r#type: Some(v1_frame::FrameType::BandwidthUpgradeNegotiation.into()),
                bandwidth_upgrade_negotiation: Some(BandwidthUpgradeNegotiationFrame {
                    event_type: Some(EventType::ClientIntroduction.into()),
                    client_introduction: Some(ClientIntroduction {
                        endpoint_id: Some(endpoint_id.to_owned()),
                        supports_disabling_encryption: Some(false),
                        last_endpoint_id: last_endpoint_id.map(str::to_owned),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        let encoded = frame.encode_to_vec();
        let mut framed = (encoded.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(&encoded);
        framed
    }

    #[tokio::test]
    async fn routes_bwu_socket_without_consuming_client_introduction() {
        let router = BwuRouter::new();
        let receiver = router.register("peer-1234".to_owned()).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        let framed = client_introduction("peer-1234");
        client.write_all(&framed).await.unwrap();
        client.flush().await.unwrap();

        assert!(route_bandwidth_upgrade_if_pending(server, &router)
            .await
            .unwrap()
            .is_none());

        let mut routed = tokio::time::timeout(std::time::Duration::from_millis(250), receiver)
            .await
            .expect("BWU manager route did not deliver the socket")
            .expect("BWU manager route sender dropped without a socket");
        let mut received = vec![0_u8; framed.len()];
        routed.read_exact(&mut received).await.unwrap();
        assert_eq!(received, framed);
    }
    #[tokio::test]
    async fn routes_dynamic_bwu_endpoint_by_last_endpoint_id() {
        let router = BwuRouter::new();
        let receiver = router.register("peer-old".to_owned()).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        let framed = client_introduction_with_last("peer-new", Some("peer-old"));
        client.write_all(&framed).await.unwrap();
        client.flush().await.unwrap();

        assert!(route_bandwidth_upgrade_if_pending(server, &router)
            .await
            .unwrap()
            .is_none());

        let mut routed = tokio::time::timeout(std::time::Duration::from_millis(250), receiver)
            .await
            .expect("dynamic BWU route did not deliver the socket")
            .expect("dynamic BWU route sender dropped without a socket");
        let mut received = vec![0_u8; framed.len()];
        routed.read_exact(&mut received).await.unwrap();
        assert_eq!(received, framed);
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn returns_socket_when_no_bwu_route_is_pending() {
        let router = BwuRouter::new();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let _client = TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        assert!(route_bandwidth_upgrade_if_pending(server, &router)
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn malformed_frame_does_not_consume_pending_bwu_route() {
        let router = BwuRouter::new();
        let receiver = router.register("peer-1234".to_owned()).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        client
            .write_all(&(BWU_PEEK_LIMIT as u32).to_be_bytes())
            .await
            .unwrap();
        client.flush().await.unwrap();

        assert!(route_bandwidth_upgrade_if_pending(server, &router)
            .await
            .unwrap()
            .is_some());
        assert!(router.has_pending().await);

        router.cancel("peer-1234").await;
        drop(receiver);
    }

    #[tokio::test]
    async fn unregistered_bwu_introduction_does_not_consume_other_pending_route() {
        let router = BwuRouter::new();
        let receiver = router.register("peer-1234".to_owned()).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        let framed = client_introduction_with_last("peer-other", Some("peer-also-other"));
        client.write_all(&framed).await.unwrap();
        client.flush().await.unwrap();

        assert!(route_bandwidth_upgrade_if_pending(server, &router)
            .await
            .unwrap()
            .is_none());
        assert!(router.has_pending().await);

        router.cancel("peer-1234").await;
        drop(receiver);
    }

    #[test]
    fn given_outbound_state_when_failure_is_reported_then_active_requests_finish_and_terminal_outcomes_survive(
    ) {
        // Arrange
        let cases = [
            (State::Initial, true),
            (State::ReceivedConnectionRequest, true),
            (State::SentUkeyServerInit, true),
            (State::SentUkeyClientInit, true),
            (State::SentUkeyClientFinish, true),
            (State::SentPairedKeyEncryption, true),
            (State::ReceivedUkeyClientFinish, true),
            (State::SentConnectionResponse, true),
            (State::SentPairedKeyResult, true),
            (State::SentIntroduction, true),
            (State::ReceivedPairedKeyResult, true),
            (State::WaitingForUserConsent, true),
            (State::ReceivingFiles, true),
            (State::SendingFiles, true),
            (State::Finished, false),
            (State::Cancelled, false),
            (State::Rejected, false),
            (State::Disconnected, false),
        ];
        let error = anyhow::anyhow!("é".repeat(MAX_UI_ERROR_CHARS + 1));
        // Act / Assert
        for (state, report) in cases {
            let metadata = TransferMetadata {
                total_bytes: 17,
                ..Default::default()
            };
            let result = outbound_failure("stable-transfer", &state, Some(metadata), &error);
            assert_eq!(result.is_some(), report, "{state:?}");
            if let Some(message) = result {
                assert_eq!(message.id, "stable-transfer");
                assert_eq!(message.direction, ChannelDirection::LibToFront);
                assert_eq!(message.rtype, Some(TransferType::Outbound));
                assert_eq!(message.state, Some(State::Disconnected));
                assert_eq!(message.meta.unwrap().total_bytes, 17);
                assert_eq!(message.error.unwrap(), "é".repeat(MAX_UI_ERROR_CHARS));
            }
        }
    }

    async fn outbound_fixture() -> (TcpServer, tokio::sync::broadcast::Receiver<ChannelMessage>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (sender, receiver) = tokio::sync::broadcast::channel(16);
        let (_, queue) = tokio::sync::mpsc::channel(1);
        (
            TcpServer::new(*b"TEST", listener, sender, queue, BwuRouter::new()).unwrap(),
            receiver,
        )
    }

    fn only_outbound_failure(
        events: &mut tokio::sync::broadcast::Receiver<ChannelMessage>,
    ) -> ChannelMessage {
        let message = events
            .try_recv()
            .expect("the completed outbound worker must report failure");
        assert_eq!(message.id, "stable-transfer");
        assert_eq!(message.direction, ChannelDirection::LibToFront);
        assert_eq!(message.rtype, Some(TransferType::Outbound));
        assert_eq!(message.state, Some(State::Disconnected));
        let error = message.error.as_ref().unwrap();
        assert!(!error.is_empty());
        assert!(error.chars().count() <= MAX_UI_ERROR_CHARS);
        assert!(matches!(
            events.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));
        message
    }

    #[tokio::test]
    async fn given_connected_peer_when_payload_constructor_fails_then_one_failure_uses_request_identity(
    ) {
        // Arrange
        let (server, mut events) = outbound_fixture().await;
        let peer_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let info = SendInfo {
            id: "stable-transfer".into(),
            addr: peer_listener.local_addr().unwrap().to_string(),
            name: "loopback peer".into(),
            ob: OutboundPayload::EphemeralFiles(vec!["/unmanaged-source".into()]),
        };
        let peer = async {
            let (mut socket, _) = peer_listener.accept().await.unwrap();
            let mut byte = [0];
            assert_eq!(socket.read(&mut byte).await.unwrap(), 0);
        };
        // Act
        tokio::join!(
            server.execute_outbound(CancellationToken::new(), info),
            peer
        );
        // Assert
        assert!(only_outbound_failure(&mut events).meta.is_none());
    }

    #[tokio::test]
    async fn given_malformed_peer_handshake_when_worker_exits_then_failure_keeps_request_identity_and_metadata(
    ) {
        // Arrange
        let (server, mut events) = outbound_fixture().await;
        let peer_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let info = SendInfo {
            id: "stable-transfer".into(),
            addr: peer_listener.local_addr().unwrap().to_string(),
            name: "loopback peer".into(),
            ob: OutboundPayload::Text("test payload".into()),
        };
        let peer = async {
            let (mut socket, _) = peer_listener.accept().await.unwrap();
            for _ in 0..2 {
                let length = socket.read_u32().await.unwrap();
                assert!(length > 0 && length < 64 * 1024);
                let mut frame = vec![0; length as usize];
                socket.read_exact(&mut frame).await.unwrap();
            }
            socket.write_all(&[0, 0, 0, 1, 0xff]).await.unwrap();
            socket.shutdown().await.unwrap();
        };
        // Act
        tokio::join!(
            server.execute_outbound(CancellationToken::new(), info),
            peer
        );
        // Assert
        let message = only_outbound_failure(&mut events);
        let metadata = message.meta.unwrap();
        assert_eq!(metadata.text_payload.as_deref(), Some("test payload"));
        assert_eq!(metadata.source.unwrap().name, "loopback peer");
    }

    #[test]
    fn ui_error_is_truncated_to_character_limit() {
        let source = "é".repeat(MAX_UI_ERROR_CHARS + 10);
        let error = anyhow::anyhow!(source);
        let rendered = error_for_ui(&error);

        assert_eq!(rendered.chars().count(), MAX_UI_ERROR_CHARS);
        assert_eq!(rendered, "é".repeat(MAX_UI_ERROR_CHARS));
    }
}
