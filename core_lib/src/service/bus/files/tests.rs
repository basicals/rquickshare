use super::*;
use crate::service::{Limits, Phase};
#[test]
fn given_completed_file_receipt_when_revision_index_or_owner_is_invalid_then_live_lookup_fails_closed(
) {
    // Arrange
    let (_directory, received, details) = crate::received_files::tests::fixture();
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    let client = coordinator.connect().unwrap();
    let target = coordinator.register_inbound().unwrap();
    coordinator
        .observe_details(target, Phase::Finished, Arc::new(details))
        .unwrap();
    coordinator
        .retain_received_files(target, Some(received.clone()))
        .unwrap();
    let revision = coordinator.transfers[&target].terminal_revision.unwrap();
    // Act / Assert
    assert!(Arc::ptr_eq(
        &resource(&coordinator, client, target, revision, 0).unwrap(),
        &received
    ));
    for revision in [0, revision + 1, u64::MAX] {
        assert_eq!(
            resource(&coordinator, client, target, revision, 0).unwrap_err(),
            Error::StaleControl
        );
    }
    for index in [1, u32::MAX] {
        assert_eq!(
            resource(&coordinator, client, target, revision, index).unwrap_err(),
            Error::InvalidInput
        );
    }
    coordinator.disconnect(client).unwrap();
    assert_eq!(
        resource(&coordinator, client, target, revision, 0).unwrap_err(),
        Error::UnknownClient
    );
}

#[test]
fn given_captured_reveal_availability_when_history_expires_then_metadata_does_not_pin_original_directory_receipt(
) {
    // Arrange
    let (_directory, received, details) = crate::received_files::tests::fixture();
    let weak = Arc::downgrade(&received);
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    let client = coordinator.connect().unwrap();
    let target = coordinator.register_inbound().unwrap();
    coordinator
        .observe_details(target, Phase::Finished, Arc::new(details))
        .unwrap();
    coordinator
        .retain_received_files(target, Some(received))
        .unwrap();
    coordinator.owner_closed(target).unwrap();
    let mut state = state::StateCache::default();
    let capture = state
        .capture(&mut coordinator, client, std::iter::empty())
        .unwrap();
    // Act
    for _ in 0..64 {
        let id = coordinator.register_inbound().unwrap();
        coordinator.observe(id, Phase::Finished).unwrap();
        coordinator.owner_closed(id).unwrap();
    }
    coordinator.maintain_history().unwrap();
    // Assert
    assert!(weak.upgrade().is_none());
    assert_eq!(
        state.received_page(client, &capture.0, 0).unwrap().1[0].2,
        1
    );
    assert_eq!(
        resource(&coordinator, client, target, 1, 0).unwrap_err(),
        Error::UnknownTransfer
    );
}
#[tokio::test]
async fn given_cancelled_file_dispatch_when_bus_closes_then_actual_blocking_worker_is_joined() {
    // Arrange: explicit boundary events model a delayed filesystem operation.
    let broker = crate::service::bus::tests::PrivateBus::new();
    let mut owner = SelectionBus::connect(broker.socket().await).await.unwrap();
    let (started, observed) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    owner.file_job = Some(tokio::task::spawn_blocking(move || {
        started.send(()).unwrap();
        wait.recv().unwrap();
        Err(Error::InvalidState)
    }));
    observed.await.unwrap();
    // Act / Assert
    let close = owner.close_connection();
    tokio::pin!(close);
    assert!(futures::poll!(&mut close).is_pending());
    release.send(()).unwrap();
    close.await;
}

#[test]
fn given_full_coordinator_receipt_capacity_when_another_transfer_finishes_then_payload_success_does_not_forge_reveal_availability(
) {
    // Arrange / Act / Assert: independent coordinator bound even for shared receipt objects.
    let (_directory, received, details) = crate::received_files::tests::fixture();
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    for index in 0..=crate::received_files::RECEIPT_LIMIT {
        let id = coordinator.register_inbound().unwrap();
        coordinator
            .observe_details(id, Phase::Finished, Arc::new(details.clone()))
            .unwrap();
        coordinator
            .retain_received_files(id, Some(received.clone()))
            .unwrap();
        assert_eq!(
            coordinator.transfers[&id].received_files.is_some(),
            index < crate::received_files::RECEIPT_LIMIT
        );
        assert_eq!(coordinator.transfers[&id].view.phase, Phase::Finished);
    }
}
