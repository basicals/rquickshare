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

#[test]
fn given_staged_text_when_chunks_retry_or_mutate_then_exact_complete_utf8_commits_once_with_shared_resource_accounting(
) {
    // Arrange / Act / Assert
    for body in [
        "x".to_owned(),
        "é".repeat(8193),
        "a".repeat(crate::sessions::text::MAX_TEXT_BYTES),
    ] {
        let mut service = Coordinator::new(123, Limits::default()).unwrap();
        let client = service.connect().unwrap();
        let other = service.connect().unwrap();
        service.set_endpoints(vec!["peer".into()]).unwrap();
        let upload = service
            .ordered_begin_text(client, body.len() as u32, 1)
            .unwrap();
        assert_eq!(
            service
                .ordered_begin_text(client, body.len() as u32, 1)
                .unwrap(),
            upload
        );
        assert_eq!(
            service.ordered_begin_text(client, body.len() as u32 + 1, 1),
            Err(Error::ReplayConflict)
        );
        assert_eq!(
            service.snapshot(client).unwrap().resources.selected_bytes,
            body.len() as u64
        );
        assert_eq!(
            service.append_text(other, upload, 0, b"x"),
            Err(Error::WrongOwner)
        );
        assert_eq!(
            service.append_files(client, upload, 0, vec![]),
            Err(Error::InvalidState)
        );
        assert_eq!(
            service.ordered_commit(client, upload, 2),
            Err(Error::InvalidInput)
        );
        for (sequence, chunk) in body.as_bytes().chunks(TEXT_CHUNK).enumerate() {
            let seq = sequence as u32;
            let before = service.revision;
            assert_eq!(service.append_text(client, upload, seq, chunk), Ok(seq + 1));
            let accepted = service.revision;
            assert!(accepted > before);
            assert_eq!(service.append_text(client, upload, seq, chunk), Ok(seq + 1));
            assert_eq!(service.revision, accepted);
            let mut changed = chunk.to_vec();
            changed[0] ^= 1;
            assert_eq!(
                service.append_text(client, upload, seq, &changed),
                Err(Error::ReplayConflict)
            );
        }
        let selected = service.ordered_commit(client, upload, 2).unwrap();
        assert_eq!(service.ordered_commit(client, upload, 2).unwrap(), selected);
        assert_eq!(
            service.selections[&selected]
                .resource
                .text()
                .unwrap()
                .text(),
            body
        );
        assert!(service.selections[&selected].resource.files().is_empty());
        assert_eq!(
            service.append_text(client, upload, 0, b"x"),
            Err(Error::InvalidState)
        );
        service.acknowledge_operations(client, 2).unwrap();
        let transfer = service.ordered_send(client, selected, "peer", 3).unwrap();
        assert_eq!(
            service.ordered_send(client, selected, "peer", 3).unwrap(),
            transfer
        );
        let dispatched = service.next_send().unwrap().unwrap();
        assert_eq!(dispatched.selection.text().unwrap().text(), body);
        service.disconnect(client).unwrap();
        service.observe(transfer, Phase::Finished).unwrap();
        service.owner_closed(transfer).unwrap();
        service.source_released(transfer).unwrap();
        assert_eq!(
            service.snapshot(other).unwrap().resources.selected_bytes,
            body.len() as u64
        );
        drop(dispatched);
        assert_eq!(
            service.snapshot(other).unwrap().resources,
            ResourceUsage::default()
        );
    }
}
#[test]
fn given_invalid_text_preparation_when_admitted_or_committed_then_no_partial_source_or_unbounded_allocation_is_accepted(
) {
    // Arrange / Act / Assert
    for size in [0, 131073, u32::MAX] {
        let mut service = Coordinator::new(123, Limits::default()).unwrap();
        let client = service.connect().unwrap();
        let revision = service.revision;
        assert_eq!(
            service.ordered_begin_text(client, size, 1),
            Err(Error::InvalidInput)
        );
        assert_eq!(service.revision, revision);
        assert!(service.uploads.is_empty());
    }
    for (declared, chunks, valid) in [
        (1, vec![vec![255]], false),
        (3, vec![b"a\0c".to_vec()], false),
        (3, vec![b"ab".to_vec()], false),
        (2, vec![vec![0xc3], vec![0xa9]], true),
    ] {
        let mut service = Coordinator::new(123, Limits::default()).unwrap();
        let client = service.connect().unwrap();
        let upload = service.ordered_begin_text(client, declared, 1).unwrap();
        for (seq, chunk) in chunks.iter().enumerate() {
            service
                .append_text(client, upload, seq as u32, chunk)
                .unwrap();
        }
        assert_eq!(service.ordered_commit(client, upload, 2).is_ok(), valid);
        if !valid {
            service.ordered_abort(client, upload, 2).unwrap();
        }
        service.disconnect(client).unwrap();
        assert_eq!(service.usage(), ResourceUsage::default());
    }
    let mut service = Coordinator::new(123, Limits::default()).unwrap();
    let client = service.connect().unwrap();
    let upload = service.ordered_begin_text(client, 1, 1).unwrap();
    for bytes in [vec![], vec![1; TEXT_CHUNK + 1], vec![1; 2]] {
        assert_eq!(
            service.append_text(client, upload, 0, &bytes),
            Err(Error::InvalidInput)
        );
        assert_eq!(service.get_upload(client, upload).unwrap().next_sequence, 0);
    }
    assert_eq!(
        service.append_text(client, upload, 1, b"x"),
        Err(Error::ReplayConflict)
    );
    assert_eq!(
        service.append_text(client, upload, u32::MAX, b"x"),
        Err(Error::ReplayConflict)
    );
    let files = service.ordered_begin(client, 2).unwrap();
    assert_eq!(
        service.append_text(client, files, 0, b"x"),
        Err(Error::InvalidState)
    );
    service.ordered_abort(client, upload, 3).unwrap();
    assert_eq!(
        service.append_text(client, upload, 0, b"x"),
        Err(Error::InvalidState)
    );
}
#[test]
fn given_global_text_memory_budget_when_reserved_before_body_then_exact_cap_and_release_are_enforced(
) {
    // Arrange
    let mut service = Coordinator::new(
        123,
        Limits {
            selections: 129,
            ..Limits::default()
        },
    )
    .unwrap();
    let mut held = Vec::new();
    // Act / Assert: 32 clients *4 preparations *128KiB =16MiB, even with no body appended.
    for _ in 0..32 {
        let client = service.connect().unwrap();
        for seq in 1..=4 {
            held.push((
                client,
                service.ordered_begin_text(client, 131072, seq).unwrap(),
            ));
        }
    }
    assert_eq!(
        service.usage().selected_bytes,
        crate::prepared_source::TEXT_RESERVATION_BYTES
    );
    let (client, upload) = held[0];
    service.ordered_abort(client, upload, 5).unwrap();
    assert!(service.ordered_begin_text(client, 131072, 6).is_ok());
    // Free a selection slot independently while retaining all text reservations.
    service.limits.selections = 256;
    service.limits.clients = 33;
    let extra = service.connect().unwrap();
    assert_eq!(
        service.ordered_begin_text(extra, 1, 1),
        Err(Error::Capacity)
    );
    for client in held
        .into_iter()
        .map(|(client, _)| client)
        .collect::<std::collections::BTreeSet<_>>()
    {
        service.disconnect(client).unwrap();
    }
    assert_eq!(service.usage(), ResourceUsage::default());
}

fn memory_file(name: &str, size: u64) -> OpenedFile {
    use std::os::fd::AsRawFd;

    use rustix::fs::*;
    let writer = File::from(memfd_create("budget-test", MemfdFlags::ALLOW_SEALING).unwrap());
    writer.set_len(size).unwrap();
    fcntl_add_seals(&writer, crate::memory_source::IMMUTABLE_SEALS).unwrap();
    let reader = File::from(
        open(
            format!("/proc/self/fd/{}", writer.as_raw_fd()),
            OFlags::RDONLY,
            Mode::empty(),
        )
        .unwrap(),
    );
    OpenedFile::adopt(reader, name.into()).unwrap()
}

#[test]
fn given_staged_and_direct_memory_when_budget_is_exceeded_then_rejection_is_atomic_and_commit_moves_charge(
) {
    // Arrange: sparse sealed descriptors exercise actual kernel classification without filling RAM.
    let (mut service, client) = setup(Limits::default());
    let size = crate::memory_source::MAX_MEMORY_FILE_BYTES as u64;
    let direct = OpenedSelection::from_files(
        vec![memory_file("direct", size)],
        SelectionLimits::default(),
    )
    .unwrap();
    let selected = service.create_selection(client, direct).unwrap();
    let upload = service.begin_selection(client, "memory").unwrap();
    service
        .append_files(
            client,
            upload,
            0,
            vec![
                memory_file("a", size),
                memory_file("b", size),
                memory_file("c", size - 1),
                file("disk", size),
            ],
        )
        .unwrap();
    assert_eq!(
        service.memory_usage(),
        crate::memory_source::MEMORY_RESERVATION_BYTES - 1
    );
    let before = service.snapshot(client).unwrap();
    // Act / Assert: mixed rejected batch leaves every preceding handle/sequence unchanged.
    assert_eq!(
        service.append_files(
            client,
            upload,
            1,
            vec![file("other-disk", 1), memory_file("overflow", 2)]
        ),
        Err(Error::Capacity)
    );
    assert_eq!(service.snapshot(client).unwrap(), before);
    assert_eq!(service.get_upload(client, upload).unwrap().next_sequence, 1);
    service
        .append_files(client, upload, 1, vec![memory_file("last", 1)])
        .unwrap();
    let before = service.snapshot(client).unwrap().resources;
    service.commit_selection(client, upload).unwrap();
    assert_eq!(service.snapshot(client).unwrap().resources, before);
    assert_eq!(
        service.memory_usage(),
        crate::memory_source::MEMORY_RESERVATION_BYTES
    );
    let extra =
        OpenedSelection::from_files(vec![memory_file("extra", 1)], SelectionLimits::default())
            .unwrap();
    assert_eq!(
        service.create_selection(client, extra),
        Err(Error::Capacity)
    );
    // Text has an independent budget even while memory Files are full.
    let text = service.begin_text_upload(client, 1).unwrap();
    service.append_text(client, text, 0, b"x").unwrap();
    service.commit_upload(client, text).unwrap();
    service.discard_selection(client, selected).unwrap();
    service.refresh_resources().unwrap();
    assert_eq!(
        service.memory_usage(),
        crate::memory_source::MEMORY_RESERVATION_BYTES - size
    );
    service.disconnect(client).unwrap();
    service.refresh_resources().unwrap();
    assert_eq!(service.memory_usage(), 0);
}

#[test]
fn given_memory_owners_when_aborted_cancelled_or_finished_then_only_actual_reader_drop_releases_charge(
) {
    // Arrange / Act / Assert
    let (mut service, client) = setup(Limits::default());
    let upload = service.begin_selection(client, "abort").unwrap();
    service
        .append_files(client, upload, 0, vec![memory_file("a", 1)])
        .unwrap();
    service.abort_selection(client, upload).unwrap();
    assert_eq!(service.memory_usage(), 0);
    service.set_endpoints(vec!["recipient".into()]).unwrap();
    for active in [false, true] {
        let files =
            OpenedSelection::from_files(vec![memory_file("a", 1)], SelectionLimits::default())
                .unwrap();
        let selected = service.create_selection(client, files).unwrap();
        let transfer = service
            .send_selection(
                client,
                selected,
                "recipient",
                if active { "active" } else { "queued" },
            )
            .unwrap();
        let reader = active.then(|| service.next_send().unwrap().unwrap());
        service.cancel(client, transfer).unwrap();
        if active {
            service.observe(transfer, Phase::Finished).unwrap();
        }
        service.refresh_resources().unwrap();
        assert_eq!(service.memory_usage(), u64::from(active));
        drop(reader);
        service.refresh_resources().unwrap();
        assert_eq!(service.memory_usage(), 0);
    }
}

#[test]
fn given_file_extension_when_committed_then_base_is_replaced_atomically_and_receipts_are_exact() {
    // Arrange
    let (mut service, client) = setup(Limits::default());
    let base = service
        .create_selection(
            client,
            OpenedSelection::from_files(vec![file("old", 3)], SelectionLimits::default()).unwrap(),
        )
        .unwrap();
    let retained = service.selections[&base].resource.clone();
    let upload = service.ordered_begin(client, 1).unwrap();
    service
        .append_files(client, upload, 0, vec![file("new", 7)])
        .unwrap();
    let before = service.snapshot(client).unwrap();
    assert_eq!(before.selections, vec![base]);
    // Act
    let updated = service.ordered_extend(client, upload, base, 2).unwrap();
    // Assert
    let state = service.snapshot(client).unwrap();
    assert_eq!(state.selections, vec![updated]);
    assert!(state.transfers.is_empty());
    assert_ne!(base, updated);
    let source = &service.selections[&updated].resource;
    assert_eq!(
        source
            .files()
            .iter()
            .map(|file| (file.relative_name(), file.size()))
            .collect::<Vec<_>>(),
        [("old", 3), ("new", 7)]
    );
    assert_eq!(retained.files().len(), 1);
    assert_eq!(retained.total_bytes(), 3);
    assert_eq!(state.resources.reserved_handles, 6); // Retained old + full replacement.
    assert_eq!(service.ordered_extend(client, upload, base, 2), Ok(updated));
    assert_eq!(
        service.ordered_extend(client, upload, updated, 2),
        Err(Error::ReplayConflict)
    );
    assert_eq!(
        service.ordered_commit(client, upload, 2),
        Err(Error::ReplayConflict)
    );
    assert_eq!(service.snapshot(client).unwrap(), state);
    service.acknowledge_operations(client, 2).unwrap();
    assert_eq!(
        service.ordered_extend(client, upload, base, 2),
        Err(Error::RequestExpired)
    );
    drop(retained);
    assert_eq!(
        service.snapshot(client).unwrap().resources.reserved_handles,
        4
    );
}

#[test]
fn given_invalid_extension_when_rejected_then_previous_selection_and_delta_remain_intact() {
    // Arrange / Act / Assert: every case uses fresh actual coordinator and file capabilities.
    for case in 0..10 {
        let (mut service, client) = setup(Limits::default());
        let base = service
            .create_selection(
                client,
                OpenedSelection::from_files(vec![file("old", 3)], SelectionLimits::default())
                    .unwrap(),
            )
            .unwrap();
        let upload = service.ordered_begin(client, 1).unwrap();
        if case != 0 {
            service
                .append_files(
                    client,
                    upload,
                    0,
                    vec![file(
                        if case == 1 {
                            "old"
                        } else if case == 2 {
                            "old/child"
                        } else {
                            "new"
                        },
                        7,
                    )],
                )
                .unwrap();
        }
        let mut target = base;
        let mut owner = client;
        let expected = match case {
            0..=2 => Error::InvalidInput,
            3 => {
                service.limits.reserved_handles = 4;
                Error::Capacity
            }
            4 => {
                service.limits.selected_bytes = 10;
                Error::Capacity
            }
            5 => {
                target = SelectionId(123, 9999);
                Error::UnknownSelection
            }
            6 => {
                service.discard_selection(client, base).unwrap();
                Error::UnknownSelection
            }
            7 => {
                owner = service.connect().unwrap();
                Error::WrongOwner
            }
            8 => {
                service.ordered_abort(client, upload, 2).unwrap();
                Error::InvalidState
            }
            9 => {
                service.lifecycle = Lifecycle::Stopped;
                Error::NotRunning
            }
            _ => unreachable!(),
        };
        let before = service.snapshot(client).unwrap();
        let sequence = if case == 8 {
            3
        } else if case == 7 {
            1
        } else {
            2
        };
        assert_eq!(
            service.ordered_extend(owner, upload, target, sequence),
            Err(expected),
            "case {case}"
        );
        assert_eq!(service.snapshot(client).unwrap(), before, "case {case}");
        if case != 6 {
            assert_eq!(service.selections[&base].resource.total_bytes(), 3);
        }
    }
}

#[test]
fn given_aggregate_file_limits_when_extending_then_max_is_accepted_and_max_plus_one_preserves_base()
{
    // Arrange / Act / Assert
    for (count, accepted) in [(254, true), (255, true), (256, false)] {
        let (mut service, client) = setup(Limits::default());
        service.limits.reserved_handles = 2048; // Isolate the per-selection count boundary.
        let files = (0..count)
            .map(|index| file(&format!("old-{index}"), 0))
            .collect();
        let base = service
            .create_selection(
                client,
                OpenedSelection::from_files(files, SelectionLimits::default()).unwrap(),
            )
            .unwrap();
        let upload = service.ordered_begin(client, 1).unwrap();
        service
            .append_files(client, upload, 0, vec![file("new", 0)])
            .unwrap();
        let before = service.snapshot(client).unwrap();
        let result = service.ordered_extend(client, upload, base, 2);
        if accepted {
            let updated = result.unwrap();
            assert_eq!(
                service.selections[&updated].resource.files().len(),
                count + 1
            );
        } else {
            assert_eq!(result, Err(Error::InvalidInput));
            assert_eq!(service.snapshot(client).unwrap(), before);
        }
    }
}

#[test]
fn given_text_or_committed_delta_when_extended_then_no_mixed_source_or_base_loss_occurs() {
    // Arrange / Act / Assert
    for case in 0..3 {
        let (mut service, client) = setup(Limits::default());
        let base = if case == 0 {
            let text = service.ordered_begin_text(client, 1, 1).unwrap();
            service.append_text(client, text, 0, b"x").unwrap();
            service.ordered_commit(client, text, 2).unwrap()
        } else {
            service
                .create_selection(
                    client,
                    OpenedSelection::from_files(vec![file("old", 3)], SelectionLimits::default())
                        .unwrap(),
                )
                .unwrap()
        };
        let seq = if case == 0 { 3 } else { 1 };
        let upload = if case == 1 {
            let text = service.ordered_begin_text(client, 1, seq).unwrap();
            service.append_text(client, text, 0, b"x").unwrap();
            text
        } else {
            let upload = service.ordered_begin(client, seq).unwrap();
            service
                .append_files(client, upload, 0, vec![file("new", 7)])
                .unwrap();
            upload
        };
        if case == 2 {
            service.ordered_commit(client, upload, seq + 1).unwrap();
        }
        let before = service.snapshot(client).unwrap();
        assert_eq!(
            service.ordered_extend(client, upload, base, seq + if case == 2 { 2 } else { 1 }),
            Err(Error::InvalidState)
        );
        assert_eq!(service.snapshot(client).unwrap(), before);
    }
}

#[test]
fn given_changed_base_size_or_aborted_delta_when_extending_then_original_capability_is_not_replaced(
) {
    // Arrange / Act / Assert
    for changed in [false, true] {
        let (mut service, client) = setup(Limits::default());
        let original = tempfile::NamedTempFile::new().unwrap();
        original.as_file().set_len(3).unwrap();
        let base = service
            .create_selection(
                client,
                OpenedSelection::from_files(
                    vec![
                        OpenedFile::adopt(File::open(original.path()).unwrap(), "old".into())
                            .unwrap(),
                    ],
                    SelectionLimits::default(),
                )
                .unwrap(),
            )
            .unwrap();
        let upload = service.ordered_begin(client, 1).unwrap();
        service
            .append_files(client, upload, 0, vec![file("new", 7)])
            .unwrap();
        if changed {
            original.as_file().set_len(4).unwrap();
            let before = service.snapshot(client).unwrap();
            assert_eq!(
                service.ordered_extend(client, upload, base, 2),
                Err(Error::InvalidInput)
            );
            assert_eq!(service.snapshot(client).unwrap(), before);
        }
        service.ordered_abort(client, upload, 2).unwrap();
        let after = service.snapshot(client).unwrap();
        assert_eq!(after.selections, vec![base]);
        assert_eq!(after.resources.reserved_handles, 2);
        assert_eq!(after.resources.selected_bytes, 3);
    }
}

#[test]
fn given_aggregate_byte_and_memory_budgets_when_extended_then_boundaries_preserve_reservations() {
    // Arrange / Act / Assert
    for added in [0, 1, 2] {
        let (mut service, client) = setup(Limits::default());
        let max = SelectionLimits::default().max_bytes;
        let base = service
            .create_selection(
                client,
                OpenedSelection::from_files(vec![file("old", max - 1)], SelectionLimits::default())
                    .unwrap(),
            )
            .unwrap();
        let upload = service.ordered_begin(client, 1).unwrap();
        service
            .append_files(client, upload, 0, vec![file("new", added)])
            .unwrap();
        let before = service.snapshot(client).unwrap();
        let result = service.ordered_extend(client, upload, base, 2);
        if added <= 1 {
            assert_eq!(
                service.selections[&result.unwrap()].resource.total_bytes(),
                max - 1 + added
            );
        } else {
            assert_eq!(result, Err(Error::InvalidInput));
            assert_eq!(service.snapshot(client).unwrap(), before);
        }
    }
    for delta in [0, 1] {
        let (mut service, client) = setup(Limits::default());
        let size = crate::memory_source::MAX_MEMORY_FILE_BYTES;
        let payload = vec![7; size];
        let base = service
            .create_selection(
                client,
                OpenedSelection::from_files(
                    (0..2)
                        .map(|n| {
                            crate::memory_source::create(&payload, format!("old-{n}")).unwrap()
                        })
                        .collect(),
                    SelectionLimits::default(),
                )
                .unwrap(),
            )
            .unwrap();
        let upload = service.ordered_begin(client, 1).unwrap();
        let added = if delta == 0 {
            file("new", 0)
        } else {
            crate::memory_source::create(b"x", "new".into()).unwrap()
        };
        service
            .append_files(client, upload, 0, vec![added])
            .unwrap();
        let before = service.snapshot(client).unwrap();
        let result = service.ordered_extend(client, upload, base, 2);
        if delta == 0 {
            assert_eq!(
                service.selections[&result.unwrap()].resource.files().len(),
                3
            );
        } else {
            assert_eq!(result, Err(Error::Capacity));
            assert_eq!(service.snapshot(client).unwrap(), before);
        }
    }
}
