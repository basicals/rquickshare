//! Bounded settings writer and typed Settings2 projection, separate from transfers.
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

pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Settings2";
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
    pub device_name: String,
    pub resolved_name: String,
    /// 0 no live identity, 1 pending, 2 locally applied, 3 unavailable.
    pub name_status: u8,
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
            device_name: String::new(),
            resolved_name: String::new(),
            name_status: 0,
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
            && valid_name(&self.device_name)
            && valid_name(&self.resolved_name)
            && self.name_status <= 3
            && (self.name_status == 0 || !self.resolved_name.is_empty())
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
            self.device_name = values
                .device_name
                .as_ref()
                .map(|name| name.as_str().to_owned())
                .unwrap_or_default();
            self.visibility = values.visibility;
            self.port = values.port.map_or(-1, i32::from);
            self.card_mode = u8::from(values.card_mode == CardMode::Full);
            self.directory_display = values
                .directory
                .as_ref()
                .map(|directory| directory.path().to_string_lossy().into_owned())
                .unwrap_or_default();
            self.restart_required =
                !self.running || effective_port(self.port) != effective_port(running_port);
        } else {
            self.restart_required = true;
        }
        // External recovery may replace/reduce the saved generation.
        if self.applied_live_revision > self.revision {
            self.applied_live_revision = 0;
        }
    }
}
fn effective_port(port: i32) -> i32 {
    if port <= 0 {
        -1
    } else {
        port
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
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
    pub device_name: String,
}
impl std::fmt::Debug for SaveIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveIntent")
            .field("revision", &self.revision)
            .field("device_name", &"[redacted]")
            .finish_non_exhaustive()
    }
}
fn valid_name(value: &str) -> bool {
    value.is_empty() || crate::identity::DeviceName::new(value).is_ok()
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
            && valid_name(&self.device_name)
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
            device_name: if self.device_name.is_empty() {
                None
            } else {
                Some(
                    crate::identity::DeviceName::new(&self.device_name)
                        .map_err(|_| StoreError::InvalidInput)?,
                )
            },
            recover: self.recover,
        })
    }
}
pub(super) fn valid_hex(value: &str, length: usize) -> bool {
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
#[derive(Clone, Copy, Default)]
pub(super) struct ApplyReceipt {
    pub policy_applied: bool,
    pub name_applied: bool,
}
pub(super) struct Apply {
    pub values: Values,
    pub root: RootUpdate,
    pub apply_visibility: bool,
    pub renew_visibility: bool,
    pub reply: oneshot::Sender<ApplyReceipt>,
}
pub(super) enum Command {
    Save(Edit, Message),
    Reload(Message),
    Visibility(super::visibility_service::VisibilityIntent, Message),
}

type IdentitySource = Arc<Mutex<Option<crate::identity::IdentityPolicy>>>;
fn project_name(mut view: SettingsView, identity: &IdentitySource) -> SettingsView {
    let identity = identity
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone();
    if let Some(identity) = identity.filter(|_| view.running) {
        let current = identity.snapshot();
        view.resolved_name = current.name.as_str().to_owned();
        let configured = if view.device_name.is_empty() {
            None
        } else {
            crate::identity::DeviceName::new(&view.device_name).ok()
        };
        view.name_status = if identity.resolve_override(configured.as_ref()) != current.name {
            1
        } else {
            match identity.applied(current.revision) {
                Ok(true) => 2,
                Ok(false) => 1,
                Err(_) => 3,
            }
        };
        if view.name_status != 2 {
            view.applied_live_revision = 0;
            if view.name_status == 3 && view.error == 0 {
                view.error = 4;
            }
        }
    }
    view
}

pub(super) struct SettingsOwner {
    commands: Option<mpsc::Sender<Command>>,
    pub apply: mpsc::Receiver<Apply>,
    view: Arc<Mutex<SettingsView>>,
    identity: IdentitySource,
    visibility: Arc<Mutex<Option<crate::VisibilityReader>>>,
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
        let active_visibility = initial
            .as_ref()
            .ok()
            .filter(|_| running)
            .and_then(|snapshot| snapshot.values.as_ref())
            .map(|values| values.visibility);
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
        let identity = Arc::new(Mutex::new(None));
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
                identity: identity.clone(),
                running_port,
                active_directory,
                active_visibility,
                last_visibility_applied: None,
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
            identity,
            visibility: Arc::new(Mutex::new(None)),
            stop,
            task: Some(task),
            outstanding,
        }
    }
    pub fn view(&self) -> SettingsView {
        let view = self
            .view
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        project_name(view, &self.identity)
    }
    pub fn bind_visibility(&self, reader: crate::VisibilityReader) {
        *self.visibility.lock().unwrap_or_else(|p| p.into_inner()) = Some(reader);
    }
    pub fn visibility_view(&self) -> super::visibility_service::VisibilityView {
        let reader = self.visibility.lock().unwrap_or_else(|p| p.into_inner());
        super::visibility_service::VisibilityView::project(&self.view().epoch, reader.as_ref())
    }
    pub fn bind_identity(&self, identity: Option<crate::identity::IdentityPolicy>) {
        *self
            .identity
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = identity;
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
    identity: IdentitySource,
    running_port: i32,
    active_directory: Option<super::native_settings::DirectoryRecord>,
    active_visibility: Option<u8>,
    last_visibility_applied: Option<String>,
    connection: Connection,
    stop: CancellationToken,
    outstanding: Arc<AtomicUsize>,
}
impl<I: super::native_settings::PersistenceIo + Send + 'static> Writer<I> {
    fn view(&self) -> SettingsView {
        let view = self
            .view
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        project_name(view, &self.identity)
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
            let (edit, quick, message) = match command {
                Command::Save(edit, message) => (Some(edit), None, message),
                Command::Reload(message) => (None, None, message),
                Command::Visibility(intent, message) => (None, Some(intent), message),
            };
            let store = self.store.take().expect("one owned store operation");
            let is_quick = quick.is_some();
            let save = edit.is_some() || is_quick;
            let active_directory = self.active_directory.clone();
            let work = tokio::task::spawn_blocking(move || {
                let result = if let Some(intent) = quick {
                    store
                        .save_visibility(intent.base(), intent.request, intent.mode)
                        .map(|(snapshot, fresh)| (snapshot, Ok(RootUpdate::Retain), fresh))
                } else {
                    match edit {
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
                                    (snapshot, root, false)
                                })
                            })
                        }
                        None => store
                            .load()
                            .map(|snapshot| (snapshot, Ok(RootUpdate::Retain), false)),
                    }
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
                Ok((snapshot, root, fresh_quick)) => {
                    let before = self.view();
                    let full_was_applied =
                        before.error == 0 && before.applied_live_revision == before.revision;
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
                    if save && running && (!is_quick || fresh_quick) {
                        let (reply, receive) = oneshot::channel();
                        let receipt = if let (Some(values), Ok(root)) =
                            (snapshot.values.clone(), root)
                        {
                            let apply_visibility =
                                self.active_visibility != Some(values.visibility);
                            tokio::select! { biased; _ = self.stop.cancelled() => ApplyReceipt::default(), result = async {
                                self.apply.send(Apply { values, root, apply_visibility, renew_visibility: is_quick, reply }).await.map_err(|_| ())?;
                                receive.await.map_err(|_| ())
                            } => result.unwrap_or_default() }
                        } else {
                            ApplyReceipt::default()
                        };
                        if is_quick && receipt.policy_applied {
                            self.last_visibility_applied =
                                snapshot.last_request().map(str::to_owned);
                        }
                        if receipt.policy_applied {
                            if !is_quick {
                                self.active_directory = snapshot
                                    .values
                                    .as_ref()
                                    .and_then(|values| values.directory.clone());
                            }
                            self.active_visibility =
                                snapshot.values.as_ref().map(|values| values.visibility);
                        }
                        let mut view = self
                            .view
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner());
                        if receipt.policy_applied
                            && receipt.name_applied
                            && (!is_quick || full_was_applied)
                        {
                            view.applied_live_revision = snapshot.base.revision;
                            view.visibility_recovered = false;
                        } else {
                            view.error = 4;
                        }
                    }
                    if is_quick
                        && (!running
                            || (!fresh_quick
                                && self.last_visibility_applied.as_deref()
                                    != snapshot.last_request()))
                    {
                        self.view.lock().unwrap_or_else(|p| p.into_inner()).error = 4;
                    }
                    if is_quick && !fresh_quick && before.error == 4 {
                        self.view.lock().unwrap_or_else(|p| p.into_inner()).error = 4;
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
            let observed = self.view();
            tokio::select! { biased; _ = self.stop.cancelled() => (), _ = async {
                let _ = self.connection.emit_signal(None::<&str>, super::bus::PATH, INTERFACE, "SettingsChanged", &(observed.epoch, observed.revision)).await;
                match response {
                    Ok(view) => {
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
