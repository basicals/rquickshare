use std::fs::File;

use super::*;
use crate::file_source::{OpenedFile, SelectionLimits};

fn selection(sizes: &[u64]) -> OpenedSelection {
    let files = sizes
        .iter()
        .enumerate()
        .map(|(index, size)| {
            let source = tempfile::NamedTempFile::new().unwrap();
            source.as_file().set_len(*size).unwrap();
            OpenedFile::adopt(
                File::open(source.path()).unwrap(),
                format!("private-source-{index}.txt"),
            )
            .unwrap()
        })
        .collect();
    OpenedSelection::from_files(files, SelectionLimits::default()).unwrap()
}

fn setup(limits: Limits) -> (Coordinator, ClientId) {
    let mut service = Coordinator::new(123, limits).unwrap();
    service
        .set_endpoints(vec!["private-endpoint".into(), "other-endpoint".into()])
        .unwrap();
    let client = service.connect().unwrap();
    (service, client)
}

fn queued(service: &mut Coordinator, client: ClientId, request: &str) -> TransferId {
    let selected = service.create_selection(client, selection(&[3])).unwrap();
    service
        .send_selection(client, selected, "private-endpoint", request)
        .unwrap()
}

fn waiting(service: &mut Coordinator, client: ClientId) -> (TransferId, u64) {
    let id = service.register_inbound().unwrap();
    service.observe(id, Phase::AwaitingConsent).unwrap();
    let revision = view(service, client, id).consent_revision.unwrap();
    (id, revision)
}

fn view(service: &mut Coordinator, client: ClientId, id: TransferId) -> TransferSnapshot {
    service
        .snapshot(client)
        .unwrap()
        .transfers
        .into_iter()
        .find(|entry| entry.id == id)
        .unwrap()
}

#[test]
fn given_zero_epoch_or_budget_when_constructed_then_input_is_rejected() {
    // Arrange
    let limits = Limits::default();
    let invalid = [
        Limits {
            clients: 0,
            ..limits
        },
        Limits {
            selections: 0,
            ..limits
        },
        Limits {
            reserved_handles: 0,
            ..limits
        },
        Limits {
            selected_bytes: 0,
            ..limits
        },
        Limits {
            queued_sends: 0,
            ..limits
        },
        Limits {
            transfers: 0,
            ..limits
        },
        Limits {
            endpoints: 0,
            ..limits
        },
        Limits {
            replay_records_per_client: 0,
            ..limits
        },
    ];
    // Act / Assert
    assert!(matches!(
        Coordinator::new(0, limits),
        Err(Error::InvalidInput)
    ));
    for limits in invalid {
        assert!(matches!(
            Coordinator::new(1, limits),
            Err(Error::InvalidInput)
        ));
    }
    assert!(Coordinator::new(1, limits).is_ok());
}

#[test]
fn given_new_epoch_when_old_tokens_are_used_then_no_identity_aliases() {
    // Arrange
    let (mut old, old_client) = setup(Limits::default());
    let old_selection = old.create_selection(old_client, selection(&[1])).unwrap();
    let old_transfer = old.register_inbound().unwrap();
    let mut new = Coordinator::new(124, Limits::default()).unwrap();
    new.set_endpoints(vec!["private-endpoint".into(), "other-endpoint".into()])
        .unwrap();
    let client = new.connect().unwrap();
    let selected = new.create_selection(client, selection(&[1])).unwrap();
    let transfer = new.register_inbound().unwrap();
    // Act / Assert
    assert_ne!(client, old_client);
    assert_ne!(selected, old_selection);
    assert_ne!(transfer, old_transfer);
    assert_eq!(new.snapshot(old_client), Err(Error::UnknownClient));
    assert_eq!(
        new.send_selection(client, old_selection, "private-endpoint", "r"),
        Err(Error::UnknownSelection)
    );
    assert_eq!(
        new.observe(old_transfer, Phase::Finished),
        Err(Error::UnknownTransfer)
    );
    assert_eq!(new.snapshot(client).unwrap().epoch, 124);
}

#[test]
fn given_full_client_budget_when_disconnected_then_slot_returns_with_fresh_token() {
    // Arrange
    let (mut service, client) = setup(Limits {
        clients: 1,
        ..Limits::default()
    });
    // Act / Assert
    assert_eq!(service.connect(), Err(Error::Capacity));
    service.disconnect(client).unwrap();
    let replacement = service.connect().unwrap();
    assert_ne!(client, replacement);
    assert_eq!(service.snapshot(client), Err(Error::UnknownClient));
    assert!(service.snapshot(replacement).is_ok());
}

#[test]
fn given_two_clients_when_snapshotted_then_only_owned_selections_are_visible() {
    // Arrange
    let (mut service, first) = setup(Limits::default());
    let second = service.connect().unwrap();
    let first_selection = service.create_selection(first, selection(&[1])).unwrap();
    let second_selection = service.create_selection(second, selection(&[2])).unwrap();
    let transfer = service.register_inbound().unwrap();
    // Act
    let first_snapshot = service.snapshot(first).unwrap();
    let second_snapshot = service.snapshot(second).unwrap();
    // Assert
    assert_eq!(first_snapshot.selections, vec![first_selection]);
    assert_eq!(second_snapshot.selections, vec![second_selection]);
    assert_eq!(first_snapshot.transfers, second_snapshot.transfers);
    assert_eq!(first_snapshot.transfers[0].id, transfer);
    assert_eq!(service.snapshot(first).unwrap(), first_snapshot);
    assert_eq!(first_snapshot.revision, second_snapshot.revision);
}

#[test]
fn given_unknown_client_when_commands_are_called_then_no_state_changes() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let unknown = ClientId(123, u64::MAX);
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    let (id, revision) = waiting(&mut service, client);
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert_eq!(service.snapshot(unknown), Err(Error::UnknownClient));
    assert_eq!(
        service.create_selection(unknown, selection(&[1])),
        Err(Error::UnknownClient)
    );
    assert_eq!(
        service.discard_selection(unknown, selected),
        Err(Error::UnknownClient)
    );
    assert_eq!(
        service.send_selection(unknown, selected, "private-endpoint", "r"),
        Err(Error::UnknownClient)
    );
    assert_eq!(
        service.respond(unknown, id, revision, true),
        Err(Error::UnknownClient)
    );
    assert_eq!(service.cancel(unknown, id), Err(Error::UnknownClient));
    service.disconnect(unknown).unwrap();
    assert_eq!(service.snapshot(client).unwrap(), before);
}

#[test]
fn given_private_resources_when_debugged_then_diagnostics_exclude_input_strings() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    queued(&mut service, client, "private-request");
    let dispatch = service.next_send().unwrap().unwrap();
    // Act
    let diagnostics = format!(
        "{service:?} {:?} {dispatch:?} {}",
        service.snapshot(client).unwrap(),
        Error::WrongOwner
    );
    // Assert
    for private in [
        "private-source",
        "private-endpoint",
        "private-request",
        "/tmp/",
    ] {
        assert!(!diagnostics.contains(private), "{private}");
    }
}

#[test]
fn given_owned_selection_when_admitted_then_reader_duplicates_are_reserved() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    // Act
    service
        .create_selection(client, selection(&[0, 2, 3]))
        .unwrap();
    // Assert
    assert_eq!(
        service.snapshot(client).unwrap().resources,
        ResourceUsage {
            selections: 1,
            reserved_handles: 6,
            selected_bytes: 5,
        }
    );
}

#[test]
fn given_descriptor_budget_when_exact_or_exceeded_then_admission_is_atomic() {
    // Arrange
    for budget in [1, 2, 3, 4] {
        let (mut service, client) = setup(Limits {
            reserved_handles: budget,
            ..Limits::default()
        });
        // Act
        let first = service.create_selection(client, selection(&[1]));
        let second = service.create_selection(client, selection(&[1]));
        // Assert
        assert_eq!(first.is_ok(), budget >= 2);
        assert_eq!(second.is_ok(), budget >= 4);
        let count = usize::from(budget >= 2) + usize::from(budget >= 4);
        assert_eq!(
            service.snapshot(client).unwrap().resources.reserved_handles,
            count * 2
        );
    }
}

#[test]
fn given_byte_budget_when_boundary_sizes_arrive_then_only_bounded_totals_are_admitted() {
    // Arrange
    for size in [0, 1, 2, 3, 4] {
        let (mut service, client) = setup(Limits {
            selected_bytes: 3,
            ..Limits::default()
        });
        // Act
        let result = service.create_selection(client, selection(&[size]));
        // Assert
        assert_eq!(result.is_ok(), size <= 3);
        assert_eq!(
            service.snapshot(client).unwrap().resources.selected_bytes,
            if size <= 3 { size } else { 0 }
        );
    }
}

#[test]
fn given_existing_selected_bytes_when_combined_limit_is_exceeded_then_old_selection_survives() {
    // Arrange
    let (mut service, client) = setup(Limits {
        selected_bytes: 3,
        ..Limits::default()
    });
    let selected = service.create_selection(client, selection(&[2])).unwrap();
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert_eq!(
        service.create_selection(client, selection(&[2])),
        Err(Error::Capacity)
    );
    assert_eq!(service.snapshot(client).unwrap(), before);
    service.create_selection(client, selection(&[1])).unwrap();
    assert_eq!(
        service.snapshot(client).unwrap().resources.selected_bytes,
        3
    );
    assert!(service
        .snapshot(client)
        .unwrap()
        .selections
        .contains(&selected));
}

#[test]
fn given_selection_budget_when_payload_is_queued_or_dispatched_then_reservation_remains() {
    // Arrange
    for dispatch_first in [false, true] {
        let (mut service, client) = setup(Limits {
            selections: 1,
            ..Limits::default()
        });
        queued(&mut service, client, "r");
        let dispatch = dispatch_first.then(|| service.next_send().unwrap().unwrap());
        // Act / Assert
        assert_eq!(
            service.create_selection(client, selection(&[1])),
            Err(Error::Capacity)
        );
        assert_eq!(service.snapshot(client).unwrap().resources.selections, 1);
        drop(dispatch);
    }
}

#[test]
fn given_accounting_overflow_when_reserved_then_every_sum_fails_closed() {
    // Arrange
    let limits = Limits {
        selections: usize::MAX,
        reserved_handles: usize::MAX,
        selected_bytes: u64::MAX,
        ..Limits::default()
    };
    let cases = [
        (
            ResourceUsage {
                selections: usize::MAX,
                ..ResourceUsage::default()
            },
            0,
            0,
        ),
        (
            ResourceUsage {
                reserved_handles: usize::MAX,
                ..ResourceUsage::default()
            },
            1,
            0,
        ),
        (
            ResourceUsage {
                selected_bytes: u64::MAX,
                ..ResourceUsage::default()
            },
            0,
            1,
        ),
    ];
    // Act / Assert
    for (current, handles, bytes) in cases {
        assert_eq!(
            reserve_usage(current, handles, bytes, limits),
            Err(Error::Capacity)
        );
    }
    assert_eq!(
        reserve_usage(ResourceUsage::default(), usize::MAX, u64::MAX, limits).unwrap(),
        ResourceUsage {
            selections: 1,
            reserved_handles: usize::MAX,
            selected_bytes: u64::MAX,
        }
    );
}

#[test]
fn given_foreign_selection_when_discarded_then_owner_resources_are_untouched() {
    // Arrange
    let (mut service, owner) = setup(Limits::default());
    let other = service.connect().unwrap();
    let selected = service.create_selection(owner, selection(&[1])).unwrap();
    let before = service.snapshot(owner).unwrap();
    // Act / Assert
    assert_eq!(
        service.discard_selection(other, selected),
        Err(Error::WrongOwner)
    );
    assert_eq!(service.snapshot(owner).unwrap(), before);
}

#[test]
fn given_owned_selection_when_discarded_twice_then_resources_are_released_once() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    // Act
    service.discard_selection(client, selected).unwrap();
    let after = service.snapshot(client).unwrap();
    service.discard_selection(client, selected).unwrap();
    // Assert
    assert_eq!(service.snapshot(client).unwrap(), after);
    assert_eq!(after.resources, ResourceUsage::default());
    assert!(after.selections.is_empty());
}

#[test]
fn given_client_with_unsent_files_when_disconnected_then_other_client_is_preserved() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let other = service.connect().unwrap();
    service.create_selection(client, selection(&[1])).unwrap();
    let kept = service.create_selection(other, selection(&[2])).unwrap();
    // Act
    service.disconnect(client).unwrap();
    let after = service.snapshot(other).unwrap();
    service.disconnect(client).unwrap();
    // Assert
    assert_eq!(after.selections, vec![kept]);
    assert_eq!(after.resources.selected_bytes, 2);
    assert_eq!(service.snapshot(other).unwrap(), after);
}

#[test]
fn given_endpoint_set_when_replaced_or_repeated_then_revision_changes_only_on_change() {
    // Arrange
    let (mut service, client) = setup(Limits {
        endpoints: 2,
        ..Limits::default()
    });
    let before = service.snapshot(client).unwrap().revision;
    // Act
    service
        .set_endpoints(vec!["other-endpoint".into(), "private-endpoint".into()])
        .unwrap();
    // Assert
    assert_eq!(service.snapshot(client).unwrap().revision, before);
    service
        .set_endpoints(vec!["private-endpoint".into(), "private-endpoint".into()])
        .unwrap();
    assert_eq!(service.endpoints.len(), 1);
    assert!(service.snapshot(client).unwrap().revision > before);
    service.set_endpoints(vec![]).unwrap();
    assert!(service.endpoints.is_empty());
}

#[test]
fn given_bad_endpoint_input_when_updated_then_existing_discovery_is_unchanged() {
    // Arrange
    for bad in [
        vec!["".into()],
        vec!["a b".into()],
        vec!["é".into()],
        vec!["a\n".into()],
        vec!["a".repeat(257)],
        vec!["a".into(), "b".into(), "c".into()],
    ] {
        let (mut service, client) = setup(Limits {
            endpoints: 2,
            ..Limits::default()
        });
        let before = service.snapshot(client).unwrap();
        // Act / Assert
        assert!(service.set_endpoints(bad).is_err());
        assert_eq!(service.snapshot(client).unwrap(), before);
        assert!(service.endpoints.contains("private-endpoint"));
    }
    let (mut service, _) = setup(Limits::default());
    service.set_endpoints(vec!["a".repeat(256)]).unwrap();
}

#[test]
fn given_unknown_endpoint_when_sent_then_selection_and_request_remain_retryable() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert_eq!(
        service.send_selection(client, selected, "missing", "r"),
        Err(Error::UnknownEndpoint)
    );
    assert_eq!(service.snapshot(client).unwrap(), before);
    assert!(service
        .send_selection(client, selected, "private-endpoint", "r")
        .is_ok());
}

#[test]
fn given_request_boundaries_when_sent_then_invalid_ids_do_not_consume_selection() {
    // Arrange
    for request in [
        "".into(),
        "a".into(),
        "a".repeat(127),
        "a".repeat(128),
        "a".repeat(129),
        "a b".into(),
        "é".into(),
        "\0".into(),
        "a\n".into(),
    ] {
        let (mut service, client) = setup(Limits::default());
        let selected = service.create_selection(client, selection(&[1])).unwrap();
        let expected = !request.is_empty()
            && request.len() <= 128
            && request.bytes().all(|b| b.is_ascii_graphic());
        // Act
        let result = service.send_selection(client, selected, "private-endpoint", &request);
        // Assert
        assert_eq!(result.is_ok(), expected);
        assert_eq!(
            service
                .snapshot(client)
                .unwrap()
                .selections
                .contains(&selected),
            !expected
        );
    }
}

#[test]
fn given_foreign_or_missing_selection_when_sent_then_transfer_is_not_created() {
    // Arrange
    let (mut service, owner) = setup(Limits::default());
    let other = service.connect().unwrap();
    let selected = service.create_selection(owner, selection(&[1])).unwrap();
    let before = service.snapshot(owner).unwrap();
    // Act / Assert
    assert_eq!(
        service.send_selection(other, selected, "private-endpoint", "r"),
        Err(Error::WrongOwner)
    );
    assert_eq!(
        service.send_selection(owner, SelectionId(123, u64::MAX), "private-endpoint", "r"),
        Err(Error::UnknownSelection)
    );
    assert_eq!(
        service.send_selection(owner, selected, "bad endpoint", "r"),
        Err(Error::InvalidInput)
    );
    assert_eq!(service.snapshot(owner).unwrap(), before);
}

#[test]
fn given_accepted_send_when_replayed_after_terminal_and_endpoint_loss_then_original_receipt_returns(
) {
    // Arrange
    let (mut service, client) = setup(Limits {
        queued_sends: 1,
        ..Limits::default()
    });
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    let transfer = service
        .send_selection(client, selected, "private-endpoint", "r")
        .unwrap();
    // Act / Assert
    let queued_snapshot = service.snapshot(client).unwrap();
    assert_eq!(
        service.send_selection(client, selected, "private-endpoint", "r"),
        Ok(transfer)
    );
    assert_eq!(service.snapshot(client).unwrap(), queued_snapshot);
    service.observe(transfer, Phase::Failed).unwrap();
    service.set_endpoints(vec![]).unwrap();
    service.mark_desynchronized().unwrap();
    let terminal_snapshot = service.snapshot(client).unwrap();
    assert_eq!(
        service.send_selection(client, selected, "private-endpoint", "r"),
        Ok(transfer)
    );
    assert_eq!(service.snapshot(client).unwrap(), terminal_snapshot);
    assert_eq!(service.queue.len(), 0);
}

#[test]
fn given_used_request_id_when_arguments_conflict_then_replay_is_rejected_atomically() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let first = service.create_selection(client, selection(&[1])).unwrap();
    let second = service.create_selection(client, selection(&[1])).unwrap();
    service
        .send_selection(client, first, "private-endpoint", "r")
        .unwrap();
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    for (selected, endpoint) in [(second, "private-endpoint"), (first, "other-endpoint")] {
        assert_eq!(
            service.send_selection(client, selected, endpoint, "r"),
            Err(Error::ReplayConflict)
        );
        assert_eq!(service.snapshot(client).unwrap(), before);
    }
}

#[test]
fn given_distinct_clients_when_same_request_id_is_used_then_ledgers_are_independent() {
    // Arrange
    let (mut service, first) = setup(Limits::default());
    let second = service.connect().unwrap();
    // Act
    let first_transfer = queued(&mut service, first, "r");
    let second_transfer = queued(&mut service, second, "r");
    // Assert
    assert_ne!(first_transfer, second_transfer);
    assert_eq!(service.snapshot(first).unwrap().transfers.len(), 2);
    assert_eq!(service.queue.len(), 2);
}

#[test]
fn given_full_replay_ledger_when_transfer_finishes_then_records_are_not_silently_evicted() {
    // Arrange
    let (mut service, client) = setup(Limits {
        replay_records_per_client: 1,
        ..Limits::default()
    });
    let id = queued(&mut service, client, "r");
    service.observe(id, Phase::Failed).unwrap();
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    // Act / Assert
    assert_eq!(
        service.send_selection(client, selected, "private-endpoint", "next"),
        Err(Error::Capacity)
    );
    assert_eq!(service.snapshot(client).unwrap().selections, vec![selected]);
    service.disconnect(client).unwrap();
    let replacement = service.connect().unwrap();
    assert!(queued(&mut service, replacement, "r") != id);
}

#[test]
fn given_full_queue_when_dispatch_releases_slot_then_failed_send_can_retry() {
    // Arrange
    let (mut service, client) = setup(Limits {
        queued_sends: 1,
        ..Limits::default()
    });
    let first = queued(&mut service, client, "first");
    let second_selection = service.create_selection(client, selection(&[1])).unwrap();
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert_eq!(
        service.send_selection(client, second_selection, "private-endpoint", "second"),
        Err(Error::Capacity)
    );
    assert_eq!(service.snapshot(client).unwrap(), before);
    let first_dispatch = service.next_send().unwrap().unwrap();
    assert_eq!(first_dispatch.transfer, first);
    let second = service
        .send_selection(client, second_selection, "private-endpoint", "second")
        .unwrap();
    assert_eq!(service.next_send().unwrap().unwrap().transfer, second);
}

#[test]
fn given_full_transfer_history_when_new_work_arrives_then_inbound_and_outbound_fail_closed() {
    // Arrange
    let (mut service, client) = setup(Limits {
        transfers: 1,
        ..Limits::default()
    });
    let terminal = queued(&mut service, client, "first");
    service.observe(terminal, Phase::Failed).unwrap();
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert_eq!(service.register_inbound(), Err(Error::Capacity));
    assert_eq!(
        service.send_selection(client, selected, "private-endpoint", "second"),
        Err(Error::Capacity)
    );
    assert_eq!(service.snapshot(client).unwrap(), before);
}

#[test]
fn given_multiple_queued_sends_when_dispatched_then_order_and_payload_are_preserved_once() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let first = queued(&mut service, client, "first");
    let second = queued(&mut service, client, "second");
    // Act / Assert
    for expected in [first, second] {
        let dispatch = service.next_send().unwrap().unwrap();
        assert_eq!(dispatch.transfer, expected);
        assert_eq!(dispatch.endpoint, "private-endpoint");
        assert_eq!(dispatch.selection.total_bytes(), 3);
        assert_eq!(
            dispatch.selection.files()[0].relative_name(),
            "private-source-0.txt"
        );
        assert_eq!(view(&mut service, client, expected).phase, Phase::Active);
    }
    assert!(service.next_send().unwrap().is_none());
}

#[test]
fn given_sent_transfer_when_client_disconnects_then_queued_and_active_payloads_survive() {
    // Arrange
    for dispatch_first in [false, true] {
        let (mut service, client) = setup(Limits::default());
        let id = queued(&mut service, client, "r");
        let held = dispatch_first.then(|| service.next_send().unwrap().unwrap());
        // Act
        service.disconnect(client).unwrap();
        let replacement = service.connect().unwrap();
        let dispatch = held.unwrap_or_else(|| service.next_send().unwrap().unwrap());
        // Assert
        assert_eq!(dispatch.transfer, id);
        assert_eq!(view(&mut service, replacement, id).phase, Phase::Active);
        let mut bytes = [9; 3];
        assert_eq!(
            dispatch.selection.files()[0]
                .read_chunk_at(0, &mut bytes)
                .unwrap(),
            3
        );
        assert_eq!(bytes, [0; 3]);
    }
}

#[test]
fn given_terminal_event_with_live_payload_when_admission_retries_then_budget_waits_for_last_reference(
) {
    // Arrange
    let (mut service, client) = setup(Limits {
        selections: 1,
        ..Limits::default()
    });
    let id = queued(&mut service, client, "r");
    let dispatch = service.next_send().unwrap().unwrap();
    let worker_payload = Arc::clone(&dispatch.selection);
    let weak = Arc::downgrade(&worker_payload);
    // Act
    service.observe(id, Phase::Finished).unwrap();
    drop(dispatch);
    // Assert
    assert_eq!(
        service.create_selection(client, selection(&[1])),
        Err(Error::Capacity)
    );
    assert_eq!(
        service.snapshot(client).unwrap().resources.reserved_handles,
        2
    );
    drop(worker_payload);
    assert!(weak.upgrade().is_none());
    assert_eq!(
        service.snapshot(client).unwrap().resources,
        ResourceUsage::default()
    );
    assert!(service.create_selection(client, selection(&[1])).is_ok());
}

#[test]
fn given_empty_queue_when_dispatch_requested_then_revision_is_stable() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert!(service.next_send().unwrap().is_none());
    assert_eq!(service.snapshot(client).unwrap(), before);
}

#[test]
fn given_adapter_phase_incompatible_with_direction_when_observed_then_snapshot_is_unchanged() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let outbound = queued(&mut service, client, "r");
    let inbound = service.register_inbound().unwrap();
    // Act / Assert
    for (id, phases) in [
        (
            inbound,
            vec![Phase::Queued, Phase::CancelRequested, Phase::Sending],
        ),
        (
            outbound,
            vec![
                Phase::Active,
                Phase::Sending,
                Phase::Receiving,
                Phase::AwaitingConsent,
            ],
        ),
    ] {
        for phase in phases {
            let before = service.snapshot(client).unwrap();
            assert_eq!(service.observe(id, phase), Err(Error::InvalidInput));
            assert_eq!(service.snapshot(client).unwrap(), before);
        }
    }
    let _dispatch = service.next_send().unwrap().unwrap();
    for phase in [Phase::Receiving, Phase::AwaitingConsent] {
        assert_eq!(service.observe(outbound, phase), Err(Error::InvalidInput));
    }
    service.observe(outbound, Phase::Sending).unwrap();
    service.observe(inbound, Phase::Receiving).unwrap();
}

#[test]
fn given_terminal_transfer_when_late_events_arrive_then_no_state_resurrects() {
    // Arrange
    for terminal in [
        Phase::Finished,
        Phase::Rejected,
        Phase::Cancelled,
        Phase::Failed,
        Phase::Disconnected,
    ] {
        let (mut service, client) = setup(Limits::default());
        let id = queued(&mut service, client, "r");
        service.observe(id, terminal).unwrap();
        let before = service.snapshot(client).unwrap();
        // Act / Assert
        for late in [
            Phase::Active,
            Phase::Disconnected,
            Phase::Finished,
            Phase::AwaitingConsent,
            Phase::Queued,
        ] {
            service.observe(id, late).unwrap();
            assert_eq!(service.snapshot(client).unwrap(), before);
        }
        assert_eq!(before.resources, ResourceUsage::default());
        assert!(service.next_send().unwrap().is_none());
    }
}

#[test]
fn given_inbound_registration_when_observed_then_no_outbound_resource_is_reserved() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    // Act
    let id = service.register_inbound().unwrap();
    let transfer = view(&mut service, client, id);
    // Assert
    assert_eq!(transfer.direction, Direction::Inbound);
    assert_eq!(transfer.phase, Phase::Active);
    assert_eq!(transfer.consent_revision, None);
    assert_eq!(
        service.snapshot(client).unwrap().resources,
        ResourceUsage::default()
    );
}

#[test]
fn given_same_waiting_event_when_repeated_then_consent_token_and_answer_remain_stable() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let (id, token) = waiting(&mut service, client);
    service.respond(client, id, token, true).unwrap();
    let before = service.snapshot(client).unwrap();
    // Act
    service.observe(id, Phase::AwaitingConsent).unwrap();
    // Assert
    assert_eq!(service.snapshot(client).unwrap(), before);
    assert_eq!(view(&mut service, client, id).consent_revision, Some(token));
    assert_eq!(
        service.respond(client, id, token, false),
        Err(Error::StaleConsent)
    );
}

#[test]
fn given_current_consent_when_answer_is_replayed_then_same_answer_is_idempotent_and_conflict_fails()
{
    // Arrange
    for accept in [false, true] {
        let (mut service, client) = setup(Limits::default());
        let (id, token) = waiting(&mut service, client);
        // Act
        service.respond(client, id, token, accept).unwrap();
        let after = service.snapshot(client).unwrap();
        // Assert
        assert_eq!(
            service.next_control().unwrap().action,
            if accept {
                ControlAction::Accept
            } else {
                ControlAction::Reject
            }
        );
        service.respond(client, id, token, accept).unwrap();
        assert_eq!(
            service.respond(client, id, token, !accept),
            Err(Error::StaleConsent)
        );
        assert_eq!(service.snapshot(client).unwrap(), after);
    }
}

#[test]
fn given_new_consent_round_when_old_token_arrives_then_only_current_round_accepts_answer() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let (id, old) = waiting(&mut service, client);
    service.respond(client, id, old, true).unwrap();
    service.observe(id, Phase::Receiving).unwrap();
    service.observe(id, Phase::AwaitingConsent).unwrap();
    let current = view(&mut service, client, id).consent_revision.unwrap();
    // Act / Assert
    assert!(current > old);
    for stale in [0, old, current - 1, current + 1, u64::MAX] {
        assert_eq!(
            service.respond(client, id, stale, true),
            Err(Error::StaleConsent)
        );
    }
    assert!(service.next_control().is_none());
    service.respond(client, id, current, false).unwrap();
    assert_eq!(
        service.next_control().unwrap().action,
        ControlAction::Reject
    );
}

#[test]
fn given_transfer_outside_inbound_wait_when_answered_then_no_control_is_emitted() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let inbound = service.register_inbound().unwrap();
    let outbound = queued(&mut service, client, "r");
    // Act / Assert
    for id in [inbound, outbound] {
        assert_eq!(
            service.respond(client, id, 0, true),
            Err(Error::StaleConsent)
        );
    }
    service.observe(inbound, Phase::AwaitingConsent).unwrap();
    let token = view(&mut service, client, inbound)
        .consent_revision
        .unwrap();
    service.observe(inbound, Phase::Finished).unwrap();
    assert_eq!(
        service.respond(client, inbound, token, true),
        Err(Error::StaleConsent)
    );
    assert!(service.next_control().is_none());
}

#[test]
fn given_unknown_transfer_when_control_or_observation_arrives_then_state_is_unchanged() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = TransferId(123, u64::MAX);
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert_eq!(
        service.observe(id, Phase::Finished),
        Err(Error::UnknownTransfer)
    );
    assert_eq!(
        service.respond(client, id, 1, true),
        Err(Error::UnknownTransfer)
    );
    assert_eq!(service.cancel(client, id), Err(Error::UnknownTransfer));
    assert_eq!(
        service.acknowledge_control(ControlIntent {
            transfer: id,
            revision: 1,
            action: ControlAction::Accept
        }),
        Err(Error::UnknownTransfer)
    );
    assert_eq!(service.snapshot(client).unwrap(), before);
}

#[test]
fn given_pending_control_when_peeked_and_acknowledged_then_delivery_does_not_invent_transfer_progress(
) {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let (id, token) = waiting(&mut service, client);
    service.respond(client, id, token, true).unwrap();
    let intent = service.next_control().unwrap();
    // Act / Assert
    assert_eq!(service.next_control(), Some(intent));
    for wrong in [
        ControlIntent {
            revision: intent.revision + 1,
            ..intent
        },
        ControlIntent {
            action: ControlAction::Reject,
            ..intent
        },
    ] {
        assert_eq!(service.acknowledge_control(wrong), Err(Error::StaleControl));
        assert_eq!(service.next_control(), Some(intent));
    }
    service.acknowledge_control(intent).unwrap();
    assert!(service.next_control().is_none());
    assert_eq!(view(&mut service, client, id).phase, Phase::AwaitingConsent);
    assert_eq!(
        service.acknowledge_control(intent),
        Err(Error::StaleControl)
    );
    service.respond(client, id, token, true).unwrap();
    assert!(service.next_control().is_none());
}

#[test]
fn given_answer_superseded_by_cancel_when_old_acknowledgement_arrives_then_cancel_remains_pending()
{
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let (id, token) = waiting(&mut service, client);
    service.respond(client, id, token, true).unwrap();
    let old = service.next_control().unwrap();
    // Act
    service.cancel(client, id).unwrap();
    let cancel = service.next_control().unwrap();
    // Assert
    assert_eq!(cancel.action, ControlAction::Cancel);
    assert!(cancel.revision > old.revision);
    assert_eq!(service.acknowledge_control(old), Err(Error::StaleControl));
    assert_eq!(service.next_control(), Some(cancel));
    assert_eq!(
        service.respond(client, id, token, true),
        Err(Error::StaleConsent)
    );
}

#[test]
fn given_queued_transfer_when_cancelled_then_no_transport_command_is_dispatched() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = queued(&mut service, client, "r");
    // Act
    service.cancel(client, id).unwrap();
    let after = service.snapshot(client).unwrap();
    service.cancel(client, id).unwrap();
    // Assert
    assert_eq!(service.snapshot(client).unwrap(), after);
    assert_eq!(view(&mut service, client, id).phase, Phase::Cancelled);
    assert_eq!(after.resources, ResourceUsage::default());
    assert!(service.next_send().unwrap().is_none());
    assert!(service.next_control().is_none());
}

#[test]
fn given_active_transfer_when_cancelled_then_only_terminal_core_observation_completes_it() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = queued(&mut service, client, "r");
    let _dispatch = service.next_send().unwrap().unwrap();
    // Act
    service.cancel(client, id).unwrap();
    let requested = service.snapshot(client).unwrap();
    service.cancel(client, id).unwrap();
    service.observe(id, Phase::Sending).unwrap();
    // Assert
    assert_eq!(service.snapshot(client).unwrap(), requested);
    let intent = service.next_control().unwrap();
    service.acknowledge_control(intent).unwrap();
    assert_eq!(view(&mut service, client, id).phase, Phase::CancelRequested);
    service.observe(id, Phase::Cancelled).unwrap();
    assert_eq!(view(&mut service, client, id).phase, Phase::Cancelled);
}

#[test]
fn given_terminal_transfer_when_cancelled_then_every_terminal_outcome_is_preserved() {
    // Arrange
    for terminal in [
        Phase::Finished,
        Phase::Rejected,
        Phase::Cancelled,
        Phase::Failed,
        Phase::Disconnected,
    ] {
        let (mut service, client) = setup(Limits::default());
        let id = service.register_inbound().unwrap();
        service.observe(id, terminal).unwrap();
        let before = service.snapshot(client).unwrap();
        // Act
        service.cancel(client, id).unwrap();
        // Assert
        assert_eq!(service.snapshot(client).unwrap(), before);
        assert!(terminal.is_terminal());
    }
    for phase in [
        Phase::Queued,
        Phase::Active,
        Phase::AwaitingConsent,
        Phase::Receiving,
        Phase::Sending,
        Phase::CancelRequested,
    ] {
        assert!(!phase.is_terminal());
    }
}

#[test]
fn given_event_loss_when_commands_arrive_then_new_send_dispatch_and_consent_pause_but_cancel_works()
{
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = queued(&mut service, client, "r");
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    let (inbound, token) = waiting(&mut service, client);
    // Act
    service.mark_desynchronized().unwrap();
    let after = service.snapshot(client).unwrap();
    service.mark_desynchronized().unwrap();
    // Assert
    assert_eq!(service.snapshot(client).unwrap(), after);
    assert!(!after.synchronized);
    assert_eq!(
        service.send_selection(client, selected, "private-endpoint", "new"),
        Err(Error::Unsynchronized)
    );
    assert!(matches!(service.next_send(), Err(Error::Unsynchronized)));
    assert_eq!(
        service.respond(client, inbound, token, true),
        Err(Error::Unsynchronized)
    );
    assert_eq!(service.register_inbound(), Err(Error::Unsynchronized));
    service.cancel(client, id).unwrap();
    service.cancel(client, inbound).unwrap();
    assert_eq!(
        service.next_control().unwrap().action,
        ControlAction::Cancel
    );
}

#[test]
fn given_lost_events_when_later_events_and_reconnect_arrive_then_synchronization_is_not_fabricated()
{
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let (id, token) = waiting(&mut service, client);
    service.respond(client, id, token, true).unwrap();
    // Act
    service.mark_desynchronized().unwrap();
    assert!(service.next_control().is_none());
    service.observe(id, Phase::Finished).unwrap();
    service.disconnect(client).unwrap();
    let replacement = service.connect().unwrap();
    // Assert
    let snapshot = service.snapshot(replacement).unwrap();
    assert!(!snapshot.synchronized);
    assert_eq!(snapshot.transfers[0].phase, Phase::Finished);
    assert!(service.next_control().is_none());
}

#[test]
fn given_pending_and_active_work_when_shutdown_begins_then_resources_and_completion_are_drained() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    service.create_selection(client, selection(&[1])).unwrap();
    let active = queued(&mut service, client, "active");
    let dispatch = service.next_send().unwrap().unwrap();
    let queued = queued(&mut service, client, "queued");
    let (inbound, token) = waiting(&mut service, client);
    service.respond(client, inbound, token, true).unwrap();
    // Act
    service.begin_shutdown().unwrap();
    let draining = service.snapshot(client).unwrap();
    // Assert
    assert_eq!(draining.lifecycle, Lifecycle::Draining);
    assert!(draining.selections.is_empty());
    assert_eq!(draining.resources.selections, 1);
    assert_eq!(view(&mut service, client, queued).phase, Phase::Cancelled);
    for id in [active, inbound] {
        let view = view(&mut service, client, id);
        assert_eq!(view.phase, Phase::CancelRequested);
        assert_eq!(view.pending_control.unwrap().action, ControlAction::Cancel);
        assert_eq!(view.consent_revision, None);
    }
    assert_eq!(service.finish_shutdown(), Err(Error::Busy));
    for id in [active, inbound] {
        service.observe(id, Phase::Cancelled).unwrap();
    }
    assert_eq!(service.finish_shutdown(), Err(Error::Busy));
    drop(dispatch);
    service.finish_shutdown().unwrap();
    assert_eq!(
        service.snapshot(client).unwrap().lifecycle,
        Lifecycle::Stopped
    );
    assert_eq!(
        service.snapshot(client).unwrap().resources,
        ResourceUsage::default()
    );
}

#[test]
fn given_shutdown_replay_when_already_draining_or_stopped_then_revision_is_stable() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    assert_eq!(service.finish_shutdown(), Err(Error::NotRunning));
    // Act / Assert
    service.begin_shutdown().unwrap();
    let draining = service.snapshot(client).unwrap();
    service.begin_shutdown().unwrap();
    assert_eq!(service.snapshot(client).unwrap(), draining);
    service.finish_shutdown().unwrap();
    let stopped = service.snapshot(client).unwrap();
    service.finish_shutdown().unwrap();
    service.begin_shutdown().unwrap();
    assert_eq!(service.snapshot(client).unwrap(), stopped);
}

#[test]
fn given_draining_or_stopped_service_when_new_work_arrives_then_every_admission_path_is_closed() {
    // Arrange
    for stopped in [false, true] {
        let (mut service, client) = setup(Limits::default());
        service.begin_shutdown().unwrap();
        if stopped {
            service.finish_shutdown().unwrap();
        }
        let before = service.snapshot(client).unwrap();
        // Act / Assert
        assert_eq!(service.connect(), Err(Error::NotRunning));
        assert_eq!(service.set_endpoints(vec![]), Err(Error::NotRunning));
        assert_eq!(
            service.create_selection(client, selection(&[1])),
            Err(Error::NotRunning)
        );
        assert_eq!(
            service.send_selection(client, SelectionId(123, 1), "private-endpoint", "r"),
            Err(Error::NotRunning)
        );
        assert_eq!(service.register_inbound(), Err(Error::NotRunning));
        assert_eq!(
            service.respond(client, TransferId(123, 1), 1, true),
            Err(Error::NotRunning)
        );
        assert!(matches!(service.next_send(), Err(Error::NotRunning)));
        assert_eq!(service.snapshot(client).unwrap(), before);
    }
}

#[test]
fn given_exhausted_revision_when_mutation_is_requested_then_no_partial_change_occurs() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    let (id, token) = waiting(&mut service, client);
    service.revision = u64::MAX;
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert_eq!(service.connect(), Err(Error::Exhausted));
    assert_eq!(
        service.create_selection(client, selection(&[1])),
        Err(Error::Exhausted)
    );
    assert_eq!(
        service.discard_selection(client, selected),
        Err(Error::Exhausted)
    );
    assert_eq!(service.disconnect(client), Err(Error::Exhausted));
    assert_eq!(service.set_endpoints(vec![]), Err(Error::Exhausted));
    assert_eq!(
        service.send_selection(client, selected, "private-endpoint", "r"),
        Err(Error::Exhausted)
    );
    assert_eq!(service.register_inbound(), Err(Error::Exhausted));
    assert_eq!(service.observe(id, Phase::Receiving), Err(Error::Exhausted));
    assert_eq!(
        service.respond(client, id, token, true),
        Err(Error::Exhausted)
    );
    assert_eq!(service.cancel(client, id), Err(Error::Exhausted));
    assert_eq!(service.mark_desynchronized(), Err(Error::Exhausted));
    assert_eq!(service.begin_shutdown(), Err(Error::Exhausted));
    assert_eq!(service.snapshot(client).unwrap(), before);
}

#[test]
fn given_last_revision_when_discard_releases_resources_then_mutation_finishes_without_post_commit_error(
) {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let selected = service.create_selection(client, selection(&[1])).unwrap();
    service.revision = u64::MAX - 1;
    // Act
    service.discard_selection(client, selected).unwrap();
    // Assert
    let snapshot = service.snapshot(client).unwrap();
    assert_eq!(snapshot.revision, u64::MAX);
    assert_eq!(snapshot.resources, ResourceUsage::default());
    assert!(snapshot.selections.is_empty());
}

#[test]
fn given_exhausted_revision_when_dispatch_or_ack_is_requested_then_pending_work_is_preserved() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    queued(&mut service, client, "r");
    let (id, token) = waiting(&mut service, client);
    service.respond(client, id, token, true).unwrap();
    let intent = service.next_control().unwrap();
    service.revision = u64::MAX;
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert!(matches!(service.next_send(), Err(Error::Exhausted)));
    assert_eq!(service.acknowledge_control(intent), Err(Error::Exhausted));
    assert_eq!(service.next_control(), Some(intent));
    assert_eq!(service.snapshot(client).unwrap(), before);
    assert_eq!(service.queue.len(), 1);
}

#[test]
fn given_exhausted_revision_when_shutdown_completion_is_requested_then_draining_is_preserved() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    service.begin_shutdown().unwrap();
    service.revision = u64::MAX;
    // Act / Assert
    assert_eq!(service.finish_shutdown(), Err(Error::Exhausted));
    assert_eq!(
        service.snapshot(client).unwrap().lifecycle,
        Lifecycle::Draining
    );
}

#[test]
fn given_exhausted_revision_when_worker_drops_last_payload_then_resource_refresh_fails_closed() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    queued(&mut service, client, "r");
    let dispatch = service.next_send().unwrap().unwrap();
    service.revision = u64::MAX;
    drop(dispatch);
    // Act / Assert
    assert_eq!(service.snapshot(client), Err(Error::Exhausted));
    assert_eq!(service.reservations.len(), 1);
    assert_eq!(service.revision, u64::MAX);
}

#[test]
fn given_pending_consent_when_metadata_changes_then_one_revision_updates_details_without_replacing_control_or_round(
) {
    // Arrange
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    let client = coordinator.connect().unwrap();
    let id = coordinator.register_inbound().unwrap();
    coordinator.observe(id, Phase::AwaitingConsent).unwrap();
    let round = view(&mut coordinator, client, id).consent_revision.unwrap();
    coordinator.respond(client, id, round, false).unwrap();
    let old = coordinator.snapshot(client).unwrap();
    let details = Arc::new(crate::sessions::details::TransferDetails {
        status: crate::sessions::details::DetailsStatus::Ready,
        sender: "phone".into(),
        pin: "0123".into(),
        files: vec![crate::sessions::details::FileDetail {
            name: "item".into(),
            size: 3,
        }]
        .into(),
        total_bytes: 3,
        ..Default::default()
    });
    // Act
    coordinator
        .observe_details(id, Phase::AwaitingConsent, details.clone())
        .unwrap();
    // Assert
    let next = coordinator.snapshot(client).unwrap();
    assert_eq!(next.revision, old.revision + 1);
    assert_eq!(next.transfers[0].consent_revision, Some(round));
    assert_eq!(
        next.transfers[0].pending_control,
        old.transfers[0].pending_control
    );
    assert_eq!(next.transfers[0].details, details);
    coordinator
        .observe_details(id, Phase::AwaitingConsent, details)
        .unwrap();
    assert_eq!(
        coordinator.snapshot(client).unwrap().revision,
        next.revision
    );
    coordinator.cancel(client, id).unwrap();
    assert!(view(&mut coordinator, client, id).details.pin.is_empty());
    assert_eq!(next.transfers[0].details.pin, "0123");
}
