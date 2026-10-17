use super::*;
use crate::service::{
    bus::tests::{Harness, PrivateBus},
    bus::SelectionBus,
    Coordinator, Limits, Phase,
};

#[tokio::test]
async fn given_partial_receive_when_private_sdk_reads_and_reveals_then_original_manifest_and_only_completed_file_authority_are_preserved(
) {
    super::super::tests::scenario(async {
        // Arrange
        let (_directory, received, mut details) = crate::received_files::tests::fixture();
        let mut manifest = details.files.to_vec();
        manifest.push(crate::sessions::details::FileDetail {
            name: "a-missing.bin".into(),
            size: 2,
        });
        manifest.sort_by(|a, b| a.name.cmp(&b.name));
        details.files = manifest.into();
        details.total_bytes += 2;
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        let target = coordinator.register_inbound().unwrap();
        coordinator
            .observe_details(target, Phase::Disconnected, Arc::new(details))
            .unwrap();
        coordinator
            .retain_received_files(target, Some(received))
            .unwrap();
        let private = PrivateBus::new();
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        // Act / Assert
        let view = client.read_state().await.unwrap();
        let row = &view.transfers[0];
        assert_eq!(row.phase, Phase::Disconnected);
        assert!(client
            .acquire_received_result(&row.transfer, row.reveal_revision.unwrap())
            .await
            .is_err());
        assert_eq!(row.details.files.len(), 2);
        assert_eq!(row.completed_files.len(), 1);
        assert_eq!(row.completed_files[0].name, "nested/actual (1).txt");
        assert_eq!(row.details.transferred_bytes, 8);
        assert_eq!(row.details.total_bytes, 10);
        assert!(!format!("{view:?}").contains("actual (1)"));
        let revision = row.reveal_revision.unwrap();
        let file = client
            .acquire_received_file(&row.transfer, revision, 0)
            .await
            .unwrap();
        let mut bytes = [0; 8];
        std::os::unix::fs::FileExt::read_exact_at(file.file(), &mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"received");
        assert!(client
            .acquire_received_file(&row.transfer, revision, 1)
            .await
            .is_err());
        assert!(client
            .acquire_received_file(&row.transfer, revision + 1, 0)
            .await
            .is_err());
        let fd = tempfile::tempfile().unwrap();
        assert_eq!(
            client
                .wire(
                    bus::files::INTERFACE,
                    "GetCompletedPage",
                    &Fd::from(fd.as_fd())
                )
                .await
                .unwrap_err(),
            ClientError::Rejected(DomainError::InvalidInput)
        );
        client.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}
#[tokio::test]
async fn given_actual_files_api_when_acquired_twice_or_abused_then_only_verified_readonly_file_is_returned_and_state_stays_private(
) {
    // Arrange
    let (_directory, received, details) = crate::received_files::tests::fixture();
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    let id = coordinator.register_inbound().unwrap();
    coordinator
        .observe_details(id, Phase::Finished, Arc::new(details))
        .unwrap();
    coordinator
        .retain_received_files(id, Some(received))
        .unwrap();
    let bus = PrivateBus::new();
    let host = Harness::with_coordinator(
        SelectionBus::connect(bus.socket().await).await.unwrap(),
        coordinator,
    );
    let mut owner = NativeClient::connect(bus.socket().await).await.unwrap();
    let mut other = NativeClient::connect(bus.socket().await).await.unwrap();
    let view = owner.read_state().await.unwrap();
    let row = &view.transfers[0];
    let revision = row.reveal_revision.unwrap();
    let result = owner
        .acquire_received_result(&row.transfer, revision)
        .await
        .unwrap();
    assert_eq!(result.kind, ResultKind::Folder);
    assert!(result.file().metadata().unwrap().is_dir());
    assert!(owner
        .acquire_received_result(&row.transfer, revision + 1)
        .await
        .is_err());
    assert_eq!(
        other
            .acquire_received_result(&row.transfer, revision)
            .await
            .unwrap_err(),
        ClientError::ForeignHandle
    );

    assert!(row.content_revision.is_none());
    assert!(!format!("{view:?}").contains("actual (1)")); // Detail Debug redacts even relative names.
                                                          // Act / Assert: repeated acquisition is a read, never a desktop operation or send.
    for _ in 0..2 {
        let read = owner
            .acquire_received_file(&row.transfer, revision, 0)
            .await
            .unwrap();
        let mut bytes = [0; 8];
        use std::os::unix::fs::FileExt;
        read.file().read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"received");
        assert_eq!(
            rustix::fs::fcntl_getfl(read.file()).unwrap() & rustix::fs::OFlags::ACCMODE,
            rustix::fs::OFlags::RDONLY
        );
        assert!(rustix::io::fcntl_getfd(read.file())
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC));
        assert!(!format!("{read:?}").contains("nested"));
    }
    assert_eq!(
        other
            .acquire_received_file(&row.transfer, revision, 0)
            .await
            .unwrap_err(),
        ClientError::ForeignHandle
    );
    for (revision, index) in [
        (0, 0),
        (revision + 1, 0),
        (revision, 1),
        (revision, u32::MAX),
    ] {
        assert!(owner
            .acquire_received_file(&row.transfer, revision, index)
            .await
            .is_err());
    }
    for (revision, index, error) in [
        (revision + 1, 0, DomainError::StaleControl),
        (revision, 1, DomainError::InvalidInput),
        (revision, u32::MAX, DomainError::InvalidInput),
    ] {
        assert_eq!(
            owner
                .wire(
                    bus::files::INTERFACE,
                    "AcquireFile",
                    &(row.transfer.id(), revision, index)
                )
                .await
                .unwrap_err(),
            ClientError::Rejected(error)
        );
    }
    let input = tempfile::tempfile().unwrap();
    for method in ["GetMetadataPage", "AcquireFile"] {
        assert_eq!(
            owner
                .wire(bus::files::INTERFACE, method, &Fd::from(input.as_fd()))
                .await
                .unwrap_err(),
            ClientError::Rejected(DomainError::InvalidInput)
        );
    }
    assert!(owner
        .wire(bus::files::INTERFACE, "Unknown", &())
        .await
        .is_err());
    let refreshed = owner.read_state().await.unwrap();
    assert_eq!(refreshed.transfers.len(), 1);
    assert_eq!(refreshed.resources.selected_bytes, 0);
    let controller = crate::service::client::controller::PanelController::spawn(owner);
    let handle = controller.handle();
    let result = handle
        .acquire_file(row.transfer.clone(), revision, 0)
        .unwrap()
        .wait()
        .await
        .unwrap();
    let crate::service::client::controller::ActionResult::File(read) = result else {
        panic!("Missing private file completion");
    };
    assert_eq!(read.revision, revision);
    assert!(!format!("{:?}", handle.subscribe().borrow()).contains("nested"));
    assert!(!handle.subscribe().borrow().pending);
    drop(read);
    controller.close().await.unwrap();
    other.close().await.unwrap();
    assert_eq!(host.finish().await.1, Ok(()));
}

#[tokio::test]
async fn given_delayed_actual_file_open_when_another_client_reads_or_controls_then_bus_remains_responsive_and_acquisition_stays_bounded(
) {
    super::super::tests::scenario(async {
        // Arrange: delay only the filesystem boundary, then run the actual resolver.
        let (_directory, received, details) = crate::received_files::tests::fixture();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        let id = coordinator.register_inbound().unwrap();
        coordinator
            .observe_details(id, Phase::Finished, Arc::new(details))
            .unwrap();
        coordinator
            .retain_received_files(id, Some(received))
            .unwrap();
        let broker = PrivateBus::new();
        let mut bus = SelectionBus::connect(broker.socket().await).await.unwrap();
        let (started, observed) = tokio::sync::oneshot::channel();
        let started = std::sync::Mutex::new(Some(started));
        let (release, wait) = std::sync::mpsc::channel();
        let wait = std::sync::Mutex::new(wait);
        bus.set_file_resolver(Arc::new(move |files, index| {
            if let Some(started) = started.lock().unwrap().take() {
                let _ = started.send(());
            }
            wait.lock()
                .unwrap()
                .recv()
                .map_err(|_| crate::service::Error::InvalidState)?;
            files
                .open(index)
                .map_err(|_| crate::service::Error::InvalidState)
        }));
        let host = Harness::with_coordinator(bus, coordinator);
        let mut first = NativeClient::connect(broker.socket().await).await.unwrap();
        let mut second = NativeClient::connect(broker.socket().await).await.unwrap();
        let first_row = first.read_state().await.unwrap().transfers.remove(0);
        let revision = first_row.reveal_revision.unwrap();
        let acquisition = tokio::spawn(async move {
            let result = first
                .acquire_received_file(&first_row.transfer, revision, 0)
                .await;
            (first, result)
        });
        observed.await.unwrap();
        // Act / Assert: these must reply before the delayed resolver is released.
        let second_row = second.read_state().await.unwrap().transfers.remove(0);
        assert_eq!(second_row.phase, Phase::Finished);
        assert_eq!(second.cancel(&second_row.transfer).await, Ok(()));
        assert_eq!(
            second
                .wire(
                    bus::files::INTERFACE,
                    "AcquireFile",
                    &(second_row.transfer.id(), revision, 0u32)
                )
                .await
                .unwrap_err(),
            ClientError::Rejected(DomainError::Busy)
        );
        release.send(()).unwrap();
        let (first, result) = acquisition.await.unwrap();
        let file = result.unwrap();
        let mut bytes = [0; 8];
        use std::os::unix::fs::FileExt;
        file.file().read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(&bytes, b"received");
        assert_eq!(
            second.read_state().await.unwrap().transfers[0].phase,
            Phase::Finished
        );
        drop(file);
        first.close().await.unwrap();
        second.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[test]
fn given_result_mode_and_descriptor_when_validated_then_only_exact_readonly_file_or_directory_types_are_accepted(
) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("file");
    std::fs::write(&path, b"received").unwrap();
    let file = File::open(&path).unwrap();
    let dir = File::open(directory.path()).unwrap();
    for kind in [0, 1, 2, 3, 255] {
        assert_eq!(result_kind(&file, kind).is_ok(), kind == 0);
        assert_eq!(result_kind(&dir, kind).is_ok(), kind == 1 || kind == 2);
    }
    let writable = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    assert_eq!(result_kind(&writable, 0), Err(ClientError::Protocol));
    let path_only = File::from(
        rustix::fs::open(&path, rustix::fs::OFlags::PATH, rustix::fs::Mode::empty()).unwrap(),
    );
    assert_eq!(result_kind(&path_only, 0), Err(ClientError::Protocol));
    let path_dir = File::from(
        rustix::fs::open(
            directory.path(),
            rustix::fs::OFlags::PATH,
            rustix::fs::Mode::empty(),
        )
        .unwrap(),
    );
    assert_eq!(result_kind(&path_dir, 2), Err(ClientError::Protocol));
}
