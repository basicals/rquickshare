use futures::FutureExt;

use super::*;
use crate::managed::{ControlLost, ManagedSender};
use crate::sessions::{SessionControl, SessionError};

async fn session_state(control: &mut SessionControl, target: State) {
    loop {
        if control.snapshot().state == target {
            return;
        }
        if control.changed().await.is_err() {
            assert_eq!(control.snapshot().state, target);
            return;
        }
    }
}

async fn session_closed(control: &mut SessionControl) {
    while control.changed().await.is_ok() {}
    assert!(control.is_closed());
}

fn inbound_control(sender: &ManagedSender) -> SessionControl {
    sender
        .sessions()
        .active()
        .unwrap()
        .into_iter()
        .find(|control| control.snapshot().direction == TransferType::Inbound)
        .unwrap()
}

async fn native_consent(accept: bool) {
    // Arrange
    let (outgoing, sends) = managed_rig(TcpConnector).await;
    let (mut incoming, receives) = managed_rig(TcpConnector).await;
    let sent = sends
        .try_send(info("native-send", incoming.address))
        .unwrap();
    let received = loop {
        let event = incoming.events.recv().await.unwrap();
        if event.state == Some(State::WaitingForUserConsent) {
            break event;
        }
    };
    let mut control = inbound_control(&receives);
    let snapshot = control.snapshot();
    assert_eq!(snapshot.event_id, received.id);
    assert_ne!(snapshot.event_id, "ble-weave");
    let token = snapshot.consent.unwrap();
    // Act: matching legacy commands must not control either native session.
    incoming
        .control
        .send(ChannelMessage {
            id: snapshot.event_id.clone(),
            direction: ChannelDirection::FrontToLib,
            action: Some(crate::channel::ChannelAction::CancelTransfer),
            ..Default::default()
        })
        .unwrap();
    outgoing
        .control
        .send(ChannelMessage {
            id: "native-send".into(),
            direction: ChannelDirection::FrontToLib,
            action: Some(crate::channel::ChannelAction::CancelTransfer),
            ..Default::default()
        })
        .unwrap();
    control
        .respond(token, 9, accept)
        .unwrap()
        .wait()
        .await
        .unwrap();
    sent.wait_released().await.unwrap();
    session_state(
        &mut control,
        if accept {
            State::Finished
        } else {
            State::Rejected
        },
    )
    .await;
    session_closed(&mut control).await;
    // Assert
    assert_eq!(
        sent.session().snapshot().state,
        if accept {
            State::Finished
        } else {
            State::Disconnected
        }
    );
    control
        .respond(token, 9, accept)
        .unwrap()
        .wait()
        .await
        .unwrap();
    let (left, right) = tokio::join!(outgoing.stop(), incoming.stop());
    assert!(!left
        .iter()
        .chain(&right)
        .any(|event| event.state == Some(State::Cancelled)));
}

#[tokio::test]
async fn given_two_managed_tcp_sessions_when_native_consent_accepts_then_transfer_finishes_and_legacy_cancel_is_ignored(
) {
    scenario(native_consent(true)).await;
}

#[tokio::test]
async fn given_two_managed_tcp_sessions_when_native_consent_rejects_then_exact_receipt_and_terminal_rejection_are_preserved(
) {
    scenario(native_consent(false)).await;
}

#[tokio::test]
async fn given_established_native_inbound_when_owned_control_cancels_then_partial_read_closes_with_fresh_session_identity(
) {
    scenario(async {
        // Arrange
        let (run, sender) = managed_rig(TcpConnector).await;
        let mut peer = TcpStream::connect(run.address).await.unwrap();
        establish_inbound(&mut peer).await;
        let mut control = inbound_control(&sender);
        session_state(&mut control, State::SentUkeyServerInit).await;
        let original = control.snapshot();
        peer.write_u32(64).await.unwrap();
        peer.write_all(&[1]).await.unwrap();
        // Act
        control.cancel();
        session_closed(&mut control).await;
        let mut next_peer = TcpStream::connect(run.address).await.unwrap();
        establish_inbound(&mut next_peer).await;
        let mut next = inbound_control(&sender);
        session_state(&mut next, State::SentUkeyServerInit).await;
        control.cancel();
        // Assert
        assert_eq!(control.snapshot().state, State::Cancelled);
        assert_ne!(next.snapshot().id, original.id);
        assert_ne!(next.snapshot().event_id, original.event_id);
        assert_eq!(next.snapshot().state, State::SentUkeyServerInit);
        next.cancel();
        session_closed(&mut next).await;
        let events = run.stop().await;
        for id in [original.event_id, next.snapshot().event_id] {
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.id == id
                        && event.rtype == Some(TransferType::Inbound)
                        && event.state == Some(State::Cancelled))
                    .count(),
                1
            );
        }
    })
    .await;
}

#[tokio::test]
async fn given_native_outbound_cancelled_before_dispatch_when_released_then_session_closes_cancelled_without_connecting(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, _, _) = script(&[("blocked", false)]);
        let (run, sender) = managed_rig(connector).await;
        let control = sender.try_send(info("early", "blocked")).unwrap();
        let mut session = control.session();
        // Act
        control.cancel();
        control.wait_released().await.unwrap();
        session_closed(&mut session).await;
        // Assert
        assert_eq!(session.snapshot().state, State::Cancelled);
        assert!(started.try_recv().is_err());
        assert!(sender.sessions().active().unwrap().is_empty());
        run.stop().await;
    })
    .await;
}

#[tokio::test]
async fn given_native_connect_failure_when_owner_releases_then_snapshot_records_disconnection_without_success(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, gates, _) = script(&[("refused", false)]);
        let (run, sender) = managed_rig(connector).await;
        let control = sender
            .try_send(info("stable-request-id", "refused"))
            .unwrap();
        started.recv().await.unwrap();
        // Act
        gates["refused"].notify_one();
        control.wait_released().await.unwrap();
        // Assert
        assert_eq!(control.session().snapshot().state, State::Disconnected);
        assert_eq!(control.session().snapshot().event_id, "stable-request-id");
        assert!(control.session().is_closed());
        let events = run.stop().await;
        assert_eq!(
            events
                .iter()
                .filter(|event| event.id == "stable-request-id"
                    && event.state == Some(State::Disconnected))
                .count(),
            1
        );
    })
    .await;
}

#[tokio::test]
async fn given_full_native_session_budget_when_inbound_probe_arrives_then_it_closes_without_fabricated_notification(
) {
    scenario(async {
        // Arrange
        let (run, sender) = managed_rig(TcpConnector).await;
        let hub = sender.sessions();
        let leases: Vec<_> = (0..32)
            .map(|_| {
                hub.open(TransferType::Inbound, None, CancellationToken::new())
                    .unwrap()
            })
            .collect();
        assert!(matches!(
            hub.open(TransferType::Inbound, None, CancellationToken::new()),
            Err(SessionError::Full)
        ));
        // Act
        let mut probe = TcpStream::connect(run.address).await.unwrap();
        eof(&mut probe).await;
        let events = run.stop().await;
        // Assert
        assert!(events.is_empty());
        assert!(leases
            .iter()
            .all(|lease| lease.cancellation().is_cancelled()));
        drop(leases);
        assert!(hub.active().unwrap().is_empty());
    })
    .await;
}

async fn managed_rig<C: Connector>(connector: C) -> (Running, ManagedSender) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (accepted, _received) = mpsc::unbounded_channel();
    let (queue, receiver) = ManagedSender::channel().unwrap();
    let (sender, events) = broadcast::channel(128);
    let mut server = TcpServer {
        receive_policy: crate::receive::ReceivePolicy::new(None),
        identity: None,
        endpoint_id: *b"MNGD",
        tcp_listener: TrackingListener { listener, accepted },
        connector: Arc::new(connector),
        sender: sender.clone(),
        connect_receiver: receiver,
        bwu_router: BwuRouter::new(),
    };
    let token = CancellationToken::new();
    let worker_token = token.clone();
    let task = tokio::spawn(async move {
        server
            .run_with_limits(
                worker_token,
                TcpLimits {
                    sessions: 1,
                    classifiers: 1,
                },
            )
            .await
    });
    let (unused, _) = mpsc::channel(1);
    (
        Running {
            address,
            queue: unused,
            events,
            control: sender,
            token,
            task,
        },
        queue,
    )
}

fn cancelled(events: &[ChannelMessage], id: &str) {
    let matching: Vec<_> = events.iter().filter(|event| event.id == id).collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].state, Some(State::Cancelled));
    assert_eq!(matching[0].rtype, Some(TransferType::Outbound));
}

#[tokio::test]
async fn given_full_sessions_when_managed_head_and_buffer_are_cancelled_then_no_cancelled_job_connects(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, _, _) = script(&[("blocked", false)]);
        let (run, sender) = managed_rig(connector).await;
        let active = sender.try_send(info("active", "blocked")).unwrap();
        assert_eq!(started.recv().await.unwrap(), "blocked");
        let head = sender.try_send(info("head", "must-not-connect")).unwrap();
        sender.wait_queue_empty().await;
        let buffered = sender
            .try_send(info("buffered", "must-not-connect"))
            .unwrap();
        // Act
        buffered.cancel();
        assert!(buffered.wait_released().now_or_never().is_none());
        head.cancel();
        head.wait_released().await.unwrap();
        buffered.wait_released().await.unwrap();
        active.cancel();
        active.wait_released().await.unwrap();
        let events = run.stop().await;
        // Assert
        assert!(started.try_recv().is_err());
        for id in ["active", "head", "buffered"] {
            cancelled(&events, id);
        }
        assert_eq!(events.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn given_connecting_managed_job_when_cancelled_then_slot_is_reused_without_stopping_service()
{
    scenario(async {
        // Arrange
        let (connector, mut started, _, dropped) = script(&[("first", false), ("second", false)]);
        let (run, sender) = managed_rig(connector).await;
        let first = sender.try_send(info("first-id", "first")).unwrap();
        assert_eq!(started.recv().await.unwrap(), "first");
        let second = sender.try_send(info("second-id", "second")).unwrap();
        // Act
        first.cancel();
        first.wait_released().await.unwrap();
        assert_eq!(started.recv().await.unwrap(), "second");
        // Assert
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        assert!(!run.token.is_cancelled() && !sender.is_closed());
        second.cancel();
        second.wait_released().await.unwrap();
        let events = run.stop().await;
        cancelled(&events, "first-id");
        cancelled(&events, "second-id");
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
    })
    .await;
}

#[tokio::test]
async fn given_stalled_managed_handshake_when_cancelled_then_socket_and_payload_release_before_acknowledgement(
) {
    scenario(async {
        // Arrange
        let (run, sender) = managed_rig(TcpConnector).await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (payload, weak) = owned_payload();
        let mut command = info("owned", listener.local_addr().unwrap());
        command.ob = payload;
        let control = sender.try_send(command).unwrap();
        let mut peer = accept_outgoing(&listener).await;
        // Act
        control.cancel();
        control.wait_released().await.unwrap();
        // Assert
        assert!(weak.upgrade().is_none());
        eof(&mut peer).await;
        assert!(!sender.is_closed());
        let events = run.stop().await;
        assert!(events
            .iter()
            .any(|event| event.id == "owned" && event.state == Some(State::Cancelled)));
        assert_eq!(
            events
                .iter()
                .filter(|event| event.state == Some(State::Cancelled))
                .count(),
            1
        );
    })
    .await;
}

#[tokio::test]
async fn given_partial_outbound_frame_when_owned_control_cancels_then_blocked_read_is_joined() {
    scenario(async {
        // Arrange
        let (run, sender) = managed_rig(TcpConnector).await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let control = sender
            .try_send(info("partial", listener.local_addr().unwrap()))
            .unwrap();
        let mut peer = accept_outgoing(&listener).await;
        peer.write_u32(64).await.unwrap();
        peer.write_all(&[1]).await.unwrap();
        // Act
        control.cancel();
        control.wait_released().await.unwrap();
        let events = run.stop().await;
        // Assert
        let mut remainder = Vec::new();
        match peer.read_to_end(&mut remainder).await {
            Ok(size) => assert_eq!(size, 0),
            Err(error) => assert_eq!(error.kind(), io::ErrorKind::ConnectionReset),
        }
        assert!(remainder.is_empty());
        assert_eq!(
            events
                .iter()
                .filter(|event| event.id == "partial" && event.state == Some(State::Cancelled))
                .count(),
            1
        );
        assert!(!events
            .iter()
            .any(|event| matches!(event.state, Some(State::Finished | State::Disconnected))));
    })
    .await;
}

#[tokio::test]
async fn given_managed_worker_panic_when_joined_then_queue_closes_and_release_loss_is_explicit() {
    scenario(async {
        // Arrange
        let (connector, mut started, gates, _) = script(&[("panic", true)]);
        let (run, sender) = managed_rig(connector).await;
        let (payload, weak) = owned_payload();
        let mut input = info("panics", "panic");
        input.ob = payload;
        let control = sender.try_send(input).unwrap();
        started.recv().await.unwrap();
        // Act
        gates["panic"].notify_one();
        sender.closed().await;
        let (outcome, events) = run.finish().await;
        // Assert
        assert!(outcome.is_err());
        assert_eq!(control.wait_released().await, Err(ControlLost));
        assert!(weak.upgrade().is_none());
        assert!(sender.is_closed());
        assert!(!events
            .iter()
            .any(|event| event.state == Some(State::Finished)));
    })
    .await;
}

#[tokio::test]
async fn given_managed_active_head_and_buffered_jobs_when_service_stops_then_all_inputs_release_before_return(
) {
    scenario(async {
        // Arrange
        let (connector, mut started, _, _) = script(&[("blocked", false)]);
        let (run, sender) = managed_rig(connector).await;
        let mut controls = Vec::new();
        let mut resources = Vec::new();
        for id in ["active", "head", "buffered"] {
            let (payload, weak) = owned_payload();
            let mut command = info(id, "blocked");
            command.ob = payload;
            controls.push(sender.try_send(command).unwrap());
            resources.push(weak);
            if id == "active" {
                started.recv().await.unwrap();
            }
            if id == "head" {
                sender.wait_queue_empty().await;
            }
        }
        // Act
        let events = run.stop().await;
        // Assert
        for control in controls {
            control.wait_released().await.unwrap();
        }
        assert!(resources.iter().all(|weak| weak.upgrade().is_none()));
        assert!(sender.is_closed());
        for id in ["active", "head", "buffered"] {
            cancelled(&events, id);
        }
    })
    .await;
}

#[tokio::test]
async fn given_managed_transfer_finished_when_owned_control_is_cancelled_then_success_is_not_overridden(
) {
    scenario(async {
        // Arrange
        let (mut outbound, sender) = managed_rig(TcpConnector).await;
        let (mut inbound, _) = rig(
            TcpConnector,
            TcpLimits {
                sessions: 1,
                classifiers: 1,
            },
            BwuRouter::new(),
        )
        .await;
        let control = sender.try_send(info("complete", inbound.address)).unwrap();
        // Act
        let receive = async {
            loop {
                let event = inbound.events.recv().await.unwrap();
                if event.state == Some(State::WaitingForUserConsent) {
                    inbound
                        .control
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
        control.wait_released().await.unwrap();
        control.cancel();
        let (left, right) = tokio::join!(outbound.stop(), inbound.stop());
        // Assert
        assert_eq!(sent.id, "complete");
        assert_eq!(
            received.meta.unwrap().text_payload.as_deref(),
            Some("payload")
        );
        assert!(!left
            .iter()
            .chain(&right)
            .any(|event| matches!(event.state, Some(State::Cancelled | State::Disconnected))));
    })
    .await;
}

#[tokio::test]
async fn given_managed_senders_disconnected_when_inbound_arrives_then_manager_still_accepts() {
    scenario(async {
        // Arrange
        let (run, sender) = managed_rig(TcpConnector).await;
        drop(sender);
        // Act
        let mut peer = TcpStream::connect(run.address).await.unwrap();
        establish_inbound(&mut peer).await;
        let events = run.stop().await;
        // Assert
        eof(&mut peer).await;
        assert_eq!(
            events
                .iter()
                .filter(|event| event.rtype == Some(TransferType::Inbound)
                    && event.state == Some(State::Cancelled))
                .count(),
            1
        );
    })
    .await;
}
