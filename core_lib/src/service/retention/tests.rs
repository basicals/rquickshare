use super::*;
use crate::service::operations::tests::{append, setup};

fn closed(service: &mut Coordinator) -> TransferId {
    let id = service.register_inbound().unwrap();
    service.observe(id, Phase::Finished).unwrap();
    service.owner_closed(id).unwrap();
    id
}

#[test]
fn given_65_closed_records_when_maintained_then_oldest_is_removed_and_stale_controls_fail() {
    // Arrange
    let (mut service, client) = setup();
    let first = closed(&mut service);
    for _ in 1..64 {
        closed(&mut service);
    }
    // Act / Assert
    assert!(!service.maintain_history().unwrap());
    assert_eq!(service.transfers.len(), 64);
    let last = closed(&mut service);
    let before = service.revision;
    assert!(service.maintain_history().unwrap());
    assert_eq!(service.revision, before + 1);
    assert_eq!(service.transfers.len(), 64);
    assert!(!service.transfers.contains_key(&first));
    assert!(service.transfers.contains_key(&last));
    assert_eq!(service.cancel(client, first), Err(Error::UnknownTransfer));
    assert_eq!(
        service.respond(client, first, 1, true),
        Err(Error::UnknownTransfer)
    );
}

#[test]
fn given_source_owner_and_control_barriers_when_interleaved_then_only_fully_closed_history_retires()
{
    // Arrange / Act / Assert
    for blocker in ["owner", "release", "source", "receipt"] {
        let (mut service, client) = setup();
        service.limits.transfers = 2;
        let upload = service.ordered_begin(client, 1).unwrap();
        append(&mut service, client, upload);
        let selection = service.ordered_commit(client, upload, 2).unwrap();
        let id = service.ordered_send(client, selection, "peer", 3).unwrap();
        service.acknowledge_operations(client, 3).unwrap();
        let dispatch = service.next_send().unwrap().unwrap();
        let mut reader = Some(dispatch.selection);
        service.observe(id, Phase::Finished).unwrap();
        if blocker != "owner" {
            service.owner_closed(id).unwrap();
        }
        if blocker != "release" {
            service.source_released(id).unwrap();
        }
        if blocker != "source" {
            reader.take();
        }
        if blocker == "receipt" {
            service.control_receipt(id, true).unwrap();
        }
        let newer = closed(&mut service);
        assert!(!service.maintain_history().unwrap());
        assert!(service.transfers.contains_key(&id));
        match blocker {
            "owner" => service.owner_closed(id).unwrap(),
            "release" => service.source_released(id).unwrap(),
            "source" => {
                reader.take();
            }
            _ => service.control_receipt(id, false).unwrap(),
        }
        assert!(service.maintain_history().unwrap());
        assert!(!service.transfers.contains_key(&id));
        assert!(service.transfers.contains_key(&newer));
    }
}

#[test]
fn given_legacy_or_unacknowledged_send_when_terminal_then_replay_pins_until_disconnect_or_ack() {
    // Arrange / Act / Assert
    for legacy in [false, true] {
        let (mut service, client) = setup();
        service.limits.transfers = 2;
        let upload = if legacy {
            service.begin_selection(client, "begin").unwrap()
        } else {
            service.ordered_begin(client, 1).unwrap()
        };
        append(&mut service, client, upload);
        let selection = if legacy {
            service.commit_selection(client, upload).unwrap()
        } else {
            service.ordered_commit(client, upload, 2).unwrap()
        };
        let id = if legacy {
            service
                .send_selection(client, selection, "peer", "send")
                .unwrap()
        } else {
            service.ordered_send(client, selection, "peer", 3).unwrap()
        };
        service.cancel(client, id).unwrap();
        closed(&mut service);
        assert!(!service.maintain_history().unwrap());
        assert!(service.transfers.contains_key(&id));
        if legacy {
            assert_eq!(
                service.send_selection(client, selection, "peer", "send"),
                Ok(id)
            );
            service.disconnect(client).unwrap();
        } else {
            service.acknowledge_operations(client, 3).unwrap();
        }
        assert!(service.maintain_history().unwrap());
        assert!(!service.transfers.contains_key(&id));
        if !legacy {
            assert_eq!(
                service.ordered_send(client, selection, "peer", 3),
                Err(Error::RequestExpired)
            );
        }
    }
}

#[test]
fn given_desync_or_revision_exhaustion_when_cleanup_is_requested_then_records_are_preserved() {
    // Arrange / Act / Assert
    for exhausted in [false, true] {
        let (mut service, _) = setup();
        service.limits.transfers = 2;
        let first = closed(&mut service);
        closed(&mut service);
        if exhausted {
            service.revision = u64::MAX;
        } else {
            service.mark_desynchronized().unwrap();
        }
        assert_eq!(
            service.maintain_history(),
            if exhausted {
                Err(Error::Exhausted)
            } else {
                Ok(false)
            }
        );
        assert_eq!(service.transfers.len(), 2);
        assert!(service.transfers.contains_key(&first));
    }
}

#[test]
fn given_completion_order_different_from_ids_when_trimmed_then_terminal_revision_determines_age() {
    // Arrange
    let (mut service, _) = setup();
    service.limits.transfers = 2;
    let first = service.register_inbound().unwrap();
    let second = closed(&mut service);
    service.observe(first, Phase::Finished).unwrap();
    service.owner_closed(first).unwrap();
    // Act / Assert
    service.maintain_history().unwrap();
    assert!(service.transfers.contains_key(&first));
    assert!(!service.transfers.contains_key(&second));
}

#[test]
fn given_thousands_of_acknowledged_operations_when_queued_work_is_cancelled_then_memory_stays_bounded(
) {
    // Arrange
    let (mut service, client) = setup();
    let source = tempfile::NamedTempFile::new().unwrap();
    // Act
    for n in 0..1000u64 {
        let seq = n * 3 + 1;
        let upload = service.ordered_begin(client, seq).unwrap();
        service
            .append_files(
                client,
                upload,
                0,
                vec![crate::file_source::OpenedFile::adopt(
                    std::fs::File::open(source.path()).unwrap(),
                    "a".into(),
                )
                .unwrap()],
            )
            .unwrap();
        let selection = service.ordered_commit(client, upload, seq + 1).unwrap();
        let id = service
            .ordered_send(client, selection, "peer", seq + 2)
            .unwrap();
        service.cancel(client, id).unwrap();
        service.acknowledge_operations(client, seq + 2).unwrap();
        service.maintain_history().unwrap();
        // Assert every iteration rather than only the final state.
        assert!(service.uploads.is_empty());
        assert!(service.operations[&client].receipts.is_empty());
        assert_eq!(service.usage(), ResourceUsage::default());
        assert!(service.transfers.len() <= 64);
        assert!(source.path().is_file());
    }
    assert_eq!(service.transfers.len(), 64);
    assert_eq!(service.operation_state(client), Ok((3000, 3000)));
    assert_eq!(service.ordered_begin(client, 1), Err(Error::RequestExpired));
}
