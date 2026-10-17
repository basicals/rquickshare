use super::*;
use crate::sessions::{
    details::{DetailsStatus, DeviceKind, PayloadKind},
    SessionControl, SessionHub,
};

async fn managed_request(
    source: crate::prepared_source::PreparedSource,
) -> (OutboundRequest, TcpStream, SessionHub, SessionControl) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let peer = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let (sender, _) = tokio::sync::broadcast::channel(16);
    let hub = SessionHub::new(17).unwrap();
    let session = hub
        .open(
            crate::channel::TransferType::Outbound,
            None,
            tokio_util::sync::CancellationToken::new(),
        )
        .unwrap();
    let control = session.control();
    let mut request = OutboundRequest::new(
        [1, 2, 3, 4],
        socket,
        "metadata-fixture".into(),
        sender,
        OutboundPayload::NativeSource(Arc::new(source)),
        RemoteDeviceInfo {
            name: "Advertised phone".into(),
            device_type: DeviceType::Phone,
        },
    )
    .unwrap()
    .attach_session(session);
    request.state.encrypt_key = Some(vec![0x11; 32]);
    request.state.send_hmac_key = Some(vec![0x22; 32]);
    (request, peer, hub, control)
}

async fn introduce(request: &mut OutboundRequest, peer: &mut TcpStream) {
    request.state.state = State::SentPairedKeyResult;
    request
        .process_transfer_setup(&sharing_nearby::Frame {
            v1: Some(paired_result()),
            ..Default::default()
        })
        .await
        .unwrap();
    // Consume actual encrypted Introduction and its final byte-payload chunk.
    read_native_frame(peer, 1).await;
    read_native_frame(peer, 2).await;
}

#[tokio::test]
async fn given_peer_reset_before_empty_file_eof_when_accepted_then_source_and_failed_state_are_preserved(
) {
    // Arrange: an established encrypted native sender with a real captured
    // zero-byte regular file; RST is delivered before the accepted FILE frame.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private-empty-source");
    std::fs::write(&path, []).unwrap();
    let (mut request, mut peer, _hub, control) =
        managed_request(crate::prepared_source::PreparedSource::from(selection(
            &path,
            "empty.bin",
        )))
        .await;
    let source = match &request.payload {
        OutboundPayload::NativeSource(source) => Arc::downgrade(source),
        _ => panic!("expected opened native source"),
    };
    introduce(&mut request, &mut peer).await;
    let original = control.snapshot();
    assert_eq!(original.details.total_bytes, 0);
    assert_eq!(original.details.files.len(), 1);
    assert_eq!(request.state.transferred_files.len(), 1);

    // Force an abortive TCP close, not a simulated missing encryption key.
    rustix::net::sockopt::set_socket_linger(&peer, Some(std::time::Duration::ZERO)).unwrap();
    drop(peer);
    tokio::time::timeout(std::time::Duration::from_secs(5), request.socket.readable())
        .await
        .expect("peer reset must wake the actual outgoing socket")
        .unwrap();
    let mut byte = [0_u8; 1];
    assert_eq!(
        request.socket.try_read(&mut byte).unwrap_err().kind(),
        std::io::ErrorKind::ConnectionReset
    );

    // Act: Accept tries to send the zero-size FILE terminal chunk over RST TCP.
    let error = request
        .process_consent(&consent_frame(
            sharing_nearby::connection_response_frame::Status::Accept,
        ))
        .await
        .unwrap_err();

    // Assert: network write error is preserved, no fake Finished/retirement,
    // final notification is failure, and the original opened source survives.
    assert!(error.root_cause().downcast_ref::<std::io::Error>().is_some());
    assert_eq!(request.state.state, State::SendingFiles);
    assert_eq!(request.state.transferred_files.len(), 1);
    assert_eq!(request.state.transfer_metadata.as_ref().unwrap().ack_bytes, 0);
    assert!(source.upgrade().is_some());
    request.record_termination(State::Disconnected);
    let failed = control.snapshot();
    assert_eq!(failed.state, State::Disconnected);
    assert_eq!(failed.details.files, original.details.files);
    assert_eq!(failed.details.transferred_bytes, 0);
    request.record_termination(State::Finished);
    assert_eq!(control.snapshot().state, State::Disconnected);
    drop(request);
    assert!(source.upgrade().is_none());
    assert_eq!(std::fs::read(&path).unwrap(), b"");
}

#[tokio::test]
async fn given_peer_reset_after_first_real_file_chunk_when_sending_then_no_false_finish_or_source_loss(
) {
    // Arrange: a sparse 32-MiB regular file cannot fit in the constrained TCP
    // buffers. Both peers use actual AES/HMAC and FILE framing.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private-large-source");
    let marker = b"first-outgoing-block";
    std::fs::write(&path, marker).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(32 * 1024 * 1024)
        .unwrap();
    let (mut request, mut peer, _hub, control) =
        managed_request(crate::prepared_source::PreparedSource::from(selection(
            &path,
            "large.bin",
        )))
        .await;
    let source = match &request.payload {
        OutboundPayload::NativeSource(source) => Arc::downgrade(source),
        _ => panic!("expected opened native source"),
    };
    introduce(&mut request, &mut peer).await;
    let original = control.snapshot();
    assert_eq!(original.details.total_bytes, 32 * 1024 * 1024);
    assert_eq!(original.details.files.len(), 1);
    rustix::net::sockopt::set_socket_send_buffer_size(&request.socket, 16 * 1024).unwrap();
    rustix::net::sockopt::set_socket_recv_buffer_size(&peer, 16 * 1024).unwrap();

    // Act: allow exactly one observable encrypted chunk to arrive, then
    // abort the receiving TCP peer. Synchronization is by real protocol
    // readiness; timeout only diagnoses a stalled test.
    let (send_result, first_chunk) = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        async {
            tokio::join!(
                request.process_consent(&consent_frame(
                    sharing_nearby::connection_response_frame::Status::Accept,
                )),
                async {
                    let frame = read_native_frame(&mut peer, 3).await;
                    let transfer = frame.v1.unwrap().payload_transfer.unwrap();
                    let header = transfer.payload_header.unwrap();
                    assert_eq!(header.total_size(), 32 * 1024 * 1024);
                    assert_eq!(header.file_name.as_deref(), Some("large.bin"));
                    let body = transfer.payload_chunk.unwrap();
                    assert_eq!(body.flags(), 0);
                    assert_eq!(body.offset(), 0);
                    assert_eq!(body.body().len(), 512 * 1024);
                    assert!(body.body().starts_with(marker));
                    rustix::net::sockopt::set_socket_linger(
                        &peer,
                        Some(std::time::Duration::ZERO),
                    )
                    .unwrap();
                    drop(peer);
                    body.body().len()
                },
            )
        },
    )
    .await
    .expect("first FILE chunk and eventual TCP reset must both complete");
    let error = send_result.expect_err("TCP reset must interrupt the large file");
    assert!(error.root_cause().downcast_ref::<std::io::Error>().is_some());

    // Assert: some accepted bytes, strictly below the advertised total.
    // TCP local write counts are not remote delivery receipts.
    assert_eq!(first_chunk, 512 * 1024);
    assert_eq!(request.state.state, State::SendingFiles);
    let acknowledged = request.state.transfer_metadata.as_ref().unwrap().ack_bytes;
    assert!(acknowledged >= first_chunk as u64);
    assert!(acknowledged < 32 * 1024 * 1024);
    assert_eq!(request.state.transferred_files.len(), 1);
    assert!(source.upgrade().is_some());
    request.record_termination(State::Disconnected);
    let failed = control.snapshot();
    assert_eq!(failed.state, State::Disconnected);
    assert_eq!(failed.details.files, original.details.files);
    assert_eq!(failed.details.total_bytes, 32 * 1024 * 1024);
    assert_eq!(failed.details.transferred_bytes, acknowledged);
    request.record_termination(State::Finished);
    assert_eq!(control.snapshot().state, State::Disconnected);

    drop(request);
    assert!(source.upgrade().is_none());
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 32 * 1024 * 1024);
    assert!(std::fs::read(&path).unwrap().starts_with(marker));
}

#[tokio::test]
async fn given_managed_outbound_files_when_completed_then_metadata_retains_manifest_and_exact_bytes(
) {
    // Arrange: opened sources use protocol display names, never their disk paths.
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("private-first");
    let second = directory.path().join("private-second");
    std::fs::write(&first, b"abc").unwrap();
    std::fs::write(&second, b"12345").unwrap();
    let selected = OpenedSelection::from_files(
        vec![
            OpenedFile::adopt(File::open(&first).unwrap(), "folder/first.txt".into()).unwrap(),
            OpenedFile::adopt(File::open(&second).unwrap(), "second.txt".into()).unwrap(),
        ],
        SelectionLimits::default(),
    )
    .unwrap();
    let (mut request, mut peer, _hub, control) = managed_request(selected.into()).await;
    // Act / Assert: the managed snapshot is ready before remote consent.
    introduce(&mut request, &mut peer).await;
    let initial = control.snapshot();
    assert_eq!(initial.state, State::SentIntroduction);
    assert_eq!(initial.details.status, DetailsStatus::Ready);
    assert_eq!(initial.details.sender, "Advertised phone");
    assert_eq!(initial.details.device, DeviceKind::Phone);
    assert_eq!(initial.details.kind, PayloadKind::Files);
    assert_eq!(
        initial
            .details
            .files
            .iter()
            .map(|f| (f.name.as_str(), f.size))
            .collect::<Vec<_>>(),
        vec![("folder/first.txt", 3), ("second.txt", 5)]
    );
    assert_eq!(initial.details.total_bytes, 8);
    assert_eq!(initial.details.transferred_bytes, 0);
    request
        .process_consent(&consent_frame(
            sharing_nearby::connection_response_frame::Status::Accept,
        ))
        .await
        .unwrap();
    let mut bodies = Vec::new();
    for sequence in 3..=6 {
        let frame = read_native_frame(&mut peer, sequence).await;
        let chunk = frame
            .v1
            .unwrap()
            .payload_transfer
            .unwrap()
            .payload_chunk
            .unwrap();
        if chunk.flags() == 0 {
            bodies.push(chunk.body.unwrap());
        }
    }
    bodies.sort();
    assert_eq!(bodies, vec![b"12345".to_vec(), b"abc".to_vec()]);
    let disconnect = read_native_frame(&mut peer, 7).await;
    assert_eq!(
        disconnect.v1.unwrap().r#type(),
        location_nearby_connections::v1_frame::FrameType::Disconnection
    );
    let completed = control.snapshot();
    assert_eq!(completed.state, State::Finished);
    assert_eq!(completed.details.status, DetailsStatus::Ready);
    assert_eq!(completed.details.files, initial.details.files);
    assert_eq!(completed.details.total_bytes, 8);
    assert_eq!(completed.details.transferred_bytes, 8);
    assert!(request.state.transferred_files.is_empty());
    assert!(completed.received_text.is_none());
    request.record_termination(State::Disconnected);
    assert_eq!(control.snapshot().state, State::Finished);
}

#[tokio::test]
async fn given_managed_outbound_text_when_completed_then_metadata_uses_kind_and_utf8_bytes() {
    for (text, kind) in [
        ("Private café text", PayloadKind::Text),
        ("https://example.com/café", PayloadKind::Url),
    ] {
        // Arrange
        let source = crate::prepared_source::PreparedSource::Text(
            crate::prepared_source::NativeText::new(text.into()).unwrap(),
        );
        let (mut request, mut peer, _hub, control) = managed_request(source).await;
        // Act / Assert
        introduce(&mut request, &mut peer).await;
        let prepared = control.snapshot();
        assert_eq!(prepared.details.status, DetailsStatus::Ready);
        assert_eq!(prepared.details.kind, kind);
        assert_eq!(prepared.details.total_bytes, text.len() as u64);
        assert_eq!(prepared.details.transferred_bytes, 0);
        assert!(prepared.details.files.is_empty());
        assert!(!format!("{:?}", prepared.details).contains(text));
        request
            .process_consent(&consent_frame(
                sharing_nearby::connection_response_frame::Status::Accept,
            ))
            .await
            .unwrap();
        let sent = read_native_frame(&mut peer, 3).await;
        assert_eq!(
            sent.v1
                .unwrap()
                .payload_transfer
                .unwrap()
                .payload_chunk
                .unwrap()
                .body(),
            text.as_bytes()
        );
        read_native_frame(&mut peer, 4).await;
        read_native_frame(&mut peer, 5).await;
        let completed = control.snapshot();
        assert_eq!(completed.state, State::Finished);
        assert_eq!(completed.details.status, DetailsStatus::Ready);
        assert_eq!(completed.details.kind, kind);
        assert_eq!(completed.details.total_bytes, text.len() as u64);
        assert_eq!(completed.details.transferred_bytes, text.len() as u64);
        assert!(completed.received_text.is_none());
    }
}

#[tokio::test]
async fn given_prepared_outbound_when_interrupted_then_terminal_metadata_preserves_actual_progress()
{
    for phase in [State::Rejected, State::Cancelled, State::Disconnected] {
        // Arrange
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"abc").unwrap();
        let (mut request, mut peer, _hub, control) =
            managed_request(selection(file.path(), "display.txt").into()).await;
        introduce(&mut request, &mut peer).await;
        // Act: termination observes the actual state, including its last progress.
        request.state.transfer_metadata.as_mut().unwrap().ack_bytes = 2;
        request.record_termination(phase.clone());
        // Assert
        let interrupted = control.snapshot();
        assert_eq!(interrupted.state, phase);
        assert_eq!(interrupted.details.status, DetailsStatus::Ready);
        assert_eq!(interrupted.details.files.len(), 1);
        assert_eq!(interrupted.details.files[0].name, "display.txt");
        assert_eq!(interrupted.details.files[0].size, 3);
        assert_eq!(interrupted.details.total_bytes, 3);
        assert_eq!(interrupted.details.transferred_bytes, 2);
        request.record_termination(State::Finished);
        assert_eq!(control.snapshot().state, phase);
        assert_eq!(control.snapshot().details.transferred_bytes, 2);
    }
}
