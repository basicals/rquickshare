use std::fs::File;
use std::future::Future;
use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;

use tokio::sync::Notify;

use super::*;
use crate::service::bus::tests::{Harness, PrivateBus};
use crate::service::bus::SelectionBus;
use crate::service::{Coordinator, Limits, ResourceUsage};

pub(super) async fn fixture() -> (PrivateBus, Harness, NativeClient) {
    let bus = PrivateBus::new();
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    coordinator.set_endpoints(vec!["peer".into()]).unwrap();
    let host = Harness::with_coordinator(
        SelectionBus::connect(bus.socket().await).await.unwrap(),
        coordinator,
    );
    let client = NativeClient::connect(bus.socket().await).await.unwrap();
    (bus, host, client)
}
pub(super) fn source() -> (tempfile::NamedTempFile, Vec<OpenedFile>) {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"original bytes").unwrap();
    let opened =
        OpenedFile::adopt(File::open(file.path()).unwrap(), "folder/document".into()).unwrap();
    (file, vec![opened])
}
pub(crate) async fn scenario(test: impl Future<Output = ()>) {
    tokio::time::timeout(std::time::Duration::from_secs(30), test)
        .await
        .expect("Owned client scenario must finish");
}
pub(crate) fn hook(client: &mut NativeClient, method: &'static str, after: bool) -> Arc<Notify> {
    let reached = Arc::new(Notify::new());
    client.hook = Some(ReplyHook {
        method,
        after,
        reached: reached.clone(),
        release: None,
    });
    reached
}
pub(crate) async fn interrupted<T>(future: impl Future<Output = Result<T>>, reached: Arc<Notify>) {
    tokio::pin!(future);
    tokio::select! {
        _=reached.notified()=>(),
        result=&mut future=>panic!("Expected cancellation barrier, success: {}",result.is_ok()),
    }
    // The borrowed future is dropped here; the caller-owned client stays alive.
}
async fn command(
    client: &mut NativeClient,
    kind: OperationKind,
) -> (Command, Vec<tempfile::NamedTempFile>) {
    if kind == OperationKind::Begin {
        return (Command::Begin, vec![]);
    }
    let upload = client.begin_selection().await.unwrap();
    if kind == OperationKind::Abort {
        return (Command::Abort(upload), vec![]);
    }
    let (_source, files) = source();
    client.append_batch(&upload, 0, &files).await.unwrap();
    if kind == OperationKind::Commit {
        return (Command::Commit(upload), vec![_source]);
    }
    if kind == OperationKind::Extend {
        let base = client.commit_selection(&upload).await.unwrap();
        let delta = client.begin_selection().await.unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let added = OpenedFile::adopt(File::open(file.path()).unwrap(), "added".into()).unwrap();
        client.append_batch(&delta, 0, &[added]).await.unwrap();
        return (Command::Extend(delta, base), vec![_source, file]);
    }
    (
        Command::Send(
            client.commit_selection(&upload).await.unwrap(),
            "peer".into(),
        ),
        vec![_source],
    )
}
fn member(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Begin => "BeginSelection",
        OperationKind::BeginText => "BeginText",
        OperationKind::Commit => "CommitSelection",
        OperationKind::Extend => "CommitExtension",
        OperationKind::Abort => "AbortSelection",
        OperationKind::Send => "SendSelection",
        OperationKind::SendMany => "Send",
    }
}

#[tokio::test]
async fn given_dedicated_client_when_connected_then_epoch_empty_ledger_and_redacted_errors_are_validated(
) {
    scenario(async {
        // Arrange / Act
        let (_bus, host, mut client) = fixture().await;
        // Assert
        assert_eq!(client.epoch, 123);
        assert_eq!(client.operation_state().await.unwrap(), (0, 0));
        assert!(!client.has_pending());
        assert!(client.last_outcome().is_none());
        assert_eq!(
            client
                .wire(OPERATIONS_INTERFACE, "NotAMethod", &())
                .await
                .unwrap_err(),
            ClientError::Incompatible
        );
        for name in include_str!("../../../dbus/native-selection-errors.txt").lines() {
            let error = domain(name).expect("Fixed daemon error must decode");
            assert_eq!(bus::error_name(error), name);
            assert!(!ClientError::Rejected(error).to_string().contains("source"));
        }
        assert_eq!(
            domain("io.github.morozhenko_apps.RQuickShare.Error.RemoteSecret"),
            None
        );
        client.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_missing_or_replaced_daemon_when_client_connects_or_recovers_then_pending_work_is_never_replayed(
) {
    scenario(async {
        // Arrange: missing service is explicit and does not start one.
        let (stream, peer) = UnixStream::pair().unwrap();
        drop(peer);
        assert_eq!(
            NativeClient::connect(stream).await.unwrap_err(),
            ClientError::Transport
        );
        let bus = PrivateBus::new();
        assert_eq!(
            NativeClient::connect(bus.socket().await).await.unwrap_err(),
            ClientError::DaemonChanged
        );
        let host = Harness::start(SelectionBus::connect(bus.socket().await).await.unwrap());
        let mut client = NativeClient::connect(bus.socket().await).await.unwrap();
        let reached = hook(&mut client, "BeginSelection", false);
        interrupted(client.begin_selection(), reached).await;
        let (old, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert!(old.uploads.is_empty());
        let replacement = Harness::with_coordinator(
            SelectionBus::connect(bus.socket().await).await.unwrap(),
            Coordinator::new(456, Limits::default()).unwrap(),
        );
        // Act / Assert
        assert_eq!(
            client.recover_pending().await,
            Err(ClientError::DaemonChanged)
        );
        assert!(client.has_pending());
        client.close().await.unwrap();
        let (new, result) = replacement.finish().await;
        assert_eq!(result, Ok(()));
        assert!(new.uploads.is_empty());
        assert!(new.transfers.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_malformed_ids_receipts_or_senders_when_decoded_then_client_accepts_only_current_typed_results(
) {
    // Arrange
    let (_bus, host, client) = fixture().await;
    let good = format!("{:032x}-{:016x}", 123, 1);
    // Act / Assert
    assert_eq!(canonical(&good, 123), Ok(()));
    for expected in [0, 1000, u32::MAX] {
        assert_eq!(same_uid(expected, expected), Ok(()));
        assert_eq!(
            same_uid(expected.wrapping_add(1), expected),
            Err(ClientError::Protocol)
        );
    }
    for kind in [
        "",
        "commit_selection",
        "send_selection",
        "abort_selection",
        "future_kind",
    ] {
        assert_eq!(
            client.decode_receipt(&Command::Begin, 1, kind, good.clone()),
            Err(ClientError::Protocol)
        );
    }
    assert_eq!(
        client.decode_receipt(&Command::Begin, 0, "begin_selection", good.clone()),
        Err(ClientError::Protocol)
    );
    for value in [
        String::new(),
        format!("{:032x}-{:016x}", 123, 0),
        format!("{:032x}-{:016x}", 456, 1),
        good.to_uppercase(),
        format!("{}-{}", "界".repeat(10) + "ab", "1".repeat(16)),
        "x".repeat(49),
        format!("{:032x}-{}", 123, "g".repeat(16)),
    ] {
        assert_eq!(canonical(&value, 123), Err(ClientError::Protocol));
        assert_eq!(
            client.value(&Command::Begin, value),
            Err(ClientError::Protocol)
        );
    }
    let upload = Upload {
        id: good.clone(),
        connection: client.identity.clone(),
    };
    assert_eq!(
        client.value(&Command::Abort(upload.clone()), good),
        Err(ClientError::Protocol)
    );
    assert_eq!(
        client.value(&Command::Abort(upload), String::new()),
        Ok(OperationValue::Empty)
    );
    let call = Message::method_call(PATH, "Test")
        .unwrap()
        .build(&())
        .unwrap();
    for sender in [":1.99", "org.freedesktop.DBus"] {
        let reply = Message::method_return(&call.header())
            .unwrap()
            .sender(sender)
            .unwrap()
            .build(&())
            .unwrap();
        assert_eq!(
            reply_sender(&reply, client.owner.as_str()),
            Err(ClientError::Protocol)
        );
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    let reply = Message::method_return(&call.header())
        .unwrap()
        .sender(client.owner.as_str())
        .unwrap()
        .build(&Fd::from(file.as_file()))
        .unwrap();
    assert_eq!(
        reply_sender(&reply, client.owner.as_str()),
        Err(ClientError::Protocol)
    );
    client.close().await.unwrap();
    assert_eq!(host.finish().await.1, Ok(()));
}

#[tokio::test]
async fn given_all_ordered_methods_when_completed_then_results_are_recorded_and_acknowledged_automatically(
) {
    scenario(async {
        // Arrange
        let (_bus, host, mut client) = fixture().await;
        let (source, files) = source();
        // Act
        let upload = client.begin_selection().await.unwrap();
        client.append_batch(&upload, 0, &files).await.unwrap();
        let selection = client.commit_selection(&upload).await.unwrap();
        let transfer = client.send_selection(&selection, "peer").await.unwrap();
        let aborted = client.begin_selection().await.unwrap();
        client.abort_selection(&aborted).await.unwrap();
        // Assert
        assert_eq!(client.operation_state().await.unwrap(), (5, 5));
        assert!(!client.has_pending());
        assert_eq!(
            client.last_outcome(),
            Some(&Outcome::Operation(Receipt {
                sequence: 5,
                operation: OperationKind::Abort,
                result: OperationValue::Empty
            }))
        );
        assert!(source.path().is_file());
        client.close().await.unwrap();
        let (mut coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert!(coordinator.uploads.is_empty());
        assert_eq!(coordinator.transfers.len(), 1);
        let dispatch = coordinator.next_send().unwrap().unwrap();
        assert_eq!(
            transfer.id(),
            format!("{:032x}-{:016x}", dispatch.transfer.0, dispatch.transfer.1)
        );
        assert_eq!(dispatch.selection.files().len(), 1);
        let mut bytes = [0; 14];
        assert_eq!(
            dispatch.selection.files()[0]
                .read_chunk_at(0, &mut bytes)
                .unwrap(),
            14
        );
        assert_eq!(&bytes, b"original bytes");
    })
    .await;
}

#[tokio::test]
async fn given_known_rejection_or_foreign_handle_when_mutated_then_sequence_and_existing_selection_are_preserved(
) {
    scenario(async {
        // Arrange
        let (bus, host, mut client) = fixture().await;
        let mut foreign = NativeClient::connect(bus.socket().await).await.unwrap();
        let upload = client.begin_selection().await.unwrap();
        // Act / Assert
        assert_eq!(
            client.commit_selection(&upload).await,
            Err(ClientError::Rejected(DomainError::InvalidInput))
        );
        assert!(!client.has_pending());
        assert_eq!(client.operation_state().await.unwrap(), (1, 1));
        assert_eq!(
            foreign.commit_selection(&upload).await,
            Err(ClientError::ForeignHandle)
        );
        assert!(!foreign.has_pending());
        assert_eq!(foreign.operation_state().await.unwrap(), (0, 0));
        let (_source, files) = source();
        client.append_batch(&upload, 0, &files).await.unwrap();
        let selection = client.commit_selection(&upload).await.unwrap();
        assert_eq!(client.operation_state().await.unwrap(), (2, 2));
        assert_eq!(
            client.send_selection(&selection, "missing").await,
            Err(ClientError::Rejected(DomainError::UnknownEndpoint))
        );
        assert_eq!(
            client.send_selection(&selection, "bad endpoint").await,
            Err(ClientError::InvalidInput)
        );
        assert!(!client.has_pending());
        client.send_selection(&selection, "peer").await.unwrap();
        assert_eq!(client.operation_state().await.unwrap(), (3, 3));
        client.close().await.unwrap();
        foreign.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_interrupted_mutation_before_or_after_acceptance_when_reconciled_then_exactly_one_operation_is_acknowledged(
) {
    scenario(async {
        // Arrange / Act / Assert: each kind crosses both reply-loss boundaries.
        for kind in [
            OperationKind::Begin,
            OperationKind::Commit,
            OperationKind::Extend,
            OperationKind::Abort,
            OperationKind::Send,
        ] {
            for after in [false, true] {
                let (_bus, host, mut client) = fixture().await;
                let (command, _source_guards) = command(&mut client, kind).await;
                let next = client.last + 1;
                let reached = hook(&mut client, member(kind), after);
                interrupted(client.mutation(command.clone()), reached).await;
                assert!(client.has_pending());
                assert!(matches!(
                    client.pending,
                    Some(Pending::Operation { receipt: None, .. })
                ));
                let receipt = client.mutation(command).await.unwrap();
                assert_eq!(receipt.sequence, next);
                assert_eq!(receipt.operation, kind);
                assert_eq!(client.operation_state().await.unwrap(), (next, next));
                assert_eq!(client.last_outcome(), Some(&Outcome::Operation(receipt)));
                assert!(!client.has_pending());
                client.close().await.unwrap();
                let (coordinator, result) = host.finish().await;
                assert_eq!(result, Ok(()));
                assert_eq!(
                    coordinator.transfers.len(),
                    usize::from(kind == OperationKind::Send)
                );
            }
        }
    })
    .await;
}

#[tokio::test]
async fn given_lost_ack_before_or_after_retirement_when_resumed_then_recorded_result_is_preserved_without_new_mutation(
) {
    scenario(async {
        // Arrange / Act / Assert
        for kind in [
            OperationKind::Begin,
            OperationKind::Commit,
            OperationKind::Extend,
            OperationKind::Abort,
            OperationKind::Send,
        ] {
            for after in [false, true] {
                let (_bus, host, mut client) = fixture().await;
                let (command, _source_guards) = command(&mut client, kind).await;
                let next = client.last + 1;
                let reached = hook(&mut client, "AcknowledgeThrough", after);
                interrupted(client.mutation(command.clone()), reached).await;
                let recorded = client.last_outcome().unwrap().clone();
                assert!(matches!(
                    client.pending,
                    Some(Pending::Operation {
                        receipt: Some(_),
                        ..
                    })
                ));
                assert_eq!(
                    client.operation_state().await.unwrap(),
                    (next, if after { next } else { next - 1 })
                );
                let receipt = client.mutation(command).await.unwrap();
                assert_eq!(recorded, Outcome::Operation(receipt));
                assert_eq!(client.operation_state().await.unwrap(), (next, next));
                assert!(!client.has_pending());
                client.close().await.unwrap();
                assert_eq!(host.finish().await.1, Ok(()));
            }
        }
    })
    .await;
}

#[tokio::test]
async fn given_pending_command_or_exhausted_sequence_when_new_work_is_requested_then_existing_work_cannot_be_overwritten(
) {
    // Arrange
    let (_bus, host, mut client) = fixture().await;
    let upload = client.begin_selection().await.unwrap();
    let reached = hook(&mut client, "AbortSelection", false);
    interrupted(client.abort_selection(&upload), reached).await;
    // Act / Assert
    assert_eq!(
        client.begin_selection().await,
        Err(ClientError::PendingOperation)
    );
    assert!(matches!(
        client.pending,
        Some(Pending::Operation {
            command: Command::Abort(_),
            ..
        })
    ));
    client.abort_selection(&upload).await.unwrap();
    client.last = u64::MAX;
    assert_eq!(client.begin_selection().await, Err(ClientError::Exhausted));
    assert!(!client.has_pending());
    client.close().await.unwrap();
    assert_eq!(host.finish().await.1, Ok(()));
}

#[tokio::test]
async fn given_batch_count_or_sequence_boundaries_when_appended_then_only_valid_bounded_batches_are_owned(
) {
    scenario(async {
        // Arrange / Act / Assert
        for count in [0, 1, 16, 17] {
            let (_bus, host, mut client) = fixture().await;
            let upload = client.begin_selection().await.unwrap();
            let file = tempfile::NamedTempFile::new().unwrap();
            let files: Vec<_> = (0..count)
                .map(|n| {
                    OpenedFile::adopt(File::open(file.path()).unwrap(), format!("f-{n}")).unwrap()
                })
                .collect();
            let expected = if count == 0 || count > 16 {
                Err(ClientError::InvalidInput)
            } else {
                Ok(1)
            };
            assert_eq!(client.append_batch(&upload, 0, &files).await, expected);
            assert!(!client.has_pending());
            if count == 1 || count == 16 {
                assert_eq!(
                    client.append_batch(&upload, u32::MAX, &files).await,
                    Err(ClientError::Exhausted)
                );
                assert!(!client.has_pending());
            }
            assert!(file.path().is_file());
            client.close().await.unwrap();
            let (coordinator, result) = host.finish().await;
            assert_eq!(result, Ok(()));
            assert!(coordinator.uploads.is_empty());
            assert_eq!(coordinator.usage(), ResourceUsage::default());
        }
    })
    .await;
}

#[tokio::test]
async fn given_lost_batch_reply_when_recovered_then_one_batch_preserves_original_bytes_and_shared_offset_independence(
) {
    scenario(async {
        // Arrange / Act / Assert
        for after in [false, true] {
            let (_bus, host, mut client) = fixture().await;
            let (source, files) = source();
            let upload = client.begin_selection().await.unwrap();
            let reached = hook(&mut client, "AppendFiles", after);
            interrupted(client.append_batch(&upload, 0, &files), reached).await;
            assert!(client.has_pending());
            assert_eq!(
                client.commit_selection(&upload).await,
                Err(ClientError::PendingOperation)
            );
            assert_eq!(
                client.recover_pending().await,
                Ok(Some(Outcome::Appended {
                    upload: upload.clone(),
                    next_sequence: 1
                }))
            );
            assert!(!client.has_pending());
            let selection = client.commit_selection(&upload).await.unwrap();
            client.send_selection(&selection, "peer").await.unwrap();
            files[0]
                .clone_handle()
                .unwrap()
                .seek(SeekFrom::End(0))
                .unwrap();
            client.close().await.unwrap();
            let (mut coordinator, result) = host.finish().await;
            assert_eq!(result, Ok(()));
            let dispatch = coordinator.next_send().unwrap().unwrap();
            assert_eq!(dispatch.selection.files().len(), 1);
            let mut bytes = [0; 14];
            assert_eq!(
                dispatch.selection.files()[0]
                    .read_chunk_at(0, &mut bytes)
                    .unwrap(),
                14
            );
            assert_eq!(&bytes, b"original bytes");
            assert!(source.path().is_file());
        }
    })
    .await;
}

#[tokio::test]
async fn given_closed_or_advanced_upload_when_batch_recovery_is_attempted_then_unexpected_state_is_not_replayed(
) {
    scenario(async {
        // Arrange / Act / Assert: the private raw wire seam injects an external contract violation.
        for closed in [false, true] {
            let (_bus, host, mut client) = fixture().await;
            let (_source, files) = source();
            let upload = client.begin_selection().await.unwrap();
            let reached = hook(&mut client, "AppendFiles", true);
            interrupted(client.append_batch(&upload, 0, &files), reached).await;
            if closed {
                client
                    .wire(
                        OPERATIONS_INTERFACE,
                        "CommitSelection",
                        &(upload.id(), 2u64),
                    )
                    .await
                    .unwrap();
            } else {
                let file = files[0].clone_handle().unwrap();
                client
                    .wire(
                        INTERFACE,
                        "AppendFiles",
                        &(upload.id(), 1u32, vec![(Fd::from(file.as_fd()), "other")]),
                    )
                    .await
                    .unwrap();
            }
            assert_eq!(client.recover_pending().await, Err(ClientError::Protocol));
            assert!(client.has_pending());
            client.close().await.unwrap();
            assert_eq!(host.finish().await.1, Ok(()));
        }
    })
    .await;
}

#[tokio::test]
async fn given_thousands_of_acknowledged_client_operations_when_run_on_one_connection_then_receipt_capacity_is_reclaimed(
) {
    scenario(async {
        // Arrange
        let (_bus, host, mut client) = fixture().await;
        // Act / Assert
        for n in 0..1000u64 {
            let upload = client.begin_selection().await.unwrap();
            client.abort_selection(&upload).await.unwrap();
            assert!(!client.has_pending());
            assert_eq!(client.last, 2 * n + 2);
        }
        assert_eq!(client.operation_state().await.unwrap(), (2000, 2000));
        assert_eq!(client.recover_pending().await, Ok(None));
        client.close().await.unwrap();
        let (coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert!(coordinator.uploads.is_empty());
        assert!(coordinator.transfers.is_empty());
        assert_eq!(coordinator.usage(), ResourceUsage::default());
    })
    .await;
}

#[tokio::test]
async fn given_client_close_with_pending_or_accepted_payload_when_disconnected_then_only_unsent_sources_are_released(
) {
    scenario(async {
        // Arrange / Act / Assert
        for accepted in [false, true] {
            let (_bus, host, mut client) = fixture().await;
            let (source, files) = source();
            let count = || {
                std::fs::read_dir("/proc/self/fd")
                    .unwrap()
                    .filter_map(std::result::Result::ok)
                    .filter_map(|entry| std::fs::read_link(entry.path()).ok())
                    .filter(|path| path == source.path())
                    .count()
            };
            let before = count();
            let upload = client.begin_selection().await.unwrap();
            if accepted {
                client.append_batch(&upload, 0, &files).await.unwrap();
                let selection = client.commit_selection(&upload).await.unwrap();
                client.send_selection(&selection, "peer").await.unwrap();
            } else {
                let reached = hook(&mut client, "AppendFiles", true);
                interrupted(client.append_batch(&upload, 0, &files), reached).await;
            }
            assert!(count() > before);
            client.close().await.unwrap();
            let (coordinator, result) = host.finish().await;
            assert_eq!(result, Ok(()));
            assert_eq!(coordinator.transfers.len(), usize::from(accepted));
            assert_eq!(count(), before + usize::from(accepted));
            assert!(source.path().is_file());
            drop(coordinator);
            assert_eq!(count(), before);
        }
    })
    .await;
}

#[tokio::test]
async fn given_accepted_send_with_failed_ack_when_daemon_disappears_then_recorded_acceptance_is_not_reported_as_rejection(
) {
    scenario(async {
        // Arrange
        let (_bus, host, mut client) = fixture().await;
        let (command, _source_guards) = command(&mut client, OperationKind::Send).await;
        let reached = hook(&mut client, "AcknowledgeThrough", false);
        interrupted(client.mutation(command), reached).await;
        let recorded = client.last_outcome().unwrap().clone();
        let (coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert_eq!(coordinator.transfers.len(), 1);
        // Act / Assert
        assert_eq!(
            client.recover_pending().await,
            Err(ClientError::AcknowledgementPending)
        );
        assert_eq!(client.last_outcome(), Some(&recorded));
        assert!(client.has_pending());
        assert!(matches!(
            recorded,
            Outcome::Operation(Receipt {
                operation: OperationKind::Send,
                result: OperationValue::Transfer(_),
                ..
            })
        ));
        client.close().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn given_daemon_notices_when_gui_clients_restart_or_repeat_dismissal_then_only_exact_pending_events_survive(
) {
    scenario(async {
        // Arrange
        const EXTRA_OUTCOMES: usize = 17;
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        for phase in [
            crate::service::Phase::AwaitingConsent,
            crate::service::Phase::Finished,
            crate::service::Phase::Failed,
            crate::service::Phase::Disconnected,
        ]
        .into_iter()
        .chain(std::iter::repeat_n(
            crate::service::Phase::Finished,
            EXTRA_OUTCOMES,
        )) {
            let id = coordinator.register_inbound().unwrap();
            coordinator.observe(id, phase).unwrap();
        }
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let initial = client.read_state().await.unwrap();
        assert_eq!(initial.notices.len(), 4 + EXTRA_OUTCOMES);
        let first = initial.notices[0].clone();
        // Act / Assert
        client
            .dismiss_notices(std::slice::from_ref(&first))
            .await
            .unwrap();
        let dismissed = client.read_state().await.unwrap();
        assert_eq!(dismissed.notices.len(), 3 + EXTRA_OUTCOMES);
        assert_eq!(dismissed.transfers, initial.transfers);
        client
            .dismiss_notices(std::slice::from_ref(&first))
            .await
            .unwrap();
        assert_eq!(
            client.read_state().await.unwrap().revision,
            dismissed.revision
        );
        assert_eq!(
            client.dismiss_notices(&[]).await,
            Err(ClientError::InvalidInput)
        );
        assert_eq!(
            client.dismiss_notices(&vec![first.clone(); 17]).await,
            Err(ClientError::InvalidInput)
        );
        client.close().await.unwrap();
        let mut next = NativeClient::connect(private.socket().await).await.unwrap();
        assert_eq!(
            next.read_state().await.unwrap().notices.len(),
            3 + EXTRA_OUTCOMES
        );
        assert_eq!(
            next.dismiss_notices(&[first]).await,
            Err(ClientError::ForeignHandle)
        );
        let controller = controller::PanelController::spawn(next);
        let handle = controller.handle();
        let mut updates = handle.subscribe();
        updates
            .wait_for(|state| state.view.is_some())
            .await
            .unwrap();
        let notices = updates.borrow().view.as_ref().unwrap().notices.clone();
        handle
            .dismiss_notices(notices)
            .unwrap()
            .wait()
            .await
            .unwrap();
        updates
            .wait_for(|state| {
                state
                    .view
                    .as_ref()
                    .is_some_and(|view| view.notices.is_empty())
            })
            .await
            .unwrap();
        controller.close().await.unwrap();
        let (coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert!(coordinator.pending_notices().is_empty());
        assert_eq!(coordinator.transfers.len(), 4 + EXTRA_OUTCOMES);
    })
    .await;
}

#[tokio::test]
async fn given_untrusted_notice_wire_batches_when_rejected_then_current_notice_and_transfer_remain_unchanged(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        let id = coordinator.register_inbound().unwrap();
        coordinator
            .observe(id, crate::service::Phase::AwaitingConsent)
            .unwrap();
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let initial = client.read_state().await.unwrap();
        let notice = &initial.notices[0];
        let valid = (
            notice.transfer.id().to_owned(),
            notice.kind.wire().to_owned(),
            notice.round,
            notice.revision,
        );
        let mut batches = vec![vec![], vec![valid.clone(); 2], vec![valid.clone(); 17]];
        for row in [
            (
                "not-a-token".into(),
                "request".into(),
                notice.round,
                notice.revision,
            ),
            (
                valid.0.clone(),
                "unknown".into(),
                notice.round,
                notice.revision,
            ),
            (valid.0.clone(), "request".into(), 0, notice.revision),
            (
                valid.0.clone(),
                "finished".into(),
                notice.round,
                notice.revision,
            ),
            (valid.0.clone(), "request".into(), notice.round, 0),
            (valid.0.clone(), "request".into(), notice.round, u64::MAX),
        ] {
            batches.push(vec![valid.clone(), row]);
        }
        // Act / Assert
        for batch in batches {
            assert_eq!(
                client
                    .wire(bus::NOTIFICATIONS_INTERFACE, "Dismiss", &batch)
                    .await
                    .unwrap_err(),
                ClientError::Rejected(DomainError::InvalidInput)
            );
            assert_eq!(client.read_state().await.unwrap(), initial);
        }
        assert_eq!(
            client
                .wire(bus::NOTIFICATIONS_INTERFACE, "Dismiss", &42u32)
                .await
                .unwrap_err(),
            ClientError::Rejected(DomainError::InvalidInput)
        );
        assert_eq!(
            client
                .wire(bus::NOTIFICATIONS_INTERFACE, "NotAMethod", &())
                .await
                .unwrap_err(),
            ClientError::Incompatible
        );
        client.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

mod batch;
