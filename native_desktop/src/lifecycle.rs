//! Process-lifetime signal ownership survives GTK reconnect generations.
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub struct Shutdown {
    pub stop: CancellationToken,
    task: JoinHandle<()>,
}
impl Shutdown {
    pub fn new() -> Result<Self, &'static str> {
        let mut signals = rqs_lib::service::process::ShutdownSignals::new()
            .map_err(|_| "Cannot observe process shutdown")?;
        Ok(Self::from_signal(async move {
            let _ = signals.wait().await;
        }))
    }
    fn from_signal(signal: impl std::future::Future<Output = ()> + Send + 'static) -> Self {
        let stop = CancellationToken::new();
        let cancelled = stop.clone();
        let task = tokio::spawn(async move {
            tokio::select! { biased;
                _ = cancelled.cancelled() => (),
                _ = signal => cancelled.cancel(),
            }
        });
        Self { stop, task }
    }
    pub async fn close(self) {
        self.stop.cancel();
        let _ = self.task.await;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn given_owned_signal_wait_when_signal_or_explicit_close_arrives_then_process_stop_is_retained_and_task_joined(
    ) {
        let (signal, observe) = tokio::sync::oneshot::channel();
        let owner = Shutdown::from_signal(async move {
            let _ = observe.await;
        });
        let stop = owner.stop.clone();
        assert!(!stop.is_cancelled());
        signal.send(()).unwrap();
        stop.cancelled().await;
        owner.close().await;
        let owner = Shutdown::from_signal(std::future::pending());
        let stop = owner.stop.clone();
        owner.close().await;
        assert!(stop.is_cancelled());
    }
}
