//! One owned coalescing activation worker; transport never waits for GTK startup.
use std::future::Future;

use tokio::{net::UnixStream, sync::watch, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use zbus::Connection;

use super::bus;

trait Activator: Send + 'static {
    fn activate(&mut self) -> impl Future<Output = bool> + Send;
    fn close(self) -> impl Future<Output = ()> + Send;
}
struct BusActivator(Connection);
impl Activator for BusActivator {
    async fn activate(&mut self) -> bool {
        super::activation::ensure_desktop(&self.0).await.is_ok()
    }
    async fn close(self) {
        let _ = self.0.close().await;
    }
}
pub(super) struct Wakeup {
    generation: watch::Sender<u64>,
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
}
impl Wakeup {
    pub(super) async fn connect(stream: UnixStream) -> zbus::Result<Self> {
        Ok(Self::spawn(BusActivator(
            bus::connection_builder(stream).build().await?,
        )))
    }
    fn spawn<A: Activator>(mut activator: A) -> Self {
        let (generation, mut updates) = watch::channel(0u64);
        let stop = CancellationToken::new();
        let cancelled = stop.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! { biased;
                    _ = cancelled.cancelled() => break,
                    changed = updates.changed() => if changed.is_err() { break; },
                }
                updates.borrow_and_update();
                tokio::select! { biased;
                    _ = cancelled.cancelled() => break,
                    _ = activator.activate() => (),
                }
            }
            activator.close().await;
        });
        Self {
            generation,
            stop,
            task: Some(task),
        }
    }
    pub(super) fn notify(&self, revision: u64) {
        self.generation.send_if_modified(|current| {
            if revision <= *current {
                false
            } else {
                *current = revision;
                true
            }
        });
    }
    pub(super) async fn close(mut self) {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}
impl Drop for Wakeup {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use tokio::sync::{mpsc, Semaphore};

    use super::*;
    use crate::service::client::desktop;
    struct Provider {
        called: mpsc::Sender<()>,
        release: Arc<Semaphore>,
        closed: Arc<AtomicBool>,
    }
    impl Activator for Provider {
        async fn activate(&mut self) -> bool {
            self.called.send(()).await.unwrap();
            self.release.acquire().await.unwrap().forget();
            false
        }
        async fn close(self) {
            self.closed.store(true, Ordering::SeqCst);
        }
    }
    #[tokio::test]
    async fn given_failed_or_blocked_activation_when_generations_coalesce_then_same_notice_never_respawns_and_shutdown_joins(
    ) {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            // Arrange: actual standard broker replies for missing/already-owned service.
            let private = crate::service::bus::tests::PrivateBus::new();
            let mut real = BusActivator(
                bus::connection_builder(private.socket().await)
                    .build()
                    .await
                    .unwrap(),
            );
            assert!(!real.activate().await);
            let gui = private.client().await;
            gui.request_name(desktop::SERVICE).await.unwrap();
            assert!(real.activate().await);
            real.close().await;
            gui.close().await.unwrap();
            // Arrange: deterministic external provider barriers, not domain mocks.
            let (called, mut calls) = mpsc::channel(2);
            let release = Arc::new(Semaphore::new(0));
            let closed = Arc::new(AtomicBool::new(false));
            let wake = Wakeup::spawn(Provider {
                called,
                release: release.clone(),
                closed: closed.clone(),
            });
            // Act / Assert
            wake.notify(1);
            calls.recv().await.unwrap();
            wake.notify(1);
            wake.notify(0);
            wake.notify(2);
            wake.notify(3);
            release.add_permits(1);
            calls.recv().await.unwrap();
            assert_eq!(*wake.generation.borrow(), 3);
            assert!(calls.try_recv().is_err());
            wake.close().await;
            assert!(closed.load(Ordering::SeqCst));
            assert_eq!(calls.recv().await, None);
        })
        .await
        .expect("Owned wake fixture must finish");
    }
    #[tokio::test]
    async fn given_blocked_activation_when_real_service_reads_and_dismisses_then_bus_progress_and_shutdown_are_independent(
    ) {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            // Arrange
            let private = crate::service::bus::tests::PrivateBus::new();
            let mut coordinator =
                crate::service::Coordinator::new(123, crate::service::Limits::default()).unwrap();
            let id = coordinator.register_inbound().unwrap();
            coordinator
                .observe(id, crate::service::Phase::AwaitingConsent)
                .unwrap();
            let (called, mut calls) = mpsc::channel(2);
            let closed = Arc::new(AtomicBool::new(false));
            let wake = Wakeup::spawn(Provider {
                called,
                release: Arc::new(Semaphore::new(0)),
                closed: closed.clone(),
            });
            let mut bus = crate::service::bus::SelectionBus::connect(private.socket().await)
                .await
                .unwrap();
            bus.set_wakeup(wake);
            let host = crate::service::bus::tests::Harness::with_coordinator(bus, coordinator);
            // Act: activation remains blocked across actual SDK calls and daemon close.
            let mut client = crate::service::client::NativeClient::connect(private.socket().await)
                .await
                .unwrap();
            calls.recv().await.unwrap();
            let view = client.read_state().await.unwrap();
            assert_eq!(view.notices.len(), 1);
            client.dismiss_notices(&view.notices).await.unwrap();
            assert!(client.read_state().await.unwrap().notices.is_empty());
            client.close().await.unwrap();
            let (coordinator, result) = host.finish().await;
            // Assert
            assert_eq!(result, Ok(()));
            assert!(closed.load(Ordering::SeqCst));
            assert_eq!(calls.recv().await, None);
            assert!(coordinator.pending_notices().is_empty());
            let transfer = &coordinator.transfers[&id].view;
            assert_eq!(transfer.phase, crate::service::Phase::AwaitingConsent);
            assert!(transfer.pending_control.is_none());
        })
        .await
        .expect("Owned blocked activation fixture must finish");
    }
}
