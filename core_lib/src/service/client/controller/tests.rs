use std::fs::File;

use super::*;
use crate::file_source::{OpenedFile, SelectionLimits};
use crate::service::bus::tests::{Harness, PrivateBus};
use crate::service::bus::SelectionBus;
use crate::service::client::state::{DetailsStatus, FileDetail, TransferDetails};
use crate::service::client::tests::{fixture, hook, interrupted, scenario, source};
use crate::service::{Coordinator, Limits};

fn opened(count: usize) -> (tempfile::NamedTempFile, OpenedSelection) {
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(3).unwrap();
    let selection = OpenedSelection::from_files(
        (0..count)
            .map(|n| {
                OpenedFile::adopt(File::open(file.path()).unwrap(), format!("folder/{n:03}"))
                    .unwrap()
            })
            .collect(),
        SelectionLimits::default(),
    )
    .unwrap();
    (file, selection)
}
async fn ready(updates: &mut watch::Receiver<PanelState>) -> PanelState {
    updates
        .wait_for(|state| state.view.is_some() && !state.busy)
        .await
        .unwrap()
        .clone()
}

#[tokio::test]
async fn given_acquired_batches_when_chosen_then_nothing_sends_until_explicit_send_and_transfer_survives_close(
) {
    scenario(async {
        // Arrange
        let (_bus, host, client) = fixture().await;
        let controller = PanelController::spawn(client);
        let handle = controller.handle();
        let mut updates = handle.subscribe();
        ready(&mut updates).await;
        let (file, selection) = opened(17);
        // Act
        let ActionResult::Selection(selection) =
            handle.choose(selection).unwrap().wait().await.unwrap()
        else {
            panic!("Missing committed selection")
        };
        // Assert
        let chosen = updates.borrow().clone();
        assert!(!chosen.busy && !chosen.pending && chosen.error.is_none());
        assert!(chosen.view.as_ref().unwrap().transfers.is_empty());
        assert_eq!(
            chosen.view.as_ref().unwrap().selections,
            vec![selection.clone()]
        );
        let ActionResult::Transfer(transfer) = handle
            .send(selection, "peer".into())
            .unwrap()
            .wait()
            .await
            .unwrap()
        else {
            panic!("Missing explicit transfer")
        };
        let sent = updates.borrow().clone();
        assert_eq!(sent.view.unwrap().transfers[0].transfer, transfer);
        controller.close().await.unwrap();
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert_eq!(coordinator.transfers.len(), 1);
        assert_eq!(
            coordinator.transfers.values().next().unwrap().view.phase,
            Phase::Queued
        );
        assert!(file.path().is_file());
        assert!(coordinator.clients.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_unsent_selection_when_discarded_or_controller_closes_then_owned_sources_release_without_deleting_originals(
) {
    scenario(async {
        // Arrange / Act / Assert
        for discard in [false, true] {
            let (_bus, host, client) = fixture().await;
            let controller = PanelController::spawn(client);
            let handle = controller.handle();
            let (file, selection) = opened(1);
            let ActionResult::Selection(selection) =
                handle.choose(selection).unwrap().wait().await.unwrap()
            else {
                panic!("Missing selection")
            };
            if discard {
                assert_eq!(
                    handle.discard(selection).unwrap().wait().await,
                    Ok(ActionResult::Completed)
                );
                assert!(handle
                    .subscribe()
                    .borrow()
                    .view
                    .as_ref()
                    .unwrap()
                    .selections
                    .is_empty());
            }
            controller.close().await.unwrap();
            let (coordinator, outcome) = host.finish().await;
            assert_eq!(outcome, Ok(()));
            assert!(coordinator.selections.is_empty() && coordinator.transfers.is_empty());
            assert_eq!(coordinator.usage().reserved_handles, 0);
            assert!(file.path().is_file());
        }
    })
    .await;
}

#[tokio::test]
async fn given_full_bounded_queue_and_dropped_waiters_when_close_interrupts_io_then_every_owned_task_and_request_closes(
) {
    scenario(async {
        // Arrange
        let (_bus, host, mut client) = fixture().await;
        let reached = hook(&mut client, "BeginSelection", false);
        let controller = PanelController::spawn(client);
        let handle = controller.handle();
        let mut updates = handle.subscribe();
        ready(&mut updates).await;
        let (file, selection) = opened(1);
        let active = handle.choose(selection).unwrap();
        reached.notified().await;
        // Act / Assert
        let mut waits = Vec::new();
        for _ in 0..QUEUE {
            waits.push(handle.recover().unwrap());
        }
        assert!(matches!(handle.recover(), Err(ControllerError::Busy)));
        drop(waits.pop()); // GUI detachment is independent from worker admission.
        assert!(updates.borrow().busy);
        controller.close().await.unwrap();
        assert_eq!(active.wait().await, Err(ControllerError::Closed));
        for waiter in waits {
            assert_eq!(waiter.wait().await, Err(ControllerError::Closed));
        }
        assert!(matches!(handle.recover(), Err(ControllerError::Closed)));
        assert!(updates.borrow().disconnected);
        assert!(!updates.borrow().busy);
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert!(coordinator.uploads.is_empty());
        assert_eq!(coordinator.usage().reserved_handles, 0);
        assert!(file.path().is_file());
    })
    .await;
}

#[tokio::test]
async fn given_ready_and_unavailable_consent_when_controller_refreshes_then_exact_round_readiness_and_controls_are_enforced(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        for ready in [true, false] {
            let id = coordinator.register_inbound().unwrap();
            if ready {
                coordinator
                    .observe_details(
                        id,
                        Phase::AwaitingConsent,
                        Arc::new(TransferDetails {
                            status: DetailsStatus::Ready,
                            sender: "phone".into(),
                            pin: "0123".into(),
                            total_bytes: 3,
                            files: vec![FileDetail {
                                name: "item".into(),
                                size: 3,
                            }]
                            .into(),
                            ..Default::default()
                        }),
                    )
                    .unwrap();
            } else {
                coordinator.observe(id, Phase::AwaitingConsent).unwrap();
            }
        }
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let controller =
            PanelController::spawn(NativeClient::connect(private.socket().await).await.unwrap());
        let handle = controller.handle();
        let mut updates = handle.subscribe();
        let initial = ready(&mut updates).await;
        let view = initial.view.unwrap();
        let first = &view.transfers[0];
        let unavailable = &view.transfers[1];
        // Act / Assert
        assert_eq!(
            handle
                .respond(
                    unavailable.transfer.clone(),
                    unavailable.consent_revision.unwrap(),
                    true
                )
                .unwrap()
                .wait()
                .await,
            Err(ControllerError::Client(ClientError::Rejected(
                crate::service::Error::InvalidState
            )))
        );
        assert_eq!(
            handle
                .respond(
                    first.transfer.clone(),
                    first.consent_revision.unwrap() + 1,
                    true
                )
                .unwrap()
                .wait()
                .await,
            Err(ControllerError::Client(ClientError::Rejected(
                crate::service::Error::StaleConsent
            )))
        );
        for _ in 0..2 {
            assert_eq!(
                handle
                    .respond(
                        first.transfer.clone(),
                        first.consent_revision.unwrap(),
                        true
                    )
                    .unwrap()
                    .wait()
                    .await,
                Ok(ActionResult::Completed)
            );
        }
        assert_eq!(
            updates.borrow().view.as_ref().unwrap().transfers[0]
                .pending_control
                .unwrap()
                .0,
            ControlAction::Accept
        );
        assert_eq!(
            handle
                .respond(
                    first.transfer.clone(),
                    first.consent_revision.unwrap(),
                    false
                )
                .unwrap()
                .wait()
                .await,
            Err(ControllerError::Client(ClientError::Rejected(
                crate::service::Error::StaleConsent
            )))
        );
        assert_eq!(
            handle
                .respond(
                    unavailable.transfer.clone(),
                    unavailable.consent_revision.unwrap(),
                    false
                )
                .unwrap()
                .wait()
                .await,
            Ok(ActionResult::Completed)
        );
        handle
            .cancel(first.transfer.clone())
            .unwrap()
            .wait()
            .await
            .unwrap();
        assert_eq!(
            updates.borrow().view.as_ref().unwrap().transfers[0].phase,
            Phase::CancelRequested
        );
        assert_eq!(
            handle
                .respond(
                    first.transfer.clone(),
                    first.consent_revision.unwrap(),
                    true
                )
                .unwrap()
                .wait()
                .await,
            Err(ControllerError::Client(ClientError::Rejected(
                crate::service::Error::StaleConsent
            )))
        );
        controller.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_disconnected_or_slow_view_consumer_when_daemon_changes_then_worker_coalesces_and_fails_closed_without_reconnect(
) {
    scenario(async {
        // Arrange
        let (_bus, host, client) = fixture().await;
        let controller = PanelController::spawn(client);
        let handle = controller.handle();
        let mut updates = handle.subscribe();
        ready(&mut updates).await;
        // Act: omit consuming updates, including every intermediate busy/view notification.
        for _ in 0..20 {
            let (_file, selection) = opened(1);
            match handle.choose(selection) {
                Ok(completion) => drop(completion),
                Err(ControllerError::Busy) => (),
                Err(_) => panic!("Unexpected queue failure"),
            }
        }
        // The bounded queue explicitly reports pressure instead of growing.
        // Assert owner loss is sticky and does not produce an event-loop spin.
        assert_eq!(host.finish().await.1, Ok(()));
        updates.wait_for(|state| state.disconnected).await.unwrap();
        assert!(matches!(
            updates.borrow().error,
            Some(ClientError::DaemonChanged | ClientError::Transport)
        ));
        assert!(handle.recover().unwrap().wait().await.is_err());
        let closed = controller.close().await;
        assert!(
            closed.is_ok()
                || matches!(closed, Err(ControllerError::Client(ClientError::Transport)))
        );
    })
    .await;
}

#[tokio::test]
async fn given_interrupted_batch_or_commit_when_preparation_recovers_then_exact_original_selection_is_committed_once(
) {
    scenario(async {
        // Arrange / Act / Assert
        for method in ["BeginSelection", "AppendFiles", "CommitSelection"] {
            for after in [false, true] {
                let (_bus, host, mut client) = fixture().await;
                let reached = hook(&mut client, method, after);
                let (state, _updates) = watch::channel(PanelState::default());
                let mut worker = Worker {
                    client,
                    preparation: None,
                    state,
                };
                let (file, selection) = opened(17);
                interrupted(worker.act(Action::Choose(selection)), reached).await;
                assert!(worker.client.has_pending() && worker.preparation.is_some());
                let ActionResult::Selection(selection) = worker.act(Action::Recover).await.unwrap()
                else {
                    panic!("Missing recovered selection")
                };
                assert!(!worker.client.has_pending() && worker.preparation.is_none());
                let view = worker.client.read_state().await.unwrap();
                assert_eq!(view.selections, vec![selection]);
                assert!(view.transfers.is_empty());
                assert_eq!(view.resources.selected_bytes, 51);
                assert!(file.path().is_file());
                worker.client.close().await.unwrap();
                assert_eq!(host.finish().await.1, Ok(()));
            }
        }
        // A send lost by the consumer is recovered as the original ordered outcome.
        let (_bus, host, mut client) = fixture().await;
        let (_file, files) = source();
        let upload = client.begin_selection().await.unwrap();
        client.append_batch(&upload, 0, &files).await.unwrap();
        let selection = client.commit_selection(&upload).await.unwrap();
        let reached = hook(&mut client, "SendSelection", true);
        interrupted(client.send_selection(&selection, "peer"), reached).await;
        let controller = PanelController::spawn(client);
        let result = controller.handle().recover().unwrap().wait().await.unwrap();
        assert!(matches!(
            result,
            ActionResult::Recovered(Some(Outcome::Operation(_)))
        ));
        controller.close().await.unwrap();
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert_eq!(coordinator.transfers.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn given_interrupted_preparation_or_abort_when_abandoned_then_recovery_releases_all_unsent_sources_without_sending(
) {
    scenario(async {
        // Arrange / Act / Assert
        for method in ["BeginSelection", "AppendFiles", "CommitSelection"] {
            for after in [false, true] {
                let (_bus, host, mut client) = fixture().await;
                let reached = hook(&mut client, method, after);
                let (state, _updates) = watch::channel(PanelState::default());
                let mut worker = Worker {
                    client,
                    preparation: None,
                    state,
                };
                let (file, selection) = opened(17);
                interrupted(worker.act(Action::Choose(selection)), reached).await;
                assert_eq!(
                    worker.act(Action::AbortPreparation).await,
                    Ok(ActionResult::Completed)
                );
                assert!(worker.preparation.is_none() && !worker.client.has_pending());
                let view = worker.client.read_state().await.unwrap();
                assert!(view.selections.is_empty() && view.transfers.is_empty());
                assert_eq!(view.resources.reserved_handles, 0);
                worker.client.close().await.unwrap();
                assert_eq!(host.finish().await.1, Ok(()));
                assert!(file.path().is_file());
            }
        }
        for after in [false, true] {
            let (_bus, host, mut client) = fixture().await;
            let reached = hook(&mut client, "AppendFiles", true);
            let (state, _updates) = watch::channel(PanelState::default());
            let mut worker = Worker {
                client,
                preparation: None,
                state,
            };
            let (file, selection) = opened(17);
            interrupted(worker.act(Action::Choose(selection)), reached).await;
            let reached = hook(&mut worker.client, "AbortSelection", after);
            interrupted(worker.act(Action::AbortPreparation), reached).await;
            assert_eq!(
                worker.act(Action::Recover).await,
                Ok(ActionResult::Completed)
            );
            let view = worker.client.read_state().await.unwrap();
            assert!(view.selections.is_empty() && view.transfers.is_empty());
            assert_eq!(view.resources.reserved_handles, 0);
            worker.client.close().await.unwrap();
            assert_eq!(host.finish().await.1, Ok(()));
            assert!(file.path().is_file());
        }
    })
    .await;
}

#[tokio::test]
async fn given_outgoing_text_when_preparation_replies_are_lost_then_exact_capture_recovers_and_send_stays_explicit(
) {
    scenario(async {
        // Arrange / Act / Assert
        for method in [
            "BeginText",
            "AppendText",
            "CommitSelection",
            "AcknowledgeThrough",
        ] {
            for after in [false, true] {
                let (_bus, host, mut client) = fixture().await;
                let reached = hook(&mut client, method, after);
                let (state, _) = watch::channel(PanelState::default());
                let mut worker = Worker {
                    client,
                    preparation: None,
                    state,
                };
                let body = "private-é".repeat(3000);
                interrupted(
                    worker.act(Action::ChooseText(NativeText::new(body.clone()).unwrap())),
                    reached,
                )
                .await;
                assert!(worker.client.has_pending() && worker.preparation.is_some());
                let ActionResult::Selection(selection) = worker.act(Action::Recover).await.unwrap()
                else {
                    panic!("Missing recovered selection")
                };
                assert!(!worker.client.has_pending() && worker.preparation.is_none());
                let view = worker.client.read_state().await.unwrap();
                assert_eq!(view.selections, vec![selection.clone()]);
                assert!(view.transfers.is_empty());
                assert_eq!(
                    view.sources[0].kind,
                    crate::sessions::details::PayloadKind::Text
                );
                assert_eq!(view.sources[0].byte_count, body.len() as u64);
                assert_eq!(view.sources[0].file_count, 0);
                assert!(!format!("{view:?}").contains("private-é"));
                assert!(worker
                    .act(Action::ChooseText(
                        NativeText::new("replacement".into()).unwrap()
                    ))
                    .await
                    .is_err());
                let (_file, files) = opened(1);
                assert!(worker.act(Action::Choose(files)).await.is_err());
                let ActionResult::Transfer(_) = worker
                    .act(Action::Send(selection, "peer".into()))
                    .await
                    .unwrap()
                else {
                    panic!("Missing transfer")
                };
                worker.client.close().await.unwrap();
                let (mut coordinator, result) = host.finish().await;
                assert_eq!(result, Ok(()));
                let dispatch = coordinator.next_send().unwrap().unwrap();
                assert_eq!(dispatch.selection.text().unwrap().text(), body);
                assert!(coordinator.next_send().unwrap().is_none());
            }
        }
    })
    .await;
}

#[tokio::test]
async fn given_anonymous_source_when_prepared_over_actual_broker_then_seals_bytes_and_explicit_send_survive_gui_close(
) {
    scenario(async {
        // Arrange: real SCM_RIGHTS/SDK/controller path, no local file or delete authority.
        let (_bus, host, client) = fixture().await;
        let controller = PanelController::spawn(client);
        let handle = controller.handle();
        let bytes = b"private-image-snapshot";
        let file = crate::memory_source::create(bytes, "Clipboard.png".into()).unwrap();
        let source = OpenedSelection::from_files(vec![file], SelectionLimits::default()).unwrap();
        // Act
        let ActionResult::Selection(selection) =
            handle.choose(source).unwrap().wait().await.unwrap()
        else {
            panic!("Missing preparation");
        };
        // Assert: prepare alone never enqueues a transfer.
        let state = handle.subscribe().borrow().clone();
        let view = state.view.unwrap();
        assert!(view.transfers.is_empty());
        assert_eq!(
            view.sources[0].kind,
            crate::sessions::details::PayloadKind::Files
        );
        assert_eq!(
            (view.sources[0].file_count, view.sources[0].byte_count),
            (1, bytes.len() as u64)
        );
        assert!(!format!("{view:?}").contains("private-image-snapshot"));
        handle
            .send(selection, "peer".into())
            .unwrap()
            .wait()
            .await
            .unwrap();
        controller.close().await.unwrap();
        let (mut coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        let dispatch = coordinator.next_send().unwrap().unwrap();
        let file = &dispatch.selection.files()[0];
        assert_eq!(file.memory_bytes(), bytes.len() as u64);
        assert_eq!(
            rustix::fs::fcntl_get_seals(file.clone_handle().unwrap()).unwrap(),
            crate::memory_source::IMMUTABLE_SEALS
        );
        let mut actual = vec![0; bytes.len()];
        assert_eq!(file.read_chunk_at(0, &mut actual).unwrap(), bytes.len());
        assert_eq!(actual, bytes);
        assert!(coordinator.next_send().unwrap().is_none());
    })
    .await;
}
