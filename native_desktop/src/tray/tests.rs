use super::*;
use crate::test_bus::Broker;

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
    assert_eq!(
        item.get_property::<String>("IconName").await.unwrap(),
        "rquickshare"
    );
    assert_eq!(
        item.get_property::<zbus::zvariant::OwnedObjectPath>("Menu")
            .await
            .unwrap()
            .as_str(),
        menu::PATH
    );
    let mut updates = tray.updates.clone();
    for (method, point) in [
        ("Activate", (1, 2)),
        ("SecondaryActivate", (-10, 20)),
        ("ContextMenu", (i32::MIN, i32::MAX)),
    ] {
        item.call::<_, _, ()>(method, &point).await.unwrap();
        updates.changed().await.unwrap();
        assert_eq!(updates.borrow_and_update().activation, Some(point));
        assert_eq!(updates.borrow().context, method == "ContextMenu");
    }
    let menu_proxy = zbus::Proxy::new(&observer, identity.as_str(), menu::PATH, menu::INTERFACE)
        .await
        .unwrap();
    let mut layouts = menu_proxy.receive_signal("LayoutUpdated").await.unwrap();
    tray.menu.publish(menu::Projection {
        enabled: 0x7ff,
        full: true,
        visibility: None,
    });
    let signal = layouts.next().await.unwrap();
    assert_eq!(signal.body().deserialize::<(u32, i32)>().unwrap(), (2, 0));
    let (revision, _): (u32, menu::Layout) = menu_proxy
        .call("GetLayout", &(0i32, 0i32, Vec::<String>::new()))
        .await
        .unwrap();
    assert_eq!(revision, 2);
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

#[tokio::test]
async fn given_late_watcher_or_shutdown_when_waiting_for_tray_then_registration_is_event_driven_and_cancelled_wait_closes(
) {
    let broker = Broker::new();
    let cancel = CancellationToken::new();
    let pending = Tray::connect_when_ready(&broker.address, cancel.clone());
    tokio::pin!(pending);
    assert!(futures::poll!(&mut pending).is_pending());
    cancel.cancel();
    assert!(pending.await.is_err());
    let pending = Tray::connect_when_ready(&broker.address, CancellationToken::new());
    tokio::pin!(pending);
    assert!(futures::poll!(&mut pending).is_pending());
    let watcher = broker.connection().await;
    let (registered, mut registration) = watch::channel(None);
    watcher
        .object_server()
        .at("/StatusNotifierWatcher", Watcher { registered })
        .await
        .unwrap();
    watcher.request_name(WATCHER).await.unwrap();
    let tray = pending.await.unwrap();
    registration.changed().await.unwrap();
    assert!(registration.borrow().is_some());
    tray.close().await.unwrap();
    watcher.close().await.unwrap();
}
