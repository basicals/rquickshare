use std::fs::File;

use super::*;
use crate::file_source::OpenedFile;

pub(crate) fn setup() -> (Coordinator, ClientId) {
    let mut service = Coordinator::new(123, Limits::default()).unwrap();
    service
        .set_endpoints(vec!["peer".into(), "other".into()])
        .unwrap();
    let client = service.connect().unwrap();
    (service, client)
}
pub(crate) fn append(service: &mut Coordinator, client: ClientId, upload: UploadId) {
    let source = tempfile::NamedTempFile::new().unwrap();
    service
        .append_files(
            client,
            upload,
            0,
            vec![OpenedFile::adopt(File::open(source.path()).unwrap(), "a".into()).unwrap()],
        )
        .unwrap();
}

#[test]
fn given_all_operation_kinds_when_retried_then_exact_receipts_replay_and_conflicts_do_not_mutate() {
    // Arrange
    let (mut service, client) = setup();
    let upload = service.ordered_begin(client, 1).unwrap();
    append(&mut service, client, upload);
    let selection = service.ordered_commit(client, upload, 2).unwrap();
    let transfer = service.ordered_send(client, selection, "peer", 3).unwrap();
    let aborted = service.ordered_begin(client, 4).unwrap();
    service.ordered_abort(client, aborted, 5).unwrap();
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    assert_eq!(service.ordered_begin(client, 1), Ok(upload));
    assert_eq!(service.ordered_commit(client, upload, 2), Ok(selection));
    assert_eq!(
        service.ordered_send(client, selection, "peer", 3),
        Ok(transfer)
    );
    assert_eq!(service.ordered_abort(client, aborted, 5), Ok(()));
    for (sequence, kind, result) in [
        (1, "begin_selection", OperationResult::Upload(upload)),
        (2, "commit_selection", OperationResult::Selection(selection)),
        (3, "send_selection", OperationResult::Transfer(transfer)),
        (5, "abort_selection", OperationResult::Empty),
    ] {
        assert_eq!(
            service.operation_receipt(client, sequence),
            Ok(OperationReceipt { kind, result })
        );
    }
    assert_eq!(
        service.ordered_abort(client, upload, 1),
        Err(Error::ReplayConflict)
    );
    assert_eq!(
        service.ordered_commit(client, aborted, 2),
        Err(Error::ReplayConflict)
    );
    assert_eq!(
        service.ordered_send(client, selection, "other", 3),
        Err(Error::ReplayConflict)
    );
    assert_eq!(
        service.ordered_send(client, SelectionId(123, 999), "peer", 3),
        Err(Error::ReplayConflict)
    );
    assert_eq!(
        service.ordered_abort(client, upload, 5),
        Err(Error::ReplayConflict)
    );
    assert_eq!(service.snapshot(client).unwrap(), before);
}

#[test]
fn given_sequence_boundaries_when_requested_then_zero_gaps_and_future_reads_are_rejected() {
    // Arrange
    let (mut service, client) = setup();
    // Act / Assert
    for sequence in [0, 2, u64::MAX] {
        assert_eq!(
            service.ordered_begin(client, sequence),
            Err(Error::InvalidInput)
        );
        assert_eq!(
            service.operation_receipt(client, sequence),
            Err(Error::InvalidInput)
        );
    }
    assert_eq!(service.operation_state(client), Ok((0, 0)));
    service.ordered_begin(client, 1).unwrap();
    assert_eq!(service.ordered_begin(client, 3), Err(Error::InvalidInput));
    assert_eq!(
        service.operation_receipt(client, 2),
        Err(Error::InvalidInput)
    );
    assert_eq!(service.operation_state(client), Ok((1, 0)));
}

#[test]
fn given_cumulative_ack_when_repeated_or_expired_then_watermark_prevents_reexecution() {
    // Arrange
    let (mut service, client) = setup();
    let upload = service.ordered_begin(client, 1).unwrap();
    service.ordered_abort(client, upload, 2).unwrap();
    // Act / Assert
    assert_eq!(
        service.acknowledge_operations(client, 3),
        Err(Error::InvalidInput)
    );
    service.acknowledge_operations(client, 0).unwrap();
    service.acknowledge_operations(client, 1).unwrap();
    assert!(service.uploads.contains_key(&upload));
    service.acknowledge_operations(client, 2).unwrap();
    assert!(!service.uploads.contains_key(&upload));
    let before = service.snapshot(client).unwrap();
    for sequence in [0, 1, 2] {
        service.acknowledge_operations(client, sequence).unwrap();
    }
    for sequence in [1, 2] {
        assert_eq!(
            service.ordered_begin(client, sequence),
            Err(Error::RequestExpired)
        );
        assert_eq!(
            service.operation_receipt(client, sequence),
            Err(Error::RequestExpired)
        );
    }
    assert_eq!(service.snapshot(client).unwrap(), before);
    assert_eq!(service.operation_state(client), Ok((2, 2)));
}

#[test]
fn given_full_unacknowledged_window_when_admitted_then_capacity_precedes_mutation_and_ack_reopens_it(
) {
    // Arrange
    let (mut service, client) = setup();
    for pair in 0..RECEIPTS / 2 {
        let upload = service.ordered_begin(client, pair as u64 * 2 + 1).unwrap();
        service
            .ordered_abort(client, upload, pair as u64 * 2 + 2)
            .unwrap();
    }
    // Act / Assert
    assert_eq!(service.operations[&client].receipts.len(), 128);
    let before = service.snapshot(client).unwrap();
    assert_eq!(service.ordered_begin(client, 129), Err(Error::Capacity));
    assert_eq!(service.snapshot(client).unwrap(), before);
    let OperationResult::Upload(original) = service.operation_receipt(client, 1).unwrap().result
    else {
        panic!("Expected begin receipt");
    };
    assert_eq!(service.ordered_begin(client, 1), Ok(original));
    service.acknowledge_operations(client, 128).unwrap();
    assert!(service.uploads.is_empty());
    assert!(service.ordered_begin(client, 129).is_ok());
}

#[test]
fn given_failed_admission_when_same_sequence_is_corrected_then_it_remains_available() {
    // Arrange
    let (mut service, client) = setup();
    let upload = service.ordered_begin(client, 1).unwrap();
    // Act / Assert
    assert_eq!(
        service.ordered_commit(client, upload, 2),
        Err(Error::InvalidInput)
    );
    assert_eq!(service.operation_state(client), Ok((1, 0)));
    append(&mut service, client, upload);
    let selection = service.ordered_commit(client, upload, 2).unwrap();
    assert_eq!(
        service.ordered_send(client, selection, "missing", 3),
        Err(Error::UnknownEndpoint)
    );
    assert!(service.selections.contains_key(&selection));
    assert_eq!(service.operation_state(client), Ok((2, 0)));
    assert!(service.ordered_send(client, selection, "peer", 3).is_ok());
}

#[test]
fn given_acknowledged_begin_when_commit_reply_is_lost_then_receipt_keeps_original_selection() {
    // Arrange
    let (mut service, client) = setup();
    let upload = service.ordered_begin(client, 1).unwrap();
    service.acknowledge_operations(client, 1).unwrap();
    assert_eq!(service.uploads[&upload].state, UploadState::Preparing);
    append(&mut service, client, upload);
    let selection = service.ordered_commit(client, upload, 2).unwrap();
    // Act / Assert
    assert_eq!(service.ordered_commit(client, upload, 2), Ok(selection));
    assert!(service.uploads.contains_key(&upload));
    service.acknowledge_operations(client, 2).unwrap();
    assert!(!service.uploads.contains_key(&upload));
    assert!(service.selections.contains_key(&selection));
    assert_eq!(
        service.ordered_commit(client, upload, 2),
        Err(Error::RequestExpired)
    );
    assert_eq!(
        service.ordered_commit(client, upload, 3),
        Err(Error::UnknownUpload)
    );
}

#[test]
fn given_foreign_or_mixed_origins_when_mutated_then_ownership_and_ordered_receipts_cannot_be_bypassed(
) {
    // Arrange
    let (mut service, client) = setup();
    let foreign = service.connect().unwrap();
    let ordered = service.ordered_begin(client, 1).unwrap();
    let legacy = service.begin_selection(client, "1").unwrap();
    append(&mut service, client, ordered);
    // Act / Assert
    assert_eq!(
        service.ordered_commit(foreign, ordered, 1),
        Err(Error::WrongOwner)
    );
    assert_eq!(
        service.ordered_abort(foreign, ordered, 1),
        Err(Error::WrongOwner)
    );
    assert_eq!(service.operation_state(foreign), Ok((0, 0)));
    assert_eq!(
        service.commit_selection(client, ordered),
        Err(Error::InvalidState)
    );
    assert_eq!(
        service.abort_selection(client, ordered),
        Err(Error::InvalidState)
    );
    assert_eq!(
        service.ordered_commit(client, legacy, 2),
        Err(Error::InvalidState)
    );
    assert_eq!(
        service.ordered_abort(client, legacy, 2),
        Err(Error::InvalidState)
    );
    assert_ne!(ordered, legacy);
    service.disconnect(client).unwrap();
    assert_eq!(service.operation_state(client), Err(Error::UnknownClient));
    assert!(service.uploads.is_empty());
}

#[test]
fn given_exhausted_counters_when_mutated_then_ledger_and_preparation_remain_intact() {
    // Arrange
    let (mut service, client) = setup();
    let upload = service.ordered_begin(client, 1).unwrap();
    append(&mut service, client, upload);
    service.revision = u64::MAX;
    // Act / Assert
    assert_eq!(
        service.ordered_commit(client, upload, 2),
        Err(Error::Exhausted)
    );
    assert_eq!(
        service.acknowledge_operations(client, 1),
        Err(Error::Exhausted)
    );
    assert_eq!(service.operation_state(client), Ok((1, 0)));
    assert_eq!(
        service.get_upload(client, upload).unwrap().state,
        UploadState::Preparing
    );
    service.revision = 20;
    let ledger = service.operations.get_mut(&client).unwrap();
    ledger.last = u64::MAX - 1;
    ledger.acknowledged = u64::MAX - 1;
    ledger.receipts.clear();
    let final_upload = service.ordered_begin(client, u64::MAX).unwrap();
    assert_eq!(service.ordered_begin(client, u64::MAX), Ok(final_upload));
    service.acknowledge_operations(client, u64::MAX).unwrap();
    assert_eq!(
        service.ordered_begin(client, u64::MAX),
        Err(Error::RequestExpired)
    );
    assert_eq!(service.operation_state(client), Ok((u64::MAX, u64::MAX)));
}
