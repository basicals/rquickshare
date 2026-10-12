use super::*;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};

struct Broker {
    child: Child,
    _directory: tempfile::TempDir,
    address: BusAddress,
}
impl Broker {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("bus.conf");
        std::fs::write(
            &config,
            include_str!("../../../docs/prototypes/preview-bus.conf"),
        )
        .unwrap();
        let mut child = Command::new("dbus-daemon")
            .args(["--nofork", "--print-address=1"])
            .arg(format!("--config-file={}", config.display()))
            .arg(format!(
                "--address=unix:path={}",
                directory.path().join("bus").display()
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        Self {
            child,
            _directory: directory,
            address: BusAddress::parse(line.trim()).unwrap(),
        }
    }
    async fn connection(&self) -> Connection {
        zbus::connection::Builder::unix_stream(self.address.connect().await.unwrap())
            .build()
            .await
            .unwrap()
    }
}
impl Drop for Broker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
struct Watcher {
    registered: watch::Sender<Option<String>>,
}
#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    fn register_status_notifier_item(&self, service: String) {
        self.registered.send_replace(Some(service));
    }
}

#[tokio::test]
async fn given_private_kde_watcher_when_registered_activated_and_lost_then_latest_hint_is_bounded_and_registration_is_owned(
) {
    // Arrange
    let broker = Broker::new();
    let watcher = broker.connection().await;
    let (registered, mut registration) = watch::channel(None);
    watcher
        .object_server()
        .at("/StatusNotifierWatcher", Watcher { registered })
        .await
        .unwrap();
    watcher.request_name(WATCHER).await.unwrap();
    let tray = Tray::connect(&broker.address).await.unwrap();
    registration.changed().await.unwrap();
    let identity = registration.borrow_and_update().clone().unwrap();
    let observer = broker.connection().await;
    let item = zbus::Proxy::new(
        &observer,
        identity.as_str(),
        PATH,
        "org.kde.StatusNotifierItem",
    )
    .await
    .unwrap();
    // Act / Assert: same-user methods, properties, no replay queue for a slow consumer.
    assert_eq!(
        item.get_property::<String>("Category").await.unwrap(),
        "Communications"
    );
    assert_eq!(
        item.get_property::<String>("Id").await.unwrap(),
        "rquickshare"
    );
    assert_eq!(
        item.get_property::<String>("Status").await.unwrap(),
        "Active"
    );
    assert!(!item.get_property::<bool>("ItemIsMenu").await.unwrap());
    let mut updates = tray.updates.clone();
    for (method, point) in [
        ("Activate", (1, 2)),
        ("SecondaryActivate", (-10, 20)),
        ("ContextMenu", (i32::MIN, i32::MAX)),
    ] {
        item.call::<_, _, ()>(method, &point).await.unwrap();
        updates.changed().await.unwrap();
        assert_eq!(updates.borrow_and_update().activation, Some(point));
    }
    for x in 0..64 {
        item.call::<_, _, ()>("Activate", &(x, 100)).await.unwrap();
    }
    updates.changed().await.unwrap();
    assert_eq!(updates.borrow_and_update().activation, Some((63, 100)));
    watcher.release_name(WATCHER).await.unwrap();
    updates.changed().await.unwrap();
    assert!(updates.borrow_and_update().failed);
    tray.close().await.unwrap();
    let dbus = fdo::DBusProxy::new(&observer).await.unwrap();
    assert!(!dbus
        .name_has_owner(identity.as_str().try_into().unwrap())
        .await
        .unwrap());
    observer.close().await.unwrap();
    watcher.close().await.unwrap();
}

#[tokio::test]
async fn given_missing_watcher_or_explicit_close_when_connecting_then_no_invisible_registration_remains(
) {
    // Arrange / Act / Assert
    assert!(same_user(1000, 1000));
    assert!(!same_user(1001, 1000));
    assert!(!same_user(0, 1000));
    let broker = Broker::new();
    assert!(Tray::connect(&broker.address).await.is_err());
    let watcher = broker.connection().await;
    let (registered, mut registration) = watch::channel(None);
    watcher
        .object_server()
        .at("/StatusNotifierWatcher", Watcher { registered })
        .await
        .unwrap();
    watcher.request_name(WATCHER).await.unwrap();
    let tray = Tray::connect(&broker.address).await.unwrap();
    registration.changed().await.unwrap();
    let identity = registration.borrow().clone().unwrap();
    tray.close().await.unwrap();
    assert!(!fdo::DBusProxy::new(&watcher)
        .await
        .unwrap()
        .name_has_owner(identity.as_str().try_into().unwrap())
        .await
        .unwrap());
    watcher.close().await.unwrap();
}
