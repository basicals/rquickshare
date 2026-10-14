use super::*;
use std::sync::{Arc, Mutex};
struct State {
    enabled: bool,
    mode: Option<String>,
    fragment: String,
    dropins: Vec<String>,
    deny: bool,
    noop: bool,
    calls: Vec<(Vec<String>, bool, bool)>,
    subscribed: usize,
    block: Option<Arc<Block>>,
}
struct Block {
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
    done: tokio::sync::Notify,
}
impl Block {
    fn new() -> Self {
        Self {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            done: tokio::sync::Notify::new(),
        }
    }
}
impl Default for State {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: None,
            fragment: "/usr/lib/systemd/user/rquickshare-desktop.service".into(),
            dropins: Vec::new(),
            deny: false,
            noop: false,
            calls: Vec::new(),
            subscribed: 0,
            block: None,
        }
    }
}
struct Fake(Arc<Mutex<State>>);
#[zbus::interface(name = "org.freedesktop.systemd1.Manager")]
impl Fake {
    fn subscribe(&self) {
        self.0.lock().unwrap().subscribed += 1;
    }
    fn load_unit(&self, name: &str) -> zbus::fdo::Result<OwnedObjectPath> {
        if name != UNIT {
            return Err(zbus::fdo::Error::InvalidArgs("Unknown unit".into()));
        }
        Ok(UNIT_PATH.try_into().unwrap())
    }
    fn get_unit_file_state(&self, name: &str) -> zbus::fdo::Result<String> {
        if name != UNIT {
            return Err(zbus::fdo::Error::InvalidArgs("Unknown unit".into()));
        }
        let state = self.0.lock().unwrap();
        Ok(state.mode.clone().unwrap_or_else(|| {
            if state.enabled {
                "enabled".into()
            } else {
                "disabled".into()
            }
        }))
    }
    async fn enable_unit_files(
        &self,
        files: Vec<String>,
        runtime: bool,
        force: bool,
    ) -> zbus::fdo::Result<(bool, Vec<(String, String, String)>)> {
        if files != [UNIT] || runtime || force {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Unexpected enable contract".into(),
            ));
        }
        let block = {
            let mut state = self.0.lock().unwrap();
            state.calls.push((files, runtime, force));
            if state.deny {
                return Err(zbus::fdo::Error::AccessDenied("Denied".into()));
            }
            state.block.clone()
        };
        if let Some(block) = &block {
            block.started.notify_one();
            block.release.notified().await;
        }
        {
            let mut state = self.0.lock().unwrap();
            if !state.noop {
                state.enabled = true;
            }
        }
        if let Some(block) = block {
            block.done.notify_one();
        }
        Ok((
            true,
            vec![("symlink".into(), "/test/link".into(), "/test/unit".into())],
        ))
    }
    fn disable_unit_files(
        &self,
        files: Vec<String>,
        runtime: bool,
    ) -> zbus::fdo::Result<Vec<(String, String, String)>> {
        if files != [UNIT] || runtime {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Unexpected disable contract".into(),
            ));
        }
        let mut state = self.0.lock().unwrap();
        state.calls.push((files, runtime, false));
        if state.deny {
            return Err(zbus::fdo::Error::AccessDenied("Denied".into()));
        }
        state.enabled = false;
        Ok(vec![("unlink".into(), "/test/link".into(), String::new())])
    }
    #[zbus(signal)]
    async fn unit_files_changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::Result<()>;
}
struct Unit(Arc<Mutex<State>>);
#[zbus::interface(name = "org.freedesktop.systemd1.Unit")]
impl Unit {
    #[zbus(property)]
    fn fragment_path(&self) -> String {
        self.0.lock().unwrap().fragment.clone()
    }
    #[zbus(property)]
    fn drop_in_paths(&self) -> Vec<String> {
        self.0.lock().unwrap().dropins.clone()
    }
    #[zbus(property)]
    fn load_state(&self) -> &str {
        "loaded"
    }
}
async fn fixture(
    state: State,
) -> (
    crate::test_bus::Broker,
    Connection,
    Arc<Mutex<State>>,
    Owner,
) {
    let broker = crate::test_bus::Broker::new();
    let connection = broker.connection().await;
    let state = Arc::new(Mutex::new(state));
    connection
        .object_server()
        .at(PATH, Fake(state.clone()))
        .await
        .unwrap();
    connection
        .object_server()
        .at(UNIT_PATH, Unit(state.clone()))
        .await
        .unwrap();
    connection.request_name(SERVICE).await.unwrap();
    let owner = Owner::connect(&broker.address, CancellationToken::new()).await;
    (broker, connection, state, owner)
}
#[tokio::test]
async fn given_actual_manager_states_when_read_then_only_verified_persistent_vendor_enablement_is_editable(
) {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        for (mode, fragment, dropins, expected) in [
            (
                "enabled",
                "/usr/lib/systemd/user/rquickshare-desktop.service",
                vec![],
                Ok(true),
            ),
            (
                "disabled",
                "/lib/systemd/user/rquickshare-desktop.service",
                vec![],
                Ok(false),
            ),
            (
                "enabled-runtime",
                "/usr/lib/systemd/user/rquickshare-desktop.service",
                vec![],
                Err(Error::Unsupported),
            ),
            (
                "masked",
                "/usr/lib/systemd/user/rquickshare-desktop.service",
                vec![],
                Err(Error::Unsupported),
            ),
            (
                "static",
                "/usr/lib/systemd/user/rquickshare-desktop.service",
                vec![],
                Err(Error::Unsupported),
            ),
            (
                "linked",
                "/usr/lib/systemd/user/rquickshare-desktop.service",
                vec![],
                Err(Error::Unsupported),
            ),
            (
                "future",
                "/usr/lib/systemd/user/rquickshare-desktop.service",
                vec![],
                Err(Error::Unsupported),
            ),
            (
                "disabled",
                "/home/custom.service",
                vec![],
                Err(Error::Unsupported),
            ),
            (
                "enabled",
                "/usr/lib/systemd/user/rquickshare-desktop.service",
                vec!["/test/override"],
                Err(Error::Unsupported),
            ),
        ] {
            let (_broker, connection, state, owner) = fixture(State {
                mode: Some(mode.into()),
                fragment: fragment.into(),
                dropins: dropins.into_iter().map(str::to_owned).collect(),
                ..State::default()
            })
            .await;
            let handle = owner.handle();
            assert_eq!(handle.read().unwrap().wait().await, expected);
            assert_eq!(handle.subscribe().borrow().enabled, expected.ok());
            assert!(state.lock().unwrap().calls.is_empty());
            assert_eq!(state.lock().unwrap().subscribed, 1);
            owner.close().await;
            connection.close().await.unwrap();
        }
    })
    .await
    .expect("Private manager scenario did not complete");
}
#[tokio::test]
async fn given_explicit_login_edit_when_stale_denied_or_readback_disagrees_then_no_guessed_success_or_automatic_write_retry(
) {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (_broker, connection, state, owner) = fixture(State::default()).await;
        let handle = owner.handle();
        assert_eq!(handle.read().unwrap().wait().await, Ok(false));
        assert!(state.lock().unwrap().calls.is_empty());
        assert_eq!(handle.set(false, false).unwrap().wait().await, Ok(false));
        assert!(state.lock().unwrap().calls.is_empty());
        assert_eq!(
            handle.set(true, false).unwrap().wait().await,
            Err(Error::Conflict)
        );
        assert!(state.lock().unwrap().calls.is_empty());
        assert_eq!(handle.set(false, true).unwrap().wait().await, Ok(true));
        assert_eq!(state.lock().unwrap().calls.len(), 1);
        assert_eq!(handle.set(true, false).unwrap().wait().await, Ok(false));
        assert_eq!(state.lock().unwrap().calls.len(), 2);
        state.lock().unwrap().deny = true;
        assert_eq!(
            handle.set(false, true).unwrap().wait().await,
            Err(Error::Denied)
        );
        assert_eq!(handle.subscribe().borrow().enabled, None);
        assert_eq!(handle.read().unwrap().wait().await, Ok(false));
        assert_eq!(state.lock().unwrap().calls.len(), 3);
        {
            let mut state = state.lock().unwrap();
            state.deny = false;
            state.noop = true;
        }
        assert_eq!(
            handle.set(false, true).unwrap().wait().await,
            Err(Error::Conflict)
        );
        assert_eq!(handle.subscribe().borrow().enabled, None);
        assert_eq!(handle.read().unwrap().wait().await, Ok(false));
        assert_eq!(state.lock().unwrap().calls.len(), 4);
        owner.close().await;
        connection.close().await.unwrap();
    })
    .await
    .expect("Private manager scenario did not complete");
}
#[tokio::test]
async fn given_admitted_login_change_when_ui_wait_is_dropped_then_owner_completes_and_external_changes_are_observed(
) {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let block = Arc::new(Block::new());
        let (_broker, connection, state, owner) = fixture(State {
            block: Some(block.clone()),
            ..State::default()
        })
        .await;
        let handle = owner.handle();
        handle.read().unwrap().wait().await.unwrap();
        drop(handle.set(false, true).unwrap());
        block.started.notified().await;
        assert!(handle.subscribe().borrow().busy);
        block.release.notify_one();
        block.done.notified().await;
        let mut view = handle.subscribe();
        view.wait_for(|view| view.enabled == Some(true) && !view.busy)
            .await
            .unwrap();
        assert_eq!(state.lock().unwrap().calls.len(), 1);
        state.lock().unwrap().enabled = false;
        let emitter = zbus::object_server::SignalEmitter::new(&connection, PATH).unwrap();
        Fake::unit_files_changed(&emitter).await.unwrap();
        view.wait_for(|view| view.enabled == Some(false) && !view.busy)
            .await
            .unwrap();
        assert_eq!(state.lock().unwrap().calls.len(), 1);
        owner.close().await;
        connection.close().await.unwrap();
    })
    .await
    .expect("Private manager scenario did not complete");
}
#[tokio::test]
async fn given_blocked_login_write_when_queue_fills_and_owner_closes_then_pending_work_is_bounded_and_cannot_stop_shared_processes(
) {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let block = Arc::new(Block::new());
        let (_broker, connection, state, owner) = fixture(State {
            block: Some(block.clone()),
            ..State::default()
        })
        .await;
        let handle = owner.handle();
        handle.read().unwrap().wait().await.unwrap();
        let completion = handle.set(false, true).unwrap();
        block.started.notified().await;
        let mut queued = Vec::new();
        for _ in 0..8 {
            queued.push(handle.read().unwrap());
        }
        assert!(matches!(handle.read(), Err(Error::Busy)));
        owner.close().await;
        assert_eq!(completion.wait().await, Err(Error::Closed));
        for completion in queued {
            assert_eq!(completion.wait().await, Err(Error::Closed));
        }
        // Closing the local owner cannot retract an OS change already admitted externally.
        block.release.notify_one();
        block.done.notified().await;
        assert!(state.lock().unwrap().enabled);
        assert_eq!(state.lock().unwrap().calls.len(), 1);
        assert!(matches!(handle.read(), Err(Error::Closed)));
        connection.close().await.unwrap();
    })
    .await
    .expect("Private manager scenario did not complete");
}
#[test]
fn given_manager_reply_when_sender_or_descriptor_is_invalid_then_reject_before_interpreting_body() {
    let call = Message::method_call(PATH, "Test")
        .unwrap()
        .build(&())
        .unwrap();
    for sender in [":1.7", ":1.8"] {
        let reply = Message::method_return(&call.header())
            .unwrap()
            .sender(sender)
            .unwrap()
            .build(&())
            .unwrap();
        assert_eq!(
            checked(&reply, ":1.7"),
            if sender == ":1.7" {
                Ok(())
            } else {
                Err(Error::Protocol)
            }
        );
    }
    let file = tempfile::tempfile().unwrap();
    let reply = Message::method_return(&call.header())
        .unwrap()
        .sender(":1.7")
        .unwrap()
        .build(&zbus::zvariant::Fd::from(&file))
        .unwrap();
    assert_eq!(checked(&reply, ":1.7"), Err(Error::Protocol));
}

#[tokio::test]
async fn given_missing_or_lost_manager_when_login_work_is_requested_then_authority_closes_without_activation_or_retry(
) {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // Arrange / Act / Assert: no manager is ever activated by the settings reader.
        let broker = crate::test_bus::Broker::new();
        let absent = Owner::connect(&broker.address, CancellationToken::new()).await;
        assert_eq!(absent.handle().subscribe().borrow().enabled, None);
        assert!(matches!(absent.handle().read(), Err(Error::Closed)));
        absent.close().await;

        let block = Arc::new(Block::new());
        let (_broker, connection, state, owner) = fixture(State {
            block: Some(block.clone()),
            ..State::default()
        })
        .await;
        let handle = owner.handle();
        assert_eq!(handle.read().unwrap().wait().await, Ok(false));
        let completion = handle.set(false, true).unwrap();
        block.started.notified().await;
        connection.release_name(SERVICE).await.unwrap();
        assert_eq!(completion.wait().await, Err(Error::Closed));
        let mut view = handle.subscribe();
        view.wait_for(|view| view.error == Some(Error::Closed) && !view.busy)
            .await
            .unwrap();
        assert_eq!(view.borrow().enabled, None);
        assert_eq!(state.lock().unwrap().calls.len(), 1);
        owner.close().await;
        block.release.notify_one();
        block.done.notified().await;
        assert!(matches!(handle.read(), Err(Error::Closed)));
        connection.close().await.unwrap();
    })
    .await
    .expect("Private manager loss scenario did not complete");
}
