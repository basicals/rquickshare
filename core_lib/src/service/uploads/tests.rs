use std::fs::File;

use super::*;

fn file(name: &str, size: u64) -> OpenedFile {
    let source = tempfile::NamedTempFile::new().unwrap();
    source.as_file().set_len(size).unwrap();
    OpenedFile::adopt(File::open(source.path()).unwrap(), name.into()).unwrap()
}

fn setup(limits: Limits) -> (Coordinator, ClientId) {
    let mut service = Coordinator::new(1, limits).unwrap();
    let client = service.connect().unwrap();
    (service, client)
}

#[test]
fn given_multiple_batches_when_committed_then_one_complete_selection_is_sendable() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    service.set_endpoints(vec!["recipient".into()]).unwrap();
    let id = service.begin_selection(client, "request").unwrap();
    // Act
    assert_eq!(
        service.append_files(client, id, 0, vec![file("folder/a", 3)]),
        Ok(1)
    );
    assert_eq!(
        service.append_files(client, id, 1, vec![file("folder/b", 7)]),
        Ok(2)
    );
    let before = service.snapshot(client).unwrap();
    let selected = service.commit_selection(client, id).unwrap();
    let after = service.snapshot(client).unwrap();
    let transfer = service
        .send_selection(client, selected, "recipient", "send")
        .unwrap();
    // Assert
    assert!(before.selections.is_empty());
    assert_eq!(before.resources, after.resources);
    assert_eq!(
        after.resources,
        ResourceUsage {
            selections: 1,
            reserved_handles: 4,
            selected_bytes: 10
        }
    );
    assert_eq!(after.selections, vec![selected]);
    let dispatched = service.next_send().unwrap().unwrap();
    assert_eq!(dispatched.transfer, transfer);
    assert_eq!(
        dispatched
            .selection
            .files()
            .iter()
            .map(OpenedFile::relative_name)
            .collect::<Vec<_>>(),
        ["folder/a", "folder/b"]
    );
    assert_eq!(service.commit_selection(client, id), Ok(selected));
    assert_eq!(service.begin_selection(client, "request"), Ok(id));
    assert!(service.snapshot(client).unwrap().selections.is_empty());
}

#[test]
fn given_rejected_batch_when_appended_then_sequence_manifest_and_budget_are_unchanged() {
    // Arrange / Act / Assert
    for names in [
        vec![],
        vec!["a"],
        vec!["a/child"],
        vec!["b", "b"],
        vec!["b", "b/child"],
        vec!["b"; 17],
    ] {
        let (mut service, client) = setup(Limits::default());
        let id = service.begin_selection(client, "request").unwrap();
        service
            .append_files(client, id, 0, vec![file("a", 1)])
            .unwrap();
        let before = service.snapshot(client).unwrap();
        assert_eq!(
            service.append_files(
                client,
                id,
                1,
                names.iter().map(|name| file(name, 2)).collect()
            ),
            Err(Error::InvalidInput)
        );
        assert_eq!(service.snapshot(client).unwrap(), before);
        assert_eq!(service.get_upload(client, id).unwrap().next_sequence, 1);
        service
            .append_files(client, id, 1, vec![file("good", 2)])
            .unwrap();
        let selected = service.commit_selection(client, id).unwrap();
        assert_eq!(service.selections[&selected].resource.files().len(), 2);
    }
}

#[test]
fn given_stale_or_future_sequence_when_appended_then_replay_conflicts_without_duplication() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = service.begin_selection(client, "request").unwrap();
    service
        .append_files(client, id, 0, vec![file("a", 1)])
        .unwrap();
    let before = service.snapshot(client).unwrap();
    // Act / Assert
    for sequence in [0, 2, u32::MAX] {
        assert_eq!(
            service.append_files(client, id, sequence, vec![file("b", 2)]),
            Err(Error::ReplayConflict)
        );
        assert_eq!(service.snapshot(client).unwrap(), before);
    }
    assert_eq!(
        service.get_upload(client, id),
        Ok(UploadSnapshot {
            next_sequence: 1,
            state: UploadState::Preparing
        })
    );
}

#[test]
fn given_empty_or_closed_upload_when_used_then_invalid_state_is_explicit() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = service.begin_selection(client, "request").unwrap();
    // Act / Assert
    assert_eq!(
        service.commit_selection(client, id),
        Err(Error::InvalidInput)
    );
    service.abort_selection(client, id).unwrap();
    assert_eq!(service.begin_selection(client, "request"), Ok(id));
    assert_eq!(service.abort_selection(client, id), Ok(()));
    assert_eq!(
        service.get_upload(client, id).unwrap().state,
        UploadState::Aborted
    );
    assert_eq!(
        service.commit_selection(client, id),
        Err(Error::InvalidState)
    );
    assert_eq!(
        service.append_files(client, id, 0, vec![file("a", 1)]),
        Err(Error::InvalidState)
    );
    let id = service.begin_selection(client, "other").unwrap();
    service
        .append_files(client, id, 0, vec![file("a", 1)])
        .unwrap();
    let selected = service.commit_selection(client, id).unwrap();
    assert_eq!(
        service.abort_selection(client, id),
        Err(Error::InvalidState)
    );
    assert_eq!(
        service.append_files(client, id, 1, vec![file("b", 2)]),
        Err(Error::InvalidState)
    );
    service.discard_selection(client, selected).unwrap();
    assert_eq!(service.commit_selection(client, id), Ok(selected));
    assert!(service.snapshot(client).unwrap().selections.is_empty());
}

#[test]
fn given_foreign_unknown_or_disconnected_client_when_upload_accessed_then_authority_is_enforced() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let other = service.connect().unwrap();
    let id = service.begin_selection(client, "request").unwrap();
    // Act / Assert
    assert_eq!(service.get_upload(other, id), Err(Error::WrongOwner));
    assert_eq!(
        service.append_files(other, id, 0, vec![file("a", 1)]),
        Err(Error::WrongOwner)
    );
    assert_eq!(service.commit_selection(other, id), Err(Error::WrongOwner));
    assert_eq!(service.abort_selection(other, id), Err(Error::WrongOwner));
    for unknown in [UploadId(1, 999), UploadId(2, 1)] {
        assert_eq!(
            service.get_upload(client, unknown),
            Err(Error::UnknownUpload)
        );
        assert_eq!(
            service.commit_selection(client, unknown),
            Err(Error::UnknownUpload)
        );
        assert_eq!(
            service.append_files(client, unknown, 0, vec![file("a", 1)]),
            Err(Error::UnknownUpload)
        );
        assert_eq!(service.abort_selection(client, unknown), Ok(()));
    }
    service.disconnect(client).unwrap();
    assert_eq!(service.get_upload(client, id), Err(Error::UnknownClient));
    assert_eq!(
        service.begin_selection(client, "request"),
        Err(Error::UnknownClient)
    );
    assert_eq!(
        service.abort_selection(client, id),
        Err(Error::UnknownClient)
    );
}

#[test]
fn given_invalid_request_when_begun_then_no_reservation_is_created() {
    // Arrange / Act / Assert
    let (mut service, client) = setup(Limits::default());
    for request in [
        "".into(),
        "has space".into(),
        "\n".into(),
        "é".into(),
        "x".repeat(129),
    ] {
        assert_eq!(
            service.begin_selection(client, &request),
            Err(Error::InvalidInput)
        );
        assert_eq!(
            service.snapshot(client).unwrap().resources,
            ResourceUsage::default()
        );
    }
    assert!(service.begin_selection(client, &"x".repeat(128)).is_ok());
}

#[test]
fn given_preparation_or_receipt_cap_when_begun_then_capacity_is_bounded_and_replay_still_works() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let ids: Vec<_> = (0..PREPARATIONS_PER_CLIENT)
        .map(|n| {
            service
                .begin_selection(client, &format!("request-{n}"))
                .unwrap()
        })
        .collect();
    // Act / Assert
    assert_eq!(
        service.begin_selection(client, "overflow"),
        Err(Error::Capacity)
    );
    assert_eq!(service.begin_selection(client, "request-0"), Ok(ids[0]));
    for id in ids {
        service.abort_selection(client, id).unwrap();
    }
    for n in PREPARATIONS_PER_CLIENT..RECEIPTS_PER_CLIENT {
        let id = service
            .begin_selection(client, &format!("request-{n}"))
            .unwrap();
        service.abort_selection(client, id).unwrap();
    }
    assert_eq!(
        service.begin_selection(client, "overflow"),
        Err(Error::Capacity)
    );
    assert!(service.begin_selection(client, "request-0").is_ok());
    assert_eq!(
        service.snapshot(client).unwrap().resources,
        ResourceUsage::default()
    );
}

#[test]
fn given_staged_and_committed_files_when_admitted_then_the_same_handle_and_byte_budgets_apply() {
    // Arrange / Act / Assert
    for limits in [
        Limits {
            reserved_handles: 4,
            ..Limits::default()
        },
        Limits {
            selected_bytes: 2,
            ..Limits::default()
        },
    ] {
        let (mut service, client) = setup(limits);
        let first = service.begin_selection(client, "first").unwrap();
        let second = service.begin_selection(client, "second").unwrap();
        service
            .append_files(client, first, 0, vec![file("a", 1)])
            .unwrap();
        service
            .append_files(client, second, 0, vec![file("b", 1)])
            .unwrap();
        let before = service.snapshot(client).unwrap().resources;
        assert_eq!(
            service.append_files(client, second, 1, vec![file("c", 1)]),
            Err(Error::Capacity)
        );
        assert_eq!(
            service.create_selection(
                client,
                OpenedSelection::from_files(vec![file("d", 1)], SelectionLimits::default())
                    .unwrap()
            ),
            Err(Error::Capacity)
        );
        service.commit_selection(client, first).unwrap();
        assert_eq!(service.snapshot(client).unwrap().resources, before);
        assert_eq!(
            service.append_files(client, second, 1, vec![file("c", 1)]),
            Err(Error::Capacity)
        );
        service.abort_selection(client, second).unwrap();
        assert!(service
            .create_selection(
                client,
                OpenedSelection::from_files(vec![file("d", 1)], SelectionLimits::default())
                    .unwrap()
            )
            .is_ok());
    }
}

#[test]
fn given_empty_preparations_when_selection_limit_reached_then_direct_admission_is_also_bounded() {
    // Arrange
    let (mut service, client) = setup(Limits {
        selections: 1,
        ..Limits::default()
    });
    let id = service.begin_selection(client, "first").unwrap();
    // Act / Assert
    assert_eq!(
        service.begin_selection(client, "second"),
        Err(Error::Capacity)
    );
    assert_eq!(
        service.create_selection(
            client,
            OpenedSelection::from_files(vec![file("a", 1)], SelectionLimits::default()).unwrap()
        ),
        Err(Error::Capacity)
    );
    service.abort_selection(client, id).unwrap();
    assert!(service.begin_selection(client, "second").is_ok());
}

#[test]
fn given_complete_selection_boundaries_when_appended_then_only_valid_manifests_commit() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = service.begin_selection(client, "request").unwrap();
    for batch in 0..16 {
        service
            .append_files(
                client,
                id,
                batch,
                (0..16)
                    .map(|n| file(&format!("f-{}", batch * 16 + n), 0))
                    .collect(),
            )
            .unwrap();
    }
    // Act / Assert
    assert_eq!(
        service.append_files(client, id, 16, vec![file("extra", 0)]),
        Err(Error::InvalidInput)
    );
    let selected = service.commit_selection(client, id).unwrap();
    assert_eq!(service.selections[&selected].resource.files().len(), 256);
    let id = service.begin_selection(client, "deep").unwrap();
    assert_eq!(
        service.append_files(
            client,
            id,
            0,
            vec![file(&format!("{}a", "d/".repeat(65)), 0)]
        ),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        service.append_files(
            client,
            id,
            0,
            vec![file(&format!("{}a", "d/".repeat(64)), 1 << 40)]
        ),
        Ok(1)
    );
    assert_eq!(
        service.append_files(client, id, 1, vec![file("extra", 1)]),
        Err(Error::InvalidInput)
    );
}

#[test]
fn given_exhausted_revision_or_sequence_when_mutated_then_preparation_is_preserved() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = service.begin_selection(client, "request").unwrap();
    service
        .append_files(client, id, 0, vec![file("a", 1)])
        .unwrap();
    service.revision = u64::MAX;
    // Act / Assert
    assert_eq!(service.commit_selection(client, id), Err(Error::Exhausted));
    assert_eq!(service.abort_selection(client, id), Err(Error::Exhausted));
    assert_eq!(
        service.append_files(client, id, 1, vec![file("b", 1)]),
        Err(Error::Exhausted)
    );
    assert_eq!(
        service.get_upload(client, id).unwrap().state,
        UploadState::Preparing
    );
    assert_eq!(service.uploads[&id].files.len(), 1);
    service.revision = 9;
    service.uploads.get_mut(&id).unwrap().next_sequence = u32::MAX;
    assert_eq!(
        service.append_files(client, id, u32::MAX, vec![file("b", 1)]),
        Err(Error::Exhausted)
    );
    assert_eq!(service.uploads[&id].files.len(), 1);
}

#[test]
fn given_source_handles_when_aborted_disconnected_or_stopped_then_handles_close_without_unlinking()
{
    // Arrange / Act / Assert
    for operation in ["abort", "disconnect", "shutdown"] {
        let source = tempfile::NamedTempFile::new().unwrap();
        let count = || {
            std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(Result::ok)
                .filter_map(|entry| std::fs::read_link(entry.path()).ok())
                .filter(|path| path == source.path())
                .count()
        };
        let (mut service, client) = setup(Limits::default());
        let id = service.begin_selection(client, "request").unwrap();
        service
            .append_files(
                client,
                id,
                0,
                vec![OpenedFile::adopt(File::open(source.path()).unwrap(), "a".into()).unwrap()],
            )
            .unwrap();
        assert_eq!(count(), 2);
        match operation {
            "abort" => service.abort_selection(client, id).unwrap(),
            "disconnect" => service.disconnect(client).unwrap(),
            _ => {
                service.begin_shutdown().unwrap();
                service.finish_shutdown().unwrap();
            }
        }
        assert_eq!(count(), 1);
        assert!(source.path().is_file());
        assert_eq!(service.usage(), ResourceUsage::default());
    }
}

#[test]
fn given_shutdown_when_new_or_staged_mutation_attempted_then_admission_stays_closed() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let id = service.begin_selection(client, "request").unwrap();
    service.begin_shutdown().unwrap();
    // Act / Assert
    assert_eq!(
        service.begin_selection(client, "next"),
        Err(Error::NotRunning)
    );
    assert_eq!(
        service.append_files(client, id, 0, vec![file("a", 1)]),
        Err(Error::NotRunning)
    );
    assert_eq!(service.commit_selection(client, id), Err(Error::NotRunning));
    assert!(service.uploads.is_empty());
}
