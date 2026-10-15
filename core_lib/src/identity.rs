//! Owner-local presentation identity. A display name never grants peer trust.
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub const MAX_NAME_BYTES: usize = 237;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    InvalidName,
    Exhausted,
    Consumer,
    Unavailable,
    Superseded,
    Cancelled,
}
impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Local identity: {self:?}")
    }
}
impl std::error::Error for IdentityError {}

#[derive(Clone, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DeviceName(String);
impl DeviceName {
    pub fn new(value: &str) -> Result<Self, IdentityError> {
        if value.is_empty()
            || value.len() > MAX_NAME_BYTES
            || value.trim() != value
            || value.chars().any(|c| {
                c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
        {
            return Err(IdentityError::InvalidName);
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for DeviceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeviceName([redacted])")
    }
}
impl<'de> Deserialize<'de> for DeviceName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::new(&value).map_err(|_| serde::de::Error::custom("Invalid device name"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IdentitySnapshot {
    pub revision: u64,
    pub name: DeviceName,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConsumerKind {
    Mdns = 0,
    Gatt = 1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ack {
    NotRequired,
    Waiting,
    Running(u64),
    Failed,
}

#[derive(Clone)]
pub(crate) struct IdentityPolicy {
    fallback: DeviceName,
    current: watch::Sender<IdentitySnapshot>,
    acknowledgements: watch::Sender<[Ack; 2]>,
}
impl fmt::Debug for IdentityPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentityPolicy")
            .field("revision", &self.current.borrow().revision)
            .finish_non_exhaustive()
    }
}
impl IdentityPolicy {
    pub fn new(fallback: DeviceName, required: &[ConsumerKind]) -> Self {
        let mut acknowledgements = [Ack::NotRequired; 2];
        for kind in required {
            acknowledgements[*kind as usize] = Ack::Waiting;
        }
        Self {
            current: watch::channel(IdentitySnapshot {
                revision: 1,
                name: fallback.clone(),
            })
            .0,
            acknowledgements: watch::channel(acknowledgements).0,
            fallback,
        }
    }
    pub fn snapshot(&self) -> IdentitySnapshot {
        self.current.borrow().clone()
    }
    pub fn set_override(&self, value: Option<&DeviceName>) -> Result<u64, IdentityError> {
        let name = value.unwrap_or(&self.fallback);
        let mut result = Err(IdentityError::Exhausted);
        let changed = self.current.send_if_modified(|current| {
            if &current.name == name {
                result = Ok(current.revision);
                return false;
            }
            let Some(revision) = current.revision.checked_add(1) else {
                return false;
            };
            *current = IdentitySnapshot {
                revision,
                name: name.clone(),
            };
            result = Ok(revision);
            true
        });
        if changed {
            self.acknowledgements.send_modify(|_| ());
        }
        result
    }
    pub fn subscribe(&self, kind: ConsumerKind) -> Result<IdentityConsumer, IdentityError> {
        let mut registered = false;
        self.acknowledgements.send_if_modified(|states| {
            let state = &mut states[kind as usize];
            if *state != Ack::Waiting {
                return false;
            }
            *state = Ack::Running(0);
            registered = true;
            true
        });
        if !registered {
            return Err(IdentityError::Consumer);
        }
        Ok(IdentityConsumer {
            kind,
            updates: self.current.subscribe(),
            acknowledgements: self.acknowledgements.clone(),
        })
    }
    pub fn applied(&self, revision: u64) -> Result<bool, IdentityError> {
        let current = self.current.borrow().revision;
        if revision == 0 || revision > current {
            return Err(IdentityError::Consumer);
        }
        if revision < current {
            return Err(IdentityError::Superseded);
        }
        let states = self.acknowledgements.borrow();
        if states.iter().any(|state| *state == Ack::Failed) {
            return Err(IdentityError::Unavailable);
        }
        Ok(states
            .iter()
            .all(|state| *state == Ack::NotRequired || *state == Ack::Running(revision)))
    }
    pub async fn wait_applied(
        &self,
        revision: u64,
        stop: &CancellationToken,
    ) -> Result<(), IdentityError> {
        let mut updates = self.acknowledgements.subscribe();
        loop {
            // Subscribe/mark observed before inspecting state, preventing a lost wakeup.
            updates.borrow_and_update();
            if stop.is_cancelled() {
                return Err(IdentityError::Cancelled);
            }
            if self.applied(revision)? {
                return Ok(());
            }
            tokio::select! {
                biased;
                _ = stop.cancelled() => return Err(IdentityError::Cancelled),
                result = updates.changed() => result.map_err(|_| IdentityError::Unavailable)?,
            }
        }
    }
}

pub(crate) struct IdentityConsumer {
    kind: ConsumerKind,
    updates: watch::Receiver<IdentitySnapshot>,
    acknowledgements: watch::Sender<[Ack; 2]>,
}
impl IdentityConsumer {
    pub fn snapshot(&mut self) -> IdentitySnapshot {
        self.updates.borrow_and_update().clone()
    }
    pub async fn changed(&mut self) -> Result<(), IdentityError> {
        self.updates
            .changed()
            .await
            .map_err(|_| IdentityError::Unavailable)
    }
    /// Stale acknowledgements cannot complete a newer generation or regress progress.
    pub fn acknowledge(&self, revision: u64) -> Result<(), IdentityError> {
        if revision == 0 || revision > self.updates.borrow().revision {
            return Err(IdentityError::Consumer);
        }
        let mut accepted = false;
        self.acknowledgements.send_if_modified(|states| {
            if let Ack::Running(previous) = &mut states[self.kind as usize] {
                if revision >= *previous {
                    accepted = true;
                    if revision != *previous {
                        *previous = revision;
                        return true;
                    }
                }
            }
            false
        });
        accepted.then_some(()).ok_or(IdentityError::Consumer)
    }
}
impl Drop for IdentityConsumer {
    fn drop(&mut self) {
        self.acknowledgements
            .send_modify(|states| states[self.kind as usize] = Ack::Failed);
    }
}

#[cfg(test)]
mod tests;
