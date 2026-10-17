//! Bounded daemon-owned metadata journal. It never owns payloads or transfer authority.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot, watch};

use super::{
    native_settings::{PersistenceIo, StoreError, SystemIo},
    Coordinator, Direction, Phase,
};

pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Journal1";
pub const COMPLETED: usize = 1000;
pub const CURRENT: usize = 256;
pub const MAX_AGE: u64 = 30 * 24 * 60 * 60;
const MAX_BYTES: usize = 1024 * 1024;
const FILE: &str = "native-transfers.json";

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, zbus::zvariant::Type)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub id: String,
    pub peer: String,
    pub inbound: bool,
    pub status: String,
    pub kind: String,
    pub files: u32,
    pub total: u64,
    pub transferred: u64,
    pub started: u64,
    pub updated: u64,
}
impl fmt::Debug for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalRecord")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}
impl Record {
    pub fn terminal(&self) -> bool {
        self.status != "active"
    }
    pub fn valid(&self) -> bool {
        self.id.len() == 49
            && self.id.as_bytes()[32] == b'-'
            && self
                .id
                .bytes()
                .enumerate()
                .all(|(i, b)| i == 32 || b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && self.peer.len() <= 256
            && !self.peer.chars().any(char::is_control)
            && matches!(
                self.status.as_str(),
                "active"
                    | "finished"
                    | "rejected"
                    | "cancelled"
                    | "failed"
                    | "disconnected"
                    | "interrupted"
            )
            && matches!(
                self.kind.as_str(),
                "files" | "text" | "url" | "image" | "unsupported" | "unknown"
            )
            && self.files <= 256
            && self.updated >= self.started
    }
}
#[derive(Clone, Default, Eq, PartialEq)]
pub struct View {
    pub token: String,
    pub error: u8,
    pub records: Vec<Record>,
}
impl fmt::Debug for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalView")
            .field("error", &self.error)
            .field("records", &self.records.len())
            .finish()
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    last_clear: String,
    records: Vec<Record>,
}
fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn valid_token(token: &str) -> bool {
    token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

struct Store<I = SystemIo> {
    path: PathBuf,
    io: I,
    doc: Document,
    raw: Option<Vec<u8>>,
    blocked: bool,
    error: u8,
}
impl<I: PersistenceIo> Store<I> {
    fn open(path: PathBuf, io: I, time: u64) -> Self {
        let mut store = Self {
            path,
            io,
            doc: Document {
                schema: 1,
                ..Default::default()
            },
            raw: None,
            blocked: false,
            error: 0,
        };
        let loaded = (|| {
            let directory = super::private_file::directory(&store.path, false)?;
            store.raw = directory
                .as_ref()
                .map(|d| super::private_file::read(d, FILE, MAX_BYTES))
                .transpose()?
                .flatten();
            if let Some(raw) = &store.raw {
                let doc: Document =
                    serde_json::from_slice(raw).map_err(|_| StoreError::RecoveryRequired)?;
                let mut ids = BTreeSet::new();
                if doc.schema != 1
                    || doc.records.len() > COMPLETED + CURRENT
                    || (!doc.last_clear.is_empty() && !valid_token(&doc.last_clear))
                    || doc
                        .records
                        .iter()
                        .any(|r| !r.valid() || !ids.insert(r.id.clone()))
                {
                    return Err(StoreError::RecoveryRequired);
                }
                store.doc = doc;
            }
            Ok::<_, StoreError>(())
        })();
        if loaded.is_err() {
            store.blocked = true;
            store.error = 1;
            return store;
        }
        let mut changed = false;
        for row in &mut store.doc.records {
            if !row.terminal() {
                row.status = "interrupted".into();
                row.updated = time.max(row.started);
                changed = true;
            }
        }
        changed |= store.prune(time);
        if changed {
            let _ = store.persist();
        }
        store
    }
    fn view(&self) -> View {
        View {
            token: digest(self.raw.as_deref().unwrap_or_default()),
            error: self.error,
            records: self.doc.records.clone(),
        }
    }
    fn prune(&mut self, time: u64) -> bool {
        let before = self.doc.records.len();
        self.doc
            .records
            .retain(|r| !r.terminal() || time.saturating_sub(r.updated) <= MAX_AGE);
        let mut remove = self
            .doc
            .records
            .iter()
            .filter(|r| r.terminal())
            .count()
            .saturating_sub(COMPLETED);
        self.doc.records.retain(|r| {
            if remove > 0 && r.terminal() {
                remove -= 1;
                false
            } else {
                true
            }
        });
        before != self.doc.records.len()
    }
    fn refresh(&mut self, time: u64, retry: bool) -> View {
        if self.prune(time) || (retry && self.error != 0) {
            let _ = self.persist();
        }
        self.view()
    }
    fn persist(&mut self) -> Result<(), StoreError> {
        let result = (|| {
            if self.blocked {
                return Err(StoreError::RecoveryRequired);
            }
            let directory =
                super::private_file::directory(&self.path, true)?.ok_or(StoreError::Write)?;
            let current = super::private_file::read(&directory, FILE, MAX_BYTES)?;
            if current != self.raw {
                self.raw = current;
                self.blocked = true;
                return Err(StoreError::Conflict);
            }
            let bytes = serde_json::to_vec(&self.doc).map_err(|_| StoreError::InvalidInput)?;
            if bytes.len() > MAX_BYTES {
                return Err(StoreError::Capacity);
            }
            let result = super::private_file::replace(&directory, FILE, &bytes, &self.io);
            if result.is_ok() || result == Err(StoreError::Indeterminate) {
                self.raw = Some(bytes);
            }
            result
        })();
        self.error = u8::from(result.is_err());
        result
    }
    fn observe(&mut self, records: Vec<Record>, time: u64) {
        for mut record in records {
            if !record.valid() {
                self.error = 1;
                continue;
            }
            if let Some(index) = self.doc.records.iter().position(|r| r.id == record.id) {
                record.started = self.doc.records[index].started;
                self.doc.records.remove(index);
            } else {
                record.started = time;
            }
            record.updated = time.max(record.started);
            self.doc.records.push(record);
        }
        self.prune(time);
        // Coordinator independently caps current records; reject invalid callers too.
        if self.doc.records.len() > COMPLETED + CURRENT {
            self.error = 1;
            return;
        }
        let _ = self.persist();
    }
    fn clear(&mut self, expected: &str, time: u64) -> Result<(), StoreError> {
        if !valid_token(expected) {
            return Err(StoreError::InvalidInput);
        }
        if self.doc.last_clear == expected {
            if self.error != 0 {
                self.persist()?;
            }
            return Ok(());
        }
        if self.view().token != expected {
            return Err(StoreError::Conflict);
        }
        if self.blocked {
            let directory =
                super::private_file::directory(&self.path, true)?.ok_or(StoreError::Write)?;
            let raw = super::private_file::read(&directory, FILE, MAX_BYTES)?;
            if digest(raw.as_deref().unwrap_or_default()) != expected {
                return Err(StoreError::Conflict);
            }
            if let Some(bytes) = &raw {
                let recovery = format!("native-transfers-recovery-{expected}.json");
                match super::private_file::read(&directory, &recovery, MAX_BYTES)? {
                    Some(previous) if previous != *bytes => return Err(StoreError::Conflict),
                    Some(_) => (),
                    None => super::private_file::replace(&directory, &recovery, bytes, &self.io)?,
                }
            }
            self.raw = raw;
            self.blocked = false;
        }
        self.doc.records.retain(|r| !r.terminal());
        self.doc.last_clear = expected.into();
        self.prune(time);
        self.persist()
    }
}

enum Command {
    Observe(Vec<Record>),
    Refresh(bool, oneshot::Sender<View>),
    Clear(String, oneshot::Sender<Result<View, StoreError>>),
}
pub(super) struct Owner {
    sender: mpsc::Sender<Command>,
    view: watch::Receiver<View>,
    task: tokio::task::JoinHandle<()>,
    seen: BTreeMap<String, Record>,
}
impl Owner {
    pub(super) fn spawn(path: PathBuf) -> Self {
        Self::with_clock(path, now)
    }
    pub(super) fn with_clock(path: PathBuf, clock: fn() -> u64) -> Self {
        let (sender, mut commands) = mpsc::channel(8);
        let (states, view) = watch::channel(View {
            error: 2,
            ..View::default()
        });
        let task = tokio::spawn(async move {
            let Ok(mut store) =
                tokio::task::spawn_blocking(move || Store::open(path, SystemIo, clock())).await
            else {
                return;
            };
            states.send_if_modified(|old| {
                let next = store.view();
                if *old == next {
                    false
                } else {
                    *old = next;
                    true
                }
            });
            while let Some(command) = commands.recv().await {
                let work = tokio::task::spawn_blocking(move || {
                    let time = clock();
                    match command {
                        Command::Observe(records) => store.observe(records, time),
                        Command::Refresh(retry, reply) => {
                            let _ = reply.send(store.refresh(time, retry));
                        }
                        Command::Clear(token, reply) => {
                            let result = store.clear(&token, time).map(|()| store.view());
                            let _ = reply.send(result);
                        }
                    }
                    store
                })
                .await;
                match work {
                    Ok(next) => {
                        store = next;
                        states.send_if_modified(|old| {
                            let next = store.view();
                            if *old == next {
                                false
                            } else {
                                *old = next;
                                true
                            }
                        });
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            sender,
            view,
            task,
            seen: BTreeMap::new(),
        }
    }
    pub(super) async fn changed(&mut self) {
        if self.view.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
    pub(super) async fn read(&self, retry: bool) -> Result<View, StoreError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(Command::Refresh(retry, tx))
            .await
            .map_err(|_| StoreError::Read)?;
        rx.await.map_err(|_| StoreError::Read)
    }
    pub(super) async fn clear(&self, token: String) -> Result<View, StoreError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(Command::Clear(token, tx))
            .await
            .map_err(|_| StoreError::Write)?;
        rx.await.map_err(|_| StoreError::Write)?
    }
    pub(super) async fn observe(&mut self, coordinator: &Coordinator) {
        let mut delta = Vec::new();
        let mut present = BTreeSet::new();
        for (id, entry) in &coordinator.transfers {
            let view = &entry.view;
            let record = Record {
                id: format!("{:032x}-{:016x}", id.0, id.1),
                peer: if view.direction == Direction::Inbound {
                    view.details.sender.clone()
                } else {
                    entry.recipient.clone().unwrap_or_default()
                },
                inbound: view.direction == Direction::Inbound,
                status: match view.phase {
                    Phase::Finished => "finished",
                    Phase::Rejected => "rejected",
                    Phase::Cancelled => "cancelled",
                    Phase::Failed => "failed",
                    Phase::Disconnected => "disconnected",
                    _ => "active",
                }
                .into(),
                kind: view.details.kind.wire().into(),
                files: view.details.files.len() as u32,
                total: view.details.total_bytes,
                transferred: if view.phase.is_terminal() {
                    view.details.transferred_bytes
                } else {
                    0
                },
                started: 0,
                updated: 0,
            };
            present.insert(record.id.clone());
            if self
                .seen
                .get(&record.id)
                .is_some_and(|old| old.terminal() || old == &record)
            {
                continue;
            }
            self.seen.insert(record.id.clone(), record.clone());
            delta.push(record);
        }
        self.seen.retain(|id, _| present.contains(id));
        if !delta.is_empty() {
            let _ = self.sender.send(Command::Observe(delta)).await;
        }
    }
    pub(super) async fn close(self) {
        drop(self.sender);
        let _ = self.task.await;
    }
}
#[cfg(test)]
mod tests;
