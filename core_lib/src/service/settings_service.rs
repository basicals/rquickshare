//! Bounded settings writer and typed Settings1 projection, separate from transfers.
use std::fs::File;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use zbus::zvariant::Type;
use zbus::{Connection, Message};

use super::native_settings::{
    Base, CardMode, DirectoryIntent, Edit, Snapshot, Source, Store, StoreError, Values,
};
use crate::receive::Directory;

pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Settings1";
pub const ERROR_PREFIX: &str = "io.github.morozhenko_apps.RQuickShare.Settings.Error.";

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
pub struct SettingsView {
    pub epoch: String,
    pub available: bool,
    pub source: u8,
    pub configured: bool,
    pub revision: u64,
    pub token: String,
    pub visibility: u8,
    pub port: i32,
    pub card_mode: u8,
    pub directory_display: String,
    pub visibility_recovered: bool,
    pub applied_live_revision: u64,
    pub running: bool,
    pub restart_required: bool,
    /// 0 none, 1 storage unreadable, 2 invalid configuration, 3 durability uncertain,
    /// 4 saved but live apply unavailable. Never contains raw diagnostics.
    pub error: u8,
    pub last_request: String,
}
impl std::fmt::Debug for SettingsView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsView")
            .field("available", &self.available)
            .field("revision", &self.revision)
            .field("applied_live_revision", &self.applied_live_revision)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
impl SettingsView {
    pub fn unavailable(epoch: u128) -> Self {
        Self {
            epoch: format!("{epoch:032x}"),
            available: false,
            source: 0,
            configured: false,
            revision: 0,
            token: String::new(),
            visibility: 1,
            port: -1,
            card_mode: 0,
            directory_display: String::new(),
            visibility_recovered: false,
            applied_live_revision: 0,
            running: false,
            restart_required: false,
            error: 0,
            last_request: String::new(),
        }
    }
    pub fn base(&self) -> Base {
        Base {
            revision: self.revision,
            token: self.token.clone(),
        }
    }
    pub fn valid(&self, epoch: u128) -> bool {
        self.epoch == format!("{epoch:032x}")
            && self.source <= 2
            && self.visibility <= 2
            && (-1..=65535).contains(&self.port)
            && self.card_mode <= 1
            && self.directory_display.len() <= 12288
            && !self.directory_display.contains('\0')
            && self.error <= 4
            && self.applied_live_revision <= self.revision
            && (self.token.is_empty() && (!self.available || self.error == 1)
                || valid_hex(&self.token, 64))
            && (self.last_request.is_empty() || valid_hex(&self.last_request, 32))
    }
    fn observe(&mut self, snapshot: &Snapshot, running_port: i32) {
        self.available = true;
        self.source = match snapshot.source {
            Source::Defaults => 0,
            Source::Legacy => 1,
            Source::Native => 2,
        };
        self.configured = snapshot.values.is_some();
        self.revision = snapshot.base.revision;
        self.token.clone_from(&snapshot.base.token);
        self.visibility_recovered = snapshot.visibility_recovered;
        self.last_request = snapshot.last_request().unwrap_or_default().into();
        self.error = if snapshot.values.is_some() { 0 } else { 2 };
        if let Some(values) = &snapshot.values {
            self.visibility = values.visibility;
            self.port = values.port.map_or(-1, i32::from);
            self.card_mode = u8::from(values.card_mode == CardMode::Full);
            self.directory_display = values
                .directory
                .as_ref()
                .map(|directory| directory.path().to_string_lossy().into_owned())
                .unwrap_or_default();
            self.restart_required = !self.running || self.port != running_port;
        } else {
            self.restart_required = true;
        }
        // External recovery may replace/reduce the saved generation.
        if self.applied_live_revision > self.revision {
            self.applied_live_revision = 0;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Type)]
pub struct SaveIntent {
    pub epoch: String,
    pub revision: u64,
    pub token: String,
    pub request: String,
    pub visibility: u8,
    pub port: i32,
    pub card_mode: u8,
    /// 0 retain, 1 default, 2 replace with exactly one directory FD.
    pub directory: u8,
    pub recover: bool,
}
impl SaveIntent {
    pub fn valid(&self, epoch: u128) -> bool {
        self.epoch == format!("{epoch:032x}")
            && valid_hex(&self.token, 64)
            && valid_hex(&self.request, 32)
            && self.visibility <= 2
            && (-1..=65535).contains(&self.port)
            && self.card_mode <= 1
            && self.directory <= 2
    }
    pub fn edit(self, file: Option<File>) -> Result<Edit, StoreError> {
        let epoch = u128::from_str_radix(&self.epoch, 16).map_err(|_| StoreError::InvalidInput)?;
        if !self.valid(epoch) {
            return Err(StoreError::InvalidInput);
        }
        let directory = match (self.directory, file) {
            (0, None) => DirectoryIntent::Retain,
            (1, None) => DirectoryIntent::Default,
            (2, Some(file)) => DirectoryIntent::Replace(file),
            _ => return Err(StoreError::InvalidInput),
        };
        Ok(Edit {
            base: Base {
                revision: self.revision,
                token: self.token,
            },
            request: self.request,
            visibility: self.visibility,
            port: if self.port == -1 {
                None
            } else {
                Some(u16::try_from(self.port).map_err(|_| StoreError::InvalidInput)?)
            },
            card_mode: if self.card_mode == 0 {
                CardMode::Compact
            } else {
                CardMode::Full
            },
            directory,
            recover: self.recover,
        })
    }
}
fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) enum RootUpdate {
    Retain,
    Default,
    Replace(Directory),
}
pub(super) struct Apply {
    pub values: Values,
    pub root: RootUpdate,
    pub reply: oneshot::Sender<bool>,
}
pub(super) enum Command {
    Save(Edit, Message),
    Reload(Message),
}

pub(super) struct SettingsOwner {
    commands: Option<mpsc::Sender<Command>>,
    pub apply: mpsc::Receiver<Apply>,
    view: Arc<Mutex<SettingsView>>,
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
    outstanding: Arc<AtomicUsize>,
}
impl SettingsOwner {
    pub fn spawn(
        store: Store,
        initial: Result<Snapshot, StoreError>,
        epoch: u128,
        running: bool,
        running_port: i32,
        connection: Connection,
    ) -> Self {
        Self::spawn_with_io(store, initial, epoch, running, running_port, connection)
    }
    pub(super) fn spawn_with_io<I: super::native_settings::PersistenceIo + Send + 'static>(
        store: Store<I>,
        initial: Result<Snapshot, StoreError>,
        epoch: u128,
        running: bool,
        running_port: i32,
        connection: Connection,
    ) -> Self {
        let mut view = SettingsView::unavailable(epoch);
        let active_directory = initial
            .as_ref()
            .ok()
            .filter(|_| running)
            .and_then(|snapshot| snapshot.values.as_ref())
            .and_then(|values| values.directory.clone());
        view.available = true;
        view.running = running;
        match initial {
            Ok(snapshot) => {
                view.observe(&snapshot, running_port);
                if running {
                    view.applied_live_revision = snapshot.base.revision;
                }
            }
            Err(_) => view.error = 1,
        }
        let view = Arc::new(Mutex::new(view));
        let (commands, requests) = mpsc::channel(8);
        let (apply_sender, apply) = mpsc::channel(1);
        let stop = CancellationToken::new();
        let outstanding = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn(
            Writer {
                store: Some(store),
                requests,
                apply: apply_sender,
                view: view.clone(),
                running_port,
                active_directory,
                connection,
                stop: stop.clone(),
                outstanding: outstanding.clone(),
            }
            .run(),
        );
        Self {
            commands: Some(commands),
            apply,
            view,
            stop,
            task: Some(task),
            outstanding,
        }
    }
    pub fn view(&self) -> SettingsView {
        self.view
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
    pub fn submit(&self, command: Command) -> Result<(), super::Error> {
        let commands = self.commands.as_ref().ok_or(super::Error::NotRunning)?;
        self.outstanding.fetch_add(1, Ordering::SeqCst);
        commands.try_send(command).map_err(|error| {
            self.outstanding.fetch_sub(1, Ordering::SeqCst);
            match error {
                mpsc::error::TrySendError::Full(_) => super::Error::Busy,
                mpsc::error::TrySendError::Closed(_) => super::Error::NotRunning,
            }
        })
    }
    pub fn idle(&self) -> bool {
        self.outstanding.load(Ordering::SeqCst) == 0
    }
    pub fn mark_unapplied(&self) {
        let mut view = self
            .view
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if view.configured {
            view.error = 4;
        }
    }
    pub async fn close(mut self) {
        self.commands.take();
        self.stop.cancel();
        self.apply.close();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}
impl Drop for SettingsOwner {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

struct Writer<I> {
    store: Option<Store<I>>,
    requests: mpsc::Receiver<Command>,
    apply: mpsc::Sender<Apply>,
    view: Arc<Mutex<SettingsView>>,
    running_port: i32,
    active_directory: Option<super::native_settings::DirectoryRecord>,
    connection: Connection,
    stop: CancellationToken,
    outstanding: Arc<AtomicUsize>,
}
impl<I: super::native_settings::PersistenceIo + Send + 'static> Writer<I> {
    fn view(&self) -> SettingsView {
        self.view
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
    async fn run(mut self) {
        loop {
            let command = if self.stop.is_cancelled() {
                self.requests.close();
                self.requests.recv().await
            } else {
                tokio::select! { biased; _ = self.stop.cancelled() => { self.requests.close(); self.requests.recv().await }, command = self.requests.recv() => command }
            };
            let Some(command) = command else { break };
            let (edit, message) = match command {
                Command::Save(edit, message) => (Some(edit), message),
                Command::Reload(message) => (None, message),
            };
            let store = self.store.take().expect("one owned store operation");
            let save = edit.is_some();
            let active_directory = self.active_directory.clone();
            let work = tokio::task::spawn_blocking(move || {
                let result = match edit {
                    Some(edit) => {
                        let root = match &edit.directory {
                            DirectoryIntent::Retain => Ok(RootUpdate::Retain),
                            DirectoryIntent::Default => Ok(RootUpdate::Default),
                            DirectoryIntent::Replace(file) => file
                                .try_clone()
                                .map_err(|_| StoreError::Directory)
                                .and_then(Directory::adopt)
                                .map(RootUpdate::Replace),
                        };
                        root.and_then(|root| {
                            store.save(edit).map(|snapshot| {
                                let root = match root {
                                    RootUpdate::Retain => match snapshot
                                        .values
                                        .as_ref()
                                        .and_then(|values| values.directory.as_ref())
                                    {
                                        Some(record)
                                            if active_directory.as_ref().is_some_and(
                                                |active| active.same_object(record),
                                            ) =>
                                        {
                                            Ok(RootUpdate::Retain)
                                        }
                                        Some(record) => record
                                            .open()
                                            .and_then(Directory::adopt)
                                            .map(RootUpdate::Replace),
                                        None => Ok(RootUpdate::Default),
                                    },
                                    root => Ok(root),
                                };
                                (snapshot, root)
                            })
                        })
                    }
                    None => store
                        .load()
                        .map(|snapshot| (snapshot, Ok(RootUpdate::Retain))),
                };
                (store, result)
            })
            .await;
            let (store, result) = match work {
                Ok(work) => work,
                Err(_) => break,
            };
            self.store = Some(store);
            let response = match result {
                Ok((snapshot, root)) => {
                    {
                        let mut view = self
                            .view
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner());
                        let uncertain = view.error == 3;
                        if view.token != snapshot.base.token {
                            view.applied_live_revision = 0;
                        }
                        view.observe(&snapshot, self.running_port);
                        if !save && uncertain {
                            view.error = 3;
                        }
                    }
                    let running = self.view().running;
                    if save && running {
                        let (reply, receive) = oneshot::channel();
                        let applied = if let (Some(values), Ok(root)) =
                            (snapshot.values.clone(), root)
                        {
                            tokio::select! { biased; _ = self.stop.cancelled() => false, result = async {
                                self.apply.send(Apply { values, root, reply }).await.map_err(|_| ())?;
                                receive.await.map_err(|_| ())
                            } => result.unwrap_or(false) }
                        } else {
                            false
                        };
                        let mut view = self
                            .view
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner());
                        if applied {
                            self.active_directory = snapshot
                                .values
                                .as_ref()
                                .and_then(|values| values.directory.clone());
                            view.applied_live_revision = snapshot.base.revision;
                            view.visibility_recovered = false;
                        } else {
                            view.error = 4;
                        }
                    }
                    Ok(self.view())
                }
                Err(error) => {
                    if error == StoreError::Indeterminate {
                        self.view
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .error = 3;
                    }
                    if !save {
                        self.view
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner())
                            .error = 1;
                    }
                    Err(error)
                }
            };
            self.outstanding.fetch_sub(1, Ordering::SeqCst);
            if self.stop.is_cancelled() {
                continue;
            }
            let header = message.header();
            tokio::select! { biased; _ = self.stop.cancelled() => (), _ = async {
                match response {
                    Ok(view) => {
                        let _ = self.connection.emit_signal(None::<&str>, super::bus::PATH, INTERFACE, "SettingsChanged", &(view.epoch.clone(), view.revision)).await;
                        let _ = self.connection.reply(&header, &(view,)).await;
                    }
                    Err(error) => { let _ = self.connection.reply_error(&header, format!("{ERROR_PREFIX}{error:?}"), &()).await; }
                }
            } => () }
        }
    }
}

pub fn store_error(name: &str) -> Option<StoreError> {
    Some(match name.strip_prefix(ERROR_PREFIX)? {
        "Read" => StoreError::Read,
        "Capacity" => StoreError::Capacity,
        "UnsafeStorage" => StoreError::UnsafeStorage,
        "InvalidInput" => StoreError::InvalidInput,
        "Directory" => StoreError::Directory,
        "Conflict" => StoreError::Conflict,
        "ReplayConflict" => StoreError::ReplayConflict,
        "RecoveryRequired" => StoreError::RecoveryRequired,
        "BackupCapacity" => StoreError::BackupCapacity,
        "Write" => StoreError::Write,
        "Indeterminate" => StoreError::Indeterminate,
        _ => return None,
    })
}
