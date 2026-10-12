//! Bounded live-session capabilities for trusted core adapters, not an IPC API.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, Weak};

use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;

use crate::channel::{ChannelMessage, TransferType};
use crate::State;

pub mod details;
use details::TransferDetails;

const SESSION_LIMIT: usize = 32;
pub(crate) const EVENT_LIMIT: usize = 512;

/// Ordered observations for a single trusted native owner. Lag requires fail-closed handling.
#[derive(Clone)]
pub enum SessionEvent {
    Opened(SessionControl),
    Changed(SessionSnapshot),
    Closed(SessionSnapshot),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionId {
    epoch: u128,
    sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsentToken {
    session: SessionId,
    round: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    Invalid,
    Full,
    Closed,
    OwnerLost,
    Exhausted,
    Stale,
    Conflict,
    Cancelled,
    WrongDirection,
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Managed session: {self:?}")
    }
}
impl std::error::Error for SessionError {}

#[derive(Clone, PartialEq)]
pub struct SessionSnapshot {
    pub id: SessionId,
    pub event_id: String,
    pub direction: TransferType,
    pub state: State,
    pub consent: Option<ConsentToken>,
    pub details: Arc<TransferDetails>,
}

impl fmt::Debug for SessionSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionSnapshot")
            .field("id", &self.id)
            .field("direction", &self.direction)
            .field("state", &self.state)
            .field("consent", &self.consent)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Decision {
    token: ConsentToken,
    command: u64,
    accept: bool,
}

#[derive(Clone)]
struct Status {
    snapshot: SessionSnapshot,
    decision: Option<Decision>,
}

pub struct CommandReceipt(oneshot::Receiver<Result<(), SessionError>>);

impl CommandReceipt {
    pub async fn wait(self) -> Result<(), SessionError> {
        self.0.await.map_err(|_| SessionError::OwnerLost)?
    }
}

#[derive(Clone)]
pub struct SessionControl {
    cancellation: CancellationToken,
    commands: mpsc::Sender<ConsentCommand>,
    status: watch::Receiver<Status>,
}

impl SessionControl {
    pub fn snapshot(&self) -> SessionSnapshot {
        self.status.borrow().snapshot.clone()
    }

    pub async fn changed(&mut self) -> Result<SessionSnapshot, SessionError> {
        self.status
            .changed()
            .await
            .map_err(|_| SessionError::Closed)?;
        Ok(self.snapshot())
    }

    pub fn is_closed(&self) -> bool {
        self.commands.is_closed()
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub fn respond(
        &self,
        token: ConsentToken,
        command: u64,
        accept: bool,
    ) -> Result<CommandReceipt, SessionError> {
        let decision = Decision {
            token,
            command,
            accept,
        };
        let status = self.status.borrow();
        let replay = validate(&status, decision)?;
        if replay {
            let (sender, receiver) = oneshot::channel();
            let _ = sender.send(Ok(()));
            return Ok(CommandReceipt(receiver));
        }
        if self.is_closed() {
            return Err(SessionError::Closed);
        }
        if self.cancellation.is_cancelled() {
            return Err(SessionError::Cancelled);
        }
        let (reply, receiver) = oneshot::channel();
        self.commands
            .try_send(ConsentCommand { decision, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => SessionError::Full,
                mpsc::error::TrySendError::Closed(_) => SessionError::Closed,
            })?;
        Ok(CommandReceipt(receiver))
    }
}

fn validate(status: &Status, decision: Decision) -> Result<bool, SessionError> {
    if decision.command == 0 {
        return Err(SessionError::Invalid);
    }
    if status.snapshot.direction != TransferType::Inbound {
        return Err(SessionError::WrongDirection);
    }
    if decision.token.session != status.snapshot.id {
        return Err(SessionError::Stale);
    }
    if let Some(previous) = status
        .decision
        .filter(|previous| previous.token == decision.token)
    {
        return if previous == decision {
            Ok(true)
        } else {
            Err(SessionError::Conflict)
        };
    }
    if status.snapshot.state != State::WaitingForUserConsent
        || status.snapshot.consent != Some(decision.token)
    {
        return Err(SessionError::Stale);
    }
    Ok(false)
}

#[derive(Clone)]
pub struct SessionHub(Arc<Mutex<HubState>>);

struct HubState {
    epoch: u128,
    sequence: u64,
    limit: usize,
    open: bool,
    sessions: BTreeMap<SessionId, SessionControl>,
    events: broadcast::Sender<SessionEvent>,
    membership: watch::Sender<()>,
}

impl SessionHub {
    pub(crate) fn new(epoch: u128) -> Result<Self, SessionError> {
        Self::with_limit(epoch, SESSION_LIMIT)
    }

    fn with_limit(epoch: u128, limit: usize) -> Result<Self, SessionError> {
        if epoch == 0 || limit == 0 {
            return Err(SessionError::Invalid);
        }
        Ok(Self(Arc::new(Mutex::new(HubState {
            epoch,
            sequence: 0,
            limit,
            open: true,
            sessions: BTreeMap::new(),
            events: broadcast::channel(EVENT_LIMIT).0,
            membership: watch::channel(()).0,
        }))))
    }

    /// Subscribe before the first session. A late subscription is not recovery.
    pub fn subscribe_fresh(&self) -> Result<broadcast::Receiver<SessionEvent>, SessionError> {
        let state = self.0.lock().map_err(|_| SessionError::OwnerLost)?;
        if !state.open {
            return Err(SessionError::Closed);
        }
        if state.sequence != 0 {
            return Err(SessionError::Stale);
        }
        Ok(state.events.subscribe())
    }

    pub(crate) async fn available(&self) -> Result<(), SessionError> {
        let mut membership = self
            .0
            .lock()
            .map_err(|_| SessionError::OwnerLost)?
            .membership
            .subscribe();
        loop {
            {
                let state = self.0.lock().map_err(|_| SessionError::OwnerLost)?;
                if !state.open {
                    return Err(SessionError::Closed);
                }
                if state.sessions.len() < state.limit {
                    return Ok(());
                }
            }
            membership
                .changed()
                .await
                .map_err(|_| SessionError::Closed)?;
        }
    }

    pub fn active(&self) -> Result<Vec<SessionControl>, SessionError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| SessionError::OwnerLost)?
            .sessions
            .values()
            .cloned()
            .collect())
    }

    pub(crate) fn close(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.open = false;
            state.membership.send_replace(());
            for control in state.sessions.values() {
                control.cancel();
            }
        }
    }

    pub(crate) fn open(
        &self,
        direction: TransferType,
        event_id: Option<String>,
        cancellation: CancellationToken,
    ) -> Result<SessionLease, SessionError> {
        let mut state = self.0.lock().map_err(|_| SessionError::OwnerLost)?;
        if !state.open {
            return Err(SessionError::Closed);
        }
        if state.sessions.len() >= state.limit {
            return Err(SessionError::Full);
        }
        let sequence = state
            .sequence
            .checked_add(1)
            .ok_or(SessionError::Exhausted)?;
        let id = SessionId {
            epoch: state.epoch,
            sequence,
        };
        let event_id =
            event_id.unwrap_or_else(|| format!("managed-{:032x}-{:016x}", id.epoch, id.sequence));
        let snapshot = SessionSnapshot {
            id,
            event_id,
            direction,
            state: State::Initial,
            consent: None,
            details: Arc::default(),
        };
        let (status, receiver) = watch::channel(Status {
            snapshot,
            decision: None,
        });
        let (commands, mailbox) = mpsc::channel(1);
        let control = SessionControl {
            cancellation: cancellation.clone(),
            commands,
            status: receiver,
        };
        state.sequence = sequence;
        state.sessions.insert(id, control.clone());
        let _ = state.events.send(SessionEvent::Opened(control.clone()));
        Ok(SessionLease {
            id,
            hub: Arc::downgrade(&self.0),
            control,
            status,
            mailbox,
            round: 0,
        })
    }
}

pub(crate) struct ConsentCommand {
    decision: Decision,
    reply: oneshot::Sender<Result<(), SessionError>>,
}

pub(crate) struct SessionLease {
    id: SessionId,
    hub: Weak<Mutex<HubState>>,
    control: SessionControl,
    status: watch::Sender<Status>,
    mailbox: mpsc::Receiver<ConsentCommand>,
    round: u64,
}

impl fmt::Debug for SessionLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.control.snapshot().fmt(formatter)
    }
}

impl SessionLease {
    fn publish(&self, status: Status) {
        let snapshot = status.snapshot.clone();
        self.status.send_replace(status);
        if let Some(hub) = self.hub.upgrade() {
            if let Ok(state) = hub.lock() {
                let _ = state.events.send(SessionEvent::Changed(snapshot));
            }
        }
    }
    pub fn control(&self) -> SessionControl {
        self.control.clone()
    }
    pub fn cancellation(&self) -> CancellationToken {
        self.control.cancellation.clone()
    }
    pub fn event_id(&self) -> String {
        self.control.snapshot().event_id
    }

    pub fn observe_state(&mut self, actual: &crate::hdl::InnerState) {
        let previous = self.status.borrow().snapshot.clone();
        let waiting = actual.state == State::WaitingForUserConsent;
        let details = if waiting && previous.state != actual.state {
            TransferDetails::default()
        } else if !waiting && previous.details.status == details::DetailsStatus::Ready {
            // Core working maps retire completed files; retain the prepared immutable manifest.
            let mut details = (*previous.details).clone();
            details.pin.clear();
            if let Some(metadata) = &actual.transfer_metadata {
                details.transferred_bytes = metadata.ack_bytes;
                if metadata.total_bytes != details.total_bytes || !details.valid(false) {
                    details = TransferDetails::unsupported();
                }
            } else {
                details = TransferDetails::unsupported();
            }
            details
        } else {
            TransferDetails::from_state(actual)
        };
        self.observe_details(&actual.state, details);
    }

    pub fn observe(&mut self, actual: &State) {
        let previous = self.status.borrow().snapshot.clone();
        let mut details = if *actual == State::WaitingForUserConsent && previous.state != *actual {
            TransferDetails::default()
        } else {
            (*previous.details).clone()
        };
        if *actual != State::WaitingForUserConsent {
            details.pin.clear();
        }
        self.observe_details(actual, details);
    }

    pub fn observe_details(&mut self, actual: &State, details: TransferDetails) {
        let mut status = self.status.borrow().clone();
        if terminal(&status.snapshot.state) {
            return;
        }
        if *actual == State::WaitingForUserConsent && status.snapshot.state != *actual {
            let Some(round) = self.round.checked_add(1) else {
                status.snapshot.state = actual.clone();
                status.snapshot.consent = None;
                status.decision = None;
                self.publish(status);
                self.control.cancel();
                return;
            };
            self.round = round;
            status.decision = None;
            status.snapshot.consent = Some(ConsentToken {
                session: self.id,
                round,
            });
        } else if *actual != State::WaitingForUserConsent {
            status.snapshot.consent = None;
        }
        let details = if *actual == State::WaitingForUserConsent
            && status.snapshot.state == *actual
            && status.snapshot.details.status == details::DetailsStatus::Ready
            && details.status == details::DetailsStatus::Ready
            && !status.snapshot.details.same_description(&details)
        {
            TransferDetails::unsupported()
        } else {
            details
        };
        let details_changed = *status.snapshot.details != details;
        if details_changed {
            status.snapshot.details = Arc::new(details);
        }
        if details_changed
            || status.snapshot.state != *actual
            || status.snapshot.consent != self.status.borrow().snapshot.consent
        {
            status.snapshot.state = actual.clone();
            self.publish(status);
        }
    }

    pub fn apply(&mut self, command: ConsentCommand, actual: &State) -> Option<bool> {
        let mut status = self.status.borrow().clone();
        let validation = validate(&status, command.decision).and_then(|replay| {
            if replay {
                return Ok(true);
            }
            if self.control.cancellation.is_cancelled() {
                return Err(SessionError::Cancelled);
            }
            if *actual != State::WaitingForUserConsent {
                return Err(SessionError::Stale);
            }
            Ok(false)
        });
        match validation {
            Ok(replay) => {
                if !replay {
                    status.decision = Some(command.decision);
                    self.status.send_replace(status);
                }
                let _ = command.reply.send(Ok(()));
                (!replay).then_some(command.decision.accept)
            }
            Err(error) => {
                let _ = command.reply.send(Err(error));
                None
            }
        }
    }
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        self.mailbox.close();
        if let Some(hub) = self.hub.upgrade() {
            if let Ok(mut state) = hub.lock() {
                state.sessions.remove(&self.id);
                let _ = state
                    .events
                    .send(SessionEvent::Closed(self.control.snapshot()));
                state.membership.send_replace(());
            }
        }
    }
}

pub(crate) fn terminal(state: &State) -> bool {
    matches!(
        state,
        State::Finished | State::Rejected | State::Cancelled | State::Disconnected
    )
}

pub(crate) async fn legacy_message(
    receiver: &mut Option<broadcast::Receiver<ChannelMessage>>,
) -> Result<ChannelMessage, broadcast::error::RecvError> {
    match receiver {
        Some(receiver) => receiver.recv().await,
        None => std::future::pending().await,
    }
}

pub(crate) async fn consent_command(session: &mut Option<SessionLease>) -> Option<ConsentCommand> {
    match session {
        Some(session) => session.mailbox.recv().await,
        None => std::future::pending().await,
    }
}

#[cfg(all(feature = "experimental", target_os = "linux"))]
pub(crate) mod tasks;

#[cfg(test)]
mod tests;
