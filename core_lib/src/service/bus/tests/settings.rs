use std::sync::{Arc, Condvar, Mutex};

use tokio::sync::oneshot;

use super::*;
use crate::service::client::settings::SettingsEdit;
use crate::service::client::{ClientError, NativeClient};
use crate::service::native_settings::{PersistenceIo, Store, StoreError, SystemIo};
use crate::service::settings_service::{
    SaveIntent, SettingsOwner, SettingsView, INTERFACE as SETTINGS,
};

struct Server {
    stop: CancellationToken,
    task: JoinHandle<Result<(), BusError>>,
    visibility: crate::visibility::VisibilityPolicy,
}
impl Server {
    fn start(bus: SelectionBus) -> Self {
        Self::start_with_identity(bus, None)
    }
    fn start_with_identity(
        mut bus: SelectionBus,
        identity: Option<crate::identity::IdentityPolicy>,
    ) -> Self {
        let stop = CancellationToken::new();
        let end = stop.clone();
        let fixture = crate::service::runtime::tests::fixture_with_identity(identity);
        let visibility = crate::service::runtime::tests::visibility_policy(&fixture);
        let task = tokio::spawn(async move {
            // Capture the whole fixture: its receiver/discovery owners must outlive the runtime borrow.
            let mut fixture = fixture;
            let outcome = bus.run_started(&mut fixture.runtime, end).await;
            let shutdown = fixture.runtime.shutdown().await.map_err(BusError::Runtime);
            bus.disconnect_clients(fixture.runtime.selection_coordinator())
                .unwrap();
            bus.close_connection().await;
            outcome.and(shutdown)
        });
        Self {
            stop,
            task,
            visibility,
        }
    }
    async fn finish(self) {
        self.stop.cancel();
        assert_eq!(self.task.await.unwrap(), Ok(()));
    }
}
fn intent(view: &SettingsView, request: u32) -> SaveIntent {
    SaveIntent {
        epoch: view.epoch.clone(),
        revision: view.revision,
        token: view.token.clone(),
        request: format!("{request:032x}"),
        visibility: 1,
        port: 1234,
        card_mode: 1,
        device_name: view.device_name.clone(),
        directory: 1,
        recover: false,
    }
}
async fn deadline(test: impl Future<Output = ()>) {
    tokio::time::timeout(std::time::Duration::from_secs(30), test)
        .await
        .expect("settings scenario must finish");
}

#[tokio::test]
async fn given_name_save_waiting_for_consumers_when_other_client_reads_then_bus_progresses_until_exact_apply(
) {
    deadline(async {
        use crate::identity::{ConsumerKind, DeviceName, IdentityPolicy};
        let private = PrivateBus::new();
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(
            directory.path().join("native"),
            directory.path().join("legacy"),
        );
        let initial = store.load();
        let identity = IdentityPolicy::new(
            DeviceName::new("Automatic").unwrap(),
            &[ConsumerKind::Mdns, ConsumerKind::Gatt],
        );
        let mut mdns = identity.subscribe(ConsumerKind::Mdns).unwrap();
        let gatt = identity.subscribe(ConsumerKind::Gatt).unwrap();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_settings(store, initial, 123, true, -1).unwrap();
        let server = Server::start_with_identity(bus, Some(identity.clone()));
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let mut observer = NativeClient::connect(private.socket().await).await.unwrap();
        let before = client.read_settings().await.unwrap();
        assert_eq!(before.name_status, 1);
        assert_eq!(before.resolved_name, "Automatic");
        mdns.acknowledge(1).unwrap();
        gatt.acknowledge(1).unwrap();
        let mut edit = intent(&before, 1);
        edit.device_name = "Рабочий ПК".into();
        assert!(!format!("{edit:?}").contains("Рабочий ПК"));
        let save = tokio::spawn(async move {
            let result = client
                .save_settings(SettingsEdit {
                    intent: edit,
                    directory: None,
                })
                .await;
            (client, result)
        });
        mdns.changed().await.unwrap();
        let name = mdns.snapshot();
        assert_eq!(name.name.as_str(), "Рабочий ПК");
        assert!(observer.read_state().await.is_ok());
        let pending = observer.read_settings().await.unwrap();
        assert_eq!(pending.device_name, "Рабочий ПК");
        assert_eq!(pending.name_status, 1);
        assert_eq!(pending.applied_live_revision, 0);
        assert!(!save.is_finished());
        mdns.acknowledge(name.revision).unwrap();
        assert_eq!(observer.read_settings().await.unwrap().name_status, 1);
        gatt.acknowledge(name.revision).unwrap();
        let (mut client, result) = save.await.unwrap();
        let saved = result.unwrap();
        assert_eq!(saved.name_status, 2);
        assert_eq!(saved.applied_live_revision, saved.revision);
        assert_eq!(saved.resolved_name, "Рабочий ПК");
        assert!(saved.valid(123));
        let mut reset = intent(&saved, 2);
        reset.device_name.clear();
        let save = tokio::spawn(async move {
            let result = client
                .save_settings(SettingsEdit {
                    intent: reset,
                    directory: None,
                })
                .await;
            (client, result)
        });
        mdns.changed().await.unwrap();
        let current = mdns.snapshot();
        assert_eq!(current.name.as_str(), "Automatic");
        mdns.acknowledge(current.revision).unwrap();
        gatt.acknowledge(current.revision).unwrap();
        let (mut client, result) = save.await.unwrap();
        let saved = result.unwrap();
        assert!(saved.device_name.is_empty());
        assert_eq!(saved.resolved_name, "Automatic");
        assert_eq!(saved.name_status, 2);
        let mut temporary = intent(&saved, 3);
        temporary.visibility = 2;
        let temporary = client
            .save_settings(SettingsEdit {
                intent: temporary,
                directory: None,
            })
            .await
            .unwrap();
        assert_eq!(
            server.visibility.subscribe().current(),
            crate::Visibility::Temporarily
        );
        server
            .visibility
            .set(crate::Visibility::Invisible, true)
            .unwrap();
        let mut visibility = server.visibility.subscribe();
        visibility.current_and_update();
        let mut only_name = intent(&temporary, 4);
        only_name.visibility = 2;
        only_name.device_name = "After expiry".into();
        let save = tokio::spawn(async move {
            let saved = client
                .save_settings(SettingsEdit {
                    intent: only_name,
                    directory: None,
                })
                .await;
            (client, saved)
        });
        mdns.changed().await.unwrap();
        let current = mdns.snapshot();
        assert_eq!(current.name.as_str(), "After expiry");
        assert!(!visibility.has_changed().unwrap());
        assert_eq!(visibility.current(), crate::Visibility::Invisible);
        mdns.acknowledge(current.revision).unwrap();
        gatt.acknowledge(current.revision).unwrap();
        let (_, saved) = save.await.unwrap();
        assert_eq!(saved.unwrap().name_status, 2);
        assert!(!visibility.has_changed().unwrap());
        assert_eq!(visibility.current(), crate::Visibility::Invisible);
        server.finish().await;
    })
    .await;
}

#[tokio::test]
async fn given_failed_name_consumer_when_saving_or_rejecting_invalid_name_then_saved_state_and_failure_remain_explicit(
) {
    deadline(async {
        use crate::identity::{ConsumerKind, DeviceName, IdentityPolicy};
        let private = PrivateBus::new();
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(
            directory.path().join("native"),
            directory.path().join("legacy"),
        );
        let initial = store.load();
        let identity = IdentityPolicy::new(
            DeviceName::new("Automatic").unwrap(),
            &[ConsumerKind::Mdns, ConsumerKind::Gatt],
        );
        let mut mdns = identity.subscribe(ConsumerKind::Mdns).unwrap();
        let gatt = identity.subscribe(ConsumerKind::Gatt).unwrap();
        drop(gatt);
        mdns.acknowledge(1).unwrap();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_settings(store, initial, 123, true, -1).unwrap();
        let server = Server::start_with_identity(bus, Some(identity.clone()));
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let before = client.read_settings().await.unwrap();
        assert_eq!(before.name_status, 3);
        let mut edit = intent(&before, 1);
        edit.device_name = "Saved name".into();
        let root = directory.path().join("receive-root");
        std::fs::create_dir(&root).unwrap();
        edit.directory = 2;
        let saved = client
            .save_settings(SettingsEdit {
                intent: edit,
                directory: Some(File::open(&root).unwrap()),
            })
            .await
            .unwrap();
        assert_eq!(saved.device_name, "Saved name");
        assert_eq!(saved.name_status, 3);
        assert_eq!(saved.error, 4);
        assert_eq!(saved.applied_live_revision, 0);
        assert_eq!(saved.revision, 1);
        assert!(!client.has_pending_settings());
        assert!(client.read_state().await.is_ok());
        mdns.changed().await.unwrap();
        let current = mdns.snapshot();
        mdns.acknowledge(current.revision).unwrap();
        // A failed name consumer must not discard the already-adopted root capability.
        std::fs::rename(&root, directory.path().join("moved-root")).unwrap();
        let mut retained = intent(&saved, 2);
        retained.directory = 0;
        retained.visibility = 0;
        let saved = client
            .save_settings(SettingsEdit {
                intent: retained,
                directory: None,
            })
            .await
            .unwrap();
        assert_eq!(saved.revision, 2);
        assert_eq!(saved.name_status, 3);
        assert_eq!(
            server.visibility.subscribe().current(),
            crate::Visibility::Visible
        );
        for value in [
            " Name",
            "Name ",
            "a\n",
            "a\u{2069}b",
            &"a".repeat(238),
            &"é".repeat(119),
        ] {
            let mut edit = intent(&saved, 3);
            edit.device_name = value.into();
            assert!(matches!(
                client
                    .save_settings(SettingsEdit {
                        intent: edit,
                        directory: None
                    })
                    .await,
                Err(ClientError::InvalidInput)
            ));
            assert!(!client.has_pending_settings());
        }
        let observed = client.read_settings().await.unwrap();
        assert_eq!(observed.revision, saved.revision);
        assert_eq!(observed.device_name, saved.device_name);
        assert_eq!(observed.name_status, 3);
        server.finish().await;
    })
    .await;
}

#[tokio::test]
async fn given_live_owner_when_saved_then_policy_adoption_port_restart_and_recovery_warning_are_truthful(
) {
    deadline(async {
        // Arrange
        let private = PrivateBus::new();
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(
            directory.path().join("native"),
            directory.path().join("legacy"),
        );
        std::fs::write(directory.path().join("legacy"), br#"{"visibility":"bad"}"#).unwrap();
        let initial = store.load();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_visibility_recovered(true);
        bus.set_settings(store, initial, 123, true, -1).unwrap();
        let server = Server::start(bus);
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let before = client.read_settings().await.unwrap();
        assert!(before.visibility_recovered);
        assert!(client.read_state().await.unwrap().warnings.len() == 1);
        // Act
        let saved = client
            .save_settings(SettingsEdit {
                intent: intent(&before, 1),
                directory: None,
            })
            .await
            .unwrap();
        // Assert
        assert_eq!(saved.revision, 1);
        assert_eq!(saved.applied_live_revision, 1);
        assert_eq!(saved.visibility, 1);
        assert_eq!(saved.card_mode, 1);
        assert_eq!(saved.error, 0);
        assert!(saved.restart_required);
        assert!(!saved.visibility_recovered);
        assert!(client.read_state().await.unwrap().warnings.is_empty());
        assert!(!client.has_pending_settings());
        assert_eq!(SettingsView::SIGNATURE.to_string(), "(sbybtsyiysbtbbysssy)");
        assert_eq!(SaveIntent::SIGNATURE.to_string(), "(stssyiyybs)");
        assert!(INTROSPECTION.contains("type=\"(sbybtsyiysbtbbysssy)\""));
        let mut changed = intent(&saved, 2);
        changed.port = -1;
        changed.directory = 2;
        let selected = directory.path().join("selected");
        std::fs::create_dir(&selected).unwrap();
        let applied = client
            .save_settings(SettingsEdit {
                intent: changed,
                directory: Some(File::open(&selected).unwrap()),
            })
            .await
            .unwrap();
        assert!(!applied.restart_required);
        assert_eq!(applied.applied_live_revision, 2);
        let mut automatic = intent(&applied, 3);
        automatic.port = 0;
        automatic.directory = 0;
        let automatic = client
            .save_settings(SettingsEdit {
                intent: automatic,
                directory: None,
            })
            .await
            .unwrap();
        assert!(!automatic.restart_required);
        assert_eq!(automatic.applied_live_revision, 3);
        assert_eq!(applied.directory_display, selected.to_string_lossy());
        assert!(!format!("{applied:?}").contains(selected.to_str().unwrap()));
        let upload = client.begin_selection().await.unwrap();
        assert_eq!(
            client.restart_service().await,
            Err(ClientError::Rejected(Error::Busy))
        );
        client.abort_selection(&upload).await.unwrap();
        client.restart_service().await.unwrap();
        client.close().await.unwrap();
        server.finish().await;
    })
    .await;
}

#[tokio::test]
async fn given_settings_intent_when_epoch_types_or_fd_cardinality_are_invalid_then_no_native_write_is_admitted(
) {
    deadline(async {
        // Arrange
        let private = PrivateBus::new(); let directory = tempfile::tempdir().unwrap();
        let store = Store::new(directory.path().join("native"), directory.path().join("legacy"));
        let initial = store.load();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_settings(store, initial, 123, false, -1).unwrap();
        let server = Server::start(bus);
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let before = client.read_settings().await.unwrap();
        let raw = private.client().await;
        let fd = File::open(directory.path()).unwrap();
        for case in 0..9 {
            let mut edit = intent(&before, 1);
            let files = match case { 6 | 8 => vec![Fd::from(fd.as_fd())], 7 => vec![Fd::from(fd.as_fd()), Fd::from(fd.as_fd())], _ => vec![] };
            match case { 0 => edit.epoch = format!("{:032x}", 124), 1 => edit.visibility = 3, 2 => edit.port = 65536, 3 => edit.card_mode = 2, 4 => edit.directory = 3, 5 => edit.directory = 2, 6 => (), 7 => edit.directory = 2, 8 => edit.token = "bad".into(), _ => unreachable!() }
            // Act / Assert
            let result = raw.call_method(Some(SERVICE), PATH, Some(SETTINGS), "Save", &(edit, files)).await;
            assert!(matches!(result, Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == error_name(Error::InvalidInput)));
        }
        for (path, method) in [(PATH, "Read"), (PATH, "Reload"), ("/wrong", "Read")] {
            assert!(raw.call_method(Some(SERVICE), path, Some(SETTINGS), method, &vec![Fd::from(fd.as_fd())]).await.is_err());
        }
        assert!(!directory.path().join("native").exists());
        assert_eq!(client.read_settings().await.unwrap(), before);
        raw.close().await.unwrap(); client.close().await.unwrap(); server.finish().await;
    }).await;
}

#[tokio::test]
async fn given_unavailable_owner_or_foreign_uid_when_settings_are_requested_then_unavailability_and_authentication_are_explicit(
) {
    deadline(async {
        // Arrange
        let private = PrivateBus::new();
        let server = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        // Act / Assert
        let view = client.read_settings().await.unwrap(); assert!(!view.available); assert!(!view.configured);
        assert_eq!(client.reload_settings().await, Err(ClientError::Rejected(Error::NotRunning)));
        client.close().await.unwrap(); server.finish().await.1.unwrap();
        let wrong_uid = rustix::process::geteuid().as_raw().wrapping_add(1);
        let server = Harness::start(SelectionBus::connect_with_uid(private.socket().await, wrong_uid).await.unwrap());
        let raw = private.client().await;
        let result = raw.call_method(Some(SERVICE), PATH, Some(SETTINGS), "Read", &()).await;
        assert!(matches!(result, Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == "org.freedesktop.DBus.Error.AccessDenied"));
        raw.close().await.unwrap(); server.finish().await.1.unwrap();
    }).await;
}

#[tokio::test]
async fn given_lost_save_response_when_retried_then_same_intent_receipt_is_recovered_and_intervening_edits_conflict(
) {
    deadline(async {
        // Arrange
        let private = PrivateBus::new();
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(
            directory.path().join("native"),
            directory.path().join("legacy"),
        );
        let initial = store.load();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_settings(store, initial, 123, true, -1).unwrap();
        let server = Server::start(bus);
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let view = client.read_settings().await.unwrap();
        let reached = crate::service::client::tests::hook(&mut client, "Save", true);
        // Act: drop only the wait after the actual service reply, preserving the SDK.
        crate::service::client::tests::interrupted(
            client.save_settings(SettingsEdit {
                intent: intent(&view, 1),
                directory: None,
            }),
            reached,
        )
        .await;
        assert!(client.has_pending_settings());
        let recovered = client.retry_settings().await.unwrap();
        // Assert
        assert_eq!(recovered.revision, 1);
        assert_eq!(recovered.applied_live_revision, 1);
        assert!(!client.has_pending_settings());
        assert_eq!(
            client
                .save_settings(SettingsEdit {
                    intent: intent(&view, 2),
                    directory: None
                })
                .await,
            Err(ClientError::Settings(StoreError::Conflict))
        );
        assert!(!client.has_pending_settings());
        assert_eq!(client.read_settings().await.unwrap().revision, 1);
        client.close().await.unwrap();
        server.finish().await;
    })
    .await;
}

struct BlockingIo {
    entered: Mutex<Option<oneshot::Sender<()>>>,
    gate: Arc<(Mutex<bool>, Condvar)>,
}
impl PersistenceIo for BlockingIo {
    fn write(&self, file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(entered) = self.entered.lock().unwrap().take() {
            let _ = entered.send(());
            let (ready, condition) = &*self.gate;
            let mut ready = ready.lock().unwrap();
            while !*ready {
                ready = condition.wait(ready).unwrap();
            }
        }
        SystemIo.write(file, bytes)
    }
    fn sync_file(&self, file: &File) -> std::io::Result<()> {
        SystemIo.sync_file(file)
    }
    fn rename(&self, directory: &File, from: &str, to: &str) -> std::io::Result<()> {
        SystemIo.rename(directory, from, to)
    }
    fn sync_directory(&self, directory: &File) -> std::io::Result<()> {
        SystemIo.sync_directory(directory)
    }
}
struct Release(Arc<(Mutex<bool>, Condvar)>);
impl Drop for Release {
    fn drop(&mut self) {
        let (ready, condition) = &*self.0;
        *ready.lock().unwrap() = true;
        condition.notify_all();
    }
}
#[tokio::test]
async fn given_blocked_disk_and_disconnected_requester_when_owner_stops_then_serial_dispatch_continues_and_admitted_write_is_joined(
) {
    deadline(async {
        // Arrange
        let private = PrivateBus::new(); let directory = tempfile::tempdir().unwrap();
        let gate = Arc::new((Mutex::new(false), Condvar::new())); let release = Release(gate.clone());
        let (entered, waiting) = oneshot::channel();
        let store = Store::with_io(directory.path().join("native"), directory.path().join("legacy"), BlockingIo { entered: Mutex::new(Some(entered)), gate });
        let initial = store.load(); let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        let connection = bus.connection.as_ref().unwrap().clone();
        bus.settings = Some(SettingsOwner::spawn_with_io(store, initial, 123, false, -1, connection));
        let server = Server::start(bus);
        let mut requester = NativeClient::connect(private.socket().await).await.unwrap();
        let view = requester.read_settings().await.unwrap();
        {
            let save = requester.save_settings(SettingsEdit { intent: intent(&view, 1), directory: None }); tokio::pin!(save);
            tokio::select! { _ = waiting => (), result = &mut save => panic!("write must reach barrier: {}", result.is_ok()) }
        }
        requester.close().await.unwrap();
        // Act / Assert: the same serial service continues processing real captures.
        let mut observer = NativeClient::connect(private.socket().await).await.unwrap();
        assert_eq!(observer.read_state().await.unwrap().epoch, 123);
        assert_eq!(observer.read_settings().await.unwrap().revision, 0);
        assert_eq!(observer.restart_service().await, Err(ClientError::Rejected(Error::Busy)));
        server.stop.cancel();
        let mut task = server.task;
        assert!(futures::poll!(&mut task).is_pending());
        drop(release);
        assert_eq!(task.await.unwrap(), Ok(()));
        let persisted = Store::new(directory.path().join("native"), directory.path().join("legacy")).load().unwrap();
        assert_eq!(persisted.base.revision, 1); assert_eq!(persisted.last_request(), Some("00000000000000000000000000000001"));
        observer.close().await.unwrap();
    }).await;
}

fn visibility_intent(
    view: &SettingsView,
    request: u32,
    mode: u8,
) -> crate::service::visibility_service::VisibilityIntent {
    crate::service::visibility_service::VisibilityIntent {
        epoch: view.epoch.clone(),
        revision: view.revision,
        token: view.token.clone(),
        request: format!("{request:032x}"),
        mode,
    }
}

#[tokio::test]
async fn given_live_visibility_when_expired_retried_renewed_or_competing_then_current_policy_and_saved_intent_stay_distinct(
) {
    tokio::time::timeout(std::time::Duration::from_secs(180), async {
        // Arrange: actual policy worker and private broker, no discovery transport.
        let private = PrivateBus::new();
        let directory = tempfile::tempdir().unwrap();
        let store = Store::new(
            directory.path().join("native"),
            directory.path().join("legacy"),
        );
        let initial = store.load();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_settings(store, initial, 123, true, -1).unwrap();
        let server = Server::start(bus);
        let lifetime = CancellationToken::new();
        let worker = tokio::spawn(server.visibility.start(lifetime.clone()).unwrap());
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let mut observer = NativeClient::connect(private.socket().await).await.unwrap();
        let raw = private.client().await;
        let proxy = zbus::Proxy::new(
            &raw,
            SERVICE,
            PATH,
            crate::service::visibility_service::INTERFACE,
        )
        .await
        .unwrap();
        let mut changes = proxy.receive_signal("Changed").await.unwrap();
        let mut policy = server.visibility.subscribe();
        let before = client.read_settings().await.unwrap();
        let exact = visibility_intent(&before, 100, 2);
        let reached = crate::service::client::tests::hook(&mut client, "Set", true);
        // Act: lose a completed reply, then expire the real core timer before retry.
        crate::service::client::tests::interrupted(client.set_visibility(exact), reached).await;
        assert!(client.has_pending_visibility());
        assert_eq!(
            client
                .save_settings(SettingsEdit {
                    intent: intent(&before, 101),
                    directory: None
                })
                .await,
            Err(ClientError::PendingOperation)
        );
        assert_eq!(policy.current_and_update(), crate::Visibility::Temporarily);
        let prior = client.read_visibility().await.unwrap();
        assert!(prior.running);
        assert_eq!(prior.mode, 2);
        tokio::time::pause();
        tokio::time::advance(std::time::Duration::from_secs(60)).await;
        policy.changed().await.unwrap();
        tokio::time::resume();
        assert_eq!(policy.current_and_update(), crate::Visibility::Invisible);
        let expired = observer.read_visibility().await.unwrap();
        loop {
            let message = changes.next().await.unwrap();
            let (epoch, revision): (String, u64) = message.body().deserialize().unwrap();
            assert_eq!(epoch, expired.epoch);
            if revision == expired.revision {
                break;
            }
            assert!(revision < expired.revision);
        }
        assert_eq!(expired.mode, 1);
        assert!(expired.revision > prior.revision);
        let recovered = client.retry_visibility().await.unwrap();
        // Assert: replay does not extend/reopen, while a fresh explicit choice does.
        assert!(!client.has_pending_visibility());
        assert_eq!(recovered.revision, 1);
        assert_eq!(recovered.visibility, 2);
        assert_eq!(client.read_visibility().await.unwrap(), expired);
        let fresh = client
            .set_visibility(visibility_intent(&recovered, 102, 2))
            .await
            .unwrap();
        let renewed = client.read_visibility().await.unwrap();
        assert_eq!(renewed.mode, 2);
        assert!(renewed.revision > expired.revision);
        let again = client
            .set_visibility(visibility_intent(&fresh, 103, 2))
            .await
            .unwrap();
        let again_policy = client.read_visibility().await.unwrap();
        assert!(again_policy.revision > renewed.revision);
        let mut full = intent(&again, 104);
        full.visibility = 2;
        let full = client
            .save_settings(SettingsEdit {
                intent: full,
                directory: None,
            })
            .await
            .unwrap();
        let sampled = client.read_visibility().await.unwrap();
        assert!(sampled.remaining_ms > 0 && sampled.remaining_ms <= again_policy.remaining_ms);
        assert_eq!(
            super::super::super::visibility_service::VisibilityView {
                remaining_ms: again_policy.remaining_ms,
                ..sampled
            },
            again_policy
        ); // Metadata is unchanged; a read cannot freeze or renew the countdown.
        assert_eq!(
            client
                .set_visibility(visibility_intent(&again, 105, 0))
                .await,
            Err(ClientError::Settings(StoreError::Conflict))
        );
        let hidden = client
            .set_visibility(visibility_intent(&full, 106, 1))
            .await
            .unwrap();
        assert_eq!(client.read_visibility().await.unwrap().mode, 1);
        client
            .set_visibility(visibility_intent(&hidden, 107, 0))
            .await
            .unwrap();
        assert_eq!(client.read_visibility().await.unwrap().mode, 0);
        // The authenticated watcher admits policy invalidations even without Manager2 changes.
        observer.next_change().await.unwrap();
        assert_eq!(observer.read_visibility().await.unwrap().mode, 0);
        let controller = crate::service::client::controller::PanelController::spawn(observer);
        let handle = controller.handle();
        let mut updates = handle.subscribe();
        updates
            .wait_for(|state| state.visibility.is_some())
            .await
            .unwrap();
        handle.set_visibility(2).unwrap().wait().await.unwrap();
        assert_eq!(updates.borrow().visibility.as_ref().unwrap().mode, 2);
        assert!(updates
            .borrow()
            .visibility_sampled_at
            .is_some_and(|sample| sample <= std::time::Instant::now()));
        assert!(updates.borrow().visibility.as_ref().unwrap().remaining_ms <= 60_000);
        policy.current_and_update();
        tokio::time::pause();
        tokio::time::advance(std::time::Duration::from_secs(60)).await;
        policy.changed().await.unwrap();
        tokio::time::resume();
        updates
            .wait_for(|state| state.visibility.as_ref().is_some_and(|view| view.mode == 1))
            .await
            .unwrap();
        assert_eq!(updates.borrow().settings.as_ref().unwrap().visibility, 2);
        controller.close().await.unwrap();
        lifetime.cancel();
        worker.await.unwrap();
        assert!(!client.read_visibility().await.unwrap().running);
        assert_eq!(client.read_visibility().await.unwrap().mode, 1);
        drop(changes);
        drop(proxy);
        raw.close().await.unwrap();
        client.close().await.unwrap();
        server.finish().await;
    })
    .await
    .expect("visibility scenario must finish including the simulated minute");
}

#[tokio::test]
async fn given_visibility_facade_when_malformed_unavailable_or_foreign_then_no_mutation_crosses_authentication_or_type_gates(
) {
    deadline(async {
        // Arrange
        use crate::service::visibility_service::INTERFACE as VISIBILITY;
        let private = PrivateBus::new(); let directory = tempfile::tempdir().unwrap();
        let store = Store::new(directory.path().join("native"), directory.path().join("legacy"));
        let initial = store.load(); let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_settings(store, initial, 123, true, -1).unwrap(); let server = Server::start(bus);
        let lifetime = CancellationToken::new(); let worker = tokio::spawn(server.visibility.start(lifetime.clone()).unwrap());
        let mut client = NativeClient::connect(private.socket().await).await.unwrap(); let before = client.read_settings().await.unwrap();
        let raw = private.client().await;
        // Act / Assert
        for case in 0..5 {
            let mut edit = visibility_intent(&before, 100, 2);
            match case { 0 => edit.epoch = format!("{:032x}", 124), 1 => edit.mode = 3,
                2 => edit.request = "bad".into(), 3 => edit.token = "bad".into(), 4 => edit.mode = u8::MAX, _ => unreachable!() }
            let result = raw.call_method(Some(SERVICE), PATH, Some(VISIBILITY), "Set", &edit).await;
            assert!(matches!(result, Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == error_name(Error::InvalidInput)));
        }
        let fd = File::open(directory.path()).unwrap();
        for method in ["Set", "Read"] {
            let result = raw.call_method(Some(SERVICE), PATH, Some(VISIBILITY), method, &vec![Fd::from(fd.as_fd())]).await;
            assert!(matches!(result, Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == error_name(Error::InvalidInput)));
        }
        assert_eq!(client.read_settings().await.unwrap(), before); assert!(!directory.path().join("native").exists());
        lifetime.cancel(); worker.await.unwrap();
        assert_eq!(client.set_visibility(visibility_intent(&before, 100, 2)).await, Err(ClientError::Rejected(Error::NotRunning)));
        raw.close().await.unwrap(); client.close().await.unwrap(); server.finish().await;
        let server = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let view = client.read_visibility().await.unwrap(); assert!(!view.available); assert!(!view.running); assert_eq!(view.mode, 1);
        client.close().await.unwrap(); server.finish().await.1.unwrap();
        let wrong = rustix::process::geteuid().as_raw().wrapping_add(1);
        let server = Harness::start(SelectionBus::connect_with_uid(private.socket().await, wrong).await.unwrap());
        let raw = private.client().await;
        let result = raw.call_method(Some(SERVICE), PATH, Some(VISIBILITY), "Read", &()).await;
        assert!(matches!(result, Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == "org.freedesktop.DBus.Error.AccessDenied"));
        raw.close().await.unwrap(); server.finish().await.1.unwrap();
    }).await;
}

struct UncertainVisibilityIo(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl PersistenceIo for UncertainVisibilityIo {
    fn write(&self, file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
        SystemIo.write(file, bytes)
    }
    fn sync_file(&self, file: &File) -> std::io::Result<()> {
        SystemIo.sync_file(file)
    }
    fn rename(&self, directory: &File, from: &str, to: &str) -> std::io::Result<()> {
        SystemIo.rename(directory, from, to)
    }
    fn sync_directory(&self, directory: &File) -> std::io::Result<()> {
        if self.0.load(std::sync::atomic::Ordering::SeqCst) {
            Err(std::io::ErrorKind::StorageFull.into())
        } else {
            SystemIo.sync_directory(directory)
        }
    }
}
#[tokio::test]
async fn given_uncertain_visibility_write_when_exactly_retried_then_saved_receipt_does_not_enable_discovery(
) {
    deadline(async {
        // Arrange
        let private = PrivateBus::new();
        let directory = tempfile::tempdir().unwrap();
        let fail = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let store = Store::with_io(
            directory.path().join("native"),
            directory.path().join("legacy"),
            UncertainVisibilityIo(fail.clone()),
        );
        let initial = store.load();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.settings = Some(SettingsOwner::spawn_with_io(
            store,
            initial,
            123,
            true,
            -1,
            bus.connection.as_ref().unwrap().clone(),
        ));
        let server = Server::start(bus);
        server
            .visibility
            .set(crate::Visibility::Invisible, true)
            .unwrap();
        let lifetime = CancellationToken::new();
        let worker = tokio::spawn(server.visibility.start(lifetime.clone()).unwrap());
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let before = client.read_settings().await.unwrap();
        let policy_before = client.read_visibility().await.unwrap();
        // Act / Assert
        assert_eq!(
            client
                .set_visibility(visibility_intent(&before, 100, 2))
                .await,
            Err(ClientError::Settings(StoreError::Indeterminate))
        );
        assert!(client.has_pending_visibility());
        assert_eq!(client.read_visibility().await.unwrap(), policy_before);
        fail.store(false, std::sync::atomic::Ordering::SeqCst);
        let recovered = client.retry_visibility().await.unwrap();
        assert_eq!(recovered.visibility, 2);
        assert_eq!(recovered.error, 4);
        assert_eq!(recovered.applied_live_revision, 0);
        assert_eq!(client.read_visibility().await.unwrap(), policy_before);
        client
            .set_visibility(visibility_intent(&recovered, 101, 2))
            .await
            .unwrap();
        assert_eq!(client.read_visibility().await.unwrap().mode, 2);
        lifetime.cancel();
        worker.await.unwrap();
        client.close().await.unwrap();
        server.finish().await;
    })
    .await;
}
