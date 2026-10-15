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
        let task = tokio::spawn(async move {
            let mut fixture = crate::service::runtime::tests::fixture_with_identity(identity);
            let outcome = bus.run_started(&mut fixture.runtime, end).await;
            let shutdown = fixture.runtime.shutdown().await.map_err(BusError::Runtime);
            bus.disconnect_clients(fixture.runtime.selection_coordinator())
                .unwrap();
            bus.close_connection().await;
            outcome.and(shutdown)
        });
        Self { stop, task }
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
        let (_, result) = save.await.unwrap();
        let saved = result.unwrap();
        assert!(saved.device_name.is_empty());
        assert_eq!(saved.resolved_name, "Automatic");
        assert_eq!(saved.name_status, 2);
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
        let saved = client
            .save_settings(SettingsEdit {
                intent: edit,
                directory: None,
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
        for value in [
            " Name",
            "Name ",
            "a\n",
            "a\u{2069}b",
            &"a".repeat(238),
            &"é".repeat(119),
        ] {
            let mut edit = intent(&saved, 2);
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
