use super::*;

async fn admitted() -> (
    TestRequest,
    crate::sessions::SessionControl,
    tokio::io::DuplexStream,
) {
    let mut request = test_request();
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
