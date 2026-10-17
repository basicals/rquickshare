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
