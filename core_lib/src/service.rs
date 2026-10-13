//! Internal native service state owner; not an authenticated IPC endpoint.
//!
//! A single adapter task serializes calls and maps authenticated bus connections to
//! client tokens. Core observations and endpoint updates are adapter-only inputs.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Weak};

use crate::file_source::OpenedSelection;

macro_rules! token {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
        pub struct $name(u128, u64);
    };
}

pub mod notices;
use notices::Notice;

token!(ClientId);
token!(SelectionId);
token!(TransferId);
token!(UploadId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Native coordinator: {self:?}")
    }
}

impl std::error::Error for Error {}

/// Provisional internal admission budgets, not protocol or product limits.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub clients: usize,
    pub selections: usize,
    pub reserved_handles: usize,
    pub selected_bytes: u64,
    pub queued_sends: usize,
    pub transfers: usize,
    pub endpoints: usize,
    pub replay_records_per_client: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            clients: 32,
            selections: 128,
            reserved_handles: 1024,
            selected_bytes: 4 * 1024 * 1024 * 1024 * 1024,
            queued_sends: 10,
            transfers: 256,
            endpoints: 128,
            replay_records_per_client: 128,
        }
    }
}

impl Limits {
    fn validate(self) -> Result<(), Error> {
        if [
            self.clients,
            self.selections,
            self.reserved_handles,
            self.queued_sends,
            self.transfers,
            self.endpoints,
            self.replay_records_per_client,
        ]
        .contains(&0)
            || self.selected_bytes == 0
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lifecycle {
    Running,
    Draining,
    Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Inbound,
    Outbound,
}

/// Coarse service phases, deliberately separate from protocol handshake states.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Queued,
    Active,
    AwaitingConsent,
    Receiving,
    Sending,
    CancelRequested,
    Finished,
    Rejected,
    Cancelled,
    Failed,
    Disconnected,
}

impl Phase {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Finished | Self::Rejected | Self::Cancelled | Self::Failed | Self::Disconnected
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlAction {
    Accept,
    Reject,
    Cancel,
}

/// Adapter delivery is acknowledged by this exact revision, not just transfer ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlIntent {
    pub transfer: TransferId,
    pub revision: u64,
    pub action: ControlAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferSnapshot {
    pub id: TransferId,
    pub direction: Direction,
    pub phase: Phase,
    pub consent_revision: Option<u64>,
    pub pending_control: Option<ControlIntent>,
    pub details: Arc<crate::sessions::details::TransferDetails>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceUsage {
    pub selections: usize,
    pub reserved_handles: usize,
    pub selected_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub epoch: u128,
    pub revision: u64,
    pub lifecycle: Lifecycle,
    pub synchronized: bool,
    pub resources: ResourceUsage,
    pub selections: Vec<SelectionId>,
    pub transfers: Vec<TransferSnapshot>,
    pub notices: Vec<Notice>,
}

/// Submit exactly once; keep the payload alive for the entire reader lifetime.
pub struct SendDispatch {
    pub transfer: TransferId,
    pub endpoint: String,
    pub selection: Arc<OpenedSelection>,
}

impl fmt::Debug for SendDispatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SendDispatch")
            .field("transfer", &self.transfer)
            .field("selection", &self.selection)
            .finish_non_exhaustive()
    }
}

struct Selection {
    owner: ClientId,
    resource: Arc<OpenedSelection>,
}

struct Reservation {
    resource: Weak<OpenedSelection>,
    handles: usize,
    bytes: u64,
}

struct Replay {
    selection: SelectionId,
    endpoint: String,
    transfer: TransferId,
}

struct Transfer {
    view: TransferSnapshot,
    resource: Option<Arc<OpenedSelection>>,
    endpoint: Option<String>,
    answer: Option<(u64, bool)>,
    source: Weak<OpenedSelection>,
    owner_closed: bool,
    released: bool,
    control_receipt: bool,
    terminal_revision: Option<u64>,
}

pub struct Coordinator {
    epoch: u128,
    revision: u64,
    limits: Limits,
    lifecycle: Lifecycle,
    synchronized: bool,
    clients: BTreeMap<ClientId, BTreeMap<String, Replay>>,
    operations: BTreeMap<ClientId, operations::Ledger>,
    selections: BTreeMap<SelectionId, Selection>,
    reservations: BTreeMap<SelectionId, Reservation>,
    uploads: BTreeMap<UploadId, uploads::Upload>,
    endpoints: BTreeSet<String>,
    transfers: BTreeMap<TransferId, Transfer>,
    queue: VecDeque<TransferId>,
    notices: BTreeMap<TransferId, notices::Record>,
}

impl fmt::Debug for Coordinator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Coordinator")
            .field("revision", &self.revision)
            .field("lifecycle", &self.lifecycle)
            .field("synchronized", &self.synchronized)
            .field("clients", &self.clients.len())
            .field("transfers", &self.transfers.len())
            .finish_non_exhaustive()
    }
}

fn identifier(value: &str, max_bytes: usize) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > max_bytes
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}

fn reserve_usage(
    current: ResourceUsage,
    handles: usize,
    bytes: u64,
    limits: Limits,
) -> Result<ResourceUsage, Error> {
    let next = ResourceUsage {
        selections: current.selections.checked_add(1).ok_or(Error::Capacity)?,
        reserved_handles: current
            .reserved_handles
            .checked_add(handles)
            .ok_or(Error::Capacity)?,
        selected_bytes: current
            .selected_bytes
            .checked_add(bytes)
            .ok_or(Error::Capacity)?,
    };
    if next.selections > limits.selections
        || next.reserved_handles > limits.reserved_handles
        || next.selected_bytes > limits.selected_bytes
    {
        return Err(Error::Capacity);
    }
    Ok(next)
}

impl Coordinator {
    pub fn new(epoch: u128, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if epoch == 0 {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            epoch,
            revision: 0,
            limits,
            lifecycle: Lifecycle::Running,
            synchronized: true,
            clients: BTreeMap::new(),
            operations: BTreeMap::new(),
            selections: BTreeMap::new(),
            reservations: BTreeMap::new(),
            uploads: BTreeMap::new(),
            endpoints: BTreeSet::new(),
            transfers: BTreeMap::new(),
            queue: VecDeque::new(),
            notices: BTreeMap::new(),
        })
    }

    fn advance(&mut self) -> Result<u64, Error> {
        self.revision = self.revision.checked_add(1).ok_or(Error::Exhausted)?;
        Ok(self.revision)
    }

    fn running(&self) -> Result<(), Error> {
        if self.lifecycle != Lifecycle::Running {
            return Err(Error::NotRunning);
        }
        Ok(())
    }

    fn client(&self, client: ClientId) -> Result<(), Error> {
        if !self.clients.contains_key(&client) {
            return Err(Error::UnknownClient);
        }
        Ok(())
    }

    fn synchronized(&self) -> Result<(), Error> {
        if !self.synchronized {
            return Err(Error::Unsynchronized);
        }
        Ok(())
    }

    fn collect_released(&mut self) {
        self.reservations
            .retain(|_, entry| entry.resource.strong_count() != 0);
    }

    fn refresh_resources(&mut self) -> Result<(), Error> {
        if self
            .reservations
            .values()
            .any(|entry| entry.resource.strong_count() == 0)
        {
            self.advance()?;
            self.collect_released();
        }
        Ok(())
    }

    fn usage(&self) -> ResourceUsage {
        // Admission checked every sum before insertion; stale entries remain charged.
        self.reservations
            .values()
            .fold(self.upload_usage(), |mut usage, reservation| {
                usage.selections += 1;
                usage.reserved_handles += reservation.handles;
                usage.selected_bytes += reservation.bytes;
                usage
            })
    }

    pub fn connect(&mut self) -> Result<ClientId, Error> {
        self.running()?;
        if self.clients.len() >= self.limits.clients {
            return Err(Error::Capacity);
        }
        let client = ClientId(self.epoch, self.advance()?);
        self.clients.insert(client, BTreeMap::new());
        self.operations
            .insert(client, operations::Ledger::default());
        Ok(client)
    }

    pub fn disconnect(&mut self, client: ClientId) -> Result<(), Error> {
        self.refresh_resources()?;
        if self.clients.contains_key(&client) {
            self.advance()?;
            self.clients.remove(&client);
            self.operations.remove(&client);
            self.selections.retain(|_, entry| entry.owner != client);
            self.uploads.retain(|_, entry| entry.owner != client);
        }
        self.collect_released();
        Ok(())
    }

    pub fn snapshot(&mut self, client: ClientId) -> Result<Snapshot, Error> {
        self.client(client)?;
        self.refresh_resources()?;
        Ok(Snapshot {
            epoch: self.epoch,
            revision: self.revision,
            lifecycle: self.lifecycle,
            synchronized: self.synchronized,
            resources: self.usage(),
            notices: self.pending_notices(),
            selections: self
                .selections
                .iter()
                .filter_map(|(id, entry)| (entry.owner == client).then_some(*id))
                .collect(),
            transfers: self
                .transfers
                .values()
                .map(|entry| entry.view.clone())
                .collect(),
        })
    }

    /// Called only with the adapter's discovered IDs, never caller-supplied addresses.
    pub fn set_endpoints(&mut self, endpoints: Vec<String>) -> Result<(), Error> {
        self.running()?;
        if endpoints.len() > self.limits.endpoints {
            return Err(Error::Capacity);
        }
        for endpoint in &endpoints {
            identifier(endpoint, 256)?;
        }
        let endpoints: BTreeSet<_> = endpoints.into_iter().collect();
        if endpoints != self.endpoints {
            self.advance()?;
            self.endpoints = endpoints;
        }
        Ok(())
    }

    pub fn create_selection(
        &mut self,
        client: ClientId,
        selection: OpenedSelection,
    ) -> Result<SelectionId, Error> {
        self.running()?;
        self.client(client)?;
        self.refresh_resources()?;
        let handles = selection
            .files()
            .len()
            .checked_mul(2)
            .ok_or(Error::Capacity)?;
        let bytes = selection.total_bytes();
        reserve_usage(self.usage(), handles, bytes, self.limits)?;
        let id = SelectionId(self.epoch, self.advance()?);
        let resource = Arc::new(selection);
        self.reservations.insert(
            id,
            Reservation {
                resource: Arc::downgrade(&resource),
                handles,
                bytes,
            },
        );
        self.selections.insert(
            id,
            Selection {
                owner: client,
                resource,
            },
        );
        Ok(id)
    }

    pub fn discard_selection(&mut self, client: ClientId, id: SelectionId) -> Result<(), Error> {
        self.client(client)?;
        self.refresh_resources()?;
        if let Some(selection) = self.selections.get(&id) {
            if selection.owner != client {
                return Err(Error::WrongOwner);
            }
            self.advance()?;
            self.selections.remove(&id);
        }
        self.collect_released();
        Ok(())
    }

    pub fn send_selection(
        &mut self,
        client: ClientId,
        selection: SelectionId,
        endpoint: &str,
        request: &str,
    ) -> Result<TransferId, Error> {
        self.running()?;
        self.client(client)?;
        identifier(request, 128)?;
        identifier(endpoint, 256)?;
        if let Some(replay) = self.clients[&client].get(request) {
            return if replay.selection == selection && replay.endpoint == endpoint {
                Ok(replay.transfer)
            } else {
                Err(Error::ReplayConflict)
            };
        }
        self.enqueue_selection(client, selection, endpoint, Some(request))
    }

    pub(super) fn enqueue_selection(
        &mut self,
        client: ClientId,
        selection: SelectionId,
        endpoint: &str,
        request: Option<&str>,
    ) -> Result<TransferId, Error> {
        self.running()?;
        self.client(client)?;
        identifier(endpoint, 256)?;
        self.synchronized()?;
        let pending = self
            .selections
            .get(&selection)
            .ok_or(Error::UnknownSelection)?;
        if pending.owner != client {
            return Err(Error::WrongOwner);
        }
        if !self.endpoints.contains(endpoint) {
            return Err(Error::UnknownEndpoint);
        }
        if self.queue.len() >= self.limits.queued_sends
            || self.transfers.len() >= self.limits.transfers
            || (request.is_some()
                && self.clients[&client].len() >= self.limits.replay_records_per_client)
        {
            return Err(Error::Capacity);
        }
        let id = TransferId(self.epoch, self.advance()?);
        // Ownership changes only after every admission check has succeeded.
        let pending = self
            .selections
            .remove(&selection)
            .ok_or(Error::UnknownSelection)?;
        self.transfers.insert(
            id,
            Transfer {
                view: TransferSnapshot {
                    id,
                    direction: Direction::Outbound,
                    phase: Phase::Queued,
                    consent_revision: None,
                    pending_control: None,
                    details: Arc::default(),
                },
                source: Arc::downgrade(&pending.resource),
                owner_closed: true,
                released: true,
                control_receipt: false,
                terminal_revision: None,
                resource: Some(pending.resource),
                endpoint: Some(endpoint.to_owned()),
                answer: None,
            },
        );
        self.queue.push_back(id);
        if let Some(request) = request {
            if let Some(ledger) = self.clients.get_mut(&client) {
                ledger.insert(
                    request.to_owned(),
                    Replay {
                        selection,
                        endpoint: endpoint.to_owned(),
                        transfer: id,
                    },
                );
            }
        }
        Ok(id)
    }

    pub fn next_send(&mut self) -> Result<Option<SendDispatch>, Error> {
        self.running()?;
        self.synchronized()?;
        let Some(id) = self.queue.front().copied() else {
            return Ok(None);
        };
        let transfer = self.transfers.get(&id).ok_or(Error::UnknownTransfer)?;
        let selection = transfer.resource.clone().ok_or(Error::UnknownSelection)?;
        let endpoint = transfer.endpoint.clone().ok_or(Error::UnknownEndpoint)?;
        self.advance()?;
        self.queue.pop_front();
        if let Some(transfer) = self.transfers.get_mut(&id) {
            transfer.resource = None;
            transfer.endpoint = None;
            transfer.view.phase = Phase::Active;
            transfer.owner_closed = false;
            transfer.released = false;
        }
        Ok(Some(SendDispatch {
            transfer: id,
            endpoint,
            selection,
        }))
    }

    pub fn register_inbound(&mut self) -> Result<TransferId, Error> {
        self.running()?;
        self.synchronized()?;
        if self.transfers.len() >= self.limits.transfers {
            return Err(Error::Capacity);
        }
        let id = TransferId(self.epoch, self.advance()?);
        self.transfers.insert(
            id,
            Transfer {
                view: TransferSnapshot {
                    id,
                    direction: Direction::Inbound,
                    phase: Phase::Active,
                    consent_revision: None,
                    pending_control: None,
                    details: Arc::default(),
                },
                resource: None,
                endpoint: None,
                answer: None,
                source: Weak::new(),
                owner_closed: false,
                released: true,
                control_receipt: false,
                terminal_revision: None,
            },
        );
        Ok(id)
    }

    /// Trusted core adapter input; terminal states absorb subsequent observations.
    pub fn observe(&mut self, id: TransferId, phase: Phase) -> Result<(), Error> {
        let mut details = (*self
            .transfers
            .get(&id)
            .ok_or(Error::UnknownTransfer)?
            .view
            .details)
            .clone();
        if phase != Phase::AwaitingConsent {
            details.pin.clear();
        }
        self.observe_details(id, phase, Arc::new(details))
    }

    pub(crate) fn observe_details(
        &mut self,
        id: TransferId,
        phase: Phase,
        details: Arc<crate::sessions::details::TransferDetails>,
    ) -> Result<(), Error> {
        if !details.valid(phase == Phase::AwaitingConsent) {
            return Err(Error::InvalidInput);
        }
        let transfer = self.transfers.get(&id).ok_or(Error::UnknownTransfer)?;
        if transfer.view.phase.is_terminal() {
            return Ok(());
        }
        if matches!(phase, Phase::Queued | Phase::CancelRequested)
            || (matches!(phase, Phase::AwaitingConsent | Phase::Receiving)
                && transfer.view.direction != Direction::Inbound)
            || (phase == Phase::Sending && transfer.view.direction != Direction::Outbound)
            || (transfer.view.phase == Phase::Queued && !phase.is_terminal())
        {
            return Err(Error::InvalidInput);
        }
        let details = if phase == Phase::AwaitingConsent
            && transfer.view.phase == phase
            && transfer.view.details.status == crate::sessions::details::DetailsStatus::Ready
            && details.status == crate::sessions::details::DetailsStatus::Ready
            && !transfer.view.details.same_description(&details)
        {
            Arc::new(crate::sessions::details::TransferDetails::unsupported())
        } else {
            details
        };
        if (phase == transfer.view.phase && transfer.view.details == details)
            || (transfer.view.phase == Phase::CancelRequested && !phase.is_terminal())
        {
            return Ok(());
        }
        let revision = self.advance()?;
        if let Some(transfer) = self.transfers.get_mut(&id) {
            let phase_changed = transfer.view.phase != phase;
            transfer.view.phase = phase;
            transfer.view.details = details;
            if phase_changed {
                transfer.view.consent_revision =
                    (phase == Phase::AwaitingConsent).then_some(revision);
                transfer.view.pending_control = None;
            }
            if phase_changed && phase == Phase::AwaitingConsent {
                transfer.answer = None;
            }
            if phase.is_terminal() {
                transfer.terminal_revision = Some(revision);
                transfer.resource = None;
                transfer.endpoint = None;
                self.queue.retain(|queued| *queued != id);
            }
        }
        self.notice_event(id);
        self.collect_released();
        Ok(())
    }

    pub fn respond(
        &mut self,
        client: ClientId,
        id: TransferId,
        consent_revision: u64,
        accept: bool,
    ) -> Result<(), Error> {
        self.running()?;
        self.client(client)?;
        self.synchronized()?;
        let transfer = self.transfers.get(&id).ok_or(Error::UnknownTransfer)?;
        if transfer.view.direction != Direction::Inbound
            || transfer.view.phase != Phase::AwaitingConsent
            || transfer.view.consent_revision != Some(consent_revision)
        {
            return Err(Error::StaleConsent);
        }
        if let Some(answer) = transfer.answer {
            return if answer == (consent_revision, accept) {
                Ok(())
            } else {
                Err(Error::StaleConsent)
            };
        }
        let revision = self.advance()?;
        if let Some(transfer) = self.transfers.get_mut(&id) {
            transfer.answer = Some((consent_revision, accept));
            transfer.view.pending_control = Some(ControlIntent {
                transfer: id,
                revision,
                action: if accept {
                    ControlAction::Accept
                } else {
                    ControlAction::Reject
                },
            });
        }
        Ok(())
    }

    /// Pending until acknowledged; callers must wait for events rather than spin.
    pub fn next_control(&self) -> Option<ControlIntent> {
        self.transfers.values().find_map(|transfer| {
            transfer
                .view
                .pending_control
                .filter(|intent| self.synchronized || intent.action == ControlAction::Cancel)
        })
    }

    pub fn acknowledge_control(&mut self, intent: ControlIntent) -> Result<(), Error> {
        let transfer = self
            .transfers
            .get(&intent.transfer)
            .ok_or(Error::UnknownTransfer)?;
        if transfer.view.pending_control != Some(intent) {
            return Err(Error::StaleControl);
        }
        self.advance()?;
        if let Some(transfer) = self.transfers.get_mut(&intent.transfer) {
            transfer.view.pending_control = None;
        }
        Ok(())
    }

    pub fn cancel(&mut self, client: ClientId, id: TransferId) -> Result<(), Error> {
        self.client(client)?;
        let transfer = self.transfers.get(&id).ok_or(Error::UnknownTransfer)?;
        if transfer.view.phase.is_terminal() || transfer.view.phase == Phase::CancelRequested {
            return Ok(());
        }
        let revision = self.advance()?;
        if let Some(transfer) = self.transfers.get_mut(&id) {
            transfer.view.consent_revision = None;
            Arc::make_mut(&mut transfer.view.details).pin.clear();
            if transfer.view.phase == Phase::Queued {
                transfer.view.phase = Phase::Cancelled;
                transfer.terminal_revision = Some(revision);
                transfer.resource = None;
                transfer.endpoint = None;
                self.queue.retain(|queued| *queued != id);
            } else {
                transfer.view.phase = Phase::CancelRequested;
                transfer.view.pending_control = Some(ControlIntent {
                    transfer: id,
                    revision,
                    action: ControlAction::Cancel,
                });
            }
        }
        self.notice_event(id);
        self.collect_released();
        Ok(())
    }

    pub fn mark_desynchronized(&mut self) -> Result<(), Error> {
        if self.synchronized {
            self.advance()?;
            self.synchronized = false;
        }
        Ok(())
    }

    pub fn begin_shutdown(&mut self) -> Result<(), Error> {
        if self.lifecycle != Lifecycle::Running {
            return Ok(());
        }
        let revision = self.advance()?;
        self.lifecycle = Lifecycle::Draining;
        self.notices.clear();
        self.selections.clear();
        self.uploads.clear();
        self.queue.clear();
        for (id, transfer) in &mut self.transfers {
            transfer.view.consent_revision = None;
            Arc::make_mut(&mut transfer.view.details).pin.clear();
            if transfer.view.phase == Phase::Queued {
                transfer.view.phase = Phase::Cancelled;
                transfer.terminal_revision = Some(revision);
                transfer.resource = None;
                transfer.endpoint = None;
            } else if !transfer.view.phase.is_terminal() {
                transfer.view.phase = Phase::CancelRequested;
                transfer.view.pending_control = Some(ControlIntent {
                    transfer: *id,
                    revision,
                    action: ControlAction::Cancel,
                });
            }
        }
        self.collect_released();
        Ok(())
    }

    /// The adapter must await core shutdown first; this only checks local ownership.
    pub fn finish_shutdown(&mut self) -> Result<(), Error> {
        if self.lifecycle == Lifecycle::Stopped {
            return Ok(());
        }
        if self.lifecycle != Lifecycle::Draining {
            return Err(Error::NotRunning);
        }
        self.refresh_resources()?;
        if self
            .transfers
            .values()
            .any(|transfer| !transfer.view.phase.is_terminal())
            || !self.reservations.is_empty()
        {
            return Err(Error::Busy);
        }
        self.advance()?;
        self.lifecycle = Lifecycle::Stopped;
        Ok(())
    }
}

#[cfg(test)]
mod tests;

pub mod runtime;

pub mod uploads;

pub mod bus;
pub mod process;

pub mod operations;

mod retention;

pub mod client;
