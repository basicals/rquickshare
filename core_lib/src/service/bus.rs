//! Serial same-UID selection/control IPC. Only owned descriptors enter the core.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::os::fd::{AsFd, AsRawFd};

use futures::StreamExt;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use zbus::fdo::{ConnectionCredentials, RequestNameFlags, RequestNameReply};
use zbus::message::Type as MessageType;
use zbus::zvariant::{Fd, Type};
use zbus::{Connection, Message, MessageStream};

use super::runtime::{NativeRuntime, RuntimeError};
use super::uploads::{UploadState, BATCH_FILES};
use super::{ClientId, Coordinator, Error, SelectionId, TransferId, UploadId};
use crate::file_source::OpenedFile;

mod batch;
pub mod files;
mod state;
pub mod text;
pub(super) mod transport;

pub const SERVICE: &str = "io.github.morozhenko_apps.RQuickShare";
pub const PATH: &str = "/io/github/morozhenko_apps/RQuickShare";
pub const OPERATIONS_INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Operations1";
pub const NOTIFICATIONS_INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Notifications1";
pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Manager2";
pub const INTROSPECTION: &str =
    include_str!("../../dbus/io.github.morozhenko_apps.RQuickShare.Manager2.xml");
const INBOX: usize = 8;

/// Shared local-bus framing for native clients/adapters. Bounds frames and received
/// descriptors before decoding, sets CLOEXEC atomically and queues eight messages.
/// Callers still authenticate actual bus senders and own connection shutdown.
pub fn connection_builder(stream: UnixStream) -> zbus::connection::Builder<'static> {
    zbus::connection::Builder::socket(transport::BoundedSocket(stream)).max_queued(INBOX)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusError {
    Transport,
    Runtime(RuntimeError),
    Coordinator(Error),
}
impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Native bus: {self:?}")
    }
}
impl std::error::Error for BusError {}

type NameApply = std::pin::Pin<
    Box<
        dyn std::future::Future<
                Output = (
                    tokio::sync::oneshot::Sender<super::settings_service::ApplyReceipt>,
                    super::settings_service::ApplyReceipt,
                ),
            > + Send
            + Sync,
    >,
>;

/// One pump and one serial state owner, plus at most one joined filesystem worker.
/// Call `run` or `close` to join cleanup. Drop only requests cancellation.
pub struct SelectionBus {
    connection: Option<Connection>,
    inbox: mpsc::Receiver<Message>,
    pump: JoinHandle<()>,
    failed: CancellationToken,
    closed: CancellationToken,
    clients: BTreeMap<String, ClientId>,
    state: state::StateCache,
    journal: Option<super::journal::Owner>,
    journal_captures: BTreeMap<ClientId, super::journal::View>,
    text: text::TextCache,
    file_job: Option<JoinHandle<Result<(u8, File), Error>>>,
    file_request: Option<files::Request>,
    file_resolver: std::sync::Arc<files::Resolver>,
    notified: BTreeMap<ClientId, u64>,
    visibility_recovered: bool,
    uid: u32,
    wakeup: Option<super::wakeup::Wakeup>,
    settings: Option<super::settings_service::SettingsOwner>,
    name_apply: Option<NameApply>,
    configuration_only: bool,
    pending_restart: Option<Message>,
    restart_requested: bool,
}

impl SelectionBus {
    /// The caller connects a local user-bus socket. This never discovers or mutates
    /// a system/session address implicitly, which also permits isolated bus fixtures.
    pub async fn connect(stream: UnixStream) -> Result<Self, BusError> {
        Self::connect_with_uid(stream, rustix::process::geteuid().as_raw()).await
    }

    async fn connect_with_uid(stream: UnixStream, uid: u32) -> Result<Self, BusError> {
        Self::connect_inner(stream, uid, CancellationToken::new())
            .await?
            .ok_or(BusError::Transport)
    }

    pub async fn connect_stoppable(
        stream: UnixStream,
        stop: CancellationToken,
    ) -> Result<Option<Self>, BusError> {
        Self::connect_inner(stream, rustix::process::geteuid().as_raw(), stop).await
    }

    async fn connect_inner(
        stream: UnixStream,
        uid: u32,
        stop: CancellationToken,
    ) -> Result<Option<Self>, BusError> {
        let connection = tokio::select! {
            biased;
            _ = stop.cancelled() => return Ok(None),
            result = zbus::connection::Builder::socket(transport::BoundedSocket(stream)).max_queued(INBOX).build() => result.map_err(|_| BusError::Transport)?,
        };
        let mut messages = MessageStream::from(&connection);
        let (sender, inbox) = mpsc::channel(INBOX);
        let failed = CancellationToken::new();
        let closed = CancellationToken::new();
        let fail = failed.clone();
        let pump_stop = closed.clone();
        let pump = tokio::spawn(async move {
            loop {
                let item = tokio::select! { _ = pump_stop.cancelled() => break, item = messages.next() => item };
                match item {
                    Some(Ok(message)) => {
                        if matches!(
                            message.message_type(),
                            MessageType::MethodCall | MessageType::Signal
                        ) && sender.try_send(message).is_err()
                        {
                            fail.cancel();
                            break;
                        }
                    }
                    _ => {
                        fail.cancel();
                        break;
                    }
                }
            }
        });
        let mut bus = Self {
            connection: Some(connection),
            inbox,
            pump,
            failed,
            closed,
            clients: BTreeMap::new(),
            state: state::StateCache::default(),
            journal: None,
            journal_captures: BTreeMap::new(),
            text: text::TextCache::default(),
            file_job: None,
            file_request: None,
            file_resolver: std::sync::Arc::new(|files, index| {
                files.open(index).map_err(|_| Error::InvalidState)
            }),
            notified: BTreeMap::new(),
            visibility_recovered: false,
            uid,
            settings: None,
            name_apply: None,
            configuration_only: false,
            pending_restart: None,
            restart_requested: false,
            wakeup: None,
        };
        let initialization = async {
            let rule = zbus::MatchRule::builder()
                .msg_type(MessageType::Signal)
                .sender("org.freedesktop.DBus")?
                .path("/org/freedesktop/DBus")?
                .interface("org.freedesktop.DBus")?
                .member("NameOwnerChanged")?
                .arg(0, SERVICE)?
                .arg(2, "")?
                .build();
            bus.broker_call("AddMatch", &rule.to_string()).await?;
            let connection = bus.connection.as_ref().ok_or(zbus::Error::NameTaken)?;
            let reply = connection
                .request_name_with_flags(SERVICE, RequestNameFlags::DoNotQueue.into())
                .await?;
            if !matches!(
                reply,
                RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner
            ) {
                return Err(zbus::Error::NameTaken);
            }
            let owner = bus.broker_call("GetNameOwner", &SERVICE).await?;
            let owner: String = owner.body().deserialize()?;
            if connection
                .unique_name()
                .is_none_or(|name| name.as_str() != owner)
            {
                return Err(zbus::Error::NameTaken);
            }
            Ok::<_, zbus::Error>(())
        };
        let initialized = tokio::select! {
            biased;
            _ = stop.cancelled() => None,
            result = initialization => Some(result),
        };
        let Some(initialized) = initialized else {
            bus.close_connection().await;
            return Ok(None);
        };
        if initialized.is_err() || bus.failed.is_cancelled() {
            bus.close_connection().await;
            return Err(BusError::Transport);
        }
        Ok(Some(bus))
    }

    pub(super) fn set_wakeup(&mut self, wakeup: super::wakeup::Wakeup) {
        self.wakeup = Some(wakeup);
    }

    pub(super) fn set_journal(&mut self, directory: std::path::PathBuf) {
        self.journal = Some(super::journal::Owner::spawn(directory));
    }
    pub(super) fn set_visibility_recovered(&mut self, value: bool) {
        self.visibility_recovered = value;
    }
    pub fn set_settings(
        &mut self,
        store: super::native_settings::Store,
        initial: Result<super::native_settings::Snapshot, super::native_settings::StoreError>,
        epoch: u128,
        running: bool,
        port: i32,
    ) -> Result<(), BusError> {
        if self.settings.is_some() {
            return Err(BusError::Coordinator(Error::InvalidState));
        }
        let connection = self.connection.as_ref().ok_or(BusError::Transport)?.clone();
        self.settings = Some(super::settings_service::SettingsOwner::spawn(
            store, initial, epoch, running, port, connection,
        ));
        Ok(())
    }
    pub(super) fn mark_settings_unapplied(&self) {
        if let Some(settings) = &self.settings {
            settings.mark_unapplied();
        }
    }
    pub(super) fn restart_requested(&self) -> bool {
        self.restart_requested
    }
    pub(super) async fn close_unstarted(&mut self) {
        self.close_connection().await;
    }

    /// Claims the name in connect() before starting any core transports. The
    /// runtime remains borrowed/owned across cancellation and is always joined.
    pub async fn run(
        &mut self,
        runtime: &mut NativeRuntime,
        stop: CancellationToken,
    ) -> Result<(), BusError> {
        let outcome = self.run_inner(runtime, stop).await;
        let shutdown = runtime.shutdown().await.map_err(BusError::Runtime);
        if let Some(journal) = self.journal.as_mut() {
            journal.observe(runtime.selection_coordinator()).await;
        }
        let disconnected = self.disconnect_clients(runtime.selection_coordinator());
        self.close_connection().await;
        outcome.and(shutdown).and(disconnected)
    }
    pub(super) async fn run_prepared(
        &mut self,
        runtime: &mut NativeRuntime,
        stop: CancellationToken,
    ) -> Result<(), BusError> {
        let outcome = self.run_started(runtime, stop).await;
        let shutdown = runtime.shutdown().await.map_err(BusError::Runtime);
        if let Some(journal) = self.journal.as_mut() {
            journal.observe(runtime.selection_coordinator()).await;
        }
        let disconnected = self.disconnect_clients(runtime.selection_coordinator());
        self.close_connection().await;
        outcome.and(shutdown).and(disconnected)
    }

    async fn run_inner(
        &mut self,
        runtime: &mut NativeRuntime,
        stop: CancellationToken,
    ) -> Result<(), BusError> {
        if self.closed.is_cancelled() || self.failed.is_cancelled() {
            return Err(BusError::Transport);
        }
        if stop.is_cancelled() {
            return Ok(());
        }
        let failure = self.failed.clone();
        if !runtime.is_configuration_only() {
            tokio::select! {
                _ = stop.cancelled() => return Ok(()),
                _ = failure.cancelled() => return Err(BusError::Transport),
                result = runtime.start() => result.map_err(BusError::Runtime)?,
            }
        }
        self.run_started(runtime, stop).await
    }

    async fn run_started(
        &mut self,
        runtime: &mut NativeRuntime,
        stop: CancellationToken,
    ) -> Result<(), BusError> {
        let failure = self.failed.clone();
        self.configuration_only = runtime.is_configuration_only();
        let mut visibility = runtime.visibility_reader();
        if let Some(settings) = self.settings.as_ref() {
            settings.bind_visibility(visibility.clone());
            settings.bind_identity(runtime.identity_policy());
        }
        loop {
            tokio::select! {
                _ = stop.cancelled() => return Ok(()),
                _ = failure.cancelled() => return Err(BusError::Transport),
                result = runtime.step(), if !self.configuration_only => result.map_err(BusError::Runtime)?,
                changed = visibility.changed() => {
                    changed.map_err(|_| BusError::Transport)?;
                    let (_, revision, _) = visibility.observation();
                    let epoch = format!("{:032x}", runtime.selection_coordinator().epoch);
                    let connection = self.connection.as_ref().ok_or(BusError::Transport)?;
                    let event = (epoch, revision);
                    tokio::select! { biased;
                        _ = stop.cancelled() => return Ok(()),
                        _ = failure.cancelled() => return Err(BusError::Transport),
                        result = connection.emit_signal(None::<&str>, PATH, super::visibility_service::INTERFACE,
                            "Changed", &event) => result.map_err(|_| BusError::Transport)?,
                    }
                }
                _ = async {
                    match &mut self.journal {
                        Some(journal) => journal.changed().await,
                        None => std::future::pending().await,
                    }
                } => {
                    let connection = self.connection.as_ref().ok_or(BusError::Transport)?;
                    connection.emit_signal(None::<&str>, PATH, super::journal::INTERFACE, "Changed",
                        &(format!("{:032x}", runtime.selection_coordinator().epoch), 1u64)).await.map_err(|_|BusError::Transport)?;
                }
                request = async {
                    match &mut self.settings {
                        Some(settings) => settings.apply.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    let request = request.ok_or(BusError::Transport)?;
                    if request.renew_visibility {
                        let applied = runtime.apply_visibility(request.values.visibility);
                        let _ = request.reply.send(super::settings_service::ApplyReceipt {
                            policy_applied: applied, name_applied: true,
                        });
                        if applied { self.visibility_recovered = false; }
                        continue;
                    }
                    let applied = runtime.apply_live_settings(&request.values, request.root, request.apply_visibility);
                    if applied { self.visibility_recovered = false; }
                    if applied {
                        if let Some(identity) = runtime.identity_policy() {
                            let revision = identity.snapshot().revision;
                            let lifetime = stop.clone();
                            self.name_apply = Some(Box::pin(async move {
                                let applied = identity.wait_applied(revision, &lifetime).await.is_ok();
                                (request.reply, super::settings_service::ApplyReceipt { policy_applied: true, name_applied: applied })
                            }));
                        } else {
                            let _ = request.reply.send(super::settings_service::ApplyReceipt { policy_applied: true, name_applied: request.values.device_name.is_none() });
                        }
                    } else { let _ = request.reply.send(super::settings_service::ApplyReceipt::default()); }
                }
                result = async {
                    match self.name_apply.as_mut() {
                        Some(work) => work.await,
                        None => std::future::pending().await,
                    }
                } => {
                    self.name_apply = None;
                    let (reply, applied) = result;
                    let _ = reply.send(applied);
                }
                result = files::next(&mut self.file_job) => {
                    tokio::select! {
                        _ = stop.cancelled() => return Ok(()),
                        _ = failure.cancelled() => return Err(BusError::Transport),
                        result = self.complete_file(runtime.selection_coordinator(), result) => result?,
                    }
                }
                message = self.inbox.recv() => {
                    let message = message.ok_or(BusError::Transport)?;
                    let (coordinator, endpoints) = runtime.bus_state_parts();
                    // Keep broker waits/replies cancellable without dropping the runtime owner.
                    tokio::select! {
                        _ = stop.cancelled() => return Ok(()),
                        _ = failure.cancelled() => return Err(BusError::Transport),
                        result = self.dispatch(coordinator, endpoints, message) => result?,
                    }
                }
            }
            if let Some(journal) = self.journal.as_mut() {
                journal.observe(runtime.selection_coordinator()).await;
            }
            if let Some(message) = self.pending_restart.take() {
                let header = message.header();
                let ready = self.settings.as_ref().is_some_and(|settings| {
                    let view = settings.view();
                    settings.idle() && view.configured && matches!(view.error, 0 | 4)
                });
                if ready && self.file_job.is_none() && runtime.restart_if_idle() {
                    self.restart_requested = true;
                    tokio::select! { biased;
                        _ = stop.cancelled() => return Ok(()),
                        _ = failure.cancelled() => return Err(BusError::Transport),
                        result = self.state_reply(&header, &()) => result?,
                    }
                    return Ok(());
                }
                tokio::select! { biased;
                    _ = stop.cancelled() => return Ok(()),
                    _ = failure.cancelled() => return Err(BusError::Transport),
                    result = self.domain_error(&header, Error::Busy) => result?,
                }
            }
            tokio::select! {
                _ = stop.cancelled() => return Ok(()),
                _ = failure.cancelled() => return Err(BusError::Transport),
                result = self.publish_changes(runtime.selection_coordinator()) => result?,
            }
        }
    }

    fn disconnect_clients(&mut self, coordinator: &mut Coordinator) -> Result<(), BusError> {
        let mut result = Ok(());
        self.state.clear();
        self.journal_captures.clear();
        self.text = text::TextCache::default();
        self.notified.clear();
        for (_, client) in std::mem::take(&mut self.clients) {
            if let Err(error) = coordinator.disconnect(client) {
                result = Err(BusError::Coordinator(error));
            }
        }
        result
    }

    pub async fn close(&mut self, runtime: &mut NativeRuntime) -> Result<(), BusError> {
        let shutdown = runtime.shutdown().await.map_err(BusError::Runtime);
        if let Some(journal) = self.journal.as_mut() {
            journal.observe(runtime.selection_coordinator()).await;
        }
        let result = self.disconnect_clients(runtime.selection_coordinator());
        self.close_connection().await;
        shutdown.and(result)
    }

    async fn close_connection(&mut self) {
        self.name_apply.take();
        if self.closed.is_cancelled() {
            return;
        }
        self.closed.cancel();
        if let Some(job) = self.file_job.take() {
            let _ = job.await;
        }
        self.file_request.take();
        if let Some(journal) = self.journal.take() {
            journal.close().await;
        }
        if let Some(settings) = self.settings.take() {
            settings.close().await;
        }
        if let Some(wakeup) = self.wakeup.take() {
            wakeup.close().await;
        }
        let _ = (&mut self.pump).await;
        self.inbox.close();
        while self.inbox.try_recv().is_ok() {}
        if let Some(connection) = self.connection.take() {
            let _ = connection.close().await;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_file_resolver(&mut self, resolver: std::sync::Arc<files::Resolver>) {
        self.file_resolver = resolver;
    }

    async fn complete_file(
        &mut self,
        coordinator: &Coordinator,
        result: Result<(u8, File), Error>,
    ) -> Result<(), BusError> {
        self.file_job.take();
        let request = self.file_request.take().ok_or(BusError::Transport)?;
        let header = request.message.header();
        // History/client lifetime may have changed while the filesystem was busy.
        let authority = if request.whole {
            files::result_resource(
                coordinator,
                request.client,
                request.target,
                request.revision,
            )
        } else {
            files::resource(
                coordinator,
                request.client,
                request.target,
                request.revision,
                request.index,
            )
        };
        let result = authority.and(result);
        match result {
            Ok((kind, file)) => {
                let connection = self.connection.as_ref().ok_or(BusError::Transport)?;
                let fd = Fd::from(file.as_fd());
                let reply = if request.whole {
                    connection.reply(&header, &(kind, fd)).await
                } else {
                    connection.reply(&header, &fd).await
                };
                reply.map_err(|_| BusError::Transport)
            }
            Err(error) => self.domain_error(&header, error).await,
        }
    }

    async fn authenticated_client(
        &mut self,
        coordinator: &mut Coordinator,
        sender: &str,
    ) -> Result<ClientId, AuthenticationError> {
        if !sender.starts_with(':') {
            return Err(AuthenticationError::Denied);
        }
        let _ = zbus::names::BusName::try_from(sender).map_err(|_| AuthenticationError::Denied)?;
        if let Some(client) = self.clients.get(sender).copied() {
            let reply = self
                .broker_call("NameHasOwner", &sender)
                .await
                .map_err(|_| AuthenticationError::Denied)?;
            if reply
                .body()
                .deserialize::<bool>()
                .map_err(|_| AuthenticationError::Denied)?
            {
                return Ok(client);
            }
            self.forget_client(coordinator, sender)
                .await
                .map_err(|_| AuthenticationError::Denied)?;
            return Err(AuthenticationError::Denied);
        }
        let uid = {
            let reply = self
                .broker_call("GetConnectionCredentials", &sender)
                .await
                .map_err(|_| AuthenticationError::Denied)?;
            let credentials: ConnectionCredentials = reply
                .body()
                .deserialize()
                .map_err(|_| AuthenticationError::Denied)?;
            credentials.unix_user_id()
        };
        if uid != Some(self.uid) {
            return Err(AuthenticationError::Denied);
        }
        let rule = owner_rule(sender).map_err(|_| AuthenticationError::Denied)?;
        let client = if self.configuration_only {
            coordinator.connect_observer()
        } else {
            coordinator.connect()
        }
        .map_err(AuthenticationError::Coordinator)?;
        // Track ownership before any cancellable broker wait. Host cleanup can
        // now release this client even if AddMatch or NameHasOwner is abandoned.
        self.clients.insert(sender.to_owned(), client);
        // Exact subscriptions keep unrelated application traffic out of the inbox.
        let watched = self
            .broker_call("AddMatch", &rule.to_string())
            .await
            .is_ok();
        let live = if watched {
            match self.broker_call("NameHasOwner", &sender).await {
                Ok(reply) => reply.body().deserialize::<bool>().unwrap_or(false),
                Err(_) => false,
            }
        } else {
            false
        };
        if !live {
            self.forget_client(coordinator, sender)
                .await
                .map_err(|_| AuthenticationError::Denied)?;
            return Err(AuthenticationError::Denied);
        }
        Ok(client)
    }

    async fn forget_client(
        &mut self,
        coordinator: &mut Coordinator,
        sender: &str,
    ) -> Result<(), BusError> {
        if let Some(client) = self.clients.remove(sender) {
            self.state.disconnect(client);
            self.journal_captures.remove(&client);
            self.text.disconnect(client);
            self.notified.remove(&client);
            coordinator
                .disconnect(client)
                .map_err(BusError::Coordinator)?;
            let rule = owner_rule(sender).map_err(|_| BusError::Transport)?;
            self.broker_call("RemoveMatch", &rule.to_string())
                .await
                .map_err(|_| BusError::Transport)?;
        }
        Ok(())
    }

    async fn broker_call<B: serde::Serialize + Type>(
        &self,
        method: &str,
        body: &B,
    ) -> zbus::Result<Message> {
        let connection = self
            .connection
            .as_ref()
            .ok_or_else(|| zbus::Error::Failure("Native bus closed".into()))?;
        let reply = connection
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                method,
                body,
            )
            .await?;
        // A matching reply serial alone is not broker authority.
        if reply
            .header()
            .sender()
            .is_none_or(|sender| sender.as_str() != "org.freedesktop.DBus")
        {
            return Err(zbus::Error::Failure("Untrusted broker reply".into()));
        }
        Ok(reply)
    }

    async fn dispatch<'a>(
        &mut self,
        coordinator: &mut Coordinator,
        endpoints: impl ExactSizeIterator<Item = (&'a str, &'a str)>,
        message: Message,
    ) -> Result<(), BusError> {
        let connection = self.connection.as_ref().ok_or(BusError::Transport)?.clone();
        let header = message.header();
        if message.message_type() == MessageType::Signal {
            if header
                .sender()
                .is_some_and(|sender| sender.as_str() == "org.freedesktop.DBus")
                && header
                    .path()
                    .is_some_and(|path| path.as_str() == "/org/freedesktop/DBus")
                && header
                    .interface()
                    .is_some_and(|interface| interface.as_str() == "org.freedesktop.DBus")
            {
                match header.member().map(|member| member.as_str()) {
                    Some("NameOwnerChanged") => {
                        let (name, _, new): (String, String, String) = message
                            .body()
                            .deserialize()
                            .map_err(|_| BusError::Transport)?;
                        if new.is_empty() {
                            if name == SERVICE {
                                return Err(BusError::Transport);
                            }
                            self.forget_client(coordinator, &name).await?;
                        }
                    }
                    Some("NameLost") => {
                        let name: String = message
                            .body()
                            .deserialize()
                            .map_err(|_| BusError::Transport)?;
                        if name == SERVICE {
                            return Err(BusError::Transport);
                        }
                    }
                    _ => (),
                }
            }
            return Ok(());
        }
        if header.unix_fds().unwrap_or(0) as usize != message.data().fds().len() {
            return self.domain_error(&header, Error::InvalidInput).await;
        }
        let Some(sender) = header.sender() else {
            return Err(BusError::Transport);
        };
        let client = match self
            .authenticated_client(coordinator, sender.as_str())
            .await
        {
            Ok(client) => client,
            Err(AuthenticationError::Denied) => {
                return self
                    .error(&header, "org.freedesktop.DBus.Error.AccessDenied")
                    .await
            }
            Err(AuthenticationError::Coordinator(error)) => {
                return self.domain_error(&header, error).await
            }
        };
        if header.path().is_none_or(|path| path.as_str() != PATH) {
            return self
                .error(&header, "org.freedesktop.DBus.Error.UnknownObject")
                .await;
        }
        let interface = header.interface().map(|interface| interface.as_str());
        let member = header.member().map(|member| member.as_str());
        if interface == Some(super::journal::INTERFACE) {
            if !message.data().fds().is_empty() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            match member {
                Some("Read" | "Retry") => {
                    if message.body().deserialize::<()>().is_err() {
                        return self.domain_error(&header, Error::InvalidInput).await;
                    }
                    let view =
                        match &self.journal {
                            Some(owner) => owner
                                .read(member == Some("Retry"))
                                .await
                                .unwrap_or_else(|_| super::journal::View {
                                    error: 1,
                                    ..Default::default()
                                }),
                            None => super::journal::View {
                                error: 3,
                                ..Default::default()
                            },
                        };
                    let response = (
                        format!("{:032x}", coordinator.epoch),
                        view.token.clone(),
                        view.error,
                        view.records.len() as u32,
                    );
                    self.journal_captures.insert(client, view);
                    return self.state_reply(&header, &response).await;
                }
                Some("GetPage") => {
                    let Ok((token, offset)) = message.body().deserialize::<(String, u32)>() else {
                        return self.domain_error(&header, Error::InvalidInput).await;
                    };
                    let Some(view) = self
                        .journal_captures
                        .get(&client)
                        .filter(|view| view.token == token)
                    else {
                        return self.domain_error(&header, Error::UnknownSnapshot).await;
                    };
                    let offset = offset as usize;
                    if offset > view.records.len() {
                        return self.domain_error(&header, Error::InvalidInput).await;
                    }
                    let rows = &view.records[offset..(offset + 16).min(view.records.len())];
                    return self
                        .state_reply(&header, &(view.records.len() as u32, rows))
                        .await;
                }
                Some("Clear") => {
                    let Ok(token) = message.body().deserialize::<String>() else {
                        return self.domain_error(&header, Error::InvalidInput).await;
                    };
                    if token.len() != 64 {
                        return self.domain_error(&header, Error::InvalidInput).await;
                    }
                    let result = match &self.journal {
                        Some(owner) => owner.clear(token).await,
                        None => Err(super::native_settings::StoreError::Read),
                    };
                    let code = match result {
                        Ok(_) => 0u8,
                        Err(super::native_settings::StoreError::Conflict) => 2,
                        Err(_) => 1,
                    };
                    return self.state_reply(&header, &code).await;
                }
                _ => {
                    return self
                        .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                        .await
                }
            }
        }
        if interface == Some(super::visibility_service::INTERFACE) {
            use super::visibility_service::{VisibilityIntent, VisibilityView};
            if !message.data().fds().is_empty() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            match member {
                Some("Read") => {
                    if message.body().deserialize::<()>().is_err() {
                        return self.domain_error(&header, Error::InvalidInput).await;
                    }
                    let view = self
                        .settings
                        .as_ref()
                        .map(|owner| owner.visibility_view())
                        .unwrap_or_else(|| {
                            VisibilityView::project(&format!("{:032x}", coordinator.epoch), None)
                        });
                    return self.state_reply(&header, &(view,)).await;
                }
                Some("Set") => {
                    let result = message
                        .body()
                        .deserialize::<VisibilityIntent>()
                        .map_err(|_| Error::InvalidInput)
                        .and_then(|intent| {
                            if !intent.valid(coordinator.epoch) {
                                return Err(Error::InvalidInput);
                            }
                            let owner = self.settings.as_ref().ok_or(Error::NotRunning)?;
                            if !owner.visibility_view().running {
                                return Err(Error::NotRunning);
                            }
                            owner.submit(super::settings_service::Command::Visibility(
                                intent,
                                message.clone(),
                            ))
                        });
                    return match result {
                        Ok(()) => Ok(()),
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                _ => {
                    return self
                        .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                        .await
                }
            }
        }
        if interface == Some(super::settings_service::INTERFACE) {
            use super::settings_service::{Command, SaveIntent, SettingsView};
            let invalid = || Error::InvalidInput;
            match member {
                Some("Restart") => {
                    let epoch = message
                        .body()
                        .deserialize::<String>()
                        .map_err(|_| invalid());
                    if !message.data().fds().is_empty()
                        || epoch.as_deref() != Ok(format!("{:032x}", coordinator.epoch).as_str())
                    {
                        return self.domain_error(&header, invalid()).await;
                    }
                    if self.settings.is_none() {
                        return self.domain_error(&header, Error::NotRunning).await;
                    }
                    if self.pending_restart.is_some() {
                        return self.domain_error(&header, Error::Busy).await;
                    }
                    self.pending_restart = Some(message.clone());
                    return Ok(());
                }
                Some("Read") => {
                    if !message.data().fds().is_empty()
                        || message.body().deserialize::<()>().is_err()
                    {
                        return self.domain_error(&header, invalid()).await;
                    }
                    let view = self
                        .settings
                        .as_ref()
                        .map(|settings| settings.view())
                        .unwrap_or_else(|| SettingsView::unavailable(coordinator.epoch));
                    return self.state_reply(&header, &(view,)).await;
                }
                Some("Reload") => {
                    if !message.data().fds().is_empty()
                        || message.body().deserialize::<()>().is_err()
                    {
                        return self.domain_error(&header, invalid()).await;
                    }
                    let result = self
                        .settings
                        .as_ref()
                        .ok_or(Error::NotRunning)
                        .and_then(|settings| settings.submit(Command::Reload(message.clone())));
                    return match result {
                        Ok(()) => Ok(()),
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                Some("Save") => {
                    let body = message.body();
                    let args = body
                        .deserialize::<(SaveIntent, Vec<Fd<'_>>)>()
                        .map_err(|_| invalid());
                    let edit = args.and_then(|(intent, files)| {
                        let expected = usize::from(intent.directory == 2);
                        if !intent.valid(coordinator.epoch)
                            || files.len() != expected
                            || message.data().fds().len() != expected
                        {
                            return Err(invalid());
                        }
                        let file = files
                            .first()
                            .map(|file| file.as_fd().try_clone_to_owned().map(File::from))
                            .transpose()
                            .map_err(|_| invalid())?;
                        intent.edit(file).map_err(|_| invalid())
                    });
                    let result = edit.and_then(|edit| {
                        self.settings
                            .as_ref()
                            .ok_or(Error::NotRunning)?
                            .submit(Command::Save(edit, message.clone()))
                    });
                    return match result {
                        Ok(()) => Ok(()),
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                _ => {
                    return self
                        .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                        .await
                }
            }
        }
        if interface == Some("org.freedesktop.DBus.Introspectable") && member == Some("Introspect")
        {
            if message.body().deserialize::<()>().is_err() || !message.data().fds().is_empty() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            return connection
                .reply(&header, &INTROSPECTION)
                .await
                .map_err(|_| BusError::Transport);
        }
        if interface == Some(files::INTERFACE) {
            if !message.data().fds().is_empty() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            match member {
                Some("GetCompletedPage") => {
                    let result = message
                        .body()
                        .deserialize::<(&str, &str, u32)>()
                        .map_err(|_| Error::InvalidInput)
                        .and_then(|(id, transfer, offset)| {
                            token_parts(id, coordinator.epoch)?;
                            token_parts(transfer, coordinator.epoch)?;
                            self.state.completed_page(client, id, transfer, offset)
                        });
                    return match result {
                        Ok(page) => self.state_reply(&header, &page).await,
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                Some("GetMetadataPage") => {
                    let result = message
                        .body()
                        .deserialize::<(&str, u32)>()
                        .map_err(|_| Error::InvalidInput)
                        .and_then(|(id, offset)| {
                            token_parts(id, coordinator.epoch)?;
                            self.state.received_page(client, id, offset)
                        });
                    return match result {
                        Ok(page) => self.state_reply(&header, &page).await,
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                Some("AcquireFile" | "AcquireResult") => {
                    let whole = member == Some("AcquireResult");
                    let body = message.body();
                    let arguments = if whole {
                        body.deserialize::<(&str, u64)>()
                            .map(|(id, revision)| (id, revision, 0))
                    } else {
                        body.deserialize::<(&str, u64, u32)>()
                    };
                    let result = arguments.map_err(|_| Error::InvalidInput).and_then(
                        |(id, revision, index)| {
                            let (epoch, sequence) = token_parts(id, coordinator.epoch)?;
                            let target = TransferId(epoch, sequence);
                            let resource = if whole {
                                files::result_resource(coordinator, client, target, revision)
                            } else {
                                files::resource(coordinator, client, target, revision, index)
                            };
                            resource.map(|files| (files, target, revision, index))
                        },
                    );
                    let (files, target, revision, index) = match result {
                        Ok(value) => value,
                        Err(error) => return self.domain_error(&header, error).await,
                    };
                    if self.file_job.is_some() {
                        return self.domain_error(&header, Error::Busy).await;
                    }
                    let resolve = self.file_resolver.clone();
                    self.file_request = Some(files::Request {
                        message,
                        client,
                        target,
                        revision,
                        index,
                        whole,
                    });
                    self.file_job = Some(tokio::task::spawn_blocking(move || {
                        if whole {
                            files.open_result().map_err(|_| Error::InvalidState)
                        } else {
                            resolve(files, index).map(|file| (0, file))
                        }
                    }));
                    return Ok(());
                }
                _ => {
                    return self
                        .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                        .await
                }
            }
        }
        if interface == Some(text::INTERFACE) {
            if !message.data().fds().is_empty() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            let body = message.body();
            let result = match member {
                Some("GetMetadataPage") => {
                    let args = body
                        .deserialize::<(&str, u32)>()
                        .map_err(|_| Error::InvalidInput);
                    match args.and_then(|(id, offset)| {
                        token_parts(id, coordinator.epoch)?;
                        self.state.text_page(client, id, offset)
                    }) {
                        Ok(page) => return self.state_reply(&header, &page).await,
                        Err(error) => Err(error),
                    }
                }
                Some("GetSelectionPage") => {
                    let args = body
                        .deserialize::<(&str, u32)>()
                        .map_err(|_| Error::InvalidInput);
                    match args.and_then(|(id, offset)| {
                        token_parts(id, coordinator.epoch)?;
                        self.state.source_page(client, id, offset)
                    }) {
                        Ok(page) => return self.state_reply(&header, &page).await,
                        Err(error) => Err(error),
                    }
                }
                Some("AppendText") => {
                    let args = body
                        .deserialize::<(&str, u32, &[u8])>()
                        .map_err(|_| Error::InvalidInput);
                    match args.and_then(|(id, sequence, bytes)| {
                        let id = upload_id(id, coordinator.epoch)?;
                        coordinator.append_text(client, id, sequence, bytes)
                    }) {
                        Ok(next) => return self.state_reply(&header, &next).await,
                        Err(error) => Err(error),
                    }
                }
                Some("BeginRead") => {
                    let args = body
                        .deserialize::<(&str, u64)>()
                        .map_err(|_| Error::InvalidInput);
                    match args.and_then(|(id, revision)| {
                        let (epoch, sequence) = token_parts(id, coordinator.epoch)?;
                        self.text
                            .begin(coordinator, client, TransferId(epoch, sequence), revision)
                    }) {
                        Ok(reader) => return self.state_reply(&header, &reader).await,
                        Err(error) => Err(error),
                    }
                }
                Some("GetChunk") => {
                    let args = body
                        .deserialize::<(&str, u32)>()
                        .map_err(|_| Error::InvalidInput);
                    match args.and_then(|(id, offset)| {
                        token_parts(id, coordinator.epoch)?;
                        self.text.chunk(client, id, offset)
                    }) {
                        Ok(chunk) => return self.state_reply(&header, &chunk).await,
                        Err(error) => Err(error),
                    }
                }
                Some("EndRead") => body
                    .deserialize::<&str>()
                    .map_err(|_| Error::InvalidInput)
                    .and_then(|id| {
                        token_parts(id, coordinator.epoch)?;
                        self.text.end(client, id)
                    }),
                _ => {
                    return self
                        .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                        .await
                }
            };
            return match result {
                Ok(()) => self.state_reply(&header, &()).await,
                Err(error) => self.domain_error(&header, error).await,
            };
        }
        if interface == Some(NOTIFICATIONS_INTERFACE) {
            let body = message.body();
            if !message.data().fds().is_empty() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            let result = match member {
                Some("GetNoticeCount") => {
                    let id = body.deserialize::<&str>().map_err(|_| Error::InvalidInput);
                    match id.and_then(|id| {
                        token_parts(id, coordinator.epoch)?;
                        self.state.notice_count(client, id)
                    }) {
                        Ok(count) => return self.state_reply(&header, &count).await,
                        Err(error) => Err(error),
                    }
                }
                Some("GetNoticePage") => {
                    let args = body
                        .deserialize::<(&str, u32)>()
                        .map_err(|_| Error::InvalidInput);
                    match args.and_then(|(id, offset)| {
                        token_parts(id, coordinator.epoch)?;
                        self.state.notice_page(client, id, offset)
                    }) {
                        Ok(page) => return self.state_reply(&header, &page).await,
                        Err(error) => Err(error),
                    }
                }
                Some("Dismiss") => self.dismiss_message(coordinator, client, &message),
                _ => {
                    return self
                        .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                        .await
                }
            };
            return match result {
                Ok(()) => self.state_reply(&header, &()).await,
                Err(error) => self.domain_error(&header, error).await,
            };
        }
        if interface == Some(super::batch::INTERFACE) {
            if !message.data().fds().is_empty() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            let body = message.body();
            match member {
                Some("Send" | "Queue") => {
                    let result = body
                        .deserialize::<(String, batch::Recipients, u64)>()
                        .map_err(|_| Error::InvalidInput)
                        .and_then(|(id, recipients, sequence)| {
                            let (epoch, counter) = token_parts(&id, coordinator.epoch)?;
                            let discovered: BTreeMap<_, _> = endpoints.collect();
                            let recipients = recipients
                                .0
                                .into_iter()
                                .map(|id| {
                                    let name = discovered
                                        .get(id.as_str())
                                        .copied()
                                        .unwrap_or("")
                                        .to_owned();
                                    (id, name)
                                })
                                .collect();
                            let enqueue = if member == Some("Queue") {
                                Coordinator::ordered_queue_many
                            } else {
                                Coordinator::ordered_send_many
                            };
                            enqueue(
                                coordinator,
                                client,
                                SelectionId(epoch, counter),
                                recipients,
                                sequence,
                            )
                        });
                    return match result {
                        Ok(ids) => {
                            self.state_reply(
                                &header,
                                &ids.into_iter()
                                    .map(|id| token(id.0, id.1))
                                    .collect::<Vec<_>>(),
                            )
                            .await
                        }
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                Some("GetReceipt") => {
                    let result = body
                        .deserialize::<u64>()
                        .map_err(|_| Error::InvalidInput)
                        .and_then(|sequence| coordinator.operation_receipt(client, sequence))
                        .and_then(|receipt| match receipt.result {
                            super::operations::OperationResult::Transfers(ids) => Ok((
                                receipt.kind,
                                ids.into_iter()
                                    .map(|id| token(id.0, id.1))
                                    .collect::<Vec<_>>(),
                            )),
                            _ => Err(Error::InvalidState),
                        });
                    return match result {
                        Ok(receipt) => self.state_reply(&header, &receipt).await,
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                Some("GetTransferEndpointPage") => {
                    let result = body
                        .deserialize::<(String, u32)>()
                        .map_err(|_| Error::InvalidInput)
                        .and_then(|(id, offset)| {
                            self.state.transfer_endpoint_page(client, &id, offset)
                        });
                    return match result {
                        Ok(page) => self.state_reply(&header, &page).await,
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                Some("GetRecipientPage") => {
                    let result = body
                        .deserialize::<(String, u32)>()
                        .map_err(|_| Error::InvalidInput)
                        .and_then(|(id, offset)| self.state.recipient_page(client, &id, offset));
                    return match result {
                        Ok(page) => self.state_reply(&header, &page).await,
                        Err(error) => self.domain_error(&header, error).await,
                    };
                }
                _ => {
                    return self
                        .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                        .await
                }
            }
        }
        if interface == Some(OPERATIONS_INTERFACE) {
            let Some(member) = member else {
                return self
                    .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                    .await;
            };
            if ![
                "BeginText",
                "BeginSelection",
                "CommitSelection",
                "CommitExtension",
                "AbortSelection",
                "SendSelection",
                "GetOperationState",
                "GetReceipt",
                "AcknowledgeThrough",
            ]
            .contains(&member)
            {
                return self
                    .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                    .await;
            }
            return match execute_operation(coordinator, client, member, &message) {
                Ok(OperationReply::Token(id)) => self.state_reply(&header, &id).await,
                Ok(OperationReply::State(state)) => self.state_reply(&header, &state).await,
                Ok(OperationReply::Receipt(receipt)) => self.state_reply(&header, &receipt).await,
                Ok(OperationReply::Empty) => self.state_reply(&header, &()).await,
                Err(error) => self.domain_error(&header, error).await,
            };
        }
        if interface != Some(INTERFACE) {
            return self
                .error(&header, "org.freedesktop.DBus.Error.UnknownInterface")
                .await;
        }
        let Some(member) = member else {
            return self
                .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                .await;
        };
        if member == "GetStartupWarnings" {
            if !message.data().fds().is_empty() || message.body().deserialize::<()>().is_err() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            let warnings: Vec<&str> = if self.visibility_recovered {
                vec!["visibility_recovered"]
            } else {
                vec![]
            };
            return self.state_reply(&header, &warnings).await;
        }
        if [
            "BeginSnapshot",
            "GetEndpointPage",
            "GetSelectionPage",
            "GetTransferPage",
            "GetTransferDetailsPage",
            "GetFilePage",
            "EndSnapshot",
        ]
        .contains(&member)
        {
            if !message.data().fds().is_empty() {
                return self.domain_error(&header, Error::InvalidInput).await;
            }
            let reply = self.execute_state(coordinator, client, endpoints, member, &message);
            return match reply {
                Ok(StateReply::Metadata(metadata)) => self.state_reply(&header, &metadata).await,
                Ok(StateReply::Endpoints(page)) => self.state_reply(&header, &page).await,
                Ok(StateReply::Selections(page)) => self.state_reply(&header, &page).await,
                Ok(StateReply::Transfers(page)) => self.state_reply(&header, &page).await,
                Ok(StateReply::Details(page)) => self.state_reply(&header, &page).await,
                Ok(StateReply::Files(page)) => self.state_reply(&header, &page).await,
                Ok(StateReply::Empty) => self.state_reply(&header, &()).await,
                Err(error) => self.domain_error(&header, error).await,
            };
        }
        if ![
            "BeginSelection",
            "AppendFiles",
            "GetUpload",
            "CommitSelection",
            "AbortSelection",
            "DiscardSelection",
            "SendSelection",
            "Respond",
            "Cancel",
        ]
        .contains(&member)
        {
            return self
                .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                .await;
        }
        match execute(coordinator, client, member, &message) {
            Ok(Reply::Token(token)) => connection.reply(&header, &token).await,
            Ok(Reply::Sequence(sequence)) => connection.reply(&header, &sequence).await,
            Ok(Reply::Upload(sequence, state, selection)) => {
                connection
                    .reply(&header, &(sequence, state, selection))
                    .await
            }
            Ok(Reply::Empty) => connection.reply(&header, &()).await,
            Err(error) => return self.domain_error(&header, error).await,
        }
        .map_err(|_| BusError::Transport)
    }

    fn dismiss_message(
        &self,
        coordinator: &mut Coordinator,
        client: ClientId,
        message: &Message,
    ) -> Result<(), Error> {
        let rows: Vec<state::NoticeRow> = message
            .body()
            .deserialize()
            .map_err(|_| Error::InvalidInput)?;
        if rows.is_empty() || rows.len() > 16 {
            return Err(Error::InvalidInput);
        }
        let notices = rows
            .into_iter()
            .map(|(id, kind, round, revision)| {
                let (epoch, counter) = token_parts(&id, coordinator.epoch)?;
                Ok(super::notices::Notice {
                    transfer: TransferId(epoch, counter),
                    kind: super::notices::NoticeKind::parse(&kind).ok_or(Error::InvalidInput)?,
                    round,
                    revision,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        coordinator.dismiss_notices(client, &notices)
    }

    fn execute_state<'a>(
        &mut self,
        coordinator: &mut Coordinator,
        client: ClientId,
        endpoints: impl ExactSizeIterator<Item = (&'a str, &'a str)>,
        member: &str,
        message: &Message,
    ) -> Result<StateReply, Error> {
        if !message.data().fds().is_empty() {
            return Err(Error::InvalidInput);
        }
        let body = message.body();
        if member == "BeginSnapshot" {
            body.deserialize::<()>().map_err(|_| Error::InvalidInput)?;
            return self
                .state
                .capture(coordinator, client, endpoints)
                .map(StateReply::Metadata);
        }
        if member == "EndSnapshot" {
            let id: &str = body.deserialize().map_err(|_| Error::InvalidInput)?;
            token_parts(id, coordinator.epoch)?;
            self.state.end(client, id)?;
            return Ok(StateReply::Empty);
        }
        if member == "GetFilePage" {
            let (id, transfer, offset): (&str, &str, u32) =
                body.deserialize().map_err(|_| Error::InvalidInput)?;
            token_parts(id, coordinator.epoch)?;
            token_parts(transfer, coordinator.epoch)?;
            return self
                .state
                .file_page(client, id, transfer, offset)
                .map(StateReply::Files);
        }
        let (id, offset): (&str, u32) = body.deserialize().map_err(|_| Error::InvalidInput)?;
        token_parts(id, coordinator.epoch)?;
        match member {
            "GetEndpointPage" => self
                .state
                .endpoint_page(client, id, offset)
                .map(StateReply::Endpoints),
            "GetSelectionPage" => self
                .state
                .selection_page(client, id, offset)
                .map(StateReply::Selections),
            "GetTransferDetailsPage" => self
                .state
                .detail_page(client, id, offset)
                .map(StateReply::Details),
            "GetTransferPage" => self
                .state
                .transfer_page(client, id, offset)
                .map(StateReply::Transfers),
            _ => Err(Error::InvalidInput),
        }
    }

    async fn state_reply<B: serde::Serialize + Type>(
        &self,
        header: &zbus::message::Header<'_>,
        body: &B,
    ) -> Result<(), BusError> {
        let connection = self.connection.as_ref().ok_or(BusError::Transport)?;
        let message = Message::method_return(header)
            .map_err(|_| BusError::Transport)?
            .sender(connection.unique_name().ok_or(BusError::Transport)?)
            .map_err(|_| BusError::Transport)?
            .build(body)
            .map_err(|_| BusError::Transport)?;
        if !state_message_bounded(&message) {
            return self.domain_error(header, Error::Capacity).await;
        }
        connection
            .send(&message)
            .await
            .map_err(|_| BusError::Transport)
    }

    async fn publish_changes(&mut self, coordinator: &Coordinator) -> Result<(), BusError> {
        if let Some(wakeup) = &self.wakeup {
            if let Some(revision) = coordinator
                .pending_notices()
                .iter()
                .map(|notice| notice.revision)
                .max()
            {
                wakeup.notify(revision);
            }
        }
        let connection = self.connection.as_ref().ok_or(BusError::Transport)?;
        for (sender, client) in &self.clients {
            if self.notified.get(client) == Some(&coordinator.revision) {
                continue;
            }
            let message = Message::signal(PATH, INTERFACE, "StateChanged")
                .map_err(|_| BusError::Transport)?
                .sender(connection.unique_name().ok_or(BusError::Transport)?)
                .map_err(|_| BusError::Transport)?
                .destination(sender.as_str())
                .map_err(|_| BusError::Transport)?
                .build(&(format!("{:032x}", coordinator.epoch), coordinator.revision))
                .map_err(|_| BusError::Transport)?;
            if !state_message_bounded(&message) {
                return Err(BusError::Transport);
            }
            connection
                .send(&message)
                .await
                .map_err(|_| BusError::Transport)?;
            self.notified.insert(*client, coordinator.revision);
        }
        Ok(())
    }

    async fn domain_error(
        &self,
        header: &zbus::message::Header<'_>,
        error: Error,
    ) -> Result<(), BusError> {
        self.error(header, error_name(error)).await
    }
    async fn error(&self, header: &zbus::message::Header<'_>, name: &str) -> Result<(), BusError> {
        self.connection
            .as_ref()
            .ok_or(BusError::Transport)?
            .reply_error(header, name, &"Native request rejected")
            .await
            .map_err(|_| BusError::Transport)
    }
}

impl Drop for SelectionBus {
    fn drop(&mut self) {
        self.closed.cancel();
        self.pump.abort();
    }
}

pub(super) fn error_name(error: Error) -> &'static str {
    match error {
        Error::InvalidInput => "io.github.morozhenko_apps.RQuickShare.Error.InvalidInput",
        Error::Capacity => "io.github.morozhenko_apps.RQuickShare.Error.Capacity",
        Error::UnknownClient => "io.github.morozhenko_apps.RQuickShare.Error.UnknownClient",
        Error::UnknownUpload => "io.github.morozhenko_apps.RQuickShare.Error.UnknownUpload",
        Error::UnknownSnapshot => "io.github.morozhenko_apps.RQuickShare.Error.UnknownSnapshot",
        Error::InvalidState => "io.github.morozhenko_apps.RQuickShare.Error.InvalidState",
        Error::UnknownSelection => "io.github.morozhenko_apps.RQuickShare.Error.UnknownSelection",
        Error::UnknownTransfer => "io.github.morozhenko_apps.RQuickShare.Error.UnknownTransfer",
        Error::UnknownEndpoint => "io.github.morozhenko_apps.RQuickShare.Error.UnknownEndpoint",
        Error::WrongOwner => "io.github.morozhenko_apps.RQuickShare.Error.WrongOwner",
        Error::RequestExpired => "io.github.morozhenko_apps.RQuickShare.Error.RequestExpired",
        Error::ReplayConflict => "io.github.morozhenko_apps.RQuickShare.Error.ReplayConflict",
        Error::StaleConsent => "io.github.morozhenko_apps.RQuickShare.Error.StaleConsent",
        Error::StaleControl => "io.github.morozhenko_apps.RQuickShare.Error.StaleControl",
        Error::Unsynchronized => "io.github.morozhenko_apps.RQuickShare.Error.Unsynchronized",
        Error::NotRunning => "io.github.morozhenko_apps.RQuickShare.Error.NotRunning",
        Error::Busy => "io.github.morozhenko_apps.RQuickShare.Error.Busy",
        Error::Exhausted => "io.github.morozhenko_apps.RQuickShare.Error.Exhausted",
    }
}

fn owner_rule(name: &str) -> zbus::Result<zbus::MatchRule<'_>> {
    Ok(zbus::MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender("org.freedesktop.DBus")?
        .path("/org/freedesktop/DBus")?
        .interface("org.freedesktop.DBus")?
        .member("NameOwnerChanged")?
        .arg(0, name)?
        .arg(2, "")?
        .build())
}

#[derive(Debug)]
enum AuthenticationError {
    Denied,
    Coordinator(Error),
}

enum Reply {
    Token(String),
    Sequence(u32),
    Upload(u32, &'static str, String),
    Empty,
}

enum StateReply {
    Metadata(state::Metadata),
    Endpoints((u32, Vec<state::EndpointRow>)),
    Selections((u32, Vec<String>)),
    Transfers((u32, Vec<state::TransferRow>)),
    Details((u32, Vec<state::DetailRow>)),
    Files((u32, Vec<state::FileRow>)),
    Empty,
}

fn state_message_bounded(message: &Message) -> bool {
    message.body().len() <= 48 * 1024 && message.data().len() <= 64 * 1024
}
fn token(epoch: u128, counter: u64) -> String {
    format!("{epoch:032x}-{counter:016x}")
}
fn token_parts(value: &str, epoch: u128) -> Result<(u128, u64), Error> {
    if value.len() != 49
        || value.as_bytes()[32] != b'-'
        || !value
            .bytes()
            .enumerate()
            .all(|(n, byte)| n == 32 || byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::InvalidInput);
    }
    let supplied_epoch = u128::from_str_radix(&value[..32], 16).map_err(|_| Error::InvalidInput)?;
    let counter = u64::from_str_radix(&value[33..], 16).map_err(|_| Error::InvalidInput)?;
    if supplied_epoch != epoch || counter == 0 {
        return Err(Error::InvalidInput);
    }
    Ok((supplied_epoch, counter))
}

fn upload_id(value: &str, epoch: u128) -> Result<UploadId, Error> {
    let (epoch, counter) = token_parts(value, epoch)?;
    Ok(UploadId(epoch, counter))
}

#[derive(Type)]
#[zvariant(signature = "a(hs)")]
struct Batch<'a>(Vec<(Fd<'a>, &'a str)>);
impl<'de> Deserialize<'de> for Batch<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Batch<'de>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("1..16 descriptor/name entries")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::with_capacity(BATCH_FILES);
                while let Some(entry) = sequence.next_element::<(Fd<'de>, &'de str)>()? {
                    if entries.len() == BATCH_FILES {
                        return Err(de::Error::custom("FD batch limit exceeded"));
                    }
                    entries.push(entry);
                }
                if entries.is_empty() {
                    return Err(de::Error::custom("Empty FD batch"));
                }
                Ok(Batch(entries))
            }
        }
        deserializer.deserialize_seq(Entries)
    }
}

fn execute(
    coordinator: &mut Coordinator,
    client: ClientId,
    method: &str,
    message: &Message,
) -> Result<Reply, Error> {
    if method != "AppendFiles" && !message.data().fds().is_empty() {
        return Err(Error::InvalidInput);
    }
    let invalid = |_| Error::InvalidInput;
    let body = message.body();
    match method {
        "BeginSelection" => {
            let request: &str = body.deserialize().map_err(invalid)?;
            let id = coordinator.begin_selection(client, request)?;
            Ok(Reply::Token(token(id.0, id.1)))
        }
        "AppendFiles" => {
            let (id, sequence, batch): (&str, u32, Batch<'_>) =
                body.deserialize().map_err(invalid)?;
            let id = upload_id(id, coordinator.epoch)?;
            let unique: BTreeSet<_> = batch.0.iter().map(|(fd, _)| fd.as_raw_fd()).collect();
            if unique.len() != batch.0.len() || unique.len() != message.data().fds().len() {
                return Err(Error::InvalidInput);
            }
            let files: Result<Vec<_>, _> = batch
                .0
                .into_iter()
                .map(|(fd, name)| {
                    let handle = fd
                        .as_fd()
                        .try_clone_to_owned()
                        .map_err(|_| Error::InvalidInput)?;
                    OpenedFile::adopt(File::from(handle), name.to_owned())
                        .map_err(|_| Error::InvalidInput)
                })
                .collect();
            Ok(Reply::Sequence(
                coordinator.append_files(client, id, sequence, files?)?,
            ))
        }
        "GetUpload" | "CommitSelection" | "AbortSelection" => {
            let id: &str = body.deserialize().map_err(invalid)?;
            let id = upload_id(id, coordinator.epoch)?;
            match method {
                "GetUpload" => {
                    let view = coordinator.get_upload(client, id)?;
                    let (state, selection) = match view.state {
                        UploadState::Preparing => ("preparing", String::new()),
                        UploadState::Aborted => ("aborted", String::new()),
                        UploadState::Committed(id) => ("committed", token(id.0, id.1)),
                    };
                    Ok(Reply::Upload(view.next_sequence, state, selection))
                }
                "CommitSelection" => {
                    let id = coordinator.commit_selection(client, id)?;
                    Ok(Reply::Token(token(id.0, id.1)))
                }
                _ => {
                    coordinator.abort_selection(client, id)?;
                    Ok(Reply::Empty)
                }
            }
        }
        "DiscardSelection" => {
            let id: &str = body.deserialize().map_err(invalid)?;
            let (epoch, counter) = token_parts(id, coordinator.epoch)?;
            coordinator.discard_selection(client, SelectionId(epoch, counter))?;
            Ok(Reply::Empty)
        }
        "SendSelection" => {
            let (id, endpoint, request): (&str, &str, &str) =
                body.deserialize().map_err(invalid)?;
            let (epoch, counter) = token_parts(id, coordinator.epoch)?;
            let transfer = coordinator.send_selection(
                client,
                SelectionId(epoch, counter),
                endpoint,
                request,
            )?;
            Ok(Reply::Token(token(transfer.0, transfer.1)))
        }
        "Respond" => {
            let (id, revision, accept): (&str, u64, bool) = body.deserialize().map_err(invalid)?;
            let (epoch, counter) = token_parts(id, coordinator.epoch)?;
            let transfer = TransferId(epoch, counter);
            if accept {
                let current = coordinator
                    .transfers
                    .get(&transfer)
                    .ok_or(Error::UnknownTransfer)?;
                // Preserve stale-round error precedence and exact admitted replays.
                if current.view.phase == crate::service::Phase::AwaitingConsent
                    && current.view.consent_revision == Some(revision)
                    && current.answer.is_none()
                    && current.view.details.status != crate::sessions::details::DetailsStatus::Ready
                {
                    return Err(Error::InvalidState);
                }
            }
            coordinator.respond(client, transfer, revision, accept)?;
            Ok(Reply::Empty)
        }
        "Cancel" => {
            let id: &str = body.deserialize().map_err(invalid)?;
            let (epoch, counter) = token_parts(id, coordinator.epoch)?;
            coordinator.cancel(client, TransferId(epoch, counter))?;
            Ok(Reply::Empty)
        }
        _ => Err(Error::InvalidInput),
    }
}

enum OperationReply {
    Token(String),
    State((u64, u64)),
    Receipt((&'static str, String)),
    Empty,
}

fn execute_operation(
    coordinator: &mut Coordinator,
    client: ClientId,
    method: &str,
    message: &Message,
) -> Result<OperationReply, Error> {
    if !message.data().fds().is_empty() {
        return Err(Error::InvalidInput);
    }
    let body = message.body();
    let invalid = |_| Error::InvalidInput;
    match method {
        "CommitExtension" => {
            let (upload, base, sequence): (&str, &str, u64) =
                body.deserialize().map_err(invalid)?;
            let upload = upload_id(upload, coordinator.epoch)?;
            let (epoch, counter) = token_parts(base, coordinator.epoch)?;
            let id = coordinator.ordered_extend(
                client,
                upload,
                SelectionId(epoch, counter),
                sequence,
            )?;
            Ok(OperationReply::Token(token(id.0, id.1)))
        }
        "BeginText" => {
            let (count, sequence): (u32, u64) = body.deserialize().map_err(invalid)?;
            let id = coordinator.ordered_begin_text(client, count, sequence)?;
            Ok(OperationReply::Token(token(id.0, id.1)))
        }
        "BeginSelection" => {
            let sequence: u64 = body.deserialize().map_err(invalid)?;
            let id = coordinator.ordered_begin(client, sequence)?;
            Ok(OperationReply::Token(token(id.0, id.1)))
        }
        "CommitSelection" | "AbortSelection" => {
            let (id, sequence): (&str, u64) = body.deserialize().map_err(invalid)?;
            let id = upload_id(id, coordinator.epoch)?;
            if method == "CommitSelection" {
                let id = coordinator.ordered_commit(client, id, sequence)?;
                Ok(OperationReply::Token(token(id.0, id.1)))
            } else {
                coordinator.ordered_abort(client, id, sequence)?;
                Ok(OperationReply::Empty)
            }
        }
        "SendSelection" => {
            let (id, endpoint, sequence): (&str, &str, u64) =
                body.deserialize().map_err(invalid)?;
            let (epoch, counter) = token_parts(id, coordinator.epoch)?;
            let id = coordinator.ordered_send(
                client,
                SelectionId(epoch, counter),
                endpoint,
                sequence,
            )?;
            Ok(OperationReply::Token(token(id.0, id.1)))
        }
        "GetOperationState" => {
            body.deserialize::<()>().map_err(invalid)?;
            Ok(OperationReply::State(coordinator.operation_state(client)?))
        }
        "GetReceipt" => {
            let sequence: u64 = body.deserialize().map_err(invalid)?;
            let receipt = coordinator.operation_receipt(client, sequence)?;
            use super::operations::OperationResult;
            let result = match receipt.result {
                OperationResult::Upload(id) => token(id.0, id.1),
                OperationResult::Selection(id) => token(id.0, id.1),
                OperationResult::Transfer(id) => token(id.0, id.1),
                OperationResult::Transfers(_) => return Err(Error::InvalidState),
                OperationResult::Empty => String::new(),
            };
            Ok(OperationReply::Receipt((receipt.kind, result)))
        }
        "AcknowledgeThrough" => {
            let sequence: u64 = body.deserialize().map_err(invalid)?;
            coordinator.acknowledge_operations(client, sequence)?;
            Ok(OperationReply::Empty)
        }
        _ => Err(Error::InvalidInput),
    }
}

#[cfg(test)]
pub(super) mod tests;
