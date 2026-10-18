use super::*;
use crate::{OutboundPayload, SendInfo};

async fn consent(accept: bool) {
    // Arrange: real protocol handlers, with a TCP/duplex adapter replacing BlueZ I/O only.
    let hub = SessionHub::new(31).unwrap();
    let session = hub
        .open(TransferType::Inbound, None, CancellationToken::new())
        .unwrap();
    let mut control = session.control();
    let (sender, mut events) = tokio::sync::broadcast::channel(128);
    let (ble, mut bridge) = tokio::io::duplex(64 * 1024);
    let request = InboundRequest::new(
        MigratableStream::Ble(ble),
        "legacy-placeholder".into(),
        sender.clone(),
    )
    .attach_session(session);
    let parent = CancellationToken::new();
    let tasks = SessionTasks::new(&parent, 3).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (dispatch, queue) = crate::managed::ManagedSender::channel().unwrap();
    let source_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut source = crate::manager::TcpServer::new(
        *b"PEER",
        source_listener,
        sender.clone(),
        queue,
        BwuRouter::new(),
    )
    .unwrap();
    let source_lifetime = CancellationToken::new();
    let source_token = source_lifetime.clone();
    let sending = tokio::spawn(async move { source.run(source_token).await });
    let sent = dispatch
        .try_send(SendInfo {
            id: "peer-send".into(),
            name: "test".into(),
            addr: listener.local_addr().unwrap().to_string(),
            ob: OutboundPayload::Text("native BLE payload".into()),
        })
        .unwrap();
    let (mut proxy, _) = listener.accept().await.unwrap();
    let bridge_lifetime = tasks.lifetime();
    tasks
        .spawn(async move {
            let _ = crate::manager::session::until_stopped(
                &bridge_lifetime,
                tokio::io::copy_bidirectional(&mut bridge, &mut proxy),
            )
            .await;
        })
        .unwrap();
    tasks
        .spawn(run_inbound_worker(
            request,
            sender.clone(),
            BwuRouter::new(),
            0,
            tasks.lifetime(),
            true,
        ))
        .unwrap();
    loop {
        if control.snapshot().state == State::WaitingForUserConsent {
            break;
        }
        control.changed().await.unwrap();
    }
    let snapshot = control.snapshot();
    // Act
    sender
        .send(ChannelMessage {
            id: snapshot.event_id.clone(),
            direction: ChannelDirection::FrontToLib,
            action: Some(crate::channel::ChannelAction::CancelTransfer),
            ..Default::default()
        })
        .unwrap();
    control
        .respond(snapshot.consent.unwrap(), 1, accept)
        .unwrap()
        .wait()
        .await
        .unwrap();
    sent.wait_released().await.unwrap();
    let expected = if accept {
        State::Finished
    } else {
        State::Rejected
    };
    loop {
        if control.snapshot().state == expected {
            break;
        }
        control.changed().await.unwrap();
    }
    tasks.stop().await.unwrap();
    source_lifetime.cancel();
    sending.await.unwrap().unwrap();
    // Assert
    assert_eq!(
        sent.session().snapshot().state,
        if accept {
            State::Finished
        } else {
            State::Disconnected
        }
    );
    assert_eq!(control.snapshot().id, snapshot.id);
    assert_eq!(control.snapshot().event_id, snapshot.event_id);
    if accept {
        let mut received_text = None;
        while let Ok(event) = events.try_recv() {
            if event.id == snapshot.event_id && event.state == Some(State::Finished) {
                received_text = event.meta.and_then(|meta| meta.text_payload);
            }
        }
        assert_eq!(received_text.as_deref(), Some("native BLE payload"));
    }
    assert!(control.is_closed());
    assert!(hub.active().unwrap().is_empty());
    assert!(!parent.is_cancelled());
}

// These fixtures exercise the real production GATT inbound worker and
// managed SessionHub ownership. The duplex replaces only external BlueZ bytes.
fn worker_fixture(
    state: State,
    epoch: u128,
) -> (
    InboundRequest<MigratableStream>,
    crate::sessions::SessionControl,
    SessionHub,
    tokio::io::DuplexStream,
    Sender<ChannelMessage>,
    tokio::sync::broadcast::Receiver<ChannelMessage>,
) {
    let hub = SessionHub::new(epoch).unwrap();
    let session = hub
        .open(TransferType::Inbound, None, CancellationToken::new())
        .unwrap();
    let control = session.control();
    let (sender, events) = tokio::sync::broadcast::channel(16);
    let (socket, peer) = tokio::io::duplex(128);
    let mut request = InboundRequest::new(
        MigratableStream::Ble(socket),
        "pre-session".into(),
        sender.clone(),
    )
    .attach_session(session);
    // This is the worker's incoming protocol-state precondition. The actual
    // worker handles the socket error/cancellation and applies the transition.
    request.state.state = state;
    (request, control, hub, peer, sender, events)
}

#[tokio::test]
async fn given_unidentified_native_ble_peer_eof_when_worker_exits_then_no_false_disconnected_event_is_published(
) {
    // Arrange: a managed initial state, before the peer is authenticated.
    let (request, control, hub, peer, sender, mut events) = worker_fixture(State::Initial, 401);
    let transfer_id = control.snapshot().event_id;
    drop(peer);

    // Act: the production worker receives a genuine duplex EOF.
    run_inbound_worker(
        request,
        sender,
        BwuRouter::new(),
        0,
        CancellationToken::new(),
        true,
    )
    .await;

    // Assert: unadmitted peer failures are silent, with no stale lease.
    assert_eq!(control.snapshot().event_id, transfer_id);
    assert_eq!(control.snapshot().state, State::Initial);
    assert!(control.snapshot().received_files.is_none());
    assert!(control.snapshot().received_text.is_none());
    assert!(events.try_recv().is_err());
    assert!(control.is_closed());
    assert!(hub.active().unwrap().is_empty());
}

#[tokio::test]
async fn given_legacy_ble_peer_eof_when_worker_exits_then_real_inbound_disconnected_event_is_sent()
{
    // Arrange: a legacy (unmanaged) inbound worker over an actual duplex.
    let (socket, peer) = tokio::io::duplex(128);
    let (sender, mut events) = tokio::sync::broadcast::channel(16);
    let request = InboundRequest::new(
        MigratableStream::Ble(socket),
        "legacy-eof".into(),
        sender.clone(),
    );
    drop(peer);

    // Act: the production worker must not hide the legacy socket failure.
    run_inbound_worker(
        request,
        sender,
        BwuRouter::new(),
        0,
        CancellationToken::new(),
        false,
    )
    .await;

    // Assert: the legacy UI receives the bounded error, never a false success.
    let message = events.try_recv().unwrap();
    assert_eq!(message.id, "legacy-eof");
    assert_eq!(message.direction, ChannelDirection::LibToFront);
    assert_eq!(message.rtype, Some(TransferType::Inbound));
    assert_eq!(message.state, Some(State::Disconnected));
    assert!(message
        .error
        .as_ref()
        .is_some_and(|s| !s.is_empty() && s.len() <= 512));
    assert!(message.meta.is_none());
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn given_progressed_native_ble_peer_eof_or_invalid_length_when_worker_exits_then_owned_state_is_disconnected(
) {
    use tokio::io::AsyncWriteExt;

    for (invalid_length, epoch) in [(false, 402_u128), (true, 403_u128)] {
        // Arrange: the GATT worker has already moved past the initial state.
        let (request, control, hub, mut peer, sender, mut events) =
            worker_fixture(State::ReceivedConnectionRequest, epoch);
        if invalid_length {
            // The production frame-size gate rejects this hostile prefix.
            peer.write_all(&u32::MAX.to_be_bytes()).await.unwrap();
        }
        drop(peer);

        // Act: genuine EOF or over-limit frame enters the actual worker.
        run_inbound_worker(
            request,
            sender,
            BwuRouter::new(),
            0,
            CancellationToken::new(),
            true,
        )
        .await;

        // Assert: a known native session is FAILED, not silently or falsely done.
        let message = events.try_recv().unwrap();
        assert_eq!(message.id, control.snapshot().event_id);
        assert_eq!(message.direction, ChannelDirection::LibToFront);
        assert_eq!(message.rtype, Some(TransferType::Inbound));
        assert_eq!(message.state, Some(State::Disconnected));
        assert!(
            message
                .error
                .as_ref()
                .is_some_and(|error| !error.is_empty() && error.len() <= 512),
            "every transport failure must carry a bounded reason"
        );
        assert!(message.meta.is_none());
        assert!(events.try_recv().is_err());
        let snapshot = control.snapshot();
        assert_eq!(snapshot.state, State::Disconnected);
        assert!(snapshot.received_text.is_none());
        assert!(snapshot.received_files.is_none());
        assert!(control.is_closed());
        assert!(hub.active().unwrap().is_empty());
    }
}

#[tokio::test]
async fn given_progressed_native_worker_parent_or_session_cancellation_when_stopped_then_exactly_cancelled_state_survives(
) {
    for (cancel_session, epoch) in [(false, 404_u128), (true, 405_u128)] {
        // Arrange: a live managed receiver, with actual duplex still connected.
        let (request, control, hub, peer, sender, mut events) =
            worker_fixture(State::ReceivedConnectionRequest, epoch);
        let lifetime = CancellationToken::new();
        if cancel_session {
            control.cancel();
        } else {
            lifetime.cancel();
        }

        // Act: the worker's real cancellation select must beat blocked I/O.
        run_inbound_worker(request, sender, BwuRouter::new(), 0, lifetime, true).await;

        // Assert: only a terminal Cancelled is observable. No retry or success.
        let message = events.try_recv().unwrap();
        assert_eq!(message.id, control.snapshot().event_id);
        assert_eq!(message.direction, ChannelDirection::LibToFront);
        assert_eq!(message.rtype, Some(TransferType::Inbound));
        assert_eq!(message.state, Some(State::Cancelled));
        assert!(message.error.is_none());
        assert!(events.try_recv().is_err());
        let snapshot = control.snapshot();
        assert_eq!(snapshot.state, State::Cancelled);
        assert!(snapshot.received_text.is_none());
        assert!(snapshot.received_files.is_none());
        assert!(control.is_closed());
        assert!(hub.active().unwrap().is_empty());
        drop(peer);
    }
}

#[tokio::test]
async fn given_unidentified_native_worker_is_cancelled_when_stopped_then_it_releases_without_spurious_ui_event(
) {
    // Arrange: native receiver has not yet established a protocol identity.
    let (request, control, hub, peer, sender, mut events) = worker_fixture(State::Initial, 406);
    let lifetime = CancellationToken::new();
    lifetime.cancel();

    // Act: a stopped parent must release a blocked initial read immediately.
    run_inbound_worker(request, sender, BwuRouter::new(), 0, lifetime, true).await;

    // Assert: there is no user-visible false cancel or finished result.
    assert_eq!(control.snapshot().state, State::Cancelled);
    assert!(control.snapshot().received_text.is_none());
    assert!(control.snapshot().received_files.is_none());
    assert!(events.try_recv().is_err());
    assert!(control.is_closed());
    assert!(hub.active().unwrap().is_empty());
    drop(peer);
}

#[tokio::test]
async fn given_unidentified_native_malformed_ble_preface_when_worker_exits_then_no_ui_or_session_is_published(
) {
    use tokio::io::AsyncWriteExt;

    // Arrange: bad wire data arrives while the BLE transport remains open.
    // This verifies the native unadmitted-error gate, not just socket EOF.
    let malformed = [
        ("zero-length", 0_u32.to_be_bytes().to_vec()),
        (
            "invalid-protobuf",
            [1_u32.to_be_bytes().as_slice(), &[0xff]].concat(),
        ),
        (
            "above-frame-limit",
            ((crate::protocol::SANE_FRAME_LENGTH as u32) + 1)
                .to_be_bytes()
                .to_vec(),
        ),
        ("u32-max-length", u32::MAX.to_be_bytes().to_vec()),
    ];

    for (index, (case, bytes)) in malformed.into_iter().enumerate() {
        let (request, control, hub, mut peer, sender, mut events) =
            worker_fixture(State::Initial, 501 + index as u128);
        let id = control.snapshot().event_id.clone();
        peer.write_all(&bytes).await.unwrap();

        // Act: production parser rejects malformed input without requiring EOF.
        run_inbound_worker(
            request,
            sender,
            BwuRouter::new(),
            0,
            CancellationToken::new(),
            true,
        )
        .await;

        // Assert: no unverified peer is exposed as a transfer or left active.
        let snapshot = control.snapshot();
        assert_eq!(snapshot.event_id, id, "unexpected identity for {case}");
        assert_eq!(snapshot.state, State::Initial, "premature state for {case}");
        assert!(
            snapshot.received_text.is_none(),
            "unexpected text for {case}"
        );
        assert!(
            snapshot.received_files.is_none(),
            "unexpected files for {case}"
        );
        assert!(events.try_recv().is_err(), "unsolicited event for {case}");
        assert!(control.is_closed(), "session was not released for {case}");
        assert!(
            hub.active().unwrap().is_empty(),
            "active session for {case}"
        );
    }
}

#[tokio::test]
async fn given_progressed_native_partial_ble_header_or_body_when_peer_eof_then_one_disconnected_and_owner_release(
) {
    use tokio::io::AsyncWriteExt;

    let valid_header = 3_u32.to_be_bytes();
    let frames = [
        ("one-prefix-byte", valid_header[..1].to_vec()),
        ("two-prefix-bytes", valid_header[..2].to_vec()),
        ("three-prefix-bytes", valid_header[..3].to_vec()),
        ("one-body-byte", [valid_header.as_slice(), &[0x0a]].concat()),
        (
            "two-body-bytes",
            [valid_header.as_slice(), &[0x0a, 0x01]].concat(),
        ),
    ];

    for (index, (case, bytes)) in frames.into_iter().enumerate() {
        // Arrange: the native session has advanced, so transport EOF is visible.
        let (request, control, hub, mut peer, sender, mut events) =
            worker_fixture(State::ReceivedConnectionRequest, 511 + index as u128);
        let id = control.snapshot().event_id.clone();
        peer.write_all(&bytes).await.unwrap();
        peer.shutdown().await.unwrap();
        drop(peer);

        // Act: the real partial-frame read encounters a definite EOF.
        run_inbound_worker(
            request,
            sender,
            BwuRouter::new(),
            0,
            CancellationToken::new(),
            true,
        )
        .await;

        // Assert: exactly one bounded transport failure, never success/consent.
        let message = events.try_recv().expect("missing disconnected event");
        assert_eq!(message.id, id, "wrong owner in {case}");
        assert_eq!(message.direction, ChannelDirection::LibToFront);
        assert_eq!(message.rtype, Some(TransferType::Inbound));
        assert_eq!(message.state, Some(State::Disconnected), "{case}");
        assert!(
            message
                .error
                .as_ref()
                .is_some_and(|reason| !reason.is_empty() && reason.len() <= 512),
            "missing bounded failure reason for {case}"
        );
        assert!(
            message.meta.is_none(),
            "unexpected payload metadata for {case}"
        );
        assert!(
            events.try_recv().is_err(),
            "duplicate terminal event for {case}"
        );
        let snapshot = control.snapshot();
        assert_eq!(snapshot.state, State::Disconnected, "{case}");
        assert!(snapshot.received_text.is_none(), "{case}");
        assert!(snapshot.received_files.is_none(), "{case}");
        assert!(control.is_closed(), "open session after EOF in {case}");
        assert!(hub.active().unwrap().is_empty(), "leaked owner in {case}");
    }
}

#[tokio::test]
async fn given_progressed_native_partial_ble_frame_when_parent_or_request_cancels_then_one_cancelled_and_no_replay(
) {
    use tokio::io::AsyncWriteExt;

    for (index, (cancel_request, partial_body)) in
        [(false, false), (false, true), (true, false), (true, true)]
            .into_iter()
            .enumerate()
    {
        // Arrange: keep the external byte transport alive after an incomplete
        // frame. The next worker poll must be blocked on real socket I/O.
        let (request, control, hub, mut peer, sender, mut events) =
            worker_fixture(State::ReceivedConnectionRequest, 521 + index as u128);
        let id = control.snapshot().event_id.clone();
        let bytes = if partial_body {
            [3_u32.to_be_bytes().as_slice(), &[0x0a]].concat()
        } else {
            vec![0_u8, 0_u8]
        };
        peer.write_all(&bytes).await.unwrap();
        let lifetime = CancellationToken::new();
        let mut worker = Box::pin(run_inbound_worker(
            request,
            sender,
            BwuRouter::new(),
            0,
            lifetime.clone(),
            true,
        ));

        // Act: establish the in-flight, incomplete read before cancelling.
        assert!(
            futures::poll!(worker.as_mut()).is_pending(),
            "partial BLE worker completed before cancellation; case {index}"
        );
        if cancel_request {
            control.cancel();
        } else {
            lifetime.cancel();
        }
        worker.await;

        // Assert: cancellation wins without publishing a late transport
        // failure or forged success, and the owned session is fully released.
        let message = events.try_recv().expect("missing cancellation event");
        assert_eq!(message.id, id, "unexpected owner in case {index}");
        assert_eq!(message.direction, ChannelDirection::LibToFront);
        assert_eq!(message.rtype, Some(TransferType::Inbound));
        assert_eq!(message.state, Some(State::Cancelled), "case {index}");
        assert!(
            message.error.is_none(),
            "spurious transport error in case {index}"
        );
        assert!(message.meta.is_none(), "spurious result in case {index}");
        assert!(
            events.try_recv().is_err(),
            "duplicate terminal for case {index}"
        );
        let snapshot = control.snapshot();
        assert_eq!(snapshot.state, State::Cancelled, "case {index}");
        assert!(snapshot.received_text.is_none(), "case {index}");
        assert!(snapshot.received_files.is_none(), "case {index}");
        assert!(control.is_closed(), "leaked control in case {index}");
        assert!(
            hub.active().unwrap().is_empty(),
            "leaked session in case {index}"
        );
        drop(peer);
    }
}

#[tokio::test]
async fn given_managed_ble_stream_when_owned_consent_accepts_then_real_encrypted_transfer_finishes()
{
    tokio::time::timeout(std::time::Duration::from_secs(5), consent(true))
        .await
        .expect("BLE consent deadlocked");
}

#[tokio::test]
async fn given_managed_ble_stream_when_owned_consent_rejects_then_rejection_survives_group_shutdown(
) {
    tokio::time::timeout(std::time::Duration::from_secs(5), consent(false))
        .await
        .expect("BLE rejection deadlocked");
}
