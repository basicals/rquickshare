use std::{fs::File, io::Write};

use super::*;
use crate::file_source::{OpenedFile, OpenedSelection, SelectionLimits};

fn fixture(limits: Limits) -> (Coordinator, ClientId, SelectionId, tempfile::NamedTempFile) {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"abc").unwrap();
    let source = OpenedSelection::from_files(
        vec![OpenedFile::adopt(File::open(file.path()).unwrap(), "source.bin".into()).unwrap()],
        SelectionLimits::default(),
    )
    .unwrap();
    let mut core = Coordinator::new(123, limits).unwrap();
    core.set_endpoints((0..10).map(|n| format!("peer{n}")).collect())
        .unwrap();
    let client = core.connect().unwrap();
    let selected = core.create_selection(client, source).unwrap();
    (core, client, selected, file)
}
fn recipients(count: usize) -> Vec<(String, String)> {
    (0..count)
        .map(|n| (format!("peer{n}"), format!("Device {n}")))
        .collect()
}

#[test]
fn given_invalid_batch_when_admitted_then_every_validation_rejects_before_any_member_or_source_changes(
) {
    for case in [
        "empty",
        "too-many",
        "duplicate",
        "invalid",
        "long-id",
        "control-name",
        "long-name",
        "unknown",
        "owner",
        "epoch",
        "stopped",
        "unsynchronized",
        "queue",
        "transfers",
        "revision",
        "handles",
    ] {
        // Arrange
        let (mut core, mut client, selected, _file) = fixture(Limits::default());
        let mut peers = recipients(2);
        let expected = match case {
            "empty" => {
                peers.clear();
                Error::InvalidInput
            }
            "too-many" => {
                peers = recipients(11);
                Error::InvalidInput
            }
            "duplicate" => {
                peers[1].0 = peers[0].0.clone();
                Error::InvalidInput
            }
            "invalid" => {
                peers[0].0 = "bad id".into();
                Error::InvalidInput
            }
            "long-id" => {
                peers[0].0 = "a".repeat(257);
                Error::InvalidInput
            }
            "control-name" => {
                peers[0].1 = "private\nname".into();
                Error::InvalidInput
            }
            "long-name" => {
                peers[0].1 = "a".repeat(257);
                Error::InvalidInput
            }
            "unknown" => {
                peers[1].0 = "missing".into();
                Error::UnknownEndpoint
            }
            "owner" => {
                client = core.connect().unwrap();
                Error::WrongOwner
            }
            "epoch" => {
                client = ClientId(124, 1);
                Error::UnknownClient
            }
            "stopped" => {
                core.lifecycle = Lifecycle::Stopped;
                Error::NotRunning
            }
            "unsynchronized" => {
                core.mark_desynchronized().unwrap();
                Error::Unsynchronized
            }
            "queue" => {
                core.limits.queued_sends = 1;
                Error::Capacity
            }
            "transfers" => {
                core.limits.transfers = 1;
                Error::Capacity
            }
            "handles" => {
                core.limits.reserved_handles = 2;
                Error::Capacity
            }
            _ => {
                core.revision = u64::MAX - 1;
                Error::Exhausted
            }
        };
        let revision = core.revision;
        let usage = core.usage();
        let operations = core
            .operations
            .values()
            .map(|ledger| ledger.receipts.len())
            .sum::<usize>();
        // Act / Assert
        assert_eq!(
            core.ordered_send_many(client, selected, peers, 1),
            Err(expected),
            "{case}"
        );
        assert_eq!(core.revision, revision, "{case}");
        assert_eq!(core.usage(), usage);
        assert!(core.selections.contains_key(&selected));
        assert!(core.transfers.is_empty());
        assert!(core.queue.is_empty());
        assert_eq!(
            core.operations
                .values()
                .map(|ledger| ledger.receipts.len())
                .sum::<usize>(),
            operations
        );
    }
}

#[test]
fn given_one_or_ten_recipients_when_dispatched_then_source_is_shared_once_and_independent_reads_release_only_after_last_reader(
) {
    for count in [1, 10] {
        // Arrange
        let (mut core, client, selected, _file) = fixture(Limits::default());
        let mut before = core.usage();
        before.reserved_handles = count + 1;
        // Act
        let ids = core
            .ordered_send_many(client, selected, recipients(count), 1)
            .unwrap();
        // Assert
        assert_eq!(ids.len(), count);
        assert!(ids.windows(2).all(|pair| pair[0].1 < pair[1].1));
        assert!(!core.selections.contains_key(&selected));
        assert_eq!(core.usage(), before);
        let mut jobs = Vec::new();
        for (index, id) in ids.iter().enumerate() {
            assert_eq!(
                core.transfers[id].recipient.as_deref(),
                Some(format!("Device {index}").as_str())
            );
            assert_eq!(
                core.transfers[id].associated_endpoint.as_deref(),
                Some(format!("peer{index}").as_str())
            );
            let job = core.next_send().unwrap().unwrap();
            assert_eq!(job.transfer, *id);
            assert_eq!(job.endpoint, format!("peer{index}"));
            assert!(core.transfers[id].endpoint.is_none());
            assert_eq!(
                core.transfers[id].associated_endpoint.as_deref(),
                Some(job.endpoint.as_str())
            );
            jobs.push(job);
        }
        assert!(core.next_send().unwrap().is_none());
        for job in &jobs {
            assert!(Arc::ptr_eq(&job.selection, &jobs[0].selection));
            let file = job.selection.files()[0].clone_handle().unwrap();
            let mut first = [0];
            let mut second = [0];
            crate::file_reader::read_file_chunk(&file, 3, 1, &mut second).unwrap();
            crate::file_reader::read_file_chunk(&file, 3, 0, &mut first).unwrap();
            assert_eq!((first, second), (*b"a", *b"b"));
        }
        while jobs.len() > 1 {
            jobs.pop();
            assert_eq!(core.snapshot(client).unwrap().resources, before);
        }
        jobs.clear();
        assert_eq!(
            core.snapshot(client).unwrap().resources,
            ResourceUsage::default()
        );
    }
}

#[test]
fn given_batch_receipt_when_replayed_altered_or_retired_then_one_ordered_intent_cannot_start_extra_sends(
) {
    // Arrange
    let (mut core, client, selected, _file) = fixture(Limits::default());
    let ids = core
        .ordered_send_many(client, selected, recipients(2), 1)
        .unwrap();
    core.set_endpoints(vec![]).unwrap();
    let before = core.snapshot(client).unwrap();
    let mut renamed = recipients(2);
    renamed[0].1 = "Renamed".into();
    // Act / Assert
    assert_eq!(
        core.ordered_send_many(client, selected, renamed, 1),
        Ok(ids.clone())
    );
    let mut changed = recipients(2);
    changed.reverse();
    assert_eq!(
        core.ordered_send_many(client, selected, changed, 1),
        Err(Error::ReplayConflict)
    );
    assert_eq!(
        core.ordered_send_many(client, SelectionId(123, 999), recipients(2), 1),
        Err(Error::ReplayConflict)
    );
    assert_eq!(core.snapshot(client).unwrap(), before);
    assert_eq!(
        core.operation_receipt(client, 1).unwrap().result,
        operations::OperationResult::Transfers(ids)
    );
    core.acknowledge_operations(client, 1).unwrap();
    assert_eq!(
        core.ordered_send_many(client, selected, recipients(2), 1),
        Err(Error::RequestExpired)
    );
    assert_eq!(core.queue.len(), 2);
}

#[test]
fn given_admitted_batch_when_client_disconnects_or_one_member_is_cancelled_then_other_members_keep_daemon_ownership(
) {
    // Arrange
    let (mut core, client, selected, _file) = fixture(Limits::default());
    let mut usage = core.usage();
    usage.reserved_handles = 3;
    let ids = core
        .ordered_send_many(client, selected, recipients(2), 1)
        .unwrap();
    // Act / Assert
    core.cancel(client, ids[0]).unwrap();
    assert_eq!(core.transfers[&ids[0]].view.phase, Phase::Cancelled);
    assert_eq!(core.transfers[&ids[1]].view.phase, Phase::Queued);
    assert_eq!(core.usage(), usage);
    core.disconnect(client).unwrap();
    assert_eq!(core.queue.len(), 1);
    let observer = core.connect().unwrap();
    assert_eq!(core.snapshot(observer).unwrap().resources, usage);
    let job = core.next_send().unwrap().unwrap();
    assert_eq!(job.transfer, ids[1]);
    assert_eq!(
        core.transfers[&ids[1]].recipient.as_deref(),
        Some("Device 1")
    );
    drop(job);
    assert_eq!(
        core.snapshot(observer).unwrap().resources,
        ResourceUsage::default()
    );
}

#[test]
fn given_terminal_batch_members_when_receipt_is_unacknowledged_then_each_member_is_pinned_until_one_shared_ack(
) {
    // Arrange
    let (mut core, client, selected, _file) = fixture(Limits::default());
    let mut peers = recipients(2);
    peers[0].1.clear();
    let ids = core.ordered_send_many(client, selected, peers, 1).unwrap();
    assert_eq!(
        core.transfers[&ids[0]].recipient.as_deref(),
        Some("Unnamed nearby device")
    );
    // Act / Assert
    for id in &ids {
        core.cancel(client, *id).unwrap();
        assert!(core.operation_pins(*id));
    }
    core.acknowledge_operations(client, 1).unwrap();
    for id in ids {
        assert!(!core.operation_pins(id));
        assert_eq!(core.transfers[&id].view.phase, Phase::Cancelled);
    }
}
