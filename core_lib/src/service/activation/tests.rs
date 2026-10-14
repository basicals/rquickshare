use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

use super::*;
use crate::service::{
    bus::{
        tests::{Harness, PrivateBus},
        SelectionBus,
    },
    client::NativeClient,
};
struct Fake {
    owners: Mutex<VecDeque<Result<Option<String>, ActivationError>>>,
    uid: u32,
    supported: Result<bool, ActivationError>,
    start: Result<u32, ActivationError>,
    calls: AtomicUsize,
}
impl Broker for Fake {
    async fn owner(&self) -> Result<Option<String>, ActivationError> {
        self.owners
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Err(ActivationError::Protocol))
    }
    async fn uid(&self, _: &str) -> Result<u32, ActivationError> {
        Ok(self.uid)
    }
    async fn supported(&self) -> Result<bool, ActivationError> {
        self.supported
    }
    async fn start(&self) -> Result<u32, ActivationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.start
    }
}
#[tokio::test]
async fn given_activation_states_when_ensured_then_only_missing_supported_service_starts_once_and_owner_is_verified(
) {
    use ActivationError::*;
    // Arrange / Act / Assert: explicit external broker contract, actual activation rules.
    for (owners, supported, start, uid, expected, calls) in [
        (
            vec![Ok(Some(":1.9")), Ok(Some(":1.9"))],
            Ok(false),
            Ok(0),
            true,
            Ok(()),
            0,
        ),
        (
            vec![Ok(None), Ok(Some(":1.9")), Ok(Some(":1.9"))],
            Ok(true),
            Ok(1),
            true,
            Ok(()),
            1,
        ),
        (
            vec![Ok(None), Ok(Some(":1.9")), Ok(Some(":1.9"))],
            Ok(true),
            Ok(2),
            true,
            Ok(()),
            1,
        ),
        (vec![Ok(None)], Ok(false), Ok(1), true, Err(Unsupported), 0),
        (
            vec![Ok(None)],
            Err(Unavailable),
            Ok(1),
            true,
            Err(Unavailable),
            0,
        ),
        (
            vec![Ok(None)],
            Ok(true),
            Err(Unavailable),
            true,
            Err(Unavailable),
            1,
        ),
        (
            vec![Err(Unavailable)],
            Ok(true),
            Ok(1),
            true,
            Err(Unavailable),
            0,
        ),
        (
            vec![Ok(None), Ok(None)],
            Ok(true),
            Ok(1),
            true,
            Err(Unavailable),
            1,
        ),
        (
            vec![Ok(Some(":1.9"))],
            Ok(true),
            Ok(1),
            false,
            Err(Protocol),
            0,
        ),
        (
            vec![Ok(Some(":1.9")), Ok(Some(":1.10"))],
            Ok(true),
            Ok(1),
            true,
            Err(Unavailable),
            0,
        ),
    ] {
        let fake = Fake {
            owners: Mutex::new(
                owners
                    .into_iter()
                    .map(|v| v.map(|s| s.map(str::to_owned)))
                    .collect(),
            ),
            supported,
            start,
            uid: rustix::process::geteuid().as_raw() ^ u32::from(!uid),
            calls: AtomicUsize::new(0),
        };
        assert_eq!(ensure(&fake).await, expected);
        assert_eq!(fake.calls.load(Ordering::SeqCst), calls);
    }
    for code in [0, 3, u32::MAX] {
        let fake = Fake {
            owners: Mutex::new([Ok(None)].into()),
            uid: rustix::process::geteuid().as_raw(),
            supported: Ok(true),
            start: Ok(code),
            calls: AtomicUsize::new(0),
        };
        assert_eq!(ensure(&fake).await, Err(Protocol));
        assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    }
    for owner in ["", "org.example.Service", ":", ":bad/name"] {
        let fake = Fake {
            owners: Mutex::new([Ok(Some(owner.into()))].into()),
            uid: rustix::process::geteuid().as_raw(),
            supported: Ok(true),
            start: Ok(1),
            calls: AtomicUsize::new(0),
        };
        assert_eq!(ensure(&fake).await, Err(Protocol));
        assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn given_actual_private_bus_when_service_is_present_or_absent_then_passive_sdk_and_activation_do_not_spawn_or_stop_daemon(
) {
    // Arrange / Act / Assert
    let bus = PrivateBus::new();
    assert_eq!(
        activate_daemon(bus.socket().await, CancellationToken::new()).await,
        Err(ActivationError::Unsupported)
    );
    assert!(NativeClient::connect(bus.socket().await).await.is_err());
    let host = Harness::start(SelectionBus::connect(bus.socket().await).await.unwrap());
    activate_daemon(bus.socket().await, CancellationToken::new())
        .await
        .unwrap();
    let mut client = NativeClient::connect(bus.socket().await).await.unwrap();
    assert!(client.read_state().await.is_ok());
    client.close().await.unwrap();
    activate_daemon(bus.socket().await, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(host.finish().await.1, Ok(()));
}
struct Blocked {
    started: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}
impl Broker for Blocked {
    async fn owner(&self) -> Result<Option<String>, ActivationError> {
        Ok(None)
    }
    async fn uid(&self, _: &str) -> Result<u32, ActivationError> {
        unreachable!()
    }
    async fn supported(&self) -> Result<bool, ActivationError> {
        Ok(true)
    }
    async fn start(&self) -> Result<u32, ActivationError> {
        if let Some(started) = self.started.lock().unwrap().take() {
            let _ = started.send(());
        }
        std::future::pending().await
    }
}
#[tokio::test]
async fn given_owned_activation_connection_when_cancelled_before_or_during_start_then_connection_closes_without_stopping_shared_service(
) {
    crate::service::client::tests::scenario(async {
        let bus = PrivateBus::new();
        for admitted in [false, true] {
            // Arrange
            let connection = connection_builder(bus.socket().await)
                .build()
                .await
                .unwrap();
            let name = connection.unique_name().unwrap().to_string();
            let (started, observed) = tokio::sync::oneshot::channel();
            let broker = Blocked {
                started: Mutex::new(Some(started)),
            };
            let cancel = CancellationToken::new();
            if !admitted {
                cancel.cancel();
            }
            let stop = cancel.clone();
            let task = tokio::spawn(async move { owned(connection, &broker, stop).await });
            // Act
            if admitted {
                observed.await.unwrap();
                cancel.cancel();
            }
            assert_eq!(task.await.unwrap(), Err(ActivationError::Cancelled));
            // Assert: actual unique connection has left the real private broker.
            let observer = bus.client().await;
            let reply = observer
                .call_method(Some(BROKER), PATH, Some(BROKER), "NameHasOwner", &name)
                .await
                .unwrap();
            assert!(!reply.body().deserialize::<bool>().unwrap());
            observer.close().await.unwrap();
        }
    })
    .await;
}
#[test]
fn given_broker_reply_when_sender_or_descriptors_are_wrong_then_activation_rejects_it() {
    // Arrange / Act / Assert
    let call = Message::method_call(PATH, "Test")
        .unwrap()
        .build(&())
        .unwrap();
    for sender in [BROKER, ":1.99"] {
        let reply = Message::method_return(&call.header())
            .unwrap()
            .sender(sender)
            .unwrap()
            .build(&())
            .unwrap();
        assert_eq!(
            checked(&reply),
            if sender == BROKER {
                Ok(())
            } else {
                Err(ActivationError::Protocol)
            }
        );
    }
    let file = tempfile::tempfile().unwrap();
    let reply = Message::method_return(&call.header())
        .unwrap()
        .sender(BROKER)
        .unwrap()
        .build(&zbus::zvariant::Fd::from(&file))
        .unwrap();
    assert_eq!(checked(&reply), Err(ActivationError::Protocol));
}

#[tokio::test]
async fn given_real_systemd_activation_broker_when_external_manager_admits_then_fixed_unit_request_precedes_actual_sdk_handshake(
) {
    crate::service::client::tests::scenario(async {
        use futures::StreamExt;
        // Arrange: a real private broker reads the actual packaged activation asset.
        // Only the external manager is replaced; the service/SDK remain real.
        let bus = PrivateBus::with_systemd_activation();
        let manager = bus.client().await;
        manager.request_name("org.freedesktop.systemd1").await.unwrap();
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(BROKER).unwrap()
            .path(PATH).unwrap()
            .interface("org.freedesktop.systemd1.Activator").unwrap()
            .member("ActivationRequest").unwrap().build();
        let mut requests = zbus::MessageStream::for_match_rule(rule, &manager, Some(8)).await.unwrap();
        let stream = bus.socket().await;
        let mut activation = tokio::spawn(activate_daemon(stream, CancellationToken::new()));
        // Act: an early protocol failure must finish the scenario rather than hang.
        let request = tokio::select! {
            request = requests.next() => request.unwrap().unwrap(),
            result = &mut activation => panic!("Expected manager admission before completion: {result:?}"),
        };
        checked(&request).unwrap();
        assert_eq!(request.body().deserialize::<String>().unwrap(), "rquickshare-daemon.service");
        let host = Harness::start(SelectionBus::connect(bus.socket().await).await.unwrap());
        assert_eq!(activation.await.unwrap(), Ok(()));
        // Assert: activation does not replace readiness/epoch checks or own the daemon.
        let mut client = NativeClient::connect(bus.socket().await).await.unwrap();
        assert!(client.read_state().await.is_ok());
        client.close().await.unwrap();
        activate_daemon(bus.socket().await, CancellationToken::new()).await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
        drop(requests);
        manager.close().await.unwrap();
    }).await;
}
