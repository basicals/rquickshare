use std::{fs::OpenOptions, future::Future};

use tokio::sync::Notify;

use super::*;
use crate::service::client::{
    controller::PanelController,
    tests::{fixture, scenario},
};

fn opened(names: impl IntoIterator<Item = String>) -> (tempfile::NamedTempFile, OpenedSelection) {
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(3).unwrap();
    let source = OpenedSelection::from_files(
        names
            .into_iter()
            .map(|name| OpenedFile::adopt(File::open(file.path()).unwrap(), name).unwrap())
            .collect(),
        SelectionLimits::default(),
    )
    .unwrap();
    (file, source)
}
async fn ready(controller: &PanelController) {
    controller
        .handle()
        .subscribe()
        .wait_for(|state| state.view.is_some() && !state.busy)
        .await
        .unwrap();
}
async fn begin(client: &mut DesktopClient) -> u64 {
    client
        .wire("Begin", &())
        .await
        .unwrap()
        .body()
        .deserialize()
        .unwrap()
}
async fn append(
    client: &mut DesktopClient,
    token: u64,
    sequence: u32,
    file: &File,
    names: &[&str],
) -> Result<u32, DesktopError> {
    let entries = names
        .iter()
        .map(|name| (Fd::from(file.as_fd()), *name))
        .collect::<Vec<_>>();
    client
        .wire("AppendFiles", &(token, sequence, entries))
        .await?
        .body()
        .deserialize()
        .map_err(|_| DesktopError::Protocol)
}
async fn commit(client: &mut DesktopClient, token: u64) -> String {
    client
        .wire("Commit", &token)
        .await
        .unwrap()
        .body()
        .deserialize()
        .unwrap()
}
async fn interrupted(future: impl Future<Output = Result<(), DesktopError>>, reached: Arc<Notify>) {
    tokio::pin!(future);
    tokio::select! {
        _=reached.notified()=>(),
        result=&mut future=>panic!("Expected cancellation barrier, success: {}", result.is_ok()),
    }
}

#[tokio::test]
async fn given_native_manifest_limits_when_choosing_then_unsupported_sources_never_enter_sdk() {
    scenario(async {
        // Arrange / Act / Assert: individual components remain legal for file-source adoption.
        let (_bus, host, client) = fixture().await;
        let controller = PanelController::spawn(client);
        ready(&controller).await;
        for (length, valid) in [(1023, true), (1024, true), (1025, false)] {
            let name = format!(
                "{}/{}/{}/{}/{}",
                "a".repeat(250),
                "b".repeat(250),
                "c".repeat(250),
                "d".repeat(250),
                "e".repeat(length - 1004)
            );
            let (_file, source) = opened([name]);
            assert_eq!(valid_selection(&source), valid);
            if !valid {
                assert!(matches!(
                    controller.handle().choose(source),
                    Err(ControllerError::Client(
                        super::super::ClientError::InvalidInput
                    ))
                ));
            }
        }
        for (count, length, valid) in [(255, 257, true), (256, 256, true), (256, 257, false)] {
            let (_file, source) =
                opened((0..count).map(|i| format!("{i:03}/{}", "n".repeat(length - 4))));
            assert_eq!(valid_selection(&source), valid);
        }
        controller.close().await.unwrap();
        let (coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert!(coordinator.selections.is_empty() && coordinator.transfers.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_multiple_fd_batches_when_imported_then_gui_retains_selection_without_sending() {
    scenario(async {
        // Arrange
        let (bus, host, client) = fixture().await;
        let controller = PanelController::spawn(client);
        ready(&controller).await;
        let server = DesktopServer::connect(
            bus.socket().await,
            controller.handle(),
            PreparationGate::new(),
        )
        .await
        .unwrap();
        let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
        let (file, source) = opened((0..17).map(|i| format!("folder/{i:03}")));
        // Act
        cli.import(source).await.unwrap();
        assert_eq!(cli.next, 2);
        assert_eq!(commit(&mut cli, cli_token(&server)).await, "ready");
        assert_eq!(begin(&mut cli).await, cli.token.unwrap());
        cli.close().await.unwrap();
        // Assert
        let state = controller.handle().subscribe().borrow().clone();
        assert_eq!(state.view.as_ref().unwrap().selections.len(), 1);
        assert!(state.view.unwrap().transfers.is_empty());
        assert!(file.path().is_file());
        server.close().await.unwrap();
        controller.close().await.unwrap();
        let (coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert!(coordinator.transfers.is_empty());
        assert_eq!(coordinator.usage().reserved_handles, 0);
    })
    .await;
}
fn cli_token(server: &DesktopServer) -> u64 {
    server
        .handle
        .shared
        .inner
        .lock()
        .unwrap()
        .record
        .as_ref()
        .unwrap()
        .token
}

#[tokio::test]
async fn given_staged_capabilities_when_replayed_or_abused_then_conflicts_do_not_replace_sources() {
    scenario(async {
        // Arrange
        let (bus, host, client) = fixture().await;
        let controller = PanelController::spawn(client);
        ready(&controller).await;
        let server = DesktopServer::connect(
            bus.socket().await,
            controller.handle(),
            PreparationGate::new(),
        )
        .await
        .unwrap();
        let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
        let mut other = DesktopClient::connect(bus.socket().await).await.unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().set_len(7).unwrap();
        let read = File::open(file.path()).unwrap();
        let token = begin(&mut cli).await;
        // Act / Assert
        assert_eq!(append(&mut cli, token, 0, &read, &["a"]).await, Ok(1));
        assert_eq!(append(&mut cli, token, 0, &read, &["a"]).await, Ok(1));
        assert_eq!(
            other.wire("Begin", &()).await.unwrap_err(),
            DesktopError::Busy
        );
        assert_eq!(
            append(&mut other, token, 1, &read, &["b"]).await,
            Err(DesktopError::Denied)
        );
        assert_eq!(
            other.wire("Abort", &token).await.unwrap_err(),
            DesktopError::Denied
        );
        assert_eq!(
            other.wire("Commit", &token).await.unwrap_err(),
            DesktopError::Denied
        );
        for (seq, names, error) in [
            (0, vec!["b"], DesktopError::Conflict),
            (2, vec!["b"], DesktopError::Conflict),
            (1, vec!["a"], DesktopError::InvalidInput),
            (1, vec!["../escape"], DesktopError::InvalidInput),
            (1, vec![], DesktopError::InvalidInput),
        ] {
            assert_eq!(
                append(&mut cli, token, seq, &read, &names).await,
                Err(error)
            );
        }
        // The shared predecode transport closes an oversized-FD sender before dispatch.
        let mut attacker = DesktopClient::connect(bus.socket().await).await.unwrap();
        assert_eq!(
            append(&mut attacker, token, 1, &read, &["b"; 17]).await,
            Err(DesktopError::Transport)
        );
        let _ = attacker.close().await;
        let writable = OpenOptions::new().write(true).open(file.path()).unwrap();
        assert_eq!(
            append(&mut cli, token, 1, &writable, &["b"]).await,
            Err(DesktopError::InvalidInput)
        );
        assert_eq!(
            append(
                &mut cli,
                token,
                1,
                &File::open("/dev/null").unwrap(),
                &["b"]
            )
            .await,
            Err(DesktopError::InvalidInput)
        );
        let replacement = tempfile::NamedTempFile::new().unwrap();
        replacement.as_file().set_len(7).unwrap();
        assert_eq!(
            append(
                &mut cli,
                token,
                0,
                &File::open(replacement.path()).unwrap(),
                &["a"]
            )
            .await,
            Err(DesktopError::Conflict)
        );
        {
            let inner = server.handle.shared.inner.lock().unwrap();
            let record = inner.record.as_ref().unwrap();
            assert_eq!(record.files.len(), 1);
            assert_eq!(record.bytes, 7);
            let fd = record.files[0].clone_handle().unwrap();
            assert!(rustix::io::fcntl_getfd(&fd)
                .unwrap()
                .contains(rustix::io::FdFlags::CLOEXEC));
            assert_eq!(
                rustix::fs::fstat(&fd).unwrap().st_ino,
                rustix::fs::fstat(&read).unwrap().st_ino
            );
        }
        assert_eq!(commit(&mut cli, token).await, "ready");
        assert!(server.handle().subscribe().borrow().show_revision > 0);
        other.close().await.unwrap();
        cli.close().await.unwrap();
        server.close().await.unwrap();
        controller.close().await.unwrap();
        let (coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert!(coordinator.transfers.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_cancelled_wire_waiters_when_recovered_then_same_import_is_prepared_once() {
    scenario(async {
        // Arrange / Act / Assert: real replies are suspended before caller state advances.
        for method in ["Begin", "AppendFiles", "Commit"] {
            for after in [false, true] {
                let (bus, host, client) = fixture().await;
                let controller = PanelController::spawn(client);
                ready(&controller).await;
                let server = DesktopServer::connect(
                    bus.socket().await,
                    controller.handle(),
                    PreparationGate::new(),
                )
                .await
                .unwrap();
                let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
                let reached = Arc::new(Notify::new());
                cli.hook = Some(super::super::ReplyHook {
                    method,
                    after,
                    reached: reached.clone(),
                    release: None,
                });
                let (file, source) = opened((0..17).map(|i| format!("folder/{i}")));
                interrupted(cli.import(source), reached).await;
                assert!(cli.source.is_some());
                assert!(!format!("{cli:?}").contains("folder"));
                cli.recover().await.unwrap();
                assert_eq!(
                    controller
                        .handle()
                        .subscribe()
                        .borrow()
                        .view
                        .as_ref()
                        .unwrap()
                        .selections
                        .len(),
                    1
                );
                assert!(cli.source.is_none());
                assert!(file.path().is_file());
                cli.close().await.unwrap();
                server.close().await.unwrap();
                controller.close().await.unwrap();
                let (coordinator, result) = host.finish().await;
                assert_eq!(result, Ok(()));
                assert!(coordinator.transfers.is_empty());
            }
        }
    })
    .await;
}

#[tokio::test]
async fn given_shared_gate_and_sender_loss_when_staging_aborts_then_resources_release_and_closed_gate_stays_closed(
) {
    scenario(async {
        // Arrange
        let (bus, host, client) = fixture().await;
        let controller = PanelController::spawn(client);
        ready(&controller).await;
        let gate = PreparationGate::new();
        let permit = gate.try_acquire().unwrap();
        let server = DesktopServer::connect(bus.socket().await, controller.handle(), gate.clone())
            .await
            .unwrap();
        let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
        // Act / Assert
        assert_eq!(
            cli.wire("Begin", &()).await.unwrap_err(),
            DesktopError::Busy
        );
        drop(permit);
        let token = begin(&mut cli).await;
        assert!(gate.try_acquire().is_none());
        let result: String = cli
            .wire("Abort", &token)
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!(result, "aborted");
        assert_eq!(commit(&mut cli, token).await, "aborted");
        assert_eq!(begin(&mut cli).await, token);
        assert!(gate.try_acquire().is_some());
        let mut updates = server.handle().subscribe();
        cli.close().await.unwrap();
        // Obtain a new sender after an event-driven barrier on broker cleanup.
        let retained = server.handle.shared.inner.lock().unwrap().record.is_some();
        if retained {
            updates.changed().await.unwrap();
        }
        let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
        begin(&mut cli).await;
        assert!(updates.borrow().staging);
        cli.close().await.unwrap();
        updates.wait_for(|state| !state.staging).await.unwrap();
        assert!(gate.try_acquire().is_some());
        server.close().await.unwrap();
        assert!(gate.try_acquire().is_none());
        controller.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_foreign_uid_or_replaced_gui_when_invoked_then_no_new_owner_receives_pending_sources()
{
    scenario(async {
        // Arrange / Act / Assert
        let (bus, host, client) = fixture().await;
        let controller = PanelController::spawn(client);
        ready(&controller).await;
        let server = DesktopServer::connect_with_uid(
            bus.socket().await,
            controller.handle(),
            PreparationGate::new(),
            rustix::process::geteuid().as_raw().wrapping_add(1),
        )
        .await
        .unwrap();
        let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
        assert_eq!(
            cli.wire("Begin", &()).await.unwrap_err(),
            DesktopError::Denied
        );
        server.close().await.unwrap();
        cli.close().await.unwrap();
        let server = DesktopServer::connect(
            bus.socket().await,
            controller.handle(),
            PreparationGate::new(),
        )
        .await
        .unwrap();
        let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
        let reached = Arc::new(Notify::new());
        cli.hook = Some(super::super::ReplyHook {
            method: "AppendFiles",
            after: false,
            reached: reached.clone(),
            release: None,
        });
        let (_file, source) = opened(["document".into()]);
        interrupted(cli.import(source), reached).await;
        server.close().await.unwrap();
        let replacement = DesktopServer::connect(
            bus.socket().await,
            controller.handle(),
            PreparationGate::new(),
        )
        .await
        .unwrap();
        assert_eq!(cli.recover().await, Err(DesktopError::GuiChanged));
        assert!(cli.source.is_some());
        assert!(!replacement.handle().subscribe().borrow().staging);
        cli.close().await.unwrap();
        replacement.close().await.unwrap();
        controller.close().await.unwrap();
        let (coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert!(coordinator.selections.is_empty() && coordinator.transfers.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_committed_import_when_cli_disconnects_then_owned_worker_finishes_and_shutdown_joins()
{
    scenario(async {
        // Arrange / Act / Assert: admitted worker either completes or is explicitly joined on shutdown.
        for finish in [true, false] {
            let (bus, host, mut client) = fixture().await;
            let reached = crate::service::client::tests::hook(&mut client, "BeginSelection", false);
            let release = Arc::new(Notify::new());
            if finish {
                client.hook.as_mut().unwrap().release = Some(release.clone());
            }
            let controller = PanelController::spawn(client);
            ready(&controller).await;
            let gate = PreparationGate::new();
            let server =
                DesktopServer::connect(bus.socket().await, controller.handle(), gate.clone())
                    .await
                    .unwrap();
            let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
            let (file, source) = opened(["document".into()]);
            interrupted(cli.import(source), reached).await;
            cli.close().await.unwrap();
            assert!(*gate.subscribe().borrow());
            assert!(controller.handle().subscribe().borrow().busy);
            if finish {
                release.notify_one();
                gate.subscribe().wait_for(|busy| !busy).await.unwrap();
                let state = controller.handle().subscribe().borrow().clone();
                assert_eq!(state.view.unwrap().selections.len(), 1);
                assert!(!state.busy && !state.pending);
            }
            controller.close().await.unwrap();
            server.close().await.unwrap();
            assert!(!*gate.subscribe().borrow());
            assert!(gate.try_acquire().is_none());
            let (coordinator, result) = host.finish().await;
            assert_eq!(result, Ok(()));
            assert!(coordinator.transfers.is_empty());
            assert_eq!(coordinator.usage().reserved_handles, 0);
            assert!(file.path().is_file());
        }
    })
    .await;
}

#[tokio::test]
async fn given_full_manifest_or_invalid_commit_when_admitted_then_budgets_are_atomic_and_receipts_retire(
) {
    scenario(async {
        // Arrange / Act / Assert: exact aggregate limit and limit+1 in the final batch.
        for (length, valid) in [(256, true), (257, false)] {
            let (bus, host, client) = fixture().await;
            let controller = PanelController::spawn(client);
            ready(&controller).await;
            let server = DesktopServer::connect(
                bus.socket().await,
                controller.handle(),
                PreparationGate::new(),
            )
            .await
            .unwrap();
            let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
            let file = tempfile::NamedTempFile::new().unwrap();
            let read = File::open(file.path()).unwrap();
            let token = begin(&mut cli).await;
            for sequence in 0..16 {
                let names = (sequence * 16..sequence * 16 + 16)
                    .map(|index| format!("{index:03}/{}", "n".repeat(length - 4)))
                    .collect::<Vec<_>>();
                let names = names.iter().map(String::as_str).collect::<Vec<_>>();
                let outcome = append(&mut cli, token, sequence, &read, &names).await;
                if sequence == 15 && !valid {
                    assert_eq!(outcome, Err(DesktopError::InvalidInput));
                } else {
                    assert_eq!(outcome, Ok(sequence + 1));
                }
            }
            let count = if valid { 256 } else { 240 };
            assert_eq!(
                server
                    .handle
                    .shared
                    .inner
                    .lock()
                    .unwrap()
                    .record
                    .as_ref()
                    .unwrap()
                    .files
                    .len(),
                count
            );
            if valid {
                assert_eq!(
                    append(&mut cli, token, 16, &read, &["extra"]).await,
                    Err(DesktopError::InvalidInput)
                );
                assert_eq!(commit(&mut cli, token).await, "ready");
            } else {
                let status: String = cli
                    .wire("Abort", &token)
                    .await
                    .unwrap()
                    .body()
                    .deserialize()
                    .unwrap();
                assert_eq!(status, "aborted");
                assert_eq!(
                    append(&mut cli, token, 15, &read, &["extra"]).await,
                    Err(DesktopError::Conflict)
                );
            }
            cli.close().await.unwrap();
            server.close().await.unwrap();
            controller.close().await.unwrap();
            let (coordinator, result) = host.finish().await;
            assert_eq!(result, Ok(()));
            assert!(coordinator.transfers.is_empty());
        }
        // Empty or prefix-colliding manifests fail as a whole, with a stable terminal receipt.
        for names in [vec![], vec!["folder", "folder/child"]] {
            let (bus, host, client) = fixture().await;
            let controller = PanelController::spawn(client);
            ready(&controller).await;
            let server = DesktopServer::connect(
                bus.socket().await,
                controller.handle(),
                PreparationGate::new(),
            )
            .await
            .unwrap();
            let mut cli = DesktopClient::connect(bus.socket().await).await.unwrap();
            let token = begin(&mut cli).await;
            let file = tempfile::NamedTempFile::new().unwrap();
            if !names.is_empty() {
                append(
                    &mut cli,
                    token,
                    0,
                    &File::open(file.path()).unwrap(),
                    &names,
                )
                .await
                .unwrap();
            }
            assert_eq!(commit(&mut cli, token).await, "failed");
            assert_eq!(commit(&mut cli, token).await, "failed");
            assert!(controller
                .handle()
                .subscribe()
                .borrow()
                .view
                .as_ref()
                .unwrap()
                .selections
                .is_empty());
            cli.close().await.unwrap();
            server.close().await.unwrap();
            controller.close().await.unwrap();
            assert_eq!(host.finish().await.1, Ok(()));
        }
    })
    .await;
}

#[tokio::test]
async fn given_no_transfer_backend_when_fixed_errors_are_presented_then_import_is_rejected_without_source_authority(
) {
    scenario(async {
        // Arrange
        let private = crate::service::bus::tests::PrivateBus::new();
        let gate = PreparationGate::new();
        let server = DesktopServer::connect_optional(private.socket().await, None, gate.clone())
            .await
            .unwrap();
        let handle = server.handle();
        let mut updates = handle.subscribe();
        let mut client = DesktopClient::connect(private.socket().await)
            .await
            .unwrap();
        // Act / Assert
        for (index, code) in [
            ErrorCode::InvalidSelection,
            ErrorCode::Busy,
            ErrorCode::Recovery,
            ErrorCode::BackendUnavailable,
            ErrorCode::PanelUnavailable,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(ErrorCode::parse(code.wire()), Some(code));
            client.show_error(code).await.unwrap();
            updates
                .wait_for(|state| state.error_revision == index as u64 + 1)
                .await
                .unwrap();
            assert_eq!(updates.borrow().error, Some(code));
            assert_eq!(updates.borrow().show_revision, 0);
            assert!(!updates.borrow().staging && !*gate.subscribe().borrow());
            assert!(!code.message().contains("secret-source"));
        }
        let before = *updates.borrow();
        for code in [
            "",
            "unknown",
            "secret-source",
            "invalid_selection/../",
            "busy\n",
        ] {
            assert_eq!(
                client.wire("ShowError", &code).await.unwrap_err(),
                DesktopError::InvalidInput
            );
            assert_eq!(*updates.borrow(), before);
        }
        let (_file, selection) = opened(["secret-source".into()]);
        assert_eq!(
            client.import(selection).await,
            Err(DesktopError::Unavailable)
        );
        assert!(!*gate.subscribe().borrow());
        assert!(server.handle.shared.inner.lock().unwrap().record.is_none());
        server.close().await.unwrap();
        let replacement =
            DesktopServer::connect_optional(private.socket().await, None, PreparationGate::new())
                .await
                .unwrap();
        assert_eq!(
            client.show_error(ErrorCode::Busy).await,
            Err(DesktopError::GuiChanged)
        );
        assert_eq!(replacement.handle().subscribe().borrow().error, None);
        client.close().await.unwrap();
        replacement.close().await.unwrap();
        let denied = DesktopServer::connect_optional_with_uid(
            private.socket().await,
            None,
            PreparationGate::new(),
            rustix::process::geteuid().as_raw() + 1,
        )
        .await
        .unwrap();
        let mut foreign = DesktopClient::connect(private.socket().await)
            .await
            .unwrap();
        assert_eq!(
            foreign.show_error(ErrorCode::Busy).await,
            Err(DesktopError::Denied)
        );
        foreign.close().await.unwrap();
        denied.close().await.unwrap();
    })
    .await;
}
