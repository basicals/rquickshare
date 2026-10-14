use super::*;
use crate::service::{
    bus::tests::{Harness, PrivateBus},
    bus::SelectionBus,
    Coordinator, Limits,
};
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
