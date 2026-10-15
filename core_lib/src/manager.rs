use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Sender;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::channel::{ChannelDirection, ChannelMessage, TransferType};
use crate::hdl::info::TransferMetadata;
use crate::hdl::{
    peek_client_introduction, BwuClientIntroductionIdentity, BwuRouter, OutboundPayload, State,
};
use crate::managed::{acknowledge_release, OutboundJob, OutboundReceiver};
pub(crate) mod session;
use session::{Connector, SessionContext, TcpConnector};

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
    if terminal(state) {
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

fn terminal(state: &State) -> bool {
    matches!(
        state,
        State::Finished | State::Cancelled | State::Rejected | State::Disconnected
    )
}

pub(crate) fn cancellation_message(
    id: &str,
    direction: TransferType,
    state: &State,
    meta: Option<TransferMetadata>,
) -> Option<ChannelMessage> {
    if terminal(state) || (direction == TransferType::Inbound && *state == State::Initial) {
        return None;
    }
    Some(ChannelMessage {
        id: id.to_owned(),
        direction: ChannelDirection::LibToFront,
        rtype: Some(direction),
        state: Some(State::Cancelled),
        meta,
        ..Default::default()
    })
}

#[derive(Clone, Copy)]
struct TcpLimits {
    sessions: usize,
    classifiers: usize,
}

impl Default for TcpLimits {
    fn default() -> Self {
        Self {
            sessions: 16,
            classifiers: 4,
        }
    }
}

impl TcpLimits {
    fn validate(self) -> Result<Self, anyhow::Error> {
        if self.sessions == 0
            || self.classifiers == 0
            || self
                .sessions
                .checked_add(self.classifiers)
                .filter(|total| *total <= Semaphore::MAX_PERMITS)
                .is_none()
        {
            return Err(anyhow::anyhow!("Invalid TCP worker budgets"));
        }
        Ok(self)
    }
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct SendInfo {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub ob: OutboundPayload,
}

pub(crate) trait AcceptSource: Send + Sync {
    fn accept(
        &self,
    ) -> impl std::future::Future<Output = std::io::Result<(TcpStream, std::net::SocketAddr)>> + Send;
}

impl AcceptSource for TcpListener {
    fn accept(
        &self,
    ) -> impl std::future::Future<Output = std::io::Result<(TcpStream, std::net::SocketAddr)>> + Send
    {
        TcpListener::accept(self)
    }
}

pub(crate) struct TcpServer<L = TcpListener, C = TcpConnector> {
    endpoint_id: [u8; 4],
    tcp_listener: L,
    connector: Arc<C>,
    sender: Sender<ChannelMessage>,
    connect_receiver: OutboundReceiver,
    bwu_router: BwuRouter,
    receive_policy: crate::receive::ReceivePolicy,
    identity: Option<crate::identity::IdentityPolicy>,
}

impl TcpServer {
    pub(crate) fn with_identity(mut self, identity: crate::identity::IdentityPolicy) -> Self {
        self.identity = Some(identity);
        self
    }
    pub(crate) fn with_receive_policy(mut self, policy: crate::receive::ReceivePolicy) -> Self {
        self.receive_policy = policy;
        self
    }
    pub fn new(
        endpoint_id: [u8; 4],
        tcp_listener: TcpListener,
        sender: Sender<ChannelMessage>,
        connect_receiver: impl Into<OutboundReceiver>,
        bwu_router: BwuRouter,
    ) -> Result<Self, anyhow::Error> {
        Ok(Self {
            endpoint_id,
            tcp_listener,
            connector: Arc::new(TcpConnector),
            sender,
            connect_receiver: connect_receiver.into(),
            bwu_router,
            receive_policy: crate::receive::ReceivePolicy::new(None),
            identity: None,
        })
    }
}

impl<L: AcceptSource, C: Connector> TcpServer<L, C> {
    fn context(&self) -> SessionContext<C> {
        SessionContext::new(
            self.endpoint_id,
            self.sender.clone(),
            Arc::clone(&self.connector),
            self.connect_receiver.sessions(),
        )
        .with_receive_policy(self.receive_policy.clone())
        .with_identity(self.identity.clone())
    }

    pub async fn run(&mut self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        self.run_with_limits(ctk, TcpLimits::default()).await
    }

    async fn run_with_limits(
        &mut self,
        ctk: CancellationToken,
        limits: TcpLimits,
    ) -> Result<(), anyhow::Error> {
        let limits = limits.validate()?;
        let sessions = Arc::new(Semaphore::new(limits.sessions));
        let classifiers = Arc::new(Semaphore::new(limits.classifiers));
        let lifetime = ctk.child_token();
        let mut workers = JoinSet::new();
        let mut active_ids = HashSet::new();
        let mut pending: Option<OutboundJob> = None;
        let context = self.context();
        info!("TcpServer: supervised service starting");

        let mut outcome = loop {
            if lifetime.is_cancelled() {
                break Ok(());
            }
            tokio::select! {
                _ = lifetime.cancelled() => break Ok(()),
                _ = async {
                    if let Some(job) = pending.as_ref() {
                        job.cancellation.cancelled().await;
                    }
                }, if pending.is_some() => {
                    if let Some(job) = pending.take() {
                        context.cancel_queued(&job.info.id);
                        job.release();
                    }
                }
                result = workers.join_next(), if !workers.is_empty() => {
                    match result {
                        Some(Ok(Some(id))) => { active_ids.remove(&id); },
                        Some(Ok(None)) | None => {},
                        Some(Err(_)) => break Err(anyhow::anyhow!("TCP worker task failed")),
                    }
                }
                Ok(permit) = Arc::clone(&sessions).acquire_owned(),
                    if pending.as_ref().is_some_and(|job| !active_ids.contains(&job.info.id)) => {
                    if let Some(job) = pending.take() {
                        let OutboundJob { info, cancellation, completion, session } = job;
                        let id = info.id.clone();
                        active_ids.insert(id.clone());
                        let context = context.clone();
                        let token = lifetime.child_token();
                        workers.spawn(async move {
                            let _permit = permit;
                            context.execute_session(token, cancellation, info, session).await;
                            acknowledge_release(completion);
                            Some(id)
                        });
                    }
                }
                Some(info) = self.connect_receiver.recv(), if pending.is_none() => {
                    pending = Some(info);
                }
                accepted = async {
                    let permit = Arc::clone(&classifiers).acquire_owned().await
                        .map_err(|_| anyhow::anyhow!("TCP classifier budget closed"))?;
                    let accepted = self.tcp_listener.accept().await?;
                    Ok::<_, anyhow::Error>((permit, accepted))
                } => {
                    match accepted {
                        Ok((permit, (socket, remote_addr))) => {
                            let context = context.clone();
                            let token = lifetime.child_token();
                            let router = self.bwu_router.clone();
                            let sessions = Arc::clone(&sessions);
                            workers.spawn(async move {
                                context.classify_inbound(
                                    token, socket, remote_addr, router, sessions, permit,
                                ).await;
                                None
                            });
                        }
                        Err(error) => break Err(error),
                    }
                }
            }
        };

        // Close admission first; then cancel and join every owned worker.
        self.connect_receiver.close();
        lifetime.cancel();
        context.close_sessions();
        if let Some(job) = pending.take() {
            context.cancel_queued(&job.info.id);
            job.release();
        }
        while let Ok(job) = self.connect_receiver.try_recv() {
            context.cancel_queued(&job.info.id);
            job.release();
        }
        while let Some(result) = workers.join_next().await {
            if result.is_err() && outcome.is_ok() {
                outcome = Err(anyhow::anyhow!("TCP worker task failed during shutdown"));
            }
        }
        info!("TcpServer: supervised workers stopped");
        outcome
    }

    #[cfg(test)]
    async fn execute_outbound(&self, ctk: CancellationToken, info: SendInfo) {
        self.context().execute_outbound(ctk, info).await;
    }
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) mod supervision_tests;

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

    pub(super) fn client_introduction(endpoint_id: &str) -> Vec<u8> {
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
