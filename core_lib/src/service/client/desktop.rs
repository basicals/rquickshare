//! Bounded desktop capability ingress. GUI owns preparation; invocation never sends.

use std::{
    fs::File,
    os::fd::AsFd,
    sync::{Arc, Mutex},
};

use futures::StreamExt;
use tokio::{net::UnixStream, sync::watch, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use zbus::{fdo, message::Header, zvariant::Fd, Connection};

use super::controller::{ControllerError, PanelHandle};
use crate::{
    file_source::{OpenedFile, OpenedSelection, SelectionLimits},
    sessions::details::{FILE_LIMIT, MANIFEST_BYTES, NAME_LIMIT},
};

pub const SERVICE: &str = "io.github.morozhenko_apps.RQuickShare.Desktop";
pub const PATH: &str = "/io/github/morozhenko_apps/RQuickShare/Desktop";
pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Desktop1";
pub const INTROSPECTION: &str =
    include_str!("../../../dbus/io.github.morozhenko_apps.RQuickShare.Desktop1.xml");

pub fn valid_selection(source: &OpenedSelection) -> bool {
    !source.files().is_empty()
        && source.files().len() <= FILE_LIMIT
        && source.total_bytes() <= SelectionLimits::default().max_bytes
        && source
            .files()
            .iter()
            .try_fold(0usize, |sum, file| {
                (file.relative_name().len() <= NAME_LIMIT).then_some(())?;
                sum.checked_add(file.relative_name().len())
                    .filter(|sum| *sum <= MANIFEST_BYTES)
            })
            .is_some()
}

struct Gate {
    ownership: Mutex<(bool, bool)>,
    state: watch::Sender<bool>,
}
#[derive(Clone)]
pub struct PreparationGate(Arc<Gate>);
pub struct PreparationPermit {
    gate: PreparationGate,
}
impl Default for PreparationGate {
    fn default() -> Self {
        Self::new()
    }
}
impl PreparationGate {
    pub fn new() -> Self {
        let (state, _) = watch::channel(false);
        Self(Arc::new(Gate {
            ownership: Mutex::new((false, false)),
            state,
        }))
    }
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.0.state.subscribe()
    }
    pub fn try_acquire(&self) -> Option<PreparationPermit> {
        let mut ownership = self.0.ownership.lock().expect("preparation ownership");
        if ownership.0 || ownership.1 {
            return None;
        }
        ownership.0 = true;
        self.0.state.send_replace(true);
        Some(PreparationPermit { gate: self.clone() })
    }
    pub fn close(&self) {
        self.0.ownership.lock().expect("preparation ownership").1 = true;
    }
}
impl Drop for PreparationPermit {
    fn drop(&mut self) {
        let mut ownership = self.gate.0.ownership.lock().expect("preparation ownership");
        ownership.0 = false;
        self.gate.0.state.send_replace(false);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    InvalidSelection,
    Busy,
    Recovery,
    BackendUnavailable,
    PanelUnavailable,
}
impl ErrorCode {
    pub fn wire(self) -> &'static str {
        match self {
            Self::InvalidSelection => "invalid_selection",
            Self::Busy => "busy",
            Self::Recovery => "recovery",
            Self::BackendUnavailable => "backend_unavailable",
            Self::PanelUnavailable => "panel_unavailable",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "invalid_selection" => Self::InvalidSelection,
            "busy" => Self::Busy,
            "recovery" => Self::Recovery,
            "backend_unavailable" => Self::BackendUnavailable,
            "panel_unavailable" => Self::PanelUnavailable,
            _ => return None,
        })
    }
    pub fn message(self) -> &'static str {
        match self {
        Self::InvalidSelection => "Cannot prepare these files. Check local access and file limits, then choose them again.",
        Self::Busy => "Files are already prepared or another preparation is active. Finish or discard it in the panel.",
        Self::Recovery => "Preparation needs recovery. Review the panel before retrying; do not repeat the send.",
        Self::BackendUnavailable => "The transfer service is unavailable. Start it, then reconnect. Files were not sent.",
        Self::PanelUnavailable => "The panel connection changed. Review the current state before retrying; no automatic send was requested.",
    }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DesktopState {
    pub show_revision: u64,
    pub error_revision: u64,
    pub error: Option<ErrorCode>,
    pub staging: bool,
    pub closed: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Status {
    Preparing,
    Ready,
    Recovery,
    Failed,
    Aborted,
}
impl Status {
    fn wire(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Ready => "ready",
            Self::Recovery => "recovery",
            Self::Failed => "failed",
            Self::Aborted => "aborted",
        }
    }
}
#[derive(Eq, PartialEq)]
struct Fingerprint {
    name: String,
    size: u64,
    device: u64,
    inode: u64,
}
struct Record {
    owner: String,
    token: u64,
    files: Vec<OpenedFile>,
    batches: Vec<Vec<Fingerprint>>,
    bytes: u64,
    names: usize,
    permit: Option<PreparationPermit>,
    status: watch::Sender<Status>,
}
struct Inner {
    record: Option<Record>,
    next: u64,
    worker: Option<JoinHandle<()>>,
}
struct Shared {
    inner: Mutex<Inner>,
    controller: Option<PanelHandle>,
    gate: PreparationGate,
    updates: watch::Sender<DesktopState>,
    stop: CancellationToken,
    uid: u32,
}
fn rejected(code: &str) -> fdo::Error {
    fdo::Error::Failed(code.into())
}
fn record<'a>(inner: &'a mut Inner, owner: &str, token: u64) -> fdo::Result<&'a mut Record> {
    let record = inner
        .record
        .as_mut()
        .ok_or_else(|| rejected("UnknownUpload"))?;
    if record.owner != owner {
        return Err(fdo::Error::AccessDenied("WrongOwner".into()));
    }
    if record.token != token {
        return Err(rejected("UnknownUpload"));
    }
    Ok(record)
}
impl Shared {
    fn show(&self) {
        self.updates
            .send_modify(|state| state.show_revision = state.show_revision.saturating_add(1));
    }
    async fn sender(&self, connection: &Connection, header: Header<'_>) -> fdo::Result<String> {
        if self.stop.is_cancelled() {
            return Err(rejected("Closed"));
        }
        let sender = header
            .sender()
            .ok_or_else(|| fdo::Error::AccessDenied("MissingSender".into()))?;
        let uid = fdo::DBusProxy::new(connection)
            .await?
            .get_connection_unix_user(sender.clone().into())
            .await?;
        if uid != self.uid {
            return Err(fdo::Error::AccessDenied("DifferentUser".into()));
        }
        Ok(sender.to_string())
    }
    fn begin(&self, owner: &str) -> fdo::Result<u64> {
        let controller = self
            .controller
            .as_ref()
            .ok_or_else(|| rejected("Unavailable"))?;
        let mut inner = self.inner.lock().expect("desktop ownership");
        if let Some(record) = &inner.record {
            if record.owner == owner {
                return Ok(record.token);
            }
            self.show();
            return Err(rejected("Busy"));
        }
        let state = controller.subscribe().borrow().clone();
        if state.busy
            || state.pending
            || state.text_pending
            || state.disconnected
            || !state.view.as_ref().is_some_and(|view| {
                view.synchronized
                    && view.lifecycle == crate::service::Lifecycle::Running
                    && view.selections.is_empty()
            })
        {
            self.show();
            return Err(rejected("Busy"));
        }
        let permit = self.gate.try_acquire().ok_or_else(|| {
            self.show();
            rejected("Busy")
        })?;
        inner.next = inner
            .next
            .checked_add(1)
            .ok_or_else(|| rejected("Closed"))?;
        let token = inner.next;
        let (status, _) = watch::channel(Status::Preparing);
        inner.record = Some(Record {
            owner: owner.to_owned(),
            token,
            files: vec![],
            batches: vec![],
            bytes: 0,
            names: 0,
            permit: Some(permit),
            status,
        });
        self.updates.send_modify(|state| state.staging = true);
        Ok(token)
    }

    fn abort_staging(&self, owner: Option<&str>, token: Option<u64>) -> fdo::Result<Status> {
        let mut inner = self.inner.lock().expect("desktop ownership");
        let record = inner
            .record
            .as_mut()
            .ok_or_else(|| rejected("UnknownUpload"))?;
        if owner.is_some_and(|owner| owner != record.owner) {
            return Err(fdo::Error::AccessDenied("WrongOwner".into()));
        }
        if token.is_some_and(|token| token != record.token) {
            return Err(rejected("UnknownUpload"));
        }
        if record.permit.is_some() {
            record.files.clear();
            record.status.send_replace(Status::Aborted);
            self.updates.send_modify(|state| state.staging = false);
            record.permit.take();
        }
        let status = *record.status.borrow();
        Ok(status)
    }
}
struct Endpoint(Arc<Shared>);
#[zbus::interface(name = "io.github.morozhenko_apps.RQuickShare.Desktop1", spawn = false)]
impl Endpoint {
    async fn show_error(
        &self,
        code: &str,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        let has_fds = header.unix_fds().unwrap_or(0) != 0;
        self.0.sender(connection, header).await?;
        if has_fds {
            return Err(rejected("InvalidInput"));
        }
        let code = ErrorCode::parse(code).ok_or_else(|| rejected("InvalidInput"))?;
        let mut result = Ok(());
        self.0.updates.send_modify(|state| {
            if let Some(revision) = state.error_revision.checked_add(1) {
                state.error_revision = revision;
                state.error = Some(code);
            } else {
                result = Err(rejected("Closed"));
            }
        });
        result
    }
    async fn show(
        &self,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        self.0.sender(connection, header).await?;
        self.0.show();
        Ok(())
    }
    async fn begin(
        &self,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<u64> {
        let owner = self.0.sender(connection, header).await?;
        let token = self.0.begin(&owner)?;
        let alive = fdo::DBusProxy::new(connection)
            .await?
            .name_has_owner(owner.as_str().try_into().map_err(|_| rejected("Closed"))?)
            .await;
        if !matches!(alive, Ok(true)) {
            let mut inner = self.0.inner.lock().expect("desktop ownership");
            if inner
                .record
                .as_ref()
                .is_some_and(|record| record.owner == owner)
            {
                inner.record = None;
                self.0.updates.send_modify(|state| state.staging = false);
            }
            return Err(rejected("Closed"));
        }
        Ok(token)
    }

    async fn append_files(
        &self,
        token: u64,
        sequence: u32,
        entries: Vec<(Fd<'_>, String)>,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<u32> {
        let owner = self.0.sender(connection, header).await?;
        if entries.is_empty() || entries.len() > 16 {
            return Err(rejected("InvalidInput"));
        }
        let mut files = Vec::new();
        let mut fingerprints = Vec::new();
        let mut bytes = 0u64;
        let mut names = 0usize;
        for (fd, name) in entries {
            if name.len() > NAME_LIMIT {
                return Err(rejected("InvalidInput"));
            }
            let file = File::from(
                fd.as_fd()
                    .try_clone_to_owned()
                    .map_err(|_| rejected("InvalidInput"))?,
            );
            let source =
                OpenedFile::adopt(file, name.clone()).map_err(|_| rejected("InvalidInput"))?;
            let stat = rustix::fs::fstat(fd.as_fd()).map_err(|_| rejected("InvalidInput"))?;
            bytes = bytes
                .checked_add(source.size())
                .ok_or_else(|| rejected("InvalidInput"))?;
            names += name.len();
            fingerprints.push(Fingerprint {
                name,
                size: source.size(),
                device: stat.st_dev,
                inode: stat.st_ino,
            });
            files.push(source);
        }
        let mut inner = self.0.inner.lock().expect("desktop ownership");
        let record = record(&mut inner, &owner, token)?;
        let sequence = usize::try_from(sequence).map_err(|_| rejected("InvalidInput"))?;
        if let Some(previous) = record.batches.get(sequence) {
            return if previous == &fingerprints {
                Ok(sequence as u32 + 1)
            } else {
                Err(rejected("Conflict"))
            };
        }
        if record.permit.is_none() || sequence != record.batches.len() {
            return Err(rejected("StaleBatch"));
        }
        if record.files.len() + files.len() > FILE_LIMIT
            || record.names + names > MANIFEST_BYTES
            || record
                .bytes
                .checked_add(bytes)
                .is_none_or(|bytes| bytes > SelectionLimits::default().max_bytes)
        {
            return Err(rejected("InvalidInput"));
        }
        let mut names = std::collections::BTreeSet::new();
        if !record
            .files
            .iter()
            .chain(&files)
            .all(|file| names.insert(file.relative_name()))
        {
            return Err(rejected("InvalidInput"));
        }
        record.bytes += bytes;
        record.names += fingerprints
            .iter()
            .map(|entry| entry.name.len())
            .sum::<usize>();
        record.files.extend(files);
        record.batches.push(fingerprints);
        Ok(record.batches.len() as u32)
    }
    async fn commit(
        &self,
        token: u64,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        let owner = self.0.sender(connection, header).await?;
        let mut updates = {
            let mut inner = self.0.inner.lock().expect("desktop ownership");
            let record = record(&mut inner, &owner, token)?;
            let updates = record.status.subscribe();
            if let Some(permit) = record.permit.take() {
                self.0.updates.send_modify(|state| state.staging = false);
                let source = OpenedSelection::from_files(
                    std::mem::take(&mut record.files),
                    SelectionLimits::default(),
                );
                match source {
                    Ok(source) if valid_selection(&source) => {
                        let status = record.status.clone();
                        let controller = self
                            .0
                            .controller
                            .clone()
                            .ok_or_else(|| rejected("Unavailable"))?;
                        let stop = self.0.stop.clone();
                        let previous = inner.worker.take();
                        inner.worker = Some(tokio::spawn(async move {
                            if let Some(previous) = previous {
                                let _ = previous.await;
                            }
                            let completion = controller.choose(source);
                            let result = match completion {
                                Ok(completion) => {
                                    tokio::select! {biased;_=stop.cancelled()=>Err(ControllerError::Closed),result=completion.wait()=>result}
                                }
                                Err(error) => Err(error),
                            };
                            let outcome = if result.is_ok() {
                                Status::Ready
                            } else if controller.subscribe().borrow().pending {
                                Status::Recovery
                            } else {
                                Status::Failed
                            };
                            status.send_replace(outcome);
                            drop(permit);
                        }));
                    }
                    _ => {
                        record.status.send_replace(Status::Failed);
                        drop(permit);
                    }
                }
                self.0.show();
            }
            updates
        };
        updates
            .wait_for(|status| *status != Status::Preparing)
            .await
            .map_err(|_| rejected("Closed"))?;
        let result = updates.borrow().wire().to_owned();
        Ok(result)
    }
    async fn abort(
        &self,
        token: u64,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        let owner = self.0.sender(connection, header).await?;
        self.0.abort_staging(Some(&owner), Some(token))?;
        let mut status = {
            let mut inner = self.0.inner.lock().expect("desktop ownership");
            record(&mut inner, &owner, token)?.status.subscribe()
        };
        status
            .wait_for(|status| *status != Status::Preparing)
            .await
            .map_err(|_| rejected("Closed"))?;
        let result = status.borrow().wire().to_owned();
        Ok(result)
    }
}

#[derive(Clone)]
pub struct DesktopHandle {
    shared: Arc<Shared>,
}
impl DesktopHandle {
    pub fn subscribe(&self) -> watch::Receiver<DesktopState> {
        self.shared.updates.subscribe()
    }
    pub fn abort_staging(&self) {
        let _ = self.shared.abort_staging(None, None);
    }
}
pub struct DesktopServer {
    stop: CancellationToken,
    task: Option<JoinHandle<zbus::Result<()>>>,
    handle: DesktopHandle,
}
impl DesktopServer {
    pub async fn connect(
        stream: UnixStream,
        controller: PanelHandle,
        gate: PreparationGate,
    ) -> zbus::Result<Self> {
        Self::connect_with_uid(
            stream,
            controller,
            gate,
            rustix::process::geteuid().as_raw(),
        )
        .await
    }
    async fn connect_with_uid(
        stream: UnixStream,
        controller: PanelHandle,
        gate: PreparationGate,
        uid: u32,
    ) -> zbus::Result<Self> {
        Self::connect_optional_with_uid(stream, Some(controller), gate, uid).await
    }
    pub async fn connect_optional(
        stream: UnixStream,
        controller: Option<PanelHandle>,
        gate: PreparationGate,
    ) -> zbus::Result<Self> {
        Self::connect_optional_with_uid(
            stream,
            controller,
            gate,
            rustix::process::geteuid().as_raw(),
        )
        .await
    }
    async fn connect_optional_with_uid(
        stream: UnixStream,
        controller: Option<PanelHandle>,
        gate: PreparationGate,
        uid: u32,
    ) -> zbus::Result<Self> {
        let stop = CancellationToken::new();
        let (updates, _) = watch::channel(DesktopState::default());
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                record: None,
                next: 0,
                worker: None,
            }),
            controller,
            gate,
            updates,
            stop: stop.clone(),
            uid,
        });
        let connection = crate::service::bus::connection_builder(stream)
            .serve_at(PATH, Endpoint(shared.clone()))?
            .build()
            .await?;
        let setup = async {
            let broker = fdo::DBusProxy::new(&connection).await?;
            let changes = broker
                .receive_name_owner_changed_with_args(&[(2, "")])
                .await?;
            connection
                .request_name_with_flags(SERVICE, fdo::RequestNameFlags::DoNotQueue.into())
                .await?;
            Ok::<_, zbus::Error>(changes)
        }
        .await;
        let mut changes = match setup {
            Ok(changes) => changes,
            Err(error) => {
                let _ = connection.close().await;
                return Err(error);
            }
        };
        let handle = DesktopHandle {
            shared: shared.clone(),
        };
        let task_stop = stop.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _=task_stop.cancelled()=>break,
                    change=changes.next()=>{
                        let Some(change)=change else {break};let Ok(args)=change.args() else {break};
                        let mut inner=shared.inner.lock().expect("desktop ownership");
                        if inner.record.as_ref().is_some_and(|record|record.owner==args.name().as_str()) {inner.record=None;shared.updates.send_modify(|state|state.staging=false);}
                    }
                }
            }
            task_stop.cancel();
            shared.gate.close();
            shared.updates.send_modify(|state| state.closed = true);
            let worker = {
                let mut inner = shared.inner.lock().expect("desktop ownership");
                inner.record = None;
                inner.worker.take()
            };
            if let Some(worker) = worker {
                let _ = worker.await;
            }
            drop(changes);
            connection.close().await
        });
        Ok(Self {
            stop,
            task: Some(task),
            handle,
        })
    }
    pub fn handle(&self) -> DesktopHandle {
        self.handle.clone()
    }
    pub async fn close(mut self) -> zbus::Result<()> {
        self.stop.cancel();
        self.task
            .take()
            .expect("desktop task")
            .await
            .map_err(|_| zbus::Error::Failure("Desktop task failed".into()))?
    }
}
impl Drop for DesktopServer {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopError {
    Unavailable,
    GuiChanged,
    Transport,
    Protocol,
    InvalidInput,
    Busy,
    Denied,
    Conflict,
    Closed,
    Recovery,
    Failed,
}
pub struct DesktopClient {
    connection: Option<Connection>,
    owner: String,
    source: Option<OpenedSelection>,
    token: Option<u64>,
    next: usize,
    spent: bool,
    #[cfg(test)]
    hook: Option<super::ReplyHook>,
}
impl std::fmt::Debug for DesktopClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopClient")
            .field("pending", &self.source.is_some())
            .finish_non_exhaustive()
    }
}
impl DesktopClient {
    pub async fn connect(stream: UnixStream) -> Result<Self, DesktopError> {
        let connection = crate::service::bus::connection_builder(stream)
            .build()
            .await
            .map_err(|_| DesktopError::Transport)?;
        let setup = async {
            let broker = fdo::DBusProxy::new(&connection)
                .await
                .map_err(|_| DesktopError::Transport)?;
            let owner = broker
                .get_name_owner(SERVICE.try_into().map_err(|_| DesktopError::Protocol)?)
                .await
                .map_err(|_| DesktopError::Unavailable)?;
            if broker
                .get_connection_unix_user(owner.clone().into())
                .await
                .map_err(|_| DesktopError::Transport)?
                != rustix::process::geteuid().as_raw()
            {
                return Err(DesktopError::Denied);
            }
            Ok(owner.to_string())
        }
        .await;
        match setup {
            Ok(owner) => Ok(Self {
                connection: Some(connection),
                owner,
                source: None,
                token: None,
                next: 0,
                spent: false,
                #[cfg(test)]
                hook: None,
            }),
            Err(error) => {
                let _ = connection.close().await;
                Err(error)
            }
        }
    }
    async fn wire<B: serde::Serialize + zbus::zvariant::Type>(
        &mut self,
        method: &str,
        body: &B,
    ) -> Result<zbus::Message, DesktopError> {
        #[cfg(test)]
        self.barrier(method, false).await;
        let connection = self.connection.as_ref().ok_or(DesktopError::Closed)?;
        let broker = fdo::DBusProxy::new(connection)
            .await
            .map_err(|_| DesktopError::Transport)?;
        if broker
            .get_name_owner(SERVICE.try_into().map_err(|_| DesktopError::Protocol)?)
            .await
            .map_err(|_| DesktopError::GuiChanged)?
            .as_str()
            != self.owner
        {
            return Err(DesktopError::GuiChanged);
        }
        let result = match connection
            .call_method(
                Some(self.owner.as_str()),
                PATH,
                Some(INTERFACE),
                method,
                body,
            )
            .await
        {
            Ok(reply)
                if reply
                    .header()
                    .sender()
                    .is_some_and(|sender| sender.as_str() == self.owner) =>
            {
                Ok(reply)
            }
            Ok(_) => Err(DesktopError::Protocol),
            Err(zbus::Error::MethodError(name, description, reply)) => {
                if !reply
                    .header()
                    .sender()
                    .is_some_and(|sender| sender.as_str() == self.owner)
                {
                    return Err(DesktopError::GuiChanged);
                }
                Err(match (name.as_str(), description.as_deref()) {
                    ("org.freedesktop.DBus.Error.AccessDenied", _) => DesktopError::Denied,
                    (_, Some("Busy")) => DesktopError::Busy,
                    (_, Some("Unavailable")) => DesktopError::Unavailable,
                    (_, Some("Conflict" | "StaleBatch")) => DesktopError::Conflict,
                    (_, Some("InvalidInput")) | ("org.freedesktop.DBus.Error.InvalidArgs", _) => {
                        DesktopError::InvalidInput
                    }
                    (_, Some("Closed")) => DesktopError::Closed,
                    _ => DesktopError::Protocol,
                })
            }
            Err(_) => Err(DesktopError::Transport),
        };
        #[cfg(test)]
        self.barrier(method, true).await;
        result
    }
    #[cfg(test)]
    async fn barrier(&mut self, method: &str, after: bool) {
        if self
            .hook
            .as_ref()
            .is_some_and(|hook| hook.method == method && hook.after == after)
        {
            let hook = self.hook.take().expect("reply hook");
            hook.reached.notify_one();
            std::future::pending::<()>().await;
        }
    }
    pub async fn show_error(&mut self, code: ErrorCode) -> Result<(), DesktopError> {
        self.wire("ShowError", &code.wire())
            .await?
            .body()
            .deserialize::<()>()
            .map_err(|_| DesktopError::Protocol)
    }
    pub async fn show(&mut self) -> Result<(), DesktopError> {
        self.wire("Show", &())
            .await?
            .body()
            .deserialize()
            .map_err(|_| DesktopError::Protocol)
    }
    pub async fn import(&mut self, source: OpenedSelection) -> Result<(), DesktopError> {
        if self.source.is_some() || self.spent || !valid_selection(&source) {
            return Err(DesktopError::InvalidInput);
        }
        self.source = Some(source);
        self.recover().await
    }
    /// Retains the same source/connection/token/batch before every await.
    pub async fn recover(&mut self) -> Result<(), DesktopError> {
        if self.source.is_none() {
            return Err(DesktopError::InvalidInput);
        }
        if self.token.is_none() {
            let token: u64 = self
                .wire("Begin", &())
                .await?
                .body()
                .deserialize()
                .map_err(|_| DesktopError::Protocol)?;
            if token == 0 {
                return Err(DesktopError::Protocol);
            }
            self.token = Some(token);
        }
        let token = self.token.ok_or(DesktopError::Protocol)?;
        while self.next * 16
            < self
                .source
                .as_ref()
                .ok_or(DesktopError::Protocol)?
                .files()
                .len()
        {
            let files =
                &self.source.as_ref().ok_or(DesktopError::Protocol)?.files()[self.next * 16..];
            let files = &files[..files.len().min(16)];
            let handles = files
                .iter()
                .map(OpenedFile::clone_handle)
                .collect::<std::io::Result<Vec<_>>>()
                .map_err(|_| DesktopError::InvalidInput)?;
            let entries = handles
                .iter()
                .zip(files)
                .map(|(file, source)| (Fd::from(file.as_fd()), source.relative_name().to_owned()))
                .collect::<Vec<_>>();
            let next: u32 = self
                .wire("AppendFiles", &(token, self.next as u32, entries))
                .await?
                .body()
                .deserialize()
                .map_err(|_| DesktopError::Protocol)?;
            if next as usize != self.next + 1 {
                return Err(DesktopError::Protocol);
            }
            self.next += 1;
        }
        let status: String = self
            .wire("Commit", &token)
            .await?
            .body()
            .deserialize()
            .map_err(|_| DesktopError::Protocol)?;
        let result = match status.as_str() {
            "ready" => Ok(()),
            "recovery" => Err(DesktopError::Recovery),
            "failed" | "aborted" => Err(DesktopError::Failed),
            _ => return Err(DesktopError::Protocol),
        };
        self.source = None;
        self.spent = true;
        result
    }
    pub async fn close(mut self) -> Result<(), DesktopError> {
        self.source = None;
        self.connection
            .take()
            .ok_or(DesktopError::Closed)?
            .close()
            .await
            .map_err(|_| DesktopError::Transport)
    }
}

#[cfg(test)]
mod tests;
