use std::collections::BTreeMap;
use std::fs::File;
use std::future::{pending, Future};
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc, oneshot, Notify};
use tokio::task::JoinHandle;

use super::*;
use crate::file_source::{OpenedFile, OpenedSelection, SelectionLimits};
use crate::location_nearby_connections::{
    offline_frame, v1_frame, ConnectionRequestFrame, OfflineFrame, V1Frame,
};
use crate::securegcm::{
    ukey2_client_init::CipherCommitment, ukey2_message, Ukey2ClientInit, Ukey2HandshakeCipher,
    Ukey2Message,
};
use crate::utils::RemoteDeviceInfo;

mod managed_tests;

async fn scenario(operation: impl Future<Output = ()>) {
    // Watchdog only: normal progress always waits on explicit I/O/channel conditions.
    tokio::time::timeout(Duration::from_secs(5), operation)
        .await
        .expect("supervision scenario deadlocked");
}

struct TrackingListener {
    listener: TcpListener,
    accepted: mpsc::UnboundedSender<SocketAddr>,
}

impl AcceptSource for TrackingListener {
    async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let accepted = self.listener.accept().await?;
        let _ = self.accepted.send(accepted.1);
        Ok(accepted)
    }
}

struct FaultListener(Arc<Notify>);

impl AcceptSource for FaultListener {
    async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.0.notified().await;
        Err(io::Error::other("controlled accept failure"))
    }
}

struct DropMark(Arc<AtomicUsize>);

impl Drop for DropMark {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct ScriptConnector {
    gates: BTreeMap<String, (Arc<Notify>, bool)>,
    started: mpsc::UnboundedSender<String>,
    dropped: Arc<AtomicUsize>,
}

impl Connector for ScriptConnector {
    async fn connect<'a>(&'a self, address: &'a str) -> io::Result<TcpStream> {
        if let Some((gate, panic)) = self.gates.get(address) {
            let _mark = DropMark(Arc::clone(&self.dropped));
            let _ = self.started.send(address.to_owned());
            gate.notified().await;
            assert!(!panic, "controlled network adapter panic");
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "controlled refusal",
            ));
        }
        TcpStream::connect(address).await
    }
}

type Gates = BTreeMap<String, Arc<Notify>>;

fn script(
    entries: &[(&str, bool)],
) -> (
    ScriptConnector,
    mpsc::UnboundedReceiver<String>,
    Gates,
    Arc<AtomicUsize>,
) {
    let gates: Gates = entries
        .iter()
        .map(|(name, _)| (name.to_string(), Arc::new(Notify::new())))
        .collect();
    let (started, receiver) = mpsc::unbounded_channel();
    let dropped = Arc::new(AtomicUsize::new(0));
    let connector = ScriptConnector {
        gates: entries
            .iter()
            .map(|(name, panic)| (name.to_string(), (Arc::clone(&gates[*name]), *panic)))
            .collect(),
        started,
        dropped: Arc::clone(&dropped),
    };
    (connector, receiver, gates, dropped)
}

struct Running {
    address: SocketAddr,
    queue: mpsc::Sender<SendInfo>,
    events: broadcast::Receiver<ChannelMessage>,
    control: broadcast::Sender<ChannelMessage>,
    token: CancellationToken,
    task: JoinHandle<Result<(), anyhow::Error>>,
}

impl Running {
    async fn finish(mut self) -> (Result<(), anyhow::Error>, Vec<ChannelMessage>) {
        let result = (&mut self.task).await.unwrap();
        let mut events = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(
                    broadcast::error::TryRecvError::Empty | broadcast::error::TryRecvError::Closed,
                ) => break,
                Err(error) => panic!("fixture event loss: {error}"),
            }
        }
        (result, events)
    }

    async fn stop(self) -> Vec<ChannelMessage> {
        self.token.cancel();
        let (result, events) = self.finish().await;
        result.unwrap();
        events
    }

    async fn send(&self, id: &str, address: impl ToString) {
        self.queue.send(info(id, address)).await.unwrap();
    }

    async fn head_received(&self) {
        // Admission condition, not polling: all ten queue slots are free again.
        drop(self.queue.reserve_many(10).await.unwrap());
    }
}

fn info(id: &str, address: impl ToString) -> SendInfo {
    SendInfo {
        id: id.to_owned(),
        addr: address.to_string(),
        name: "loopback".into(),
        ob: OutboundPayload::Text("payload".into()),
    }
}

async fn rig<C: Connector>(
    connector: C,
    limits: TcpLimits,
    router: BwuRouter,
) -> (Running, mpsc::UnboundedReceiver<SocketAddr>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (accepted, received) = mpsc::unbounded_channel();
    let (queue, receiver) = mpsc::channel(10);
    let (sender, events) = broadcast::channel(128);
    let mut server = TcpServer {
        endpoint_id: *b"TEST",
        tcp_listener: TrackingListener { listener, accepted },
        connector: Arc::new(connector),
        sender: sender.clone(),
        connect_receiver: receiver.into(),
        bwu_router: router,
    };
    let token = CancellationToken::new();
    let worker_token = token.clone();
    let task = tokio::spawn(async move { server.run_with_limits(worker_token, limits).await });
    (
        Running {
            address,
            queue,
            events,
            control: sender,
            token,
            task,
        },
        received,
    )
}

async fn read_frame(socket: &mut TcpStream) -> Vec<u8> {
    let length = socket.read_u32().await.unwrap();
    assert!(length > 0 && length < 128 * 1024);
    let mut data = vec![0; length as usize];
    socket.read_exact(&mut data).await.unwrap();
    data
}

async fn write_frame(socket: &mut TcpStream, frame: &[u8]) {
    socket.write_u32(frame.len() as u32).await.unwrap();
    socket.write_all(frame).await.unwrap();
}

async fn accept_outgoing(listener: &TcpListener) -> TcpStream {
    let (mut socket, _) = listener.accept().await.unwrap();
    read_frame(&mut socket).await;
    read_frame(&mut socket).await;
    socket
}

pub(crate) async fn establish_inbound(socket: &mut TcpStream) {
    let connection = OfflineFrame {
        version: Some(offline_frame::Version::V1.into()),
        v1: Some(V1Frame {
            r#type: Some(v1_frame::FrameType::ConnectionRequest.into()),
            connection_request: Some(ConnectionRequestFrame {
                endpoint_id: Some("PEER".into()),
                endpoint_info: Some(
                    RemoteDeviceInfo {
                        name: "loopback".into(),
                        device_type: crate::DeviceType::Laptop,
                    }
                    .serialize(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        }),
    };
    write_frame(socket, &connection.encode_to_vec()).await;
    let init = Ukey2Message {
        message_type: Some(ukey2_message::Type::ClientInit.into()),
        message_data: Some(
            Ukey2ClientInit {
                version: Some(1),
                random: Some(vec![0; 32]),
                next_protocol: Some("AES_256_CBC-HMAC_SHA256".into()),
                cipher_commitments: vec![CipherCommitment {
                    handshake_cipher: Some(Ukey2HandshakeCipher::P256Sha512.into()),
                    commitment: Some(vec![1; 64]),
                }],
            }
            .encode_to_vec(),
        ),
    };
    write_frame(socket, &init.encode_to_vec()).await;
    let response = Ukey2Message::decode(read_frame(socket).await.as_slice()).unwrap();
    assert_eq!(response.message_type(), ukey2_message::Type::ServerInit);
}

async fn eof(socket: &mut TcpStream) {
    let mut byte = [0];
    assert_eq!(socket.read(&mut byte).await.unwrap(), 0);
}

fn owned_payload() -> (OutboundPayload, std::sync::Weak<OpenedSelection>) {
    let source = tempfile::NamedTempFile::new().unwrap();
    source.as_file().set_len(3).unwrap();
    let file = OpenedFile::adopt(File::open(source.path()).unwrap(), "source.txt".into()).unwrap();
    let selection =
        Arc::new(OpenedSelection::from_files(vec![file], SelectionLimits::default()).unwrap());
    let weak = Arc::downgrade(&selection);
    (OutboundPayload::OpenedFiles(selection), weak)
}

#[test]
fn given_invalid_worker_limits_when_validated_then_zero_and_overflow_are_rejected() {
    // Arrange / Act / Assert
    for limits in [
        TcpLimits {
            sessions: 0,
            classifiers: 1,
        },
        TcpLimits {
            sessions: 1,
            classifiers: 0,
        },
        TcpLimits {
            sessions: usize::MAX,
            classifiers: 1,
        },
        TcpLimits {
            sessions: Semaphore::MAX_PERMITS,
            classifiers: 1,
        },
    ] {
        assert!(limits.validate().is_err());
    }
    for sessions in [1, 2, 16, Semaphore::MAX_PERMITS - 1] {
        assert!(TcpLimits {
            sessions,
            classifiers: 1
        }
        .validate()
        .is_ok());
    }
}

#[test]
fn given_cancellation_when_message_is_built_then_initial_probes_and_terminal_outcomes_are_preserved(
) {
    // Arrange
    let states = [
        State::Initial,
        State::ReceivedConnectionRequest,
        State::SentUkeyServerInit,
        State::SentUkeyClientInit,
        State::SentUkeyClientFinish,
        State::SentPairedKeyEncryption,
        State::ReceivedUkeyClientFinish,
        State::SentConnectionResponse,
        State::SentPairedKeyResult,
        State::SentIntroduction,
        State::ReceivedPairedKeyResult,
        State::WaitingForUserConsent,
        State::ReceivingFiles,
        State::SendingFiles,
        State::Finished,
        State::Rejected,
        State::Cancelled,
        State::Disconnected,
    ];
    // Act / Assert
    for direction in [TransferType::Inbound, TransferType::Outbound] {
        for state in &states {
            let expected = !matches!(
                state,
                State::Finished | State::Rejected | State::Cancelled | State::Disconnected
            ) && !(direction == TransferType::Inbound && *state == State::Initial);
            let message = cancellation_message(
                "stable",
                direction.clone(),
                state,
                Some(TransferMetadata {
                    total_bytes: 3,
                    ..Default::default()
                }),
            );
            assert_eq!(message.is_some(), expected);
            if let Some(message) = message {
                assert_eq!(message.id, "stable");
                assert_eq!(message.direction, ChannelDirection::LibToFront);
                assert_eq!(message.rtype, Some(direction.clone()));
                assert_eq!(message.state, Some(State::Cancelled));
                assert_eq!(message.meta.unwrap().total_bytes, 3);
                assert!(message.error.is_none());
            }
        }
    }
}

#[tokio::test]
async fn given_live_lifetime_when_operation_completes_then_original_result_returns() {
    // Arrange / Act
    let result = session::until_stopped(&CancellationToken::new(), async { 17 }).await;
    // Assert
    assert_eq!(result, Some(17));
}

#[tokio::test]
async fn given_cancelled_lifetime_when_operation_is_ready_then_operation_is_not_polled() {
    // Arrange
    let token = CancellationToken::new();
    token.cancel();
    let polled = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let mark = DropMark(Arc::clone(&dropped));
    let observed = Arc::clone(&polled);
    // Act
    let result = session::until_stopped(&token, async move {
        let _mark = mark;
        observed.fetch_add(1, Ordering::SeqCst);
        17
    })
    .await;
    // Assert
    assert_eq!(result, None);
    assert_eq!(polled.load(Ordering::SeqCst), 0);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn given_blocked_operation_when_lifetime_stops_then_future_is_dropped_before_join_returns() {
    scenario(async {
        // Arrange
        let token = CancellationToken::new();
        let cancelled = token.clone();
        let dropped = Arc::new(AtomicUsize::new(0));
        let mark = DropMark(Arc::clone(&dropped));
        let (ready, started) = oneshot::channel();
        let operation = async move {
            let _mark = mark;
            ready.send(()).unwrap();
            pending::<()>().await
        };
        // Act
        let (result, ()) = tokio::join!(session::until_stopped(&token, operation), async {
            started.await.unwrap();
            cancelled.cancel();
        });
        // Assert
        assert_eq!(result, None);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    })
    .await;
}

#[tokio::test]
async fn given_stalled_outgoing_when_new_inbound_and_outgoing_arrive_then_all_handshakes_progress()
{
    scenario(async {
        // Arrange
        let (run, _) = rig(
            TcpConnector,
            TcpLimits {
                sessions: 3,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        let first = TcpListener::bind("127.0.0.1:0").await.unwrap();
        run.send("first", first.local_addr().unwrap()).await;
        let mut first_peer = accept_outgoing(&first).await;
        // Act
        let mut inbound = TcpStream::connect(run.address).await.unwrap();
        let inbound_id = inbound.local_addr().unwrap().to_string();
        establish_inbound(&mut inbound).await;
        let second = TcpListener::bind("127.0.0.1:0").await.unwrap();
        run.send("second", second.local_addr().unwrap()).await;
        let mut second_peer = accept_outgoing(&second).await;
        let events = run.stop().await;
        // Assert
        for id in ["first", "second", &inbound_id] {
            let terminal: Vec<_> = events
                .iter()
                .filter(|event| event.id == id && event.state == Some(State::Cancelled))
                .collect();
            assert_eq!(terminal.len(), 1, "{id}");
        }
        eof(&mut first_peer).await;
        eof(&mut second_peer).await;
        eof(&mut inbound).await;
    })
    .await;
}

#[tokio::test]
async fn given_shared_session_budget_when_inbound_arrives_at_capacity_then_probe_is_closed_without_ui_transfer(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, _, _) = script(&[("blocked", false)]);
        let (run, mut accepted) = rig(
            connector,
            TcpLimits {
                sessions: 1,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        run.send("active", "blocked").await;
        started.recv().await.unwrap();
        // Act
        let mut inbound = TcpStream::connect(run.address).await.unwrap();
        accepted.recv().await.unwrap();
        eof(&mut inbound).await;
        let events = run.stop().await;
        // Assert
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "active");
        assert_eq!(events[0].rtype, Some(TransferType::Outbound));
    })
    .await;
}

#[tokio::test]
async fn given_full_session_budget_when_worker_fails_then_next_queued_worker_starts() {
    scenario(async {
        // Arrange
        let (connector, mut started, gates, dropped) =
            script(&[("first", false), ("second", false)]);
        let (run, _) = rig(
            connector,
            TcpLimits {
                sessions: 1,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        run.send("one", "first").await;
        assert_eq!(started.recv().await.unwrap(), "first");
        run.send("two", "second").await;
        run.head_received().await;
        assert!(matches!(
            started.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        // Act
        gates["first"].notify_one();
        assert_eq!(started.recv().await.unwrap(), "second");
        let events = run.stop().await;
        // Assert
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
        assert!(events
            .iter()
            .any(|event| event.id == "one" && event.state == Some(State::Disconnected)));
        assert!(events
            .iter()
            .any(|event| event.id == "two" && event.state == Some(State::Cancelled)));
    })
    .await;
}

#[tokio::test]
async fn given_distinct_outgoing_ids_when_two_slots_are_full_then_third_waits_until_capacity_returns(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, gates, dropped) =
            script(&[("first", false), ("second", false), ("third", false)]);
        let (run, _) = rig(
            connector,
            TcpLimits {
                sessions: 2,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        run.send("one", "first").await;
        started.recv().await.unwrap();
        run.send("two", "second").await;
        started.recv().await.unwrap();
        run.send("three", "third").await;
        run.head_received().await;
        // Act / Assert
        assert!(matches!(
            started.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        gates["first"].notify_one();
        assert_eq!(started.recv().await.unwrap(), "third");
        let events = run.stop().await;
        assert_eq!(dropped.load(Ordering::SeqCst), 3);
        assert_eq!(events.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn given_reused_legacy_id_when_another_job_is_queued_then_same_id_handlers_never_overlap() {
    scenario(async {
        // Arrange
        let (connector, mut started, gates, _) = script(&[("first", false), ("second", false)]);
        let (run, mut accepted) = rig(
            connector,
            TcpLimits {
                sessions: 2,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        run.send("legacy", "first").await;
        started.recv().await.unwrap();
        run.send("legacy", "second").await;
        run.head_received().await;
        // Act / Assert
        assert!(matches!(
            started.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        let mut inbound = TcpStream::connect(run.address).await.unwrap();
        accepted.recv().await.unwrap();
        establish_inbound(&mut inbound).await;
        gates["first"].notify_one();
        assert_eq!(started.recv().await.unwrap(), "second");
        let events = run.stop().await;
        assert_eq!(
            events.iter().filter(|event| event.id == "legacy").count(),
            2
        );
        eof(&mut inbound).await;
    })
    .await;
}

#[tokio::test]
async fn given_buffered_head_and_active_owned_payloads_when_stopped_then_all_resources_drop_before_return(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, _, dropped) = script(&[("blocked", false)]);
        let (run, _) = rig(
            connector,
            TcpLimits {
                sessions: 1,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        let mut resources = Vec::new();
        for id in ["active", "head", "buffered"] {
            let (ob, weak) = owned_payload();
            resources.push(weak);
            let mut command = info(id, "blocked");
            command.ob = ob;
            run.queue.send(command).await.unwrap();
            if id == "active" {
                started.recv().await.unwrap();
            }
            if id == "head" {
                run.head_received().await;
            }
        }
        // Act
        let queue = run.queue.clone();
        let events = run.stop().await;
        // Assert
        assert!(resources
            .iter()
            .all(|resource| resource.upgrade().is_none()));
        assert!(queue.is_closed());
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        for id in ["active", "head", "buffered"] {
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.id == id && event.state == Some(State::Cancelled))
                    .count(),
                1
            );
        }
    })
    .await;
}

#[tokio::test]
async fn given_pre_cancelled_manager_when_queue_contains_work_then_no_connector_is_polled() {
    scenario(async {
        // Arrange
        let (connector, mut started, _, _) = script(&[("blocked", false)]);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (queue, receiver) = mpsc::channel(10);
        let (sender, mut events) = broadcast::channel(16);
        queue.send(info("one", "blocked")).await.unwrap();
        queue.send(info("two", "blocked")).await.unwrap();
        let token = CancellationToken::new();
        token.cancel();
        let mut server = TcpServer {
            endpoint_id: *b"TEST",
            tcp_listener: listener,
            connector: Arc::new(connector),
            sender,
            connect_receiver: receiver.into(),
            bwu_router: BwuRouter::new(),
        };
        // Act
        server.run(token).await.unwrap();
        // Assert
        assert!(matches!(
            started.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        for id in ["one", "two"] {
            let event = events.try_recv().unwrap();
            assert_eq!(event.id, id);
            assert_eq!(event.state, Some(State::Cancelled));
        }
        assert!(queue.is_closed());
    })
    .await;
}

#[tokio::test]
async fn given_pre_cancelled_outbound_context_when_started_then_payload_closes_without_connecting()
{
    // Arrange
    let (connector, mut started, _, _) = script(&[("blocked", false)]);
    let (sender, mut events) = broadcast::channel(8);
    let context = SessionContext::new(*b"TEST", sender, Arc::new(connector), None);
    let token = CancellationToken::new();
    token.cancel();
    let (ob, weak) = owned_payload();
    let mut command = info("one", "blocked");
    command.ob = ob;
    // Act
    context.execute_outbound(token, command).await;
    // Assert
    assert!(weak.upgrade().is_none());
    assert!(matches!(
        started.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert_eq!(events.try_recv().unwrap().state, Some(State::Cancelled));
}

#[tokio::test]
async fn given_initial_inbound_probe_when_service_stops_then_socket_closes_without_ui_transfer() {
    scenario(async {
        // Arrange
        let (run, mut accepted) = rig(TcpConnector, TcpLimits::default(), BwuRouter::new()).await;
        let mut probe = TcpStream::connect(run.address).await.unwrap();
        accepted.recv().await.unwrap();
        // Act
        let events = run.stop().await;
        // Assert
        assert!(events.is_empty());
        eof(&mut probe).await;
    })
    .await;
}

#[tokio::test]
async fn given_established_inbound_when_service_stops_then_one_typed_cancellation_is_published() {
    scenario(async {
        // Arrange
        let (run, _) = rig(TcpConnector, TcpLimits::default(), BwuRouter::new()).await;
        let mut peer = TcpStream::connect(run.address).await.unwrap();
        let id = peer.local_addr().unwrap().to_string();
        establish_inbound(&mut peer).await;
        // Act
        let events = run.stop().await;
        // Assert
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, id);
        assert_eq!(events[0].rtype, Some(TransferType::Inbound));
        assert_eq!(events[0].state, Some(State::Cancelled));
        eof(&mut peer).await;
    })
    .await;
}

#[tokio::test]
async fn given_malformed_initial_probe_when_worker_exits_then_no_transfer_notification_is_created()
{
    scenario(async {
        // Arrange
        let (run, _) = rig(TcpConnector, TcpLimits::default(), BwuRouter::new()).await;
        let mut peer = TcpStream::connect(run.address).await.unwrap();
        // Act
        write_frame(&mut peer, &[0xff]).await;
        eof(&mut peer).await;
        let events = run.stop().await;
        // Assert
        assert!(events.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_malformed_established_peer_when_worker_exits_then_failure_keeps_inbound_identity() {
    scenario(async {
        // Arrange
        let (run, _) = rig(TcpConnector, TcpLimits::default(), BwuRouter::new()).await;
        let mut peer = TcpStream::connect(run.address).await.unwrap();
        let id = peer.local_addr().unwrap().to_string();
        establish_inbound(&mut peer).await;
        // Act
        write_frame(&mut peer, &[0xff]).await;
        eof(&mut peer).await;
        let events = run.stop().await;
        // Assert
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, id);
        assert_eq!(events[0].rtype, Some(TransferType::Inbound));
        assert_eq!(events[0].state, Some(State::Disconnected));
        assert!(!events[0].error.as_ref().unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_partial_inbound_body_when_service_stops_then_blocked_frame_read_is_joined() {
    scenario(async {
        // Arrange
        let (run, mut accepted) = rig(TcpConnector, TcpLimits::default(), BwuRouter::new()).await;
        let mut peer = TcpStream::connect(run.address).await.unwrap();
        accepted.recv().await.unwrap();
        peer.write_all(&[0, 0, 0, 20, 1, 2]).await.unwrap();
        // Act
        let events = run.stop().await;
        // Assert
        assert!(events.is_empty());
        let mut data = Vec::new();
        let closure = peer.read_to_end(&mut data).await;
        // Closing with unread input may send RST; neither outcome leaves a live socket.
        match closure {
            Ok(count) => assert_eq!(count, 0),
            Err(error) => assert_eq!(error.kind(), io::ErrorKind::ConnectionReset),
        }
        assert!(data.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_full_normal_budget_when_bwu_socket_arrives_then_reserved_classifier_routes_without_consuming_bytes(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, _, _) = script(&[("blocked", false)]);
        let router = BwuRouter::new();
        let routed = router.register("upgrade".into()).await.unwrap();
        let (run, _) = rig(
            connector,
            TcpLimits {
                sessions: 1,
                classifiers: 1,
            },
            router,
        )
        .await;
        run.send("active", "blocked").await;
        started.recv().await.unwrap();
        let mut peer = TcpStream::connect(run.address).await.unwrap();
        let frame = super::tests::client_introduction("upgrade");
        // Act
        peer.write_all(&frame).await.unwrap();
        let mut socket = routed.await.unwrap();
        let mut bytes = vec![0; frame.len()];
        socket.read_exact(&mut bytes).await.unwrap();
        let events = run.stop().await;
        // Assert
        assert_eq!(bytes, frame);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "active");
        drop(socket);
        eof(&mut peer).await;
    })
    .await;
}

#[tokio::test]
async fn given_pending_classification_workers_when_service_stops_then_every_socket_closes_before_return(
) {
    scenario(async {
        // Arrange
        let router = BwuRouter::new();
        let pending_route = router.register("upgrade".into()).await.unwrap();
        let (run, mut accepted) = rig(
            TcpConnector,
            TcpLimits {
                sessions: 1,
                classifiers: 2,
            },
            router,
        )
        .await;
        let mut first = TcpStream::connect(run.address).await.unwrap();
        accepted.recv().await.unwrap();
        let mut second = TcpStream::connect(run.address).await.unwrap();
        accepted.recv().await.unwrap();
        // Act
        let events = run.stop().await;
        // Assert
        assert!(events.is_empty());
        eof(&mut first).await;
        eof(&mut second).await;
        drop(pending_route);
    })
    .await;
}

#[tokio::test]
async fn given_fatal_accept_failure_when_outbound_is_blocked_then_child_workers_stop_without_cancelling_caller(
) {
    scenario(async {
        // Arrange
        let fault = Arc::new(Notify::new());
        let (connector, mut started, _, dropped) = script(&[("blocked", false)]);
        let (queue, receiver) = mpsc::channel(10);
        let (sender, mut events) = broadcast::channel(16);
        let mut server = TcpServer {
            endpoint_id: *b"TEST",
            tcp_listener: FaultListener(Arc::clone(&fault)),
            connector: Arc::new(connector),
            sender,
            connect_receiver: receiver.into(),
            bwu_router: BwuRouter::new(),
        };
        let token = CancellationToken::new();
        let child = token.clone();
        let task = tokio::spawn(async move { server.run(child).await });
        queue.send(info("active", "blocked")).await.unwrap();
        started.recv().await.unwrap();
        // Act
        fault.notify_one();
        let result = task.await.unwrap();
        // Assert
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("controlled accept failure"));
        assert!(!token.is_cancelled());
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        assert!(queue.is_closed());
        let event = events.try_recv().unwrap();
        assert_eq!(event.id, "active");
        assert_eq!(event.state, Some(State::Cancelled));
    })
    .await;
}

#[tokio::test]
async fn given_panicking_network_worker_when_sibling_is_active_then_fatal_result_joins_and_releases_all_resources(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, gates, dropped) =
            script(&[("blocked", false), ("panic", true)]);
        let (run, _) = rig(
            connector,
            TcpLimits {
                sessions: 2,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        run.send("sibling", "blocked").await;
        started.recv().await.unwrap();
        let (ob, weak) = owned_payload();
        let mut command = info("failed", "panic");
        command.ob = ob;
        run.queue.send(command).await.unwrap();
        started.recv().await.unwrap();
        let parent = run.token.clone();
        // Act
        gates["panic"].notify_one();
        let (result, events) = run.finish().await;
        // Assert
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("TCP worker task failed"));
        assert!(!parent.is_cancelled());
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
        assert!(weak.upgrade().is_none());
        assert!(events
            .iter()
            .any(|event| event.id == "sibling" && event.state == Some(State::Cancelled)));
        assert!(!events
            .iter()
            .any(|event| event.state == Some(State::Finished)));
    })
    .await;
}

#[tokio::test]
async fn given_closed_outbound_queue_when_inbound_arrives_then_accept_loop_remains_responsive() {
    scenario(async {
        // Arrange
        let (mut run, _) = rig(TcpConnector, TcpLimits::default(), BwuRouter::new()).await;
        let (unused, _) = mpsc::channel(1);
        let original = std::mem::replace(&mut run.queue, unused);
        drop(original);
        // Act
        let mut peer = TcpStream::connect(run.address).await.unwrap();
        establish_inbound(&mut peer).await;
        let events = run.stop().await;
        // Assert
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].rtype, Some(TransferType::Inbound));
        eof(&mut peer).await;
    })
    .await;
}

#[tokio::test]
async fn given_invalid_budgets_when_manager_runs_then_network_and_queue_are_untouched() {
    // Arrange
    let (connector, mut started, _, _) = script(&[("blocked", false)]);
    let (queue, receiver) = mpsc::channel(10);
    queue.send(info("one", "blocked")).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (sender, _) = broadcast::channel(8);
    let mut server = TcpServer {
        endpoint_id: *b"TEST",
        tcp_listener: listener,
        connector: Arc::new(connector),
        sender,
        connect_receiver: receiver.into(),
        bwu_router: BwuRouter::new(),
    };
    let token = CancellationToken::new();
    // Act
    let result = server
        .run_with_limits(
            token.clone(),
            TcpLimits {
                sessions: 0,
                classifiers: 1,
            },
        )
        .await;
    // Assert
    assert!(result.is_err());
    assert!(!token.is_cancelled());
    assert!(!queue.is_closed());
    assert!(matches!(
        started.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert_eq!(server.connect_receiver.try_recv().unwrap().info.id, "one");
}

#[tokio::test]
async fn given_constructor_failure_when_another_send_is_queued_then_session_capacity_is_reused() {
    scenario(async {
        // Arrange
        let (connector, mut started, _, _) = script(&[("blocked", false)]);
        let (run, _) = rig(
            connector,
            TcpLimits {
                sessions: 1,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        let peer_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut command = info("bad", peer_listener.local_addr().unwrap());
        command.ob = OutboundPayload::EphemeralFiles(vec!["/unmanaged-source".into()]);
        run.queue.send(command).await.unwrap();
        run.send("next", "blocked").await;
        let (mut peer, _) = peer_listener.accept().await.unwrap();
        // Act
        eof(&mut peer).await;
        started.recv().await.unwrap();
        let events = run.stop().await;
        // Assert
        assert!(events
            .iter()
            .any(|event| event.id == "bad" && event.state == Some(State::Disconnected)));
        assert!(events
            .iter()
            .any(|event| event.id == "next" && event.state == Some(State::Cancelled)));
        assert_eq!(events.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn given_two_real_managers_when_text_is_accepted_then_repeated_transfers_finish_without_shutdown_overrides(
) {
    scenario(async {
        // Arrange
        let limits = TcpLimits {
            sessions: 1,
            classifiers: 1,
        };
        let (mut outbound, _) = rig(TcpConnector, limits, BwuRouter::new()).await;
        let (mut inbound, _) = rig(TcpConnector, limits, BwuRouter::new()).await;
        let consent = inbound.control.clone();
        for id in ["first-transfer", "second-transfer"] {
            // Act
            outbound.send(id, inbound.address).await;
            let receive = async {
                loop {
                    let event = inbound.events.recv().await.unwrap();
                    if event.state == Some(State::WaitingForUserConsent) {
                        consent
                            .send(ChannelMessage {
                                id: event.id,
                                direction: ChannelDirection::FrontToLib,
                                action: Some(crate::channel::ChannelAction::AcceptTransfer),
                                ..Default::default()
                            })
                            .unwrap();
                    } else if event.state == Some(State::Finished) {
                        break event;
                    }
                }
            };
            let send = async {
                loop {
                    let event = outbound.events.recv().await.unwrap();
                    if event.state == Some(State::Finished) {
                        break event;
                    }
                }
            };
            let (received, sent) = tokio::join!(receive, send);
            // Assert
            assert_eq!(sent.id, id);
            assert_eq!(sent.rtype, Some(TransferType::Outbound));
            assert_eq!(received.rtype, Some(TransferType::Inbound));
            assert_eq!(
                received.meta.unwrap().text_payload.as_deref(),
                Some("payload")
            );
        }
        let (left, right) = tokio::join!(outbound.stop(), inbound.stop());
        assert!(!left
            .iter()
            .chain(&right)
            .any(|event| event.state == Some(State::Cancelled)));
    })
    .await;
}
