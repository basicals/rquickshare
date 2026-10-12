//! GATT-owned tasks: bridge completion does not detach a migrated protocol worker.

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use super::SessionError;

#[derive(Clone)]
pub(crate) struct SessionTasks {
    admission: Arc<Mutex<(bool, TaskTracker)>>,
    permits: Arc<Semaphore>,
    lifetime: CancellationToken,
    failed: Arc<AtomicBool>,
}

impl SessionTasks {
    pub fn new(parent: &CancellationToken, limit: usize) -> Result<Self, SessionError> {
        if limit == 0 || limit > Semaphore::MAX_PERMITS {
            return Err(SessionError::Invalid);
        }
        Ok(Self {
            admission: Arc::new(Mutex::new((true, TaskTracker::new()))),
            permits: Arc::new(Semaphore::new(limit)),
            lifetime: parent.child_token(),
            failed: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn lifetime(&self) -> CancellationToken {
        self.lifetime.clone()
    }

    /// Each owned operation cooperates with lifetime; no abort-and-detach fallback.
    pub fn spawn(
        &self,
        operation: impl Future<Output = ()> + Send + 'static,
    ) -> Result<JoinHandle<()>, SessionError> {
        let admission = self.admission.lock().map_err(|_| SessionError::OwnerLost)?;
        if !admission.0 || self.lifetime.is_cancelled() {
            return Err(SessionError::Closed);
        }
        let permit = Arc::clone(&self.permits)
            .try_acquire_owned()
            .map_err(|_| SessionError::Full)?;
        let failed = Arc::clone(&self.failed);
        let lifetime = self.lifetime.clone();
        Ok(admission.1.spawn(async move {
            let _permit = permit;
            if AssertUnwindSafe(operation).catch_unwind().await.is_err() {
                failed.store(true, Ordering::SeqCst);
                lifetime.cancel();
            }
        }))
    }

    pub async fn stop(&self) -> Result<(), SessionError> {
        let tracker = {
            let mut admission = self.admission.lock().map_err(|_| SessionError::OwnerLost)?;
            admission.0 = false;
            self.lifetime.cancel();
            admission.1.close();
            admission.1.clone()
        };
        tracker.wait().await;
        if self.failed.load(Ordering::SeqCst) {
            Err(SessionError::OwnerLost)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
