//! Owner-local discovery visibility. Transports consume policy; they never expire it.

use std::fmt;
use std::future::Future;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Visibility {
    Visible = 0,
    Invisible = 1,
    Temporarily = 2,
}

#[allow(dead_code)]
impl Visibility {
    pub fn from_raw_value(value: u64) -> Self {
        Self::try_from_raw_value(value).unwrap_or(Self::Invisible)
    }

    pub fn try_from_raw_value(value: u64) -> Option<Self> {
        match value {
            0 => Some(Visibility::Visible),
            1 => Some(Visibility::Invisible),
            2 => Some(Visibility::Temporarily),
            _ => None,
        }
    }
}

const TEMPORARY_DURATION: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Snapshot {
    mode: Visibility,
    revision: u64,
    deadline: Option<Instant>,
    running: bool,
}

/// Read-only visibility subscription. A policy event is not an on-air acknowledgement.
#[derive(Debug, Clone)]
pub struct VisibilityReader {
    receiver: watch::Receiver<Snapshot>,
}

impl VisibilityReader {
    pub fn current(&self) -> Visibility {
        self.receiver.borrow().mode
    }

    pub fn current_and_update(&mut self) -> Visibility {
        self.receiver.borrow_and_update().mode
    }

    pub async fn changed(&mut self) -> Result<(), watch::error::RecvError> {
        self.receiver.changed().await
    }

    pub fn has_changed(&self) -> Result<bool, watch::error::RecvError> {
        self.receiver.has_changed()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityError {
    AlreadyRunning,
    RevisionExhausted,
}

impl fmt::Display for VisibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AlreadyRunning => "Visibility policy is already running",
            Self::RevisionExhausted => "Visibility policy revision exhausted",
        })
    }
}

impl std::error::Error for VisibilityError {}

#[derive(Debug, Clone)]
pub(crate) struct VisibilityPolicy {
    sender: watch::Sender<Snapshot>,
}

impl VisibilityPolicy {
    pub(crate) fn new(mode: Visibility) -> Self {
        let (sender, _) = watch::channel(Snapshot {
            mode,
            revision: 0,
            deadline: None,
            running: false,
        });
        Self { sender }
    }

    pub(crate) fn subscribe(&self) -> VisibilityReader {
        VisibilityReader {
            receiver: self.sender.subscribe(),
        }
    }

    pub(crate) fn set(&self, mode: Visibility, renew_same: bool) -> Result<(), VisibilityError> {
        let mut outcome = Ok(());
        self.sender.send_if_modified(|state| {
            if !renew_same && state.mode == mode {
                return false;
            }
            let Some(revision) = state.revision.checked_add(1) else {
                outcome = Err(VisibilityError::RevisionExhausted);
                state.mode = Visibility::Invisible;
                state.deadline = None;
                return true;
            };
            state.mode = mode;
            state.revision = revision;
            state.deadline = (state.running && mode == Visibility::Temporarily)
                .then(|| Instant::now() + TEMPORARY_DURATION);
            true
        });
        outcome
    }

    /// Activate before any transport initialization can yield. The returned future
    /// owns the guard even if it is dropped without ever being polled.
    pub(crate) fn start(
        &self,
        lifetime: CancellationToken,
    ) -> Result<impl Future<Output = ()> + Send + 'static, VisibilityError> {
        let mut outcome = Ok(());
        self.sender.send_if_modified(|state| {
            if state.running {
                outcome = Err(VisibilityError::AlreadyRunning);
                return false;
            }
            let Some(revision) = state.revision.checked_add(1) else {
                outcome = Err(VisibilityError::RevisionExhausted);
                state.mode = Visibility::Invisible;
                state.deadline = None;
                return true;
            };
            state.revision = revision;
            state.running = true;
            state.deadline = (state.mode == Visibility::Temporarily)
                .then(|| Instant::now() + TEMPORARY_DURATION);
            true
        });
        outcome?;
        let mut reader = self.sender.subscribe();
        let guard = WorkerGuard {
            policy: self.clone(),
            lifetime,
            finished: false,
        };
        Ok(async move {
            loop {
                let snapshot = *reader.borrow_and_update();
                tokio::select! {
                    biased;
                    _ = guard.lifetime.cancelled() => {
                        guard.finish();
                        return;
                    }
                    changed = reader.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                    _ = async {
                        match snapshot.deadline {
                            Some(deadline) => tokio::time::sleep_until(deadline).await,
                            None => std::future::pending().await,
                        }
                    } => {
                        guard.policy.expire(snapshot, Instant::now());
                    }
                }
            }
        })
    }

    fn expire(&self, expected: Snapshot, now: Instant) {
        self.sender.send_if_modified(|state| {
            if !state.running
                || state.mode != Visibility::Temporarily
                || state.revision != expected.revision
                || state.deadline != expected.deadline
                || !state.deadline.is_some_and(|deadline| now >= deadline)
            {
                return false;
            }
            state.mode = Visibility::Invisible;
            state.deadline = None;
            state.revision = state.revision.saturating_add(1);
            true
        });
    }

    fn stop(&self, failed: bool) {
        self.sender.send_modify(|state| {
            state.running = false;
            state.deadline = None;
            if failed {
                state.mode = Visibility::Invisible;
            }
        });
    }
}

struct WorkerGuard {
    policy: VisibilityPolicy,
    lifetime: CancellationToken,
    finished: bool,
}

impl WorkerGuard {
    fn finish(mut self) {
        self.policy.stop(false);
        self.finished = true;
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.policy.stop(true);
            self.lifetime.cancel();
        }
    }
}

#[cfg(test)]
mod tests;
