use super::*;
use crate::test_bus::Broker;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::sync::Notify;
struct ReadBarrier {
    reached: Arc<Notify>,
    release: Arc<Notify>,
}

struct Locker {
    active: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
    pause: Arc<Mutex<Option<ReadBarrier>>>,
}
#[zbus::interface(name = "org.freedesktop.ScreenSaver")]
impl Locker {
    async fn get_active(&self) -> bool {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let value = self.active.load(Ordering::SeqCst);
        let pause = self.pause.lock().unwrap().take();
        if let Some(pause) = pause {
            pause.reached.notify_one();
            pause.release.notified().await;
        }
        value
    }
}
struct Dnd {
    active: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}
#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Dnd {
    #[zbus(property)]
    fn inhibited(&self) -> bool {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.active.load(Ordering::SeqCst)
    }
}
struct Fixture {
    connection: Connection,
    pause: Arc<Mutex<Option<ReadBarrier>>>,
    locked: Arc<AtomicBool>,
    inhibited: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}
impl Fixture {
    async fn new(bus: &Broker) -> Self {
        Self::with_state(bus, false, false).await
    }
    async fn with_state(bus: &Broker, locked: bool, inhibited: bool) -> Self {
        let locked = Arc::new(AtomicBool::new(locked));
        let inhibited = Arc::new(AtomicBool::new(inhibited));
        let pause = Arc::new(Mutex::new(None));
        let calls = Arc::new(AtomicUsize::new(0));
        let connection =
            zbus::connection::Builder::unix_stream(bus.address.connect().await.unwrap())
                .serve_at(
                    Kind::Lock.path(),
                    Locker {
                        active: locked.clone(),
                        calls: calls.clone(),
                        pause: pause.clone(),
                    },
                )
                .unwrap()
                .serve_at(
                    Kind::Inhibition.path(),
                    Dnd {
                        active: inhibited.clone(),
                        calls: calls.clone(),
                    },
                )
                .unwrap()
                .build()
                .await
                .unwrap();
        connection.request_name(Kind::Lock.service()).await.unwrap();
        connection
            .request_name(Kind::Inhibition.service())
            .await
            .unwrap();
        Self {
            connection,
            pause,
            locked,
            inhibited,
            calls,
        }
    }
    async fn lock(&self, value: bool) {
        self.locked.store(value, Ordering::SeqCst);
        self.connection
            .emit_signal(
                None::<&str>,
                Kind::Lock.path(),
                Kind::Lock.interface(),
                Kind::Lock.member(),
                &value,
            )
            .await
            .unwrap();
    }
    async fn dnd(&self, value: bool, invalidated: bool) {
        self.inhibited.store(value, Ordering::SeqCst);
        let changed = if invalidated {
            std::collections::BTreeMap::new()
        } else {
            std::collections::BTreeMap::from([("Inhibited", zbus::zvariant::Value::from(value))])
        };
        self.connection
            .emit_signal(
                None::<&str>,
                Kind::Inhibition.path(),
                Kind::Inhibition.interface(),
                Kind::Inhibition.member(),
                &(
                    Kind::Inhibition.service(),
                    changed,
                    if invalidated {
                        vec!["Inhibited"]
                    } else {
                        vec![]
                    },
                ),
            )
            .await
            .unwrap();
    }
}
async fn scenario(test: impl std::future::Future<Output = ()>) {
    tokio::time::timeout(std::time::Duration::from_secs(30), test)
        .await
        .expect("Owned desktop safety scenario must finish");
}

#[tokio::test]
async fn given_real_provider_signals_when_safety_or_owner_changes_then_unknown_suppresses_and_new_generation_recovers(
) {
    scenario(async {
        // Arrange: external desktop providers use the real private broker, not domain mocks.
        let bus = Broker::new();
        let fixture = Fixture::new(&bus).await;
        let guard = DesktopGuard::connect(&bus.address).await.unwrap();
        let mut updates = guard.updates.clone();
        updates
            .wait_for(|state| state.automatic_allowed())
            .await
            .unwrap();
        assert!(fixture.calls.load(Ordering::SeqCst) >= 2);
        // Act / Assert: initial state, effective DND, invalidation reread, lock and unlock.
        fixture.dnd(true, false).await;
        updates
            .wait_for(|state| state.inhibited == Some(true))
            .await
            .unwrap();
        assert!(!updates.borrow().automatic_allowed());
        assert!(updates.borrow().manual_allowed());
        fixture.dnd(false, true).await;
        updates
            .wait_for(|state| state.automatic_allowed())
            .await
            .unwrap();
        fixture.lock(true).await;
        updates
            .wait_for(|state| state.locked == Some(true))
            .await
            .unwrap();
        assert!(!updates.borrow().manual_allowed());
        fixture.lock(false).await;
        updates
            .wait_for(|state| state.automatic_allowed())
            .await
            .unwrap();
        // Pure decoder rejects spoofed headers and malformed types, ignores unrelated properties.
        let owner = fixture.connection.unique_name().unwrap().as_str();
        for (sender, key, value, expected) in [
            (
                owner,
                "Other",
                zbus::zvariant::Value::from(false),
                Ok(false),
            ),
            (
                ":1.999",
                "Inhibited",
                zbus::zvariant::Value::from(false),
                Err(()),
            ),
            (
                owner,
                "Inhibited",
                zbus::zvariant::Value::from("invalid"),
                Err(()),
            ),
        ] {
            let message = Message::signal(
                Kind::Inhibition.path(),
                Kind::Inhibition.interface(),
                Kind::Inhibition.member(),
            )
            .unwrap()
            .sender(sender)
            .unwrap()
            .build(&(
                Kind::Inhibition.service(),
                std::collections::BTreeMap::from([(key, value)]),
                Vec::<String>::new(),
            ))
            .unwrap();
            assert_eq!(relevant_signal(&message, owner, Kind::Inhibition), expected);
        }
        fixture.connection.close().await.unwrap();
        updates
            .wait_for(|state| state.locked.is_none() && state.inhibited.is_none())
            .await
            .unwrap();
        assert!(!updates.borrow().automatic_allowed());
        let replacement = Fixture::new(&bus).await;
        updates
            .wait_for(|state| state.automatic_allowed())
            .await
            .unwrap();
        // A late reply from an old, still-live unique owner cannot publish an unlocked state.
        let reader = bus.connection().await;
        let provider = resolve(&reader, Kind::Lock, rustix::process::geteuid().as_raw())
            .await
            .unwrap();
        let reached = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        *replacement.pause.lock().unwrap() = Some(ReadBarrier {
            reached: reached.clone(),
            release: release.clone(),
        });
        let reading = tokio::spawn(async move {
            let value = read(&reader, &provider, Kind::Lock).await;
            drop(provider);
            reader.close().await.unwrap();
            value
        });
        reached.notified().await;
        replacement
            .connection
            .release_name(Kind::Lock.service())
            .await
            .unwrap();
        replacement
            .connection
            .release_name(Kind::Inhibition.service())
            .await
            .unwrap();
        updates
            .wait_for(|state| state.locked.is_none() && state.inhibited.is_none())
            .await
            .unwrap();
        let current = Fixture::with_state(&bus, true, true).await;
        updates
            .wait_for(|state| state.locked == Some(true) && state.inhibited == Some(true))
            .await
            .unwrap();
        release.notify_one();
        assert_eq!(reading.await.unwrap(), None);
        replacement.connection.close().await.unwrap();
        let replacement = current;
        // An invalid property from the actual pinned owner is unknown, never interpreted as false.
        replacement
            .connection
            .emit_signal(
                None::<&str>,
                Kind::Inhibition.path(),
                Kind::Inhibition.interface(),
                Kind::Inhibition.member(),
                &(
                    Kind::Inhibition.service(),
                    std::collections::BTreeMap::from([(
                        "Inhibited",
                        zbus::zvariant::Value::from("invalid"),
                    )]),
                    Vec::<String>::new(),
                ),
            )
            .await
            .unwrap();
        updates
            .wait_for(|state| state.inhibited.is_none())
            .await
            .unwrap();
        assert!(!updates.borrow().automatic_allowed());
        guard.close().await;
        assert!(updates.borrow().closed);
        assert!(!updates.borrow().manual_allowed());
        replacement.connection.close().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn given_missing_or_foreign_uid_providers_when_observed_then_no_default_unlock_or_mutating_desktop_call_occurs(
) {
    scenario(async {
        // Arrange / Act / Assert
        let bus = Broker::new();
        let guard = DesktopGuard::connect(&bus.address).await.unwrap();
        let mut updates = guard.updates.clone();
        assert_eq!(*updates.borrow(), SafetyState::default());
        assert!(!updates.borrow().automatic_allowed());
        let fixture = Fixture::new(&bus).await;
        updates
            .wait_for(|state| state.automatic_allowed())
            .await
            .unwrap();
        guard.close().await;
        assert!(updates.borrow().closed);
        let connection = bus.connection().await;
        let foreign = rustix::process::geteuid().as_raw().wrapping_add(1);
        for kind in [Kind::Lock, Kind::Inhibition] {
            assert!(resolve(&connection, kind, foreign).await.is_none());
        }
        connection.close().await.unwrap();
        let guard = DesktopGuard::connect_with_uid(&bus.address, foreign)
            .await
            .unwrap();
        let updates = guard.updates.clone();
        guard.close().await;
        assert!(
            updates.borrow().locked.is_none()
                && updates.borrow().inhibited.is_none()
                && updates.borrow().closed
        );
        fixture.connection.close().await.unwrap();
    })
    .await;
}
