use std::fs::File;

use super::*;
use crate::file_source::{OpenedFile, OpenedSelection, SelectionLimits};

fn setup() -> (Coordinator, ClientId, StateCache) {
    let mut coordinator = Coordinator::new(123, crate::service::Limits::default()).unwrap();
    let client = coordinator.connect().unwrap();
    (coordinator, client, StateCache::default())
}
fn selection(coordinator: &mut Coordinator, client: ClientId) -> crate::service::SelectionId {
    let source = tempfile::NamedTempFile::new().unwrap();
    source.as_file().set_len(3).unwrap();
    coordinator
        .create_selection(
            client,
            OpenedSelection::from_files(
                vec![
                    OpenedFile::adopt(File::open(source.path()).unwrap(), "secret-source".into())
                        .unwrap(),
                ],
                SelectionLimits::default(),
            )
            .unwrap(),
        )
        .unwrap()
}
fn endpoints(rows: &[(String, String)]) -> impl ExactSizeIterator<Item = (&str, &str)> {
    rows.iter().map(|(id, name)| (id.as_str(), name.as_str()))
}

#[test]
fn given_collection_boundaries_when_every_offset_is_read_then_pages_are_complete_ordered_and_bounded(
) {
    for count in [0, 1, 15, 16, 17, 128, 256] {
        // Arrange
        let (mut coordinator, client, mut cache) = setup();
        let endpoints_count = count.min(128);
        let names: Vec<_> = (0..endpoints_count)
            .rev()
            .map(|n| (format!("ep-{n:03}"), "界".repeat(85)))
            .collect();
        for _ in 0..endpoints_count {
            selection(&mut coordinator, client);
        }
        for _ in 0..count {
            coordinator.register_inbound().unwrap();
        }
        // Act
        let header = cache
            .capture(&mut coordinator, client, endpoints(&names))
            .unwrap();
        let id = &header.0;
        // Assert
        assert_eq!(
            (header.8, header.9, header.10),
            (endpoints_count as u32, endpoints_count as u32, count as u32)
        );
        for offset in 0..=count {
            let (next, rows) = cache.transfer_page(client, id, offset as u32).unwrap();
            assert_eq!(next as usize, (offset + PAGE).min(count));
            assert_eq!(rows.len(), (count - offset).min(PAGE));
            assert_eq!(
                rows,
                cache.captures[&client].transfers[offset..next as usize]
            );
        }
        for offset in 0..=endpoints_count {
            let (next, rows) = cache.endpoint_page(client, id, offset as u32).unwrap();
            assert_eq!(next as usize, (offset + PAGE).min(endpoints_count));
            assert_eq!(
                rows,
                cache.captures[&client].endpoints[offset..next as usize]
            );
            let (next, rows) = cache.selection_page(client, id, offset as u32).unwrap();
            assert_eq!(next as usize, (offset + PAGE).min(endpoints_count));
            assert_eq!(
                rows,
                cache.captures[&client].selections[offset..next as usize]
            );
        }
        for offset in [count as u32 + 1, u32::MAX] {
            assert_eq!(
                cache.transfer_page(client, id, offset),
                Err(Error::InvalidInput)
            );
        }
        for offset in [endpoints_count as u32 + 1, u32::MAX] {
            assert_eq!(
                cache.endpoint_page(client, id, offset),
                Err(Error::InvalidInput)
            );
            assert_eq!(
                cache.selection_page(client, id, offset),
                Err(Error::InvalidInput)
            );
        }
        assert!(cache.captures[&client]
            .endpoints
            .windows(2)
            .all(|rows| rows[0].0 < rows[1].0));
        assert!(cache.captures[&client]
            .selections
            .windows(2)
            .all(|rows| rows[0] < rows[1]));
        assert!(cache.captures[&client]
            .transfers
            .windows(2)
            .all(|rows| rows[0].0 < rows[1].0));
    }
}

#[test]
fn given_captured_state_when_live_state_changes_then_old_pages_remain_consistent_until_replacement()
{
    // Arrange
    let (mut coordinator, client, mut cache) = setup();
    coordinator.set_endpoints(vec!["nearby".into()]).unwrap();
    let selected = selection(&mut coordinator, client);
    let id = token(selected.0, selected.1);
    let names = vec![("nearby".into(), "First name".into())];
    let first = cache
        .capture(&mut coordinator, client, endpoints(&names))
        .unwrap();
    let revision = coordinator.revision;
    // Act
    let transfer = coordinator
        .send_selection(client, selected, "nearby", "send")
        .unwrap();
    coordinator.mark_desynchronized().unwrap();
    // Assert
    assert_eq!(
        cache.selection_page(client, &first.0, 0).unwrap(),
        (1, vec![id])
    );
    assert_eq!(
        cache.transfer_page(client, &first.0, 0).unwrap(),
        (0, vec![])
    );
    assert_eq!(
        cache.endpoint_page(client, &first.0, 0).unwrap(),
        (1, names)
    );
    assert_eq!(first.2, revision);
    assert!(first.4);
    let second = cache
        .capture(&mut coordinator, client, std::iter::empty())
        .unwrap();
    assert!(second.2 > revision);
    assert!(!second.4);
    assert_eq!(second.9, 0);
    assert_eq!(second.10, 1);
    assert_eq!(
        cache.transfer_page(client, &second.0, 0).unwrap().1[0].0,
        token(transfer.0, transfer.1)
    );
    assert_eq!(
        cache.selection_page(client, &first.0, 0),
        Err(Error::UnknownSnapshot)
    );
}

#[test]
fn given_two_callers_when_captured_or_released_then_only_own_selections_and_captures_are_accessible(
) {
    // Arrange
    let (mut coordinator, first, mut cache) = setup();
    let second = coordinator.connect().unwrap();
    let selected = selection(&mut coordinator, first);
    let selected_other = selection(&mut coordinator, second);
    let before = coordinator.snapshot(first).unwrap();
    // Act
    let a = cache
        .capture(&mut coordinator, first, std::iter::empty())
        .unwrap();
    let b = cache
        .capture(&mut coordinator, second, std::iter::empty())
        .unwrap();
    // Assert
    assert_eq!(
        cache.selection_page(first, &a.0, 0).unwrap().1,
        vec![token(selected.0, selected.1)]
    );
    assert_eq!(
        cache.selection_page(second, &b.0, 0).unwrap().1,
        vec![token(selected_other.0, selected_other.1)]
    );
    assert_eq!(cache.endpoint_page(second, &a.0, 0), Err(Error::WrongOwner));
    assert_eq!(
        cache.selection_page(second, &a.0, 0),
        Err(Error::WrongOwner)
    );
    assert_eq!(cache.transfer_page(second, &a.0, 0), Err(Error::WrongOwner));
    assert_eq!(cache.end(second, &a.0), Err(Error::WrongOwner));
    for _ in 0..2 {
        cache.end(first, &a.0).unwrap();
    }
    assert_eq!(cache.captures.len(), 1);
    cache.disconnect(second);
    assert!(cache.captures.is_empty());
    assert_eq!(coordinator.snapshot(first).unwrap(), before);
}

#[test]
fn given_full_cache_when_new_or_replacement_capture_arrives_then_budget_is_explicit_and_release_returns_capacity(
) {
    // Arrange
    let (mut coordinator, first, mut cache) = setup();
    let mut clients = vec![first];
    for _ in 1..17 {
        clients.push(coordinator.connect().unwrap());
    }
    let mut views = Vec::new();
    for client in &clients[..16] {
        views.push(
            cache
                .capture(&mut coordinator, *client, std::iter::empty())
                .unwrap()
                .0,
        );
    }
    // Act / Assert
    assert_eq!(
        cache.capture(&mut coordinator, clients[16], std::iter::empty()),
        Err(Error::Capacity)
    );
    let replacement = cache
        .capture(&mut coordinator, first, std::iter::empty())
        .unwrap();
    assert_ne!(replacement.0, views[0]);
    assert_eq!(cache.captures.len(), 16);
    assert_eq!(
        cache.endpoint_page(first, &views[0], 0),
        Err(Error::UnknownSnapshot)
    );
    cache.end(first, &replacement.0).unwrap();
    cache
        .capture(&mut coordinator, clients[16], std::iter::empty())
        .unwrap();
    assert_eq!(cache.captures.len(), 16);
    cache.clear();
    assert!(cache.captures.is_empty());
}

#[test]
fn given_bad_endpoint_projection_when_replacing_then_prior_view_and_state_are_preserved() {
    // Arrange
    let (mut coordinator, client, mut cache) = setup();
    let original = cache
        .capture(&mut coordinator, client, std::iter::empty())
        .unwrap();
    let before = coordinator.snapshot(client).unwrap();
    // Act / Assert
    for rows in [
        vec![("".into(), "name".into())],
        vec![("x".repeat(257), "name".into())],
        vec![("bad id".into(), "name".into())],
        vec![("ep".into(), "界".repeat(86))],
        vec![("ep".into(), "bad\nlabel".into())],
        vec![("ep".into(), "one".into()), ("ep".into(), "two".into())],
    ] {
        assert_eq!(
            cache.capture(&mut coordinator, client, endpoints(&rows)),
            Err(Error::InvalidInput)
        );
        assert_eq!(cache.captures[&client].metadata, original);
        assert_eq!(coordinator.snapshot(client).unwrap(), before);
    }
    let rows = vec![("x".repeat(256), "é".repeat(128))];
    let valid = cache
        .capture(&mut coordinator, client, endpoints(&rows))
        .unwrap();
    assert_eq!(cache.endpoint_page(client, &valid.0, 0).unwrap().1, rows);
}

#[test]
fn given_over_schema_count_or_exhausted_capture_id_when_captured_then_no_view_is_replaced() {
    // Arrange
    for over in ["endpoints", "selections", "transfers", "counter"] {
        let mut coordinator = Coordinator::new(
            123,
            crate::service::Limits {
                selections: 129,
                transfers: 257,
                ..Default::default()
            },
        )
        .unwrap();
        let client = coordinator.connect().unwrap();
        let mut cache = StateCache::default();
        let original = cache
            .capture(&mut coordinator, client, std::iter::empty())
            .unwrap();
        let rows: Vec<_> = if over == "endpoints" {
            (0..129)
                .map(|n| (format!("ep-{n}"), "Peer".into()))
                .collect()
        } else {
            vec![]
        };
        if over == "selections" {
            for _ in 0..129 {
                selection(&mut coordinator, client);
            }
        }
        if over == "transfers" {
            for _ in 0..257 {
                coordinator.register_inbound().unwrap();
            }
        }
        if over == "counter" {
            cache.sequence = u64::MAX;
        }
        let before = coordinator.snapshot(client).unwrap();
        // Act / Assert
        assert_eq!(
            cache.capture(&mut coordinator, client, endpoints(&rows)),
            Err(if over == "counter" {
                Error::Exhausted
            } else {
                Error::Capacity
            })
        );
        assert_eq!(cache.captures[&client].metadata, original);
        assert_eq!(coordinator.snapshot(client).unwrap(), before);
    }
}

#[test]
fn given_all_native_state_values_when_projected_then_spellings_match_fixed_fixture() {
    // Arrange / Act
    let values: Vec<_> = [Lifecycle::Running, Lifecycle::Draining, Lifecycle::Stopped]
        .map(lifecycle)
        .into_iter()
        .chain([Direction::Inbound, Direction::Outbound].map(direction))
        .chain(
            [
                Phase::Queued,
                Phase::Active,
                Phase::AwaitingConsent,
                Phase::Receiving,
                Phase::Sending,
                Phase::CancelRequested,
                Phase::Finished,
                Phase::Rejected,
                Phase::Cancelled,
                Phase::Failed,
                Phase::Disconnected,
            ]
            .map(phase),
        )
        .chain(
            [
                ControlAction::Accept,
                ControlAction::Reject,
                ControlAction::Cancel,
            ]
            .map(action),
        )
        .collect();
    // Assert
    assert_eq!(
        values.join("\n") + "\n",
        include_str!("../../../../dbus/native-state-enums.txt")
    );
}

#[test]
fn given_consent_and_pending_control_when_captured_then_presence_and_revisions_are_truthful() {
    for accept in [false, true] {
        // Arrange
        let (mut coordinator, client, mut cache) = setup();
        let inbound = coordinator.register_inbound().unwrap();
        coordinator
            .observe(inbound, Phase::AwaitingConsent)
            .unwrap();
        let revision = coordinator.transfers[&inbound]
            .view
            .consent_revision
            .unwrap();
        // Act
        coordinator
            .respond(client, inbound, revision, accept)
            .unwrap();
        let header = cache
            .capture(&mut coordinator, client, std::iter::empty())
            .unwrap();
        let intent = coordinator.next_control().unwrap();
        // Assert
        assert_eq!(
            cache.transfer_page(client, &header.0, 0).unwrap().1,
            vec![(
                token(inbound.0, inbound.1),
                "inbound",
                "awaiting_consent",
                true,
                revision,
                true,
                if accept { "accept" } else { "reject" },
                intent.revision
            )]
        );
        coordinator.cancel(client, inbound).unwrap();
        coordinator.mark_desynchronized().unwrap();
        let header = cache
            .capture(&mut coordinator, client, std::iter::empty())
            .unwrap();
        assert!(!header.4);
        let row = &cache.transfer_page(client, &header.0, 0).unwrap().1[0];
        assert_eq!(
            (row.2, row.3, row.4, row.5, row.6),
            ("cancel_requested", false, 0, true, "cancel")
        );
        coordinator.observe(inbound, Phase::Cancelled).unwrap();
        coordinator.begin_shutdown().unwrap();
        coordinator.finish_shutdown().unwrap();
        let header = cache
            .capture(&mut coordinator, client, std::iter::empty())
            .unwrap();
        assert_eq!(header.3, "stopped");
        let row = &cache.transfer_page(client, &header.0, 0).unwrap().1[0];
        assert_eq!(
            (row.2, row.3, row.4, row.5, row.6, row.7),
            ("cancelled", false, 0, false, "", 0)
        );
    }
}

#[test]
fn given_released_source_reservation_when_captured_then_metadata_uses_refreshed_resource_totals() {
    // Arrange
    let (mut coordinator, client, mut cache) = setup();
    coordinator.set_endpoints(vec!["nearby".into()]).unwrap();
    let selected = selection(&mut coordinator, client);
    let transfer = coordinator
        .send_selection(client, selected, "nearby", "send")
        .unwrap();
    let dispatch = coordinator.next_send().unwrap().unwrap();
    coordinator.observe(transfer, Phase::Finished).unwrap();
    drop(dispatch);
    assert_eq!(coordinator.usage().selections, 1); // refresh happens at read admission
                                                   // Act
    let header = cache
        .capture(&mut coordinator, client, std::iter::empty())
        .unwrap();
    // Assert
    assert_eq!((header.5, header.6, header.7), (0, 0, 0));
    assert_eq!(header.2, coordinator.revision);
    assert_eq!(coordinator.usage(), Default::default());
}

#[test]
fn given_immutable_capture_when_live_history_is_trimmed_then_old_pages_remain_stable() {
    // Arrange
    let (mut coordinator, client, mut cache) = setup();
    coordinator.limits.transfers = 2;
    let first = coordinator.register_inbound().unwrap();
    coordinator
        .observe(first, crate::service::Phase::Finished)
        .unwrap();
    coordinator.owner_closed(first).unwrap();
    let second = coordinator.register_inbound().unwrap();
    coordinator
        .observe(second, crate::service::Phase::Finished)
        .unwrap();
    coordinator.owner_closed(second).unwrap();
    let capture = cache
        .capture(&mut coordinator, client, std::iter::empty())
        .unwrap();
    let before = cache.transfer_page(client, &capture.0, 0).unwrap();
    // Act / Assert
    assert!(coordinator.maintain_history().unwrap());
    assert_eq!(cache.transfer_page(client, &capture.0, 0).unwrap(), before);
    let current = cache
        .capture(&mut coordinator, client, std::iter::empty())
        .unwrap();
    assert!(current.2 > capture.2);
    assert_eq!(current.10, 1);
    let rows = cache.transfer_page(client, &current.0, 0).unwrap().1;
    assert_eq!(rows[0].0, token(second.0, second.1));
}
