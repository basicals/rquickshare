use super::*;

async fn admitted_with_directory(
    directory: Option<&Path>,
) -> (
    TestRequest,
    crate::sessions::SessionControl,
    tokio::io::DuplexStream,
) {
    let mut request = test_request();
    if let Some(directory) = directory {
        request.receive_directory =
            crate::receive::ReceivePolicy::new(Some(directory.to_owned())).snapshot();
    }
    let (socket, peer) = tokio::io::duplex(8192);
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
                file_metadata("a.bin", 101, 1),
                file_metadata("b.bin", 102, 3),
            ],
            vec![],
            vec![],
        ))
        .await
        .unwrap();
    let token = control.snapshot().consent.unwrap();
    let receipt = control.respond(token, 1, true).unwrap();
    let command = crate::sessions::consent_command(&mut request.session)
        .await
        .unwrap();
    let state = request.state.state.clone();
    assert_eq!(
        request.session.as_mut().unwrap().apply(command, &state),
        Some(true)
    );
    receipt.wait().await.unwrap();
    (request, control, peer)
}

async fn admitted() -> (
    TestRequest,
    crate::sessions::SessionControl,
    tokio::io::DuplexStream,
) {
    admitted_with_directory(None).await
}

// This critical user-decline fixture uses the actual managed consent lane and
// encrypted production reply. Unlike admitted(), it never grants file access.
async fn declined() -> (
    TestRequest,
    crate::sessions::SessionControl,
    tokio::io::DuplexStream,
) {
    let mut request = test_request();
    let (socket, peer) = tokio::io::duplex(8192);
    request.socket = socket;
    request.state.encrypt_key = Some(vec![0x11; 32]);
    request.state.send_hmac_key = Some(vec![0x22; 32]);
    request.state.remote_device_info = Some(RemoteDeviceInfo {
        name: "phone".into(),
        device_type: crate::utils::DeviceType::Phone,
    });
    request.state.pin_code = Some("0123".into());
    let hub = crate::sessions::SessionHub::new(43).unwrap();
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
                file_metadata("a.bin", 101, 1),
                file_metadata("b.bin", 102, 3),
            ],
            vec![],
            vec![],
        ))
        .await
        .unwrap();
    assert_eq!(request.state.state, State::WaitingForUserConsent);
    let token = control.snapshot().consent.unwrap();
    let receipt = control.respond(token, 1, false).unwrap();
    let command = crate::sessions::consent_command(&mut request.session)
        .await
        .unwrap();
    let state = request.state.state.clone();
    assert_eq!(
        request.session.as_mut().unwrap().apply(command, &state),
        Some(false)
    );
    receipt.wait().await.unwrap();
    (request, control, peer)
}

#[tokio::test]
async fn given_managed_user_decline_when_two_files_are_offered_then_real_encrypted_reject_has_no_file_authority(
) {
    use tokio::io::AsyncReadExt;

    // Arrange: a real pending two-file consent, with no staged file descriptors.
    let (mut request, control, peer_socket) = declined().await;
    let directory = request._directory.path().to_owned();
    assert!(request.state.transferred_files.values().all(|file| file.file.is_none()));
    assert!(request.publications.is_empty());
    assert!(std::fs::read_dir(&directory).unwrap().next().is_none());

    let (sender, _) = tokio::sync::broadcast::channel(4);
    let mut peer = InboundRequest::new(peer_socket, "reject-peer".into(), sender);
    peer.state.decrypt_key = Some(vec![0x11; 32]);
    peer.state.recv_hmac_key = Some(vec![0x22; 32]);

    // Act: deny via the same production consent handler used by the daemon.
    let stopped = request.apply_consent(false).await.unwrap_err();

    // Assert: the peer sees exactly a real encrypted Reject, never Accept.
    assert!(stopped.is::<crate::errors::AppError>());
    let raw = read_plain_frame_from(&mut peer.socket).await.unwrap();
    let secure = SecureMessage::decode(raw.as_slice()).unwrap();
    let offline = peer.decrypt_secure_message(&secure).await.unwrap();
    let transfer = offline.v1.unwrap().payload_transfer.unwrap();
    let header = transfer.payload_header.as_ref().unwrap();
    let first = transfer.payload_chunk.as_ref().unwrap();
    assert_eq!(first.offset(), 0);
    assert_eq!(first.flags(), 0);
    let frame = sharing_nearby::Frame::decode(first.body.as_ref().unwrap().as_slice()).unwrap();
    let response = frame.v1.unwrap();
    assert_eq!(response.r#type(), sharing_nearby::v1_frame::FrameType::Response);
    assert_eq!(
        response.connection_response.unwrap().status(),
        sharing_nearby::connection_response_frame::Status::Reject
    );
    let size = first.body.as_ref().unwrap().len();
    assert_eq!(header.total_size(), size as i64);

    let raw = read_plain_frame_from(&mut peer.socket).await.unwrap();
    let secure = SecureMessage::decode(raw.as_slice()).unwrap();
    let final_chunk = peer
        .decrypt_secure_message(&secure)
        .await
        .unwrap()
        .v1
        .unwrap()
        .payload_transfer
        .unwrap()
        .payload_chunk
        .unwrap();
    assert_eq!(final_chunk.offset(), size as i64);
    assert_eq!(final_chunk.flags(), 1);
    assert!(final_chunk.body.as_deref().unwrap_or_default().is_empty());
    let mut probe = [0_u8; 1];
    let no_more_frames = peer.socket.read(&mut probe);
    tokio::pin!(no_more_frames);
    assert!(futures::poll!(no_more_frames.as_mut()).is_pending());

    let snapshot = control.snapshot();
    assert_eq!(request.state.state, State::Rejected);
    assert_eq!(snapshot.state, State::Rejected);
    assert_eq!(snapshot.details.transferred_bytes, 0);
    assert!(snapshot.received_files.is_none());
    assert!(request.pending_files.is_none());
    assert!(request.publications.is_empty());
    assert!(request.state.transferred_files.values().all(|file| file.file.is_none()));
    assert!(std::fs::read_dir(&directory).unwrap().next().is_none());
    request.record_termination(State::Disconnected);
    assert_eq!(request.state.state, State::Rejected);
    assert_eq!(control.snapshot().state, State::Rejected);
}

#[tokio::test]
async fn given_user_decline_and_peer_socket_loss_when_reject_cannot_be_sent_then_terminal_rejected_has_no_publication(
) {
    // Arrange: the real managed negative answer is already committed, but
    // the peer drops the transport before the Reject write.
    let (mut request, control, peer) = declined().await;
    let directory = request._directory.path().to_owned();
    drop(peer);

    // Act: network failure must not reverse the user's refusal.
    let failed_send = request.apply_consent(false).await.unwrap_err();

    // Assert: no implicit Accept, retry, publication or receipt exists.
    assert!(!failed_send.is::<crate::errors::AppError>());
    assert_eq!(request.state.state, State::Rejected);
    assert_eq!(control.snapshot().state, State::Rejected);
    assert_eq!(control.snapshot().details.transferred_bytes, 0);
    assert!(control.snapshot().received_files.is_none());
    assert!(request.pending_files.is_none());
    assert!(request.publications.is_empty());
    assert!(request.state.transferred_files.values().all(|file| file.file.is_none()));
    assert!(std::fs::read_dir(&directory).unwrap().next().is_none());
    request.record_termination(State::Disconnected);
    assert_eq!(request.state.state, State::Rejected);
    assert_eq!(control.snapshot().state, State::Rejected);
}

#[tokio::test]
async fn given_remote_cancel_after_accept_before_or_after_first_file_when_handled_then_only_durable_members_survive(
) {
    use crate::byte_payload::tests::frame;
    use tokio::io::AsyncReadExt;

    for first_completed in [false, true] {
        // Arrange: the *real* two-file consent and encrypted Accept precede
        // remote cancellation. The second member has no terminal data chunk.
        let (mut request, control, peer_socket) = admitted().await;
        let directory = request._directory.path().to_owned();
        request.state.encryption_done = true;
        request.accept_transfer().await.unwrap();
        assert_eq!(request.state.state, State::ReceivingFiles);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);

        if first_completed {
            request
                .process_offline_frame(received_file_frame(101, 1, 0, b"a", 1))
                .await
                .unwrap();
            request
                .process_offline_frame(received_file_frame(102, 3, 0, b"b", 0))
                .await
                .unwrap();
            assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
            assert!(!directory.join("b.bin").exists());
        }

        let mut peer = InboundRequest::new(
            peer_socket,
            "cancel-peer".into(),
            tokio::sync::broadcast::channel(4).0,
        );
        peer.state.decrypt_key = Some(vec![0; 32]);
        peer.state.recv_hmac_key = Some(vec![0; 32]);

        // Read and validate both previously emitted encrypted Accept chunks.
        let raw = read_plain_frame_from(&mut peer.socket).await.unwrap();
        let secure = SecureMessage::decode(raw.as_slice()).unwrap();
        let transfer = peer
            .decrypt_secure_message(&secure)
            .await
            .unwrap()
            .v1
            .unwrap()
            .payload_transfer
            .unwrap();
        let accepted = sharing_nearby::Frame::decode(
            transfer.payload_chunk.unwrap().body.unwrap().as_slice(),
        )
        .unwrap();
        assert_eq!(
            accepted.v1.unwrap().connection_response.unwrap().status(),
            sharing_nearby::connection_response_frame::Status::Accept
        );
        let raw = read_plain_frame_from(&mut peer.socket).await.unwrap();
        let secure = SecureMessage::decode(raw.as_slice()).unwrap();
        let end = peer
            .decrypt_secure_message(&secure)
            .await
            .unwrap()
            .v1
            .unwrap()
            .payload_transfer
            .unwrap()
            .payload_chunk
            .unwrap();
        assert_eq!(end.flags(), 1);

        let cancel = sharing_nearby::Frame {
            version: Some(sharing_nearby::frame::Version::V1.into()),
            v1: Some(sharing_nearby::V1Frame {
                r#type: Some(sharing_nearby::v1_frame::FrameType::Cancel.into()),
                ..Default::default()
            }),
        }
        .encode_to_vec();

        // Act: the actual byte-payload/reassembly/Cancel path handles a
        // complete untrusted sharing control frame after file admission.
        let terminated = request
            .process_offline_frame(frame(901, cancel.len(), 0, &cancel, true))
            .await
            .unwrap_err();

        // Assert: the daemon sends Disconnection and never reports Finished.
        assert!(terminated.is::<crate::errors::AppError>());
        assert_eq!(request.state.state, State::Cancelled);
        let raw = read_plain_frame_from(&mut peer.socket).await.unwrap();
        let secure = SecureMessage::decode(raw.as_slice()).unwrap();
        let offline = peer.decrypt_secure_message(&secure).await.unwrap();
        assert_eq!(
            offline.v1.unwrap().r#type(),
            location_nearby_connections::v1_frame::FrameType::Disconnection
        );
        let mut extra = [0_u8; 1];
        let no_more = peer.socket.read(&mut extra);
        tokio::pin!(no_more);
        assert!(futures::poll!(no_more.as_mut()).is_pending());

        let snapshot = control.snapshot();
        assert_eq!(snapshot.state, State::Cancelled);
        // Progress counts bytes actually written, including an unfinished
        // staged second member; durable receipt still contains only complete files.
        assert_eq!(
            snapshot.details.transferred_bytes,
            if first_completed { 2 } else { 0 }
        );
        assert!(request.publications.is_empty());
        assert!(request.pending_files.is_none());
        assert!(request
            .state
            .transferred_files
            .values()
            .all(|file| file.file.is_none()));
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), usize::from(first_completed));
        assert!(!directory.join("b.bin").exists());
        if first_completed {
            assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
            let received = snapshot.received_files.as_ref().unwrap();
            assert_eq!(received.len(), 1);
            assert!(received.subset_of(&snapshot.details));
            assert!(!received.valid_for(&snapshot.details));
            assert_eq!(received.details()[0].name, "a.bin");
            let mut body = [0_u8; 1];
            std::os::unix::fs::FileExt::read_exact_at(&received.open(0).unwrap(), &mut body, 0)
                .unwrap();
            assert_eq!(&body, b"a");
        } else {
            assert!(!directory.join("a.bin").exists());
            assert!(snapshot.received_files.is_none());
        }
        request.record_termination(State::Disconnected);
        assert_eq!(request.state.state, State::Cancelled);
        assert_eq!(control.snapshot().state, State::Cancelled);
    }
}

// A fresh end-to-end managed Introduction fixture shared by the P0 text,
// nested-folder and explicit-retry cases. Never shortcuts the consent handler.
async fn critical_introduction(
    files: Vec<sharing_nearby::FileMetadata>,
    text: Vec<sharing_nearby::TextMetadata>,
    directory: Option<&Path>,
    seed: u128,
) -> (
    TestRequest,
    crate::sessions::SessionControl,
    tokio::io::DuplexStream,
) {
    let mut request = test_request();
    if let Some(path) = directory {
        request.receive_directory =
            crate::receive::ReceivePolicy::new(Some(path.to_owned())).snapshot();
    }
    let (socket, peer) = tokio::io::duplex(8192);
    request.socket = socket;
    request.state.encrypt_key = Some(vec![0; 32]);
    request.state.send_hmac_key = Some(vec![0; 32]);
    request.state.remote_device_info = Some(RemoteDeviceInfo {
        name: "phone".into(),
        device_type: crate::utils::DeviceType::Phone,
    });
    request.state.pin_code = Some("0123".into());
    let hub = crate::sessions::SessionHub::new(seed).unwrap();
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
        .process_introduction(&introduction_frame(files, text, vec![]))
        .await
        .unwrap();
    assert_eq!(request.state.state, State::WaitingForUserConsent);
    assert!(control.snapshot().received_files.is_none());
    assert!(control.snapshot().received_text.is_none());
    (request, control, peer)
}

async fn critical_accept(
    request: &mut TestRequest,
    control: &crate::sessions::SessionControl,
) {
    let token = control.snapshot().consent.unwrap();
    let receipt = control.respond(token, 1, true).unwrap();
    let command = crate::sessions::consent_command(&mut request.session)
        .await
        .unwrap();
    let state = request.state.state.clone();
    assert_eq!(
        request.session.as_mut().unwrap().apply(command, &state),
        Some(true)
    );
    receipt.wait().await.unwrap();
    request.apply_consent(true).await.unwrap();
    assert_eq!(request.state.state, State::ReceivingFiles);
}

#[tokio::test]
async fn given_managed_text_or_link_when_explicitly_accepted_and_fragmented_then_exact_utf8_is_exposed_only_after_finished(
) {
    use crate::byte_payload::tests::frame;
    use crate::sessions::details::PayloadKind;

    for (kind, wire_kind, expected_kind, body) in [
        (
            "text",
            text_metadata::Type::Text,
            PayloadKind::Text,
            "olá 🌍",
        ),
        (
            "url",
            text_metadata::Type::Url,
            PayloadKind::Url,
            "https://example.test/café",
        ),
    ] {
        // Arrange: actual two-byte-codepoint split, full metadata, real token.
        let source = body.as_bytes();
        let (mut request, control, _peer) = critical_introduction(
            vec![],
            vec![sharing_nearby::TextMetadata {
                payload_id: Some(701),
                size: Some(source.len() as i64),
                r#type: Some(wire_kind.into()),
                text_title: Some(kind.into()),
                ..Default::default()
            }],
            None,
            if kind == "text" { 71 } else { 72 },
        )
        .await;
        assert!(request.state.transfer_metadata.as_ref().unwrap().text_payload.is_none());
        assert!(std::fs::read_dir(request._directory.path()).unwrap().next().is_none());

        // Act: approval is a real managed command; bytes are not exposed early.
        critical_accept(&mut request, &control).await;
        assert!(control.snapshot().received_text.is_none());
        let split = source.len() - 1; // End the first chunk inside the final UTF-8 scalar.
        request
            .process_offline_frame(frame(701, source.len(), 0, &source[..split], false))
            .await
            .unwrap();
        assert_eq!(request.state.state, State::ReceivingFiles);
        assert!(control.snapshot().received_text.is_none());
        let finished = request
            .process_offline_frame(frame(701, source.len(), split, &source[split..], true))
            .await
            .unwrap_err();

        // Assert: immutable exact bytes are available only after true completion.
        assert!(finished.is::<crate::errors::AppError>(), "{kind}: {finished:?}");
        assert_eq!(request.state.state, State::Finished);
        let snapshot = control.snapshot();
        assert_eq!(snapshot.state, State::Finished);
        assert_eq!(snapshot.details.kind, expected_kind);
        assert_eq!(snapshot.details.transferred_bytes, source.len() as u64);
        let received = snapshot.received_text.as_ref().unwrap();
        assert_eq!(received.kind(), expected_kind);
        assert_eq!(received.text().as_bytes(), source);
        assert!(snapshot.received_files.is_none());
        assert!(request.state.byte_payloads.is_empty());
        assert!(std::fs::read_dir(request._directory.path()).unwrap().next().is_none());
        request.record_termination(State::Disconnected);
        assert_eq!(control.snapshot().state, State::Finished);
    }
}

#[tokio::test]
async fn given_nested_folder_and_existing_root_when_managed_transfer_completes_then_original_tree_and_verified_files_coexist(
) {
    use std::os::unix::fs::FileExt;

    let target = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(target.path().join("Trip")).unwrap();
    std::fs::write(target.path().join("Trip/user.txt"), b"user-owned").unwrap();
    let (mut request, control, _peer) = critical_introduction(
        vec![
            file_metadata_with_parent("readme.txt", 801, 3, "Trip"),
            file_metadata_with_parent("photo.bin", 802, 2, "Trip/photos"),
        ],
        vec![],
        Some(target.path()),
        81,
    )
    .await;

    // Arrange: both nested names are collision-remapped as one tree before consent.
    let listed = request.state.transfer_metadata.as_ref().unwrap().files.as_ref().unwrap();
    assert_eq!(listed, &vec!["1_Trip/readme.txt", "1_Trip/photos/photo.bin"]);
    assert!(!target.path().join("1_Trip").exists());
    assert!(control.snapshot().received_files.is_none());

    // Act: real managed Accept and distinct terminal FILE frames.
    critical_accept(&mut request, &control).await;
    assert!(!target.path().join("1_Trip/readme.txt").exists());
    request
        .process_offline_frame(received_file_frame(801, 3, 0, b"abc", 1))
        .await
        .unwrap();
    assert!(!target.path().join("1_Trip/photos/photo.bin").exists());
    let finished = request
        .process_offline_frame(received_file_frame(802, 2, 0, b"xy", 1))
        .await
        .unwrap_err();

    // Assert: both durable members and original user tree are immutable.
    assert!(finished.is::<crate::errors::AppError>());
    assert_eq!(request.state.state, State::Finished);
    assert_eq!(std::fs::read(target.path().join("Trip/user.txt")).unwrap(), b"user-owned");
    assert_eq!(std::fs::read(target.path().join("1_Trip/readme.txt")).unwrap(), b"abc");
    assert_eq!(
        std::fs::read(target.path().join("1_Trip/photos/photo.bin")).unwrap(),
        b"xy"
    );
    let snapshot = control.snapshot();
    assert_eq!(snapshot.details.total_bytes, 5);
    assert_eq!(snapshot.details.transferred_bytes, 5);
    let receipt = snapshot.received_files.as_ref().unwrap();
    assert_eq!(receipt.len(), 2);
    assert!(receipt.valid_for(&snapshot.details));
    // Receipts are sorted by relative path, not by incoming payload order.
    assert_eq!(receipt.details()[0].name, "1_Trip/photos/photo.bin");
    assert_eq!(receipt.details()[1].name, "1_Trip/readme.txt");
    let mut photo = [0; 2];
    FileExt::read_exact_at(&receipt.open(0).unwrap(), &mut photo, 0).unwrap();
    assert_eq!(&photo, b"xy");
    let mut readme = [0; 3];
    FileExt::read_exact_at(&receipt.open(1).unwrap(), &mut readme, 0).unwrap();
    assert_eq!(&readme, b"abc");
    assert!(request.publications.is_empty());
}

#[tokio::test]
async fn given_interrupted_multifile_receive_when_retry_is_explicit_then_new_consent_preserves_first_and_writes_only_new_names(
) {
    use tokio::io::AsyncWriteExt;

    let root = tempfile::tempdir().unwrap();
    let files = || vec![
        file_metadata("memo.txt", 901, 1),
        file_metadata("photo.bin", 902, 3),
    ];

    // Arrange: initial user-approved transfer finishes one member only.
    let (mut failed, first_control, mut first_peer) =
        critical_introduction(files(), vec![], Some(root.path()), 91).await;
    critical_accept(&mut failed, &first_control).await;
    failed
        .process_offline_frame(received_file_frame(901, 1, 0, b"a", 1))
        .await
        .unwrap();
    failed
        .process_offline_frame(received_file_frame(902, 3, 0, b"b", 0))
        .await
        .unwrap();
    failed.receiver = None;
    let prefix = 5_u32.to_be_bytes();
    first_peer.write_all(&prefix[..2]).await.unwrap();
    drop(first_peer);

    // Act: the first real transport is interrupted and cannot become Finished.
    let interrupted = failed.handle().await.unwrap_err();
    assert_eq!(
        interrupted
            .root_cause()
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
        Some(std::io::ErrorKind::UnexpectedEof)
    );
    failed.record_termination(State::Disconnected);
    assert_eq!(first_control.snapshot().state, State::Disconnected);
    assert_eq!(std::fs::read(root.path().join("memo.txt")).unwrap(), b"a");
    assert!(!root.path().join("photo.bin").exists());
    let partial = first_control.snapshot().received_files.as_ref().unwrap().clone();
    assert!(partial.subset_of(&first_control.snapshot().details));
    assert!(!partial.valid_for(&first_control.snapshot().details));

    // A fresh separate transfer must wait at consent; no automatic retry.
    let (mut retried, retry_control, _retry_peer) =
        critical_introduction(files(), vec![], Some(root.path()), 92).await;
    assert_ne!(first_control.snapshot().id, retry_control.snapshot().id);
    assert_eq!(retried.state.state, State::WaitingForUserConsent);
    let renamed = retried.state.transfer_metadata.as_ref().unwrap().files.as_ref().unwrap();
    assert_eq!(renamed, &vec!["1_memo.txt", "photo.bin"]);
    assert!(!root.path().join("1_memo.txt").exists());
    assert!(!root.path().join("photo.bin").exists());
    assert!(retry_control.snapshot().received_files.is_none());

    // The user explicitly grants a NEW token; only then are bytes admitted.
    critical_accept(&mut retried, &retry_control).await;
    retried
        .process_offline_frame(received_file_frame(901, 1, 0, b"z", 1))
        .await
        .unwrap();
    let finished = retried
        .process_offline_frame(received_file_frame(902, 3, 0, b"bcd", 1))
        .await
        .unwrap_err();
    assert!(finished.is::<crate::errors::AppError>());
    assert_eq!(retry_control.snapshot().state, State::Finished);
    assert_eq!(std::fs::read(root.path().join("memo.txt")).unwrap(), b"a");
    assert_eq!(std::fs::read(root.path().join("1_memo.txt")).unwrap(), b"z");
    assert_eq!(std::fs::read(root.path().join("photo.bin")).unwrap(), b"bcd");
    assert_eq!(retry_control.snapshot().details.transferred_bytes, 4);
    let complete = retry_control.snapshot().received_files.as_ref().unwrap().clone();
    assert!(complete.valid_for(&retry_control.snapshot().details));
    assert_eq!(complete.len(), 2);
    assert!(retried.publications.is_empty());
    failed.record_termination(State::Finished);
    assert_eq!(first_control.snapshot().state, State::Disconnected);
}

#[tokio::test]
async fn given_missing_receive_root_before_introduction_when_pinning_then_no_file_consent_or_authority_is_created(
) {
    // Arrange: remove only this test's empty download directory before the
    // request attempts to pin it. No accepted manifest can be constructed.
    let mut request = test_request();
    let root = request._directory.path().to_owned();
    std::fs::remove_dir(&root).unwrap();
    let introduction = introduction_frame(
        vec![file_metadata("a.bin", 101, 1)],
        vec![],
        vec![],
    );

    // Act: pinning an absent directory fails before any Accept can be sent.
    let error = request.process_introduction(&introduction).await.unwrap_err();

    // Assert: the request neither acquires file resources nor invents success.
    assert_eq!(error.to_string(), "Receive directory is unavailable");
    assert!(request.receive_root.is_none());
    assert!(request.state.transfer_metadata.is_none());
    assert!(request.state.transferred_files.is_empty());
    assert!(request.pending_files.is_none());
    assert!(request.publications.is_empty());
    request.record_termination(State::Disconnected);
    assert_eq!(request.state.state, State::Disconnected);
    assert!(!root.exists());
}

#[tokio::test]
async fn given_inaccessible_or_unsafe_receive_storage_after_introduction_when_preparing_then_no_accept_or_partial_owner_is_left(
) {
    use std::os::unix::fs::symlink;
    use tokio::io::AsyncReadExt;

    for case in ["removed-root", "parent-file", "parent-symlink", "unsupported-fs"] {
        // Arrange: real accepted managed consent and pinned receive root.
        // procfs is read-only/unsupported for O_TMPFILE; only an unnamed
        // creation syscall is attempted, never a named write there.
        let unsupported = case == "unsupported-fs";
        let (mut request, control, mut peer) = admitted_with_directory(
            if unsupported { Some(Path::new("/proc")) } else { None },
        )
        .await;
        let directory = request._directory.path().to_owned();
        let outside = tempfile::tempdir().unwrap();
        let before = control.snapshot();
        assert_eq!(before.state, State::WaitingForUserConsent);
        assert_eq!(before.details.total_bytes, 4);
        assert_eq!(before.details.transferred_bytes, 0);
        assert_eq!(before.details.files.len(), 2);
        assert!(before.received_files.is_none());
        assert!(request.pending_files.is_some());
        let mut sentinel = None;

        match case {
            "removed-root" => {
                // The original directory FD survives, but the unlinked
                // directory cannot allocate an anonymous inode.
                std::fs::remove_dir(&directory).unwrap();
            }
            "parent-file" | "parent-symlink" => {
                // Preserve the actual HashMap iteration order used by
                // accept_transfer: stage the first member successfully,
                // then force the second destination-parent lookup to fail.
                let ids = request
                    .state
                    .transferred_files
                    .keys()
                    .copied()
                    .collect::<Vec<_>>();
                assert_eq!(ids.len(), 2);
                let blocked = directory.join("blocked");
                if case == "parent-file" {
                    std::fs::write(&blocked, b"user-owned").unwrap();
                } else {
                    symlink(outside.path(), &blocked).unwrap();
                }
                request
                    .state
                    .transferred_files
                    .get_mut(&ids[1])
                    .unwrap()
                    .file_url = blocked.join("second.bin");
                sentinel = Some(blocked);
            }
            "unsupported-fs" => {
                assert!(Path::new("/proc").is_dir());
            }
            _ => unreachable!(),
        }

        // Act: storage preparation fails before the encrypted Accept frame.
        let error = request.accept_transfer().await.unwrap_err();

        // Assert before termination: no frame, local progress, state advance
        // or unexpectedly committed file/receipt. This is not a timed read.
        assert_eq!(
            error.to_string(),
            "Receive filesystem cannot prepare a safe file",
            "{case}"
        );
        let mut probe = [0_u8; 1];
        let read = peer.read(&mut probe);
        tokio::pin!(read);
        assert!(futures::poll!(read.as_mut()).is_pending(), "{case}");
        assert_eq!(request.state.state, State::WaitingForUserConsent);
        assert_eq!(request.state.transfer_metadata.as_ref().unwrap().ack_bytes, 0);
        assert_eq!(request.state.transferred_files.len(), 2);
        assert!(request
            .state
            .transferred_files
            .values()
            .all(|file| file.file.is_none()));
        assert!(request.publications.is_empty());
        assert!(control.snapshot().received_files.is_none());
        assert_eq!(control.snapshot().details, before.details);

        request.record_termination(State::Disconnected);
        let after = control.snapshot();
        assert_eq!(after.state, State::Disconnected, "{case}");
        assert_eq!(after.details.files, before.details.files);
        assert_eq!(after.details.total_bytes, 4);
        assert_eq!(after.details.transferred_bytes, 0);
        assert!(after.received_files.is_none());
        assert!(request.pending_files.is_none());
        assert!(request.publications.is_empty());
        assert!(request
            .state
            .transferred_files
            .values()
            .all(|file| file.file.is_none()));
        request.record_termination(State::Finished);
        assert_eq!(control.snapshot().state, State::Disconnected, "{case}");

        if case == "removed-root" {
            assert!(!directory.exists());
        } else {
            assert!(!directory.join("a.bin").exists(), "{case}");
            assert!(!directory.join("b.bin").exists(), "{case}");
            if let Some(blocked) = sentinel {
                if case == "parent-file" {
                    assert_eq!(std::fs::read(blocked).unwrap(), b"user-owned");
                } else {
                    assert!(blocked.symlink_metadata().unwrap().file_type().is_symlink());
                }
            } else {
                assert!(std::fs::read_dir(&directory).unwrap().next().is_none());
            }
        }
        assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
    }
}

#[tokio::test]
async fn given_atomic_receive_when_second_file_is_interrupted_then_only_complete_first_member_is_retained_and_revealable(
) {
    for case in ["cancel", "disconnect", "short", "write", "collision"] {
        // Arrange: actual consent/anonymous destinations/frame writes.
        let (mut request, control, _peer) = admitted().await;
        let directory = request._directory.path().to_owned();
        request.accept_transfer().await.unwrap();
        assert!(std::fs::read_dir(&directory).unwrap().next().is_none());
        request
            .process_offline_frame(received_file_frame(101, 1, 0, b"a", 0))
            .await
            .unwrap();
        assert!(!directory.join("a.bin").exists());
        request
            .process_offline_frame(received_file_frame(101, 1, 1, &[], 1))
            .await
            .unwrap();
        assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
        assert!(control.snapshot().received_files.is_none());
        // Act
        if case == "write" {
            request.state.transferred_files.get_mut(&102).unwrap().file = Some(
                std::fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .unwrap(),
            );
            assert!(request
                .process_offline_frame(received_file_frame(102, 3, 0, b"b", 0))
                .await
                .is_err());
        } else {
            request
                .process_offline_frame(received_file_frame(102, 3, 0, b"b", 0))
                .await
                .unwrap();
        }
        if case == "short" {
            assert!(request
                .process_offline_frame(received_file_frame(102, 3, 1, &[], 1))
                .await
                .is_err());
        }
        if case == "collision" {
            std::fs::write(directory.join("b.bin"), b"user-owned").unwrap();
            request
                .process_offline_frame(received_file_frame(102, 3, 1, b"cd", 0))
                .await
                .unwrap();
            assert!(request
                .process_offline_frame(received_file_frame(102, 3, 3, &[], 1))
                .await
                .is_err());
        }
        let terminal = if case == "cancel" {
            State::Cancelled
        } else {
            State::Disconnected
        };
        request.record_termination(terminal.clone());
        // Assert: no whole-group success, exact retained member capability.
        let snapshot = control.snapshot();
        assert_eq!(snapshot.state, terminal);
        assert_eq!(snapshot.details.files.len(), 2);
        let files = snapshot.received_files.as_ref().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files.subset_of(&snapshot.details));
        assert!(!files.valid_for(&snapshot.details));
        assert_eq!(files.details()[0].name, "a.bin");
        assert!(files.open(1).is_err());
        let mut body = [0];
        std::os::unix::fs::FileExt::read_exact_at(&files.open(0).unwrap(), &mut body, 0).unwrap();
        assert_eq!(&body, b"a");
        assert!(request.publications.is_empty());
        assert!(request
            .state
            .transferred_files
            .values()
            .all(|file| file.file.is_none()));
        if case == "collision" {
            assert_eq!(
                std::fs::read(directory.join("b.bin")).unwrap(),
                b"user-owned"
            );
        } else {
            assert!(!directory.join("b.bin").exists());
        }
    }
}

#[tokio::test]
async fn given_receive_preparation_or_accept_socket_failure_when_aborted_then_no_destination_files_are_published(
) {
    for case in ["prepare", "socket", "drop"] {
        // Arrange
        let (mut request, _control, peer) = admitted().await;
        let directory = request._directory.path().to_owned();
        if case == "prepare" {
            let missing = directory.join("parent");
            std::fs::write(&missing, b"untouched").unwrap();
            request
                .state
                .transferred_files
                .get_mut(&102)
                .unwrap()
                .file_url = missing.join("b.bin");
        }
        if case == "socket" {
            drop(peer);
        }
        // Act / Assert
        let result = request.accept_transfer().await;
        if case == "drop" {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
        request.record_termination(State::Disconnected);
        assert!(!directory.join("a.bin").exists());
        assert!(!directory.join("b.bin").exists());
        assert!(request
            .state
            .transferred_files
            .values()
            .all(|file| file.file.is_none()));
        assert!(request.publications.is_empty());
        assert!(request.session.as_ref().unwrap().completed_files.is_none());
        if case == "prepare" {
            assert_eq!(
                std::fs::read(directory.join("parent")).unwrap(),
                b"untouched"
            );
        } else {
            assert!(std::fs::read_dir(directory).unwrap().next().is_none());
        }
    }
}

#[tokio::test]
async fn given_final_data_chunk_when_completed_then_each_member_is_published_once_and_receipt_is_exact(
) {
    for split_second_file in [false, true] {
        // Arrange: real staged destinations, accepted consent and managed receipt.
        let (mut request, control, _peer) = admitted().await;
        let directory = request._directory.path().to_owned();
        request.accept_transfer().await.unwrap();
        assert!(std::fs::read_dir(&directory).unwrap().next().is_none());

        // Act: data and final flag share the same first-file frame.
        request
            .process_offline_frame(received_file_frame(101, 1, 0, b"a", 1))
            .await
            .unwrap();
        assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
        assert!(!directory.join("b.bin").exists());
        assert!(control.snapshot().received_files.is_none());

        if split_second_file {
            request
                .process_offline_frame(received_file_frame(102, 3, 0, b"b", 0))
                .await
                .unwrap();
            assert!(!directory.join("b.bin").exists());
        }

        let (offset, tail) = if split_second_file {
            (1, b"cd".as_slice())
        } else {
            (0, b"bcd".as_slice())
        };
        let end = request
            .process_offline_frame(received_file_frame(102, 3, offset, tail, 1))
            .await
            .unwrap_err();

        // Assert: exactly two final names, exact bytes and read-only receipt.
        assert!(end.is::<crate::errors::AppError>());
        assert_eq!(request.state.state, State::Finished);
        assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
        assert_eq!(std::fs::read(directory.join("b.bin")).unwrap(), b"bcd");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
        assert!(request.publications.is_empty());
        assert!(request.state.transferred_files.is_empty());
        let snapshot = control.snapshot();
        assert_eq!(snapshot.details.transferred_bytes, 4);
        let receipt = snapshot.received_files.as_ref().unwrap();
        assert_eq!(receipt.len(), 2);
        assert!(receipt.valid_for(&snapshot.details));
        assert_eq!(receipt.details()[0].name, "a.bin");
        assert_eq!(receipt.details()[1].name, "b.bin");
        let mut body = [0_u8; 3];
        std::os::unix::fs::FileExt::read_exact_at(
            &receipt.open(1).unwrap(),
            &mut body,
            0,
        )
        .unwrap();
        assert_eq!(&body, b"bcd");
    }
}

#[tokio::test]
async fn given_premature_or_oversized_last_data_chunk_when_rejected_then_no_final_name_is_published(
) {
    for (body, written) in [(b"b".as_slice(), 0_u64), (b"bcde".as_slice(), 0)] {
        // Arrange: consent admits the same three-byte target.
        let (mut request, control, _peer) = admitted().await;
        let directory = request._directory.path().to_owned();
        request.accept_transfer().await.unwrap();

        // Act: the last flag is not permission to change the admitted byte limit.
        let error = request
            .process_offline_frame(received_file_frame(102, 3, 0, body, 1))
            .await
            .unwrap_err();

        // Assert: a failed frame neither finishes nor exposes the partial name.
        assert!(!error.is::<crate::errors::AppError>());
        assert_eq!(request.state.state, State::ReceivingFiles);
        assert!(!directory.join("b.bin").exists());
        assert_eq!(
            request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
            written
        );
        request.record_termination(State::Disconnected);
        assert_eq!(control.snapshot().state, State::Disconnected);
        assert!(control.snapshot().received_files.is_none());
        assert!(std::fs::read_dir(&directory).unwrap().next().is_none());
    }
}

#[tokio::test]
async fn given_peer_eof_during_active_multifile_receive_when_terminated_then_only_published_member_survives(
) {
    use tokio::io::AsyncWriteExt;

    for interruption in 0..=4 {
        // Arrange: real accepted session and O_TMPFILE destinations.
        let (mut request, control, mut peer) = admitted().await;
        // The test fixture retains a legacy subscriber in addition to the
        // managed session. Real attach_session disables it before the read loop.
        request.receiver = None;
        let directory = request._directory.path().to_owned();
        request.accept_transfer().await.unwrap();
        request
            .process_offline_frame(received_file_frame(101, 1, 0, b"a", 0))
            .await
            .unwrap();
        request
            .process_offline_frame(received_file_frame(101, 1, 1, &[], 1))
            .await
            .unwrap();
        request
            .process_offline_frame(received_file_frame(102, 3, 0, b"b", 0))
            .await
            .unwrap();
        let before = control.snapshot();
        assert_eq!(before.state, State::ReceivingFiles);
        assert_eq!(before.details.transferred_bytes, 2);
        assert!(before.received_files.is_none());
        assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
        assert!(!directory.join("b.bin").exists());

        // Act: control the peer only, not the handler or filesystem.
        let frame_length = 5_u32.to_be_bytes();
        if interruption < 4 {
            peer.write_all(&frame_length[..interruption]).await.unwrap();
        } else {
            peer.write_all(&frame_length).await.unwrap();
            peer.write_all(&[0x01, 0x02]).await.unwrap();
        }
        drop(peer);
        let error = request.handle().await.unwrap_err();
        assert_eq!(
            error
                .root_cause()
                .downcast_ref::<std::io::Error>()
                .map(std::io::Error::kind),
            Some(std::io::ErrorKind::UnexpectedEof),
            "interruption={interruption}"
        );
        request.record_termination(State::Disconnected);

        // Assert: exact partial outcome and no auto-resend or final partial file.
        let after = control.snapshot();
        assert_eq!(after.state, State::Disconnected);
        assert_eq!(after.details.transferred_bytes, 2);
        assert_eq!(after.details.files, before.details.files);
        assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
        assert!(!directory.join("b.bin").exists());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        let completed = after.received_files.as_ref().unwrap();
        assert!(completed.subset_of(&after.details));
        assert!(!completed.valid_for(&after.details));
        assert_eq!(completed.len(), 1);
        assert_eq!(completed.details()[0].name, "a.bin");
        assert!(completed.open(1).is_err());
        let mut body = [0_u8; 1];
        std::os::unix::fs::FileExt::read_exact_at(
            &completed.open(0).unwrap(),
            &mut body,
            0,
        )
        .unwrap();
        assert_eq!(&body, b"a");
        assert!(request.publications.is_empty());
        assert!(request.pending_files.is_none());
        assert!(request
            .state
            .transferred_files
            .values()
            .all(|file| file.file.is_none()));
        // Terminal state must be absorbing: no double-free or invented success.
        request.record_termination(State::Finished);
        assert_eq!(control.snapshot().state, State::Disconnected);
        assert_eq!(control.snapshot().received_files, after.received_files);
    }
}

#[tokio::test]
async fn given_real_storage_fault_at_receive_or_publish_when_terminated_then_only_durable_members_remain(
) {
    for first_completed in [false, true] {
        for fault in ["no-space", "read-only", "truncated-inode"] {
            // Arrange: the actual accepted Linux O_TMPFILE and managed receipt.
            let (mut request, control, _peer) = admitted().await;
            let directory = request._directory.path().to_owned();
            request.accept_transfer().await.unwrap();
            if first_completed {
                request
                    .process_offline_frame(received_file_frame(101, 1, 0, b"a", 1))
                    .await
                    .unwrap();
                assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
            }
            assert!(!directory.join("b.bin").exists());
            assert!(control.snapshot().received_files.is_none());
            let before = control.snapshot();
            let prior = u64::from(first_completed);
            assert_eq!(before.state, State::ReceivingFiles);
            assert_eq!(before.details.total_bytes, 4);
            assert_eq!(before.details.transferred_bytes, prior);

            // Select real kernel I/O failure at the external descriptor
            // boundary. A normal unrelated read-only file is never modified.
            let read_only_source = tempfile::NamedTempFile::new().unwrap();
            std::fs::write(read_only_source.path(), b"user-original").unwrap();
            match fault {
                "no-space" => {
                    request.state.transferred_files.get_mut(&102).unwrap().file =
                        Some(std::fs::OpenOptions::new().write(true).open("/dev/full").unwrap());
                }
                "read-only" => {
                    request.state.transferred_files.get_mut(&102).unwrap().file =
                        Some(std::fs::File::open(read_only_source.path()).unwrap());
                }
                "truncated-inode" => {
                    request
                        .process_offline_frame(received_file_frame(102, 3, 0, b"bcd", 0))
                        .await
                        .unwrap();
                    assert_eq!(
                        control.snapshot().details.transferred_bytes,
                        prior + 3
                    );
                    // The anonymous inode no longer matches the admitted
                    // size, even though all its payload bytes were received.
                    request
                        .state
                        .transferred_files
                        .get(&102)
                        .unwrap()
                        .file
                        .as_ref()
                        .unwrap()
                        .set_len(2)
                        .unwrap();
                }
                _ => unreachable!(),
            }

            // Act: attempt the real FILE handler's write/final publication.
            let result = if fault == "truncated-inode" {
                request
                    .process_offline_frame(received_file_frame(102, 3, 3, &[], 1))
                    .await
            } else {
                request
                    .process_offline_frame(received_file_frame(102, 3, 0, b"b", 0))
                    .await
            };
            let error = result.unwrap_err();

            // Assert before cleanup: no success, no failed member publication
            // or receipt; only complete writes may advance local progress.
            let expected_ack = prior + u64::from(fault == "truncated-inode") * 3;
            assert_eq!(request.state.state, State::ReceivingFiles);
            assert_eq!(
                request.state.transfer_metadata.as_ref().unwrap().ack_bytes,
                expected_ack,
                "{fault} first_completed={first_completed}"
            );
            assert_eq!(control.snapshot().details.transferred_bytes, expected_ack);
            assert!(!directory.join("b.bin").exists());
            assert!(control.snapshot().received_files.is_none());
            if fault == "truncated-inode" {
                assert_eq!(error.to_string(), "Cannot finish received file safely");
            } else {
                let kind = error
                    .root_cause()
                    .downcast_ref::<std::io::Error>()
                    .and_then(std::io::Error::raw_os_error);
                // Linux ENOSPC and EBADF come from actual kernel writes.
                assert_eq!(kind, Some(if fault == "no-space" { 28 } else { 9 }));
            }
            assert_eq!(
                std::fs::read(read_only_source.path()).unwrap(),
                b"user-original"
            );

            request.record_termination(State::Disconnected);
            let after = control.snapshot();
            assert_eq!(after.state, State::Disconnected);
            assert_eq!(after.details.files, before.details.files);
            assert_eq!(after.details.total_bytes, 4);
            assert_eq!(after.details.transferred_bytes, expected_ack);
            assert_eq!(std::fs::read_dir(&directory).unwrap().count(), usize::from(first_completed));
            assert!(!directory.join("b.bin").exists());
            assert!(request.publications.is_empty());
            assert!(request.pending_files.is_none());
            assert!(request
                .state
                .transferred_files
                .values()
                .all(|file| file.file.is_none()));

            if first_completed {
                assert_eq!(std::fs::read(directory.join("a.bin")).unwrap(), b"a");
                let received = after.received_files.as_ref().unwrap();
                assert_eq!(received.len(), 1);
                assert!(received.subset_of(&after.details));
                assert!(!received.valid_for(&after.details));
                assert_eq!(received.details()[0].name, "a.bin");
                assert!(received.open(1).is_err());
                let mut body = [0_u8; 1];
                std::os::unix::fs::FileExt::read_exact_at(
                    &received.open(0).unwrap(),
                    &mut body,
                    0,
                )
                .unwrap();
                assert_eq!(&body, b"a");
            } else {
                assert!(after.received_files.is_none());
                assert!(!directory.join("a.bin").exists());
            }
            // A late synthetic Finish must not alter a failed transfer.
            request.record_termination(State::Finished);
            assert_eq!(control.snapshot().state, State::Disconnected);
            assert_eq!(control.snapshot().received_files, after.received_files);
        }
    }
}
