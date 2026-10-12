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

mod state;
pub(super) mod transport;

pub const SERVICE: &str = "io.github.morozhenko_apps.RQuickShare";
pub const PATH: &str = "/io/github/morozhenko_apps/RQuickShare";
pub const OPERATIONS_INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Operations1";
pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Manager1";
pub const INTROSPECTION: &str =
    include_str!("../../dbus/io.github.morozhenko_apps.RQuickShare.Manager1.xml");
const INBOX: usize = 8;

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

/// One pump and one serial owner; no task is spawned for an individual method.
/// Call `run` or `close` to join cleanup. Drop only requests cancellation.
pub struct SelectionBus {
    connection: Option<Connection>,
    inbox: mpsc::Receiver<Message>,
    pump: JoinHandle<()>,
    failed: CancellationToken,
    closed: CancellationToken,
    clients: BTreeMap<String, ClientId>,
    state: state::StateCache,
    notified: BTreeMap<ClientId, u64>,
    visibility_recovered: bool,
    uid: u32,
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
            notified: BTreeMap::new(),
            visibility_recovered: false,
            uid,
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

    pub(super) fn set_visibility_recovered(&mut self, value: bool) {
        self.visibility_recovered = value;
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
        tokio::select! {
            _ = stop.cancelled() => return Ok(()),
            _ = failure.cancelled() => return Err(BusError::Transport),
            result = runtime.start() => result.map_err(BusError::Runtime)?,
        }
        loop {
            tokio::select! {
                _ = stop.cancelled() => return Ok(()),
                _ = failure.cancelled() => return Err(BusError::Transport),
                result = runtime.step() => result.map_err(BusError::Runtime)?,
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
        let result = self.disconnect_clients(runtime.selection_coordinator());
        self.close_connection().await;
        shutdown.and(result)
    }

    async fn close_connection(&mut self) {
        if self.closed.is_cancelled() {
            return;
        }
        self.closed.cancel();
        let _ = (&mut self.pump).await;
        self.inbox.close();
        while self.inbox.try_recv().is_ok() {}
        if let Some(connection) = self.connection.take() {
            let _ = connection.close().await;
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
        let client = coordinator
            .connect()
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
        if interface == Some(OPERATIONS_INTERFACE) {
            let Some(member) = member else {
                return self
                    .error(&header, "org.freedesktop.DBus.Error.UnknownMethod")
                    .await;
            };
            if ![
                "BeginSelection",
                "CommitSelection",
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
            coordinator.respond(client, TransferId(epoch, counter), revision, accept)?;
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
