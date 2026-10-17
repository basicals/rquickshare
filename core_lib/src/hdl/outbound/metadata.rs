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
