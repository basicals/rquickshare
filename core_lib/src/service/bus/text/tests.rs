use super::*;
use crate::{
    service::Limits,
    sessions::details::{DetailsStatus, PayloadKind, TransferDetails},
};

pub(crate) fn completed(
    coordinator: &mut Coordinator,
    kind: PayloadKind,
    body: &str,
) -> TransferId {
    let target = coordinator.register_inbound().unwrap();
    coordinator
        .observe_details(
            target,
            Phase::Finished,
            Arc::new(TransferDetails {
                status: DetailsStatus::Ready,
                kind,
                sender: "phone".into(),
                total_bytes: body.len() as u64,
                transferred_bytes: body.len() as u64,
                ..Default::default()
            }),
        )
        .unwrap();
    coordinator
        .retain_received_text(
            target,
            Some(Arc::new(ReceivedText::new(kind, body.into()).unwrap())),
        )
        .unwrap();
    target
}
#[test]
fn given_retained_text_when_read_retries_cross_owner_and_history_expiry_interleave_then_capability_is_bounded_and_immutable(
) {
    // Arrange / Act / Assert
    for size in [0, 1, CHUNK - 1, CHUNK, CHUNK + 1, 128 * 1024] {
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        let owner = coordinator.connect().unwrap();
        let other = coordinator.connect().unwrap();
        let body = "x".repeat(size);
        let target = completed(&mut coordinator, PayloadKind::Text, &body);
        let revision = coordinator.transfers[&target].terminal_revision.unwrap();
        let mut cache = TextCache::default();
        let first = cache.begin(&coordinator, owner, target, revision).unwrap();
        assert_eq!(first.1, "text");
        assert_eq!(first.2 as usize, size);
        assert_eq!(
            cache.begin(&coordinator, owner, target, revision).unwrap(),
            first
        );
        assert_eq!(
            cache.begin(&coordinator, owner, target, revision + 1),
            Err(Error::Busy)
        );
        assert_eq!(cache.chunk(other, &first.0, 0), Err(Error::WrongOwner));
        assert_eq!(cache.end(other, &first.0), Err(Error::WrongOwner));
        for offset in [size as u32 + 1, u32::MAX] {
            assert_eq!(
                cache.chunk(owner, &first.0, offset),
                Err(Error::InvalidInput)
            );
        }
        if size > 1 {
            assert_eq!(cache.chunk(owner, &first.0, 1), Err(Error::InvalidInput));
        }
        coordinator.owner_closed(target).unwrap();
        // Retire actual history; the acquired immutable reader remains alive.
        for _ in 0..64 {
            let id = coordinator.register_inbound().unwrap();
            coordinator.observe(id, Phase::Finished).unwrap();
            coordinator.owner_closed(id).unwrap();
        }
        coordinator.maintain_history().unwrap();
        assert!(!coordinator.transfers.contains_key(&target));
        let mut bytes = Vec::new();
        let mut offset = 0;
        loop {
            let (next, chunk) = cache.chunk(owner, &first.0, offset).unwrap();
            assert!(chunk.len() <= CHUNK);
            assert_eq!(next, offset + chunk.len() as u32);
            bytes.extend(chunk);
            offset = next;
            if offset as usize == size {
                break;
            }
        }
        assert_eq!(bytes, body.as_bytes());
        assert_eq!(
            cache.chunk(owner, &first.0, size as u32).unwrap(),
            (size as u32, vec![])
        );
        assert_eq!(cache.end(owner, &first.0), Ok(()));
        assert_eq!(cache.end(owner, &first.0), Ok(()));
        assert_eq!(cache.chunk(owner, &first.0, 0), Err(Error::UnknownSnapshot));
        assert_eq!(
            cache.begin(&coordinator, owner, target, revision),
            Err(Error::UnknownTransfer)
        );
        cache.disconnect(owner);
        assert_eq!(cache.end(owner, &first.0), Err(Error::UnknownSnapshot));
    }
}
#[test]
fn given_resource_or_capability_boundaries_when_admitted_then_invalid_or_exhausted_reads_do_not_mutate(
) {
    // Arrange
    let mut coordinator = Coordinator::new(
        123,
        Limits {
            clients: 64,
            ..Limits::default()
        },
    )
    .unwrap();
    let owner = coordinator.connect().unwrap();
    let target = completed(
        &mut coordinator,
        PayloadKind::Url,
        "https://example.invalid/",
    );
    let revision = coordinator.transfers[&target].terminal_revision.unwrap();
    let mut cache = TextCache::default();
    // Act / Assert
    for wrong in [0, revision + 1, u64::MAX] {
        assert_eq!(
            cache.begin(&coordinator, owner, target, wrong),
            Err(Error::StaleControl)
        );
    }
    cache.sequence = u64::MAX;
    assert_eq!(
        cache.begin(&coordinator, owner, target, revision),
        Err(Error::Exhausted)
    );
    assert!(cache.readers.is_empty());
    cache.sequence = 0;
    for _ in 0..32 {
        let client = coordinator.connect().unwrap();
        cache.begin(&coordinator, client, target, revision).unwrap();
    }
    assert_eq!(
        cache.begin(&coordinator, owner, target, revision),
        Err(Error::Capacity)
    );
    assert_eq!(cache.readers.len(), 32);
    assert!(!format!("{:?}", coordinator.snapshot(owner).unwrap()).contains("example.invalid"));
    let original = coordinator.transfers[&target]
        .received_text
        .clone()
        .unwrap();
    assert_eq!(
        coordinator.retain_received_text(target, Some(original.clone())),
        Ok(())
    );
    let other =
        Arc::new(ReceivedText::new(PayloadKind::Url, "x".repeat(original.text().len())).unwrap());
    assert_eq!(
        coordinator.retain_received_text(target, Some(other)),
        Err(Error::ReplayConflict)
    );
    for phase in [
        Phase::Active,
        Phase::Cancelled,
        Phase::Rejected,
        Phase::Failed,
    ] {
        coordinator.transfers.get_mut(&target).unwrap().view.phase = phase;
        assert_eq!(
            coordinator.retain_received_text(target, Some(original.clone())),
            Err(Error::InvalidState)
        );
    }
}

#[test]
fn given_captured_text_metadata_when_resource_arrives_then_existing_capture_never_pins_or_changes_body_availability(
) {
    // Arrange
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    let owner = coordinator.connect().unwrap();
    let other = coordinator.connect().unwrap();
    let target = completed(&mut coordinator, PayloadKind::Text, "private-body");
    let body = coordinator
        .transfers
        .get_mut(&target)
        .unwrap()
        .received_text
        .take()
        .unwrap();
    let mut cache = crate::service::bus::state::StateCache::default();
    let captured = cache
        .capture(&mut coordinator, owner, std::iter::empty())
        .unwrap();
    // Act / Assert
    coordinator
        .retain_received_text(target, Some(body))
        .unwrap();
    let page = cache.text_page(owner, &captured.0, 0).unwrap();
    assert_eq!(page.1, vec![(token(target.0, target.1), "text", false, 0)]);
    assert!(!format!("{page:?}").contains("private-body"));
    assert_eq!(
        cache.text_page(other, &captured.0, 0),
        Err(Error::WrongOwner)
    );
    assert_eq!(
        cache.text_page(owner, &captured.0, 2),
        Err(Error::InvalidInput)
    );
    let current = cache
        .capture(&mut coordinator, owner, std::iter::empty())
        .unwrap();
    assert!(cache.text_page(owner, &current.0, 0).unwrap().1[0].2);
    assert_eq!(
        cache.text_page(owner, &captured.0, 0),
        Err(Error::UnknownSnapshot)
    );
}
#[tokio::test]
async fn given_real_uid_mismatch_when_text_route_is_called_then_bus_authentication_precedes_body_decode(
) {
    // Arrange
    use crate::service::bus::{
        tests::{Harness, PrivateBus},
        SelectionBus, PATH, SERVICE,
    };
    let private = PrivateBus::new();
    let bus = SelectionBus::connect_with_uid(
        private.socket().await,
        rustix::process::geteuid().as_raw() + 1,
    )
    .await
    .unwrap();
    let host = Harness::start(bus);
    let client = private.client().await;
    // Act / Assert
    for method in ["GetMetadataPage", "BeginRead", "GetChunk", "EndRead"] {
        let error = client
            .call_method(Some(SERVICE), PATH, Some(INTERFACE), method, &())
            .await
            .unwrap_err();
        assert!(
            matches!(error, zbus::Error::MethodError(name, _, _) if name.as_str() == "org.freedesktop.DBus.Error.AccessDenied")
        );
    }
    client.close().await.unwrap();
    assert_eq!(host.finish().await.1, Ok(()));
}
