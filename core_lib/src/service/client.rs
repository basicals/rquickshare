//! Dedicated, serialized native client. Cancellation retains reconciliation state.

use std::fmt;
use std::os::fd::AsFd;

use serde::Serialize;
use tokio::net::UnixStream;
use zbus::zvariant::{Fd, Type};
use zbus::{Connection, Message};

use super::bus::{self, INTERFACE, OPERATIONS_INTERFACE, PATH, SERVICE};
use super::uploads::BATCH_FILES;
use super::Error as DomainError;
use crate::file_source::OpenedFile;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientError {
    Transport,
    DaemonChanged,
    Protocol,
    Incompatible,
    PendingOperation,
    AcknowledgementPending,
    InvalidInput,
    ForeignHandle,
    Source,
    Exhausted,
    Closed,
    Rejected(DomainError),
    Settings(super::native_settings::StoreError),
}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Native client: {self:?}")
    }
}
impl std::error::Error for ClientError {}

pub mod controller;
pub mod desktop;
pub mod files;
pub mod journal;
pub mod notices;
pub mod settings;
pub mod state;
pub mod text;
pub mod visibility;
mod watch;

type Result<T> = std::result::Result<T, ClientError>;

macro_rules! handle {
    ($name:ident) => {
        #[derive(Clone, Eq, PartialEq)]
        pub struct $name {
            id: String,
            connection: String,
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
        impl $name {
            pub fn id(&self) -> &str {
                &self.id
            }
        }
    };
}
handle!(Upload);
handle!(Selection);
handle!(Transfer);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
    Begin,
    BeginText,
    Commit,
    Extend,
    Abort,
    Send,
    SendMany,
}
impl OperationKind {
    fn wire(self) -> &'static str {
        match self {
            Self::Begin => "begin_selection",
            Self::BeginText => "begin_text",
            Self::Commit => "commit_selection",
            Self::Extend => "commit_extension",
            Self::Abort => "abort_selection",
            Self::Send => "send_selection",
            Self::SendMany => "send_many",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationValue {
    Upload(Upload),
    Selection(Selection),
    Transfer(Transfer),
    Transfers(Vec<Transfer>),
    Empty,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub sequence: u64,
    pub operation: OperationKind,
    pub result: OperationValue,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    Operation(Receipt),
    Appended { upload: Upload, next_sequence: u32 },
}

#[derive(Clone, Eq, PartialEq)]
enum Command {
    Begin,
    BeginText(u32),
    Commit(Upload),
    Extend(Upload, Selection),
    Abort(Upload),
    Send(Selection, String),
    SendMany(Selection, Vec<String>),
}
impl Command {
    fn kind(&self) -> OperationKind {
        match self {
            Self::Begin => OperationKind::Begin,
            Self::BeginText(_) => OperationKind::BeginText,
            Self::Commit(_) => OperationKind::Commit,
            Self::Extend(..) => OperationKind::Extend,
            Self::Abort(_) => OperationKind::Abort,
            Self::Send(_, _) => OperationKind::Send,
            Self::SendMany(_, _) => OperationKind::SendMany,
        }
    }
}

enum Pending {
    TextAppend {
        upload: Upload,
        sequence: u32,
        bytes: Vec<u8>,
    },
    Operation {
        command: Command,
        sequence: u64,
        receipt: Option<Receipt>,
    },
    Append {
        upload: Upload,
        sequence: u32,
        files: Vec<OpenedFile>,
    },
}

/// Do not share the underlying connection with another operation owner.
/// Explicit close releases only client-owned preparations, never daemon transfers.
pub struct NativeClient {
    connection: Option<Connection>,
    owner: String,
    identity: String,
    epoch: u128,
    last: u64,
    pending: Option<Pending>,
    last_outcome: Option<Outcome>,
    watcher: Option<watch::WatchOwner>,
    capture: Option<String>,
    last_view: Option<state::StateView>,
    view_incomplete: bool,
    pending_settings: Option<settings::SettingsEdit>,
    pending_visibility: Option<visibility::VisibilityIntent>,
    pending_text: Option<text::PendingText>,
    settings_seen: u64,
    #[cfg(test)]
    hook: Option<ReplyHook>,
}
impl fmt::Debug for NativeClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeClient")
            .field("connected", &self.connection.is_some())
            .field("pending", &self.pending.is_some())
            .field("last_sequence", &self.last)
            .finish_non_exhaustive()
    }
}

fn epoch(value: &str) -> Result<u128> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ClientError::Protocol);
    }
    let value = u128::from_str_radix(value, 16).map_err(|_| ClientError::Protocol)?;
    if value == 0 {
        return Err(ClientError::Protocol);
    }
    Ok(value)
}
fn canonical(value: &str, current: u128) -> Result<()> {
    if value.len() != 49 || value.as_bytes()[32] != b'-' || epoch(&value[..32])? != current {
        return Err(ClientError::Protocol);
    }
    let counter = &value[33..];
    if !counter
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || u64::from_str_radix(counter, 16).map_err(|_| ClientError::Protocol)? == 0
    {
        return Err(ClientError::Protocol);
    }
    Ok(())
}

fn reply_sender(message: &Message, expected: &str) -> Result<()> {
    reply_sender_fds(message, expected, 0)
}
fn reply_sender_fds(message: &Message, expected: &str, count: usize) -> Result<()> {
    if message
        .header()
        .sender()
        .is_none_or(|sender| sender.as_str() != expected)
        || message.data().fds().len() != count
    {
        return Err(ClientError::Protocol);
    }
    Ok(())
}

fn domain(name: &str) -> Option<DomainError> {
    use DomainError::*;
    [
        InvalidInput,
        Capacity,
        UnknownClient,
        UnknownUpload,
        UnknownSnapshot,
        InvalidState,
        UnknownSelection,
        UnknownTransfer,
        UnknownEndpoint,
        WrongOwner,
        ReplayConflict,
        RequestExpired,
        StaleConsent,
        StaleControl,
        Unsynchronized,
        NotRunning,
        Busy,
        Exhausted,
    ]
    .into_iter()
    .find(|error| bus::error_name(*error) == name)
}

fn same_uid(actual: u32, expected: u32) -> Result<()> {
    if actual != expected {
        return Err(ClientError::Protocol);
    }
    Ok(())
}

impl NativeClient {
    pub async fn connect(stream: UnixStream) -> Result<Self> {
        let connection = bus::connection_builder(stream)
            .build()
            .await
            .map_err(|_| ClientError::Transport)?;
        // Keep later bootstrap resources in one value which drops on cancellation.
        let mut client = Self {
            identity: connection
                .unique_name()
                .ok_or(ClientError::Protocol)?
                .to_string(),
            connection: Some(connection),
            owner: String::new(),
            epoch: 0,
            last: 0,
            pending: None,
            last_outcome: None,
            watcher: None,
            capture: None,
            last_view: None,
            view_incomplete: true,
            pending_settings: None,
            pending_visibility: None,
            pending_text: None,
            settings_seen: 0,
            #[cfg(test)]
            hook: None,
        };
        client.owner = client.name_owner().await?;
        zbus::names::UniqueName::try_from(client.owner.as_str())
            .map_err(|_| ClientError::Protocol)?;
        let uid = client
            .broker("GetConnectionUnixUser", &client.owner)
            .await?
            .body()
            .deserialize::<u32>()
            .map_err(|_| ClientError::Protocol)?;
        same_uid(uid, rustix::process::geteuid().as_raw())?;
        client.watcher = Some(
            watch::WatchOwner::subscribe(
                client.connection.as_ref().ok_or(ClientError::Closed)?,
                &client.owner,
            )
            .await?,
        );
        let reply = client.wire(INTERFACE, "BeginSnapshot", &()).await?;
        type Metadata = (
            String,
            String,
            u64,
            String,
            bool,
            u32,
            u32,
            u64,
            u32,
            u32,
            u32,
        );
        let metadata: Metadata = reply
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)?;
        client.epoch = epoch(&metadata.1)?;
        canonical(&metadata.0, client.epoch)?;
        if metadata.8 > 128
            || metadata.9 > 128
            || metadata.10 > 256
            || !["running", "draining", "stopped"].contains(&metadata.3.as_str())
        {
            return Err(ClientError::Protocol);
        }
        client
            .wire(INTERFACE, "EndSnapshot", &metadata.0)
            .await?
            .body()
            .deserialize::<()>()
            .map_err(|_| ClientError::Protocol)?;
        if client.operation_state().await? != (0, 0) {
            return Err(ClientError::Protocol);
        }
        Ok(client)
    }

    async fn broker<B: Serialize + Type>(&self, method: &str, body: &B) -> Result<Message> {
        let result = self
            .connection
            .as_ref()
            .ok_or(ClientError::Closed)?
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                method,
                body,
            )
            .await;
        match result {
            Ok(reply) => {
                reply_sender(&reply, "org.freedesktop.DBus")?;
                Ok(reply)
            }
            Err(zbus::Error::MethodError(_, _, reply)) => {
                reply_sender(&reply, "org.freedesktop.DBus")?;
                Err(ClientError::DaemonChanged)
            }
            Err(_) => Err(ClientError::Transport),
        }
    }
    async fn name_owner(&self) -> Result<String> {
        self.broker("GetNameOwner", &SERVICE)
            .await?
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)
    }
    async fn wire<B: Serialize + Type>(
        &mut self,
        interface: &str,
        method: &'static str,
        body: &B,
    ) -> Result<Message> {
        if let Some(watcher) = &self.watcher {
            let hint = watcher.receiver.borrow();
            if let Some(error) = hint.failure {
                return Err(error);
            }
            if self.epoch != 0 && hint.epoch.is_some_and(|epoch| epoch != self.epoch) {
                return Err(ClientError::DaemonChanged);
            }
        }
        if self.name_owner().await? != self.owner {
            return Err(ClientError::DaemonChanged);
        }
        #[cfg(test)]
        self.barrier(method, false).await;
        let result = self
            .connection
            .as_ref()
            .ok_or(ClientError::Closed)?
            .call_method(
                Some(self.owner.as_str()),
                PATH,
                Some(interface),
                method,
                body,
            )
            .await;
        #[cfg(test)]
        self.barrier(method, true).await;
        match result {
            Ok(reply) => {
                let count =
                    usize::from(interface == bus::files::INTERFACE && method == "AcquireFile");
                reply_sender_fds(&reply, &self.owner, count)?;
                Ok(reply)
            }
            Err(zbus::Error::MethodError(name, _, reply)) => {
                if reply
                    .header()
                    .sender()
                    .is_some_and(|sender| sender.as_str() == "org.freedesktop.DBus")
                {
                    return Err(ClientError::DaemonChanged);
                }
                reply_sender(&reply, &self.owner)?;
                if let Some(error) = domain(name.as_str()) {
                    return Err(ClientError::Rejected(error));
                }
                if let Some(error) = super::settings_service::store_error(name.as_str()) {
                    return Err(ClientError::Settings(error));
                }
                if matches!(
                    name.as_str(),
                    "org.freedesktop.DBus.Error.UnknownInterface"
                        | "org.freedesktop.DBus.Error.UnknownMethod"
                ) {
                    return Err(ClientError::Incompatible);
                }
                Err(ClientError::Protocol)
            }
            Err(_) => Err(ClientError::Transport),
        }
    }
    async fn operation_state(&mut self) -> Result<(u64, u64)> {
        self.wire(OPERATIONS_INTERFACE, "GetOperationState", &())
            .await?
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)
    }
    fn owned(&self, id: &str, identity: &str) -> Result<()> {
        if identity != self.identity {
            return Err(ClientError::ForeignHandle);
        }
        canonical(id, self.epoch).map_err(|_| ClientError::ForeignHandle)
    }
    fn value(&self, command: &Command, value: String) -> Result<OperationValue> {
        if matches!(command, Command::Abort(_)) {
            if !value.is_empty() {
                return Err(ClientError::Protocol);
            }
            return Ok(OperationValue::Empty);
        }
        canonical(&value, self.epoch)?;
        let connection = self.identity.clone();
        Ok(match command {
            Command::Begin | Command::BeginText(_) => OperationValue::Upload(Upload {
                id: value,
                connection,
            }),
            Command::Commit(_) | Command::Extend(..) => OperationValue::Selection(Selection {
                id: value,
                connection,
            }),
            Command::Send(_, _) => OperationValue::Transfer(Transfer {
                id: value,
                connection,
            }),
            Command::Abort(_) | Command::SendMany(_, _) => return Err(ClientError::Protocol),
        })
    }

    fn decode_receipt(
        &self,
        command: &Command,
        sequence: u64,
        kind: &str,
        value: String,
    ) -> Result<Receipt> {
        if sequence == 0 || kind != command.kind().wire() {
            return Err(ClientError::Protocol);
        }
        Ok(Receipt {
            sequence,
            operation: command.kind(),
            result: self.value(command, value)?,
        })
    }

    fn decode_many(
        &self,
        command: &Command,
        sequence: u64,
        kind: &str,
        ids: Vec<String>,
    ) -> Result<Receipt> {
        let Command::SendMany(_, endpoints) = command else {
            return Err(ClientError::Protocol);
        };
        if sequence == 0
            || kind != "send_many"
            || ids.len() != endpoints.len()
            || ids.is_empty()
            || ids.len() > super::batch::RECIPIENTS
            || ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ClientError::Protocol);
        }
        let mut transfers = Vec::with_capacity(ids.len());
        for id in ids {
            canonical(&id, self.epoch)?;
            transfers.push(Transfer {
                id,
                connection: self.identity.clone(),
            });
        }
        Ok(Receipt {
            sequence,
            operation: OperationKind::SendMany,
            result: OperationValue::Transfers(transfers),
        })
    }
    async fn execute(&mut self, command: &Command, sequence: u64) -> Result<Receipt> {
        let reply = match command {
            Command::Extend(upload, base) => {
                self.wire(
                    OPERATIONS_INTERFACE,
                    "CommitExtension",
                    &(upload.id(), base.id(), sequence),
                )
                .await?
            }
            Command::SendMany(id, endpoints) => {
                let reply = self
                    .wire(
                        super::batch::INTERFACE,
                        "Send",
                        &(id.id(), endpoints, sequence),
                    )
                    .await?;
                let ids = reply
                    .body()
                    .deserialize()
                    .map_err(|_| ClientError::Protocol)?;
                return self.decode_many(command, sequence, "send_many", ids);
            }
            Command::BeginText(count) => {
                self.wire(OPERATIONS_INTERFACE, "BeginText", &(*count, sequence))
                    .await?
            }
            Command::Begin => {
                self.wire(OPERATIONS_INTERFACE, "BeginSelection", &sequence)
                    .await?
            }
            Command::Commit(id) => {
                self.wire(
                    OPERATIONS_INTERFACE,
                    "CommitSelection",
                    &(id.id(), sequence),
                )
                .await?
            }
            Command::Abort(id) => {
                self.wire(OPERATIONS_INTERFACE, "AbortSelection", &(id.id(), sequence))
                    .await?
            }
            Command::Send(id, endpoint) => {
                self.wire(
                    OPERATIONS_INTERFACE,
                    "SendSelection",
                    &(id.id(), endpoint, sequence),
                )
                .await?
            }
        };
        let value = if matches!(command, Command::Abort(_)) {
            reply
                .body()
                .deserialize::<()>()
                .map_err(|_| ClientError::Protocol)?;
            String::new()
        } else {
            reply
                .body()
                .deserialize::<String>()
                .map_err(|_| ClientError::Protocol)?
        };
        self.decode_receipt(command, sequence, command.kind().wire(), value)
    }
    fn record(&mut self, receipt: Receipt) -> Result<()> {
        let Some(Pending::Operation {
            receipt: stored,
            sequence,
            ..
        }) = self.pending.as_mut()
        else {
            return Err(ClientError::Protocol);
        };
        if *sequence != receipt.sequence {
            return Err(ClientError::Protocol);
        }
        self.last = receipt.sequence;
        self.last_outcome = Some(Outcome::Operation(receipt.clone()));
        *stored = Some(receipt);
        Ok(())
    }
    async fn acknowledge(&mut self) -> Result<Outcome> {
        let Some(Pending::Operation {
            receipt: Some(receipt),
            ..
        }) = self.pending.as_ref()
        else {
            return Err(ClientError::Protocol);
        };
        let receipt = receipt.clone();
        self.wire(
            OPERATIONS_INTERFACE,
            "AcknowledgeThrough",
            &receipt.sequence,
        )
        .await
        .map_err(|_| ClientError::AcknowledgementPending)?
        .body()
        .deserialize::<()>()
        .map_err(|_| ClientError::AcknowledgementPending)?;
        self.pending = None;
        Ok(Outcome::Operation(receipt))
    }
    async fn mutation(&mut self, command: Command) -> Result<Receipt> {
        if let Some(pending) = self.pending.as_ref() {
            if !matches!(pending,Pending::Operation{command:previous,..} if previous==&command) {
                return Err(ClientError::PendingOperation);
            }
            return match self.recover_pending().await? {
                Some(Outcome::Operation(receipt)) => Ok(receipt),
                _ => Err(ClientError::Protocol),
            };
        }
        match &command {
            Command::Extend(upload, base) => {
                self.owned(upload.id(), &upload.connection)?;
                self.owned(base.id(), &base.connection)?;
            }
            Command::Commit(id) | Command::Abort(id) => self.owned(id.id(), &id.connection)?,
            Command::SendMany(id, endpoints) => {
                self.owned(id.id(), &id.connection)?;
                super::batch::valid_recipients(endpoints).map_err(|_| ClientError::InvalidInput)?;
            }
            Command::Send(id, endpoint) => {
                self.owned(id.id(), &id.connection)?;
                if endpoint.is_empty()
                    || endpoint.len() > 256
                    || !endpoint.bytes().all(|b| b.is_ascii_graphic())
                {
                    return Err(ClientError::InvalidInput);
                }
            }
            Command::Begin | Command::BeginText(_) => (),
        }
        let sequence = self.last.checked_add(1).ok_or(ClientError::Exhausted)?;
        self.pending = Some(Pending::Operation {
            command: command.clone(),
            sequence,
            receipt: None,
        });
        let result = self.execute(&command, sequence).await;
        if matches!(result, Err(ClientError::Rejected(_))) {
            self.pending = None;
        }
        self.record(result?)?;
        match self.acknowledge().await? {
            Outcome::Operation(receipt) => Ok(receipt),
            _ => Err(ClientError::Protocol),
        }
    }

    pub async fn begin_selection(&mut self) -> Result<Upload> {
        match self.mutation(Command::Begin).await?.result {
            OperationValue::Upload(id) => Ok(id),
            _ => Err(ClientError::Protocol),
        }
    }
    pub async fn begin_text(&mut self, byte_count: u32) -> Result<Upload> {
        if byte_count == 0 || byte_count as usize > crate::sessions::text::MAX_TEXT_BYTES {
            return Err(ClientError::InvalidInput);
        }
        match self.mutation(Command::BeginText(byte_count)).await?.result {
            OperationValue::Upload(id) => Ok(id),
            _ => Err(ClientError::Protocol),
        }
    }
    pub async fn append_text(
        &mut self,
        upload: &Upload,
        sequence: u32,
        bytes: &[u8],
    ) -> Result<u32> {
        self.owned(upload.id(), &upload.connection)?;
        if bytes.is_empty() || bytes.len() > super::uploads::TEXT_CHUNK {
            return Err(ClientError::InvalidInput);
        }
        sequence.checked_add(1).ok_or(ClientError::Exhausted)?;
        if self.pending.is_some() {
            return Err(ClientError::PendingOperation);
        }
        self.pending = Some(Pending::TextAppend {
            upload: upload.clone(),
            sequence,
            bytes: bytes.to_vec(),
        });
        match self.recover_pending().await? {
            Some(Outcome::Appended { next_sequence, .. }) => Ok(next_sequence),
            _ => Err(ClientError::Protocol),
        }
    }
    pub async fn commit_selection(&mut self, upload: &Upload) -> Result<Selection> {
        match self.mutation(Command::Commit(upload.clone())).await?.result {
            OperationValue::Selection(id) => Ok(id),
            _ => Err(ClientError::Protocol),
        }
    }
    pub async fn abort_selection(&mut self, upload: &Upload) -> Result<()> {
        self.mutation(Command::Abort(upload.clone()))
            .await
            .map(|_| ())
    }
    pub async fn extend_selection(
        &mut self,
        upload: &Upload,
        base: &Selection,
    ) -> Result<Selection> {
        match self
            .mutation(Command::Extend(upload.clone(), base.clone()))
            .await?
            .result
        {
            OperationValue::Selection(id) => Ok(id),
            _ => Err(ClientError::Protocol),
        }
    }
    pub async fn send_selection(
        &mut self,
        selection: &Selection,
        endpoint: &str,
    ) -> Result<Transfer> {
        match self
            .mutation(Command::Send(selection.clone(), endpoint.to_owned()))
            .await?
            .result
        {
            OperationValue::Transfer(id) => Ok(id),
            _ => Err(ClientError::Protocol),
        }
    }

    pub async fn send_many(
        &mut self,
        selection: &Selection,
        endpoints: Vec<String>,
    ) -> Result<Vec<Transfer>> {
        match self
            .mutation(Command::SendMany(selection.clone(), endpoints))
            .await?
            .result
        {
            OperationValue::Transfers(ids) => Ok(ids),
            _ => Err(ClientError::Protocol),
        }
    }
    pub async fn append_batch(
        &mut self,
        upload: &Upload,
        sequence: u32,
        files: &[OpenedFile],
    ) -> Result<u32> {
        if self.pending.is_some() {
            return Err(ClientError::PendingOperation);
        }
        self.owned(upload.id(), &upload.connection)?;
        if files.is_empty() || files.len() > BATCH_FILES {
            return Err(ClientError::InvalidInput);
        }
        sequence.checked_add(1).ok_or(ClientError::Exhausted)?;
        let files: std::result::Result<Vec<_>, _> = files
            .iter()
            .map(|file| OpenedFile::adopt(file.clone_handle()?, file.relative_name().to_owned()))
            .collect();
        self.pending = Some(Pending::Append {
            upload: upload.clone(),
            sequence,
            files: files.map_err(|_| ClientError::Source)?,
        });
        let result = self.submit_batch().await;
        if matches!(result, Err(ClientError::Rejected(_))) {
            self.pending = None;
        }
        let next = result?;
        self.record_batch(next)?;
        Ok(next)
    }
    async fn submit_batch(&mut self) -> Result<u32> {
        let Some(Pending::Append {
            upload,
            sequence,
            files,
        }) = self.pending.as_ref()
        else {
            return Err(ClientError::Protocol);
        };
        // Own temporary send handles across the call without borrowing pending state.
        let handles: std::result::Result<Vec<_>, _> =
            files.iter().map(|file| file.clone_handle()).collect();
        let handles = handles.map_err(|_| ClientError::Source)?;
        let names: Vec<_> = files
            .iter()
            .map(|file| file.relative_name().to_owned())
            .collect();
        let upload = upload.clone();
        let sequence = *sequence;
        let entries: Vec<_> = handles
            .iter()
            .zip(names.iter())
            .map(|(file, name)| (Fd::from(file.as_fd()), name.as_str()))
            .collect();
        let next: u32 = self
            .wire(INTERFACE, "AppendFiles", &(upload.id(), sequence, entries))
            .await?
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)?;
        if next != sequence.checked_add(1).ok_or(ClientError::Exhausted)? {
            return Err(ClientError::Protocol);
        }
        Ok(next)
    }
    fn record_batch(&mut self, next_sequence: u32) -> Result<Outcome> {
        let Some(Pending::Append {
            upload, sequence, ..
        }) = self.pending.as_ref()
        else {
            return Err(ClientError::Protocol);
        };
        if next_sequence != sequence.checked_add(1).ok_or(ClientError::Exhausted)? {
            return Err(ClientError::Protocol);
        }
        let outcome = Outcome::Appended {
            upload: upload.clone(),
            next_sequence,
        };
        self.last_outcome = Some(outcome.clone());
        self.pending = None;
        Ok(outcome)
    }

    /// Continue on this exact connection, without changing command, epoch or sequence.
    pub async fn recover_pending(&mut self) -> Result<Option<Outcome>> {
        match self.pending.as_ref() {
            Some(Pending::TextAppend {
                upload,
                sequence,
                bytes,
            }) => {
                let upload = upload.clone();
                let sequence = *sequence;
                let bytes = bytes.clone();
                let result = self
                    .wire(
                        bus::text::INTERFACE,
                        "AppendText",
                        &(upload.id(), sequence, bytes),
                    )
                    .await;
                if matches!(result, Err(ClientError::Rejected(_))) {
                    self.pending = None;
                }
                let next: u32 = result?
                    .body()
                    .deserialize()
                    .map_err(|_| ClientError::Protocol)?;
                if next != sequence.checked_add(1).ok_or(ClientError::Exhausted)? {
                    return Err(ClientError::Protocol);
                }
                let outcome = Outcome::Appended {
                    upload,
                    next_sequence: next,
                };
                self.last_outcome = Some(outcome.clone());
                self.pending = None;
                Ok(Some(outcome))
            }
            None => Ok(None),
            Some(Pending::Operation {
                receipt: Some(_), ..
            }) => self.acknowledge().await.map(Some),
            Some(Pending::Operation {
                command,
                sequence,
                receipt: None,
            }) => {
                let command = command.clone();
                let sequence = *sequence;
                let (accepted, acknowledged) = self.operation_state().await?;
                if acknowledged != self.last {
                    return Err(ClientError::Protocol);
                }
                let result = if accepted == self.last {
                    self.execute(&command, sequence).await
                } else if accepted == sequence && matches!(command, Command::SendMany(_, _)) {
                    let reply = self
                        .wire(super::batch::INTERFACE, "GetReceipt", &sequence)
                        .await?;
                    let (kind, ids): (String, Vec<String>) = reply
                        .body()
                        .deserialize()
                        .map_err(|_| ClientError::Protocol)?;
                    self.decode_many(&command, sequence, &kind, ids)
                } else if accepted == sequence {
                    let reply = self
                        .wire(OPERATIONS_INTERFACE, "GetReceipt", &sequence)
                        .await?;
                    let (kind, value): (String, String) = reply
                        .body()
                        .deserialize()
                        .map_err(|_| ClientError::Protocol)?;
                    self.decode_receipt(&command, sequence, &kind, value)
                } else {
                    return Err(ClientError::Protocol);
                };
                if matches!(result, Err(ClientError::Rejected(_))) {
                    self.pending = None;
                }
                self.record(result?)?;
                self.acknowledge().await.map(Some)
            }
            Some(Pending::Append {
                upload, sequence, ..
            }) => {
                let upload = upload.clone();
                let sequence = *sequence;
                let reply = self.wire(INTERFACE, "GetUpload", &upload.id()).await?;
                let (next, state, selection): (u32, String, String) = reply
                    .body()
                    .deserialize()
                    .map_err(|_| ClientError::Protocol)?;
                if state != "preparing" || !selection.is_empty() {
                    return Err(ClientError::Protocol);
                }
                let expected = sequence.checked_add(1).ok_or(ClientError::Exhausted)?;
                let next = if next == sequence {
                    self.submit_batch().await?
                } else if next == expected {
                    next
                } else {
                    return Err(ClientError::Protocol);
                };
                self.record_batch(next).map(Some)
            }
        }
    }
    pub fn last_outcome(&self) -> Option<&Outcome> {
        self.last_outcome.as_ref()
    }
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
    pub async fn close(mut self) -> Result<()> {
        self.pending = None;
        self.pending_text = None;
        let mut failed = false;
        if let Some(watcher) = self.watcher.as_mut() {
            watcher.stop.cancel();
            if let Some(task) = watcher.task.take() {
                failed = task.await.is_err();
            }
        }
        let closed = self
            .connection
            .take()
            .ok_or(ClientError::Closed)?
            .close()
            .await;
        if failed || closed.is_err() {
            return Err(ClientError::Transport);
        }
        Ok(())
    }

    #[cfg(test)]
    async fn barrier(&mut self, method: &str, after: bool) {
        if self
            .hook
            .as_ref()
            .is_some_and(|hook| hook.method == method && hook.after == after)
        {
            let hook = self.hook.take().unwrap();
            hook.reached.notify_one();
            if let Some(release) = hook.release {
                release.notified().await;
            } else {
                std::future::pending::<()>().await;
            }
        }
    }
}

#[cfg(test)]
struct ReplyHook {
    method: &'static str,
    after: bool,
    reached: std::sync::Arc<tokio::sync::Notify>,
    release: Option<std::sync::Arc<tokio::sync::Notify>>,
}

#[cfg(test)]
pub(crate) mod tests;

impl Drop for NativeClient {
    fn drop(&mut self) {
        if let Some(watcher) = &self.watcher {
            watcher.stop.cancel();
        }
    }
}
