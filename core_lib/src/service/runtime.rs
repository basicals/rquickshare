//! Serial native runtime. Call `step` from one owner; authenticate IPC outside this module.

use std::collections::BTreeMap;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;

use futures::stream::{FuturesUnordered, StreamExt};
use futures::FutureExt;
use tokio::sync::broadcast;

use super::*;
use crate::managed::{ControlLost, ManagedSender, OutboundReceiver, RejectionKind};
use crate::sessions::{ConsentToken, SessionControl, SessionError, SessionEvent, SessionId};
use crate::{EndpointInfo, OutboundPayload, SendInfo, State, RQS};

const DISCOVERY_EVENTS: usize = 128;
const OWNED_DISPATCHES: usize = 32;

type Receipt = Pin<Box<dyn Future<Output = (ControlIntent, Result<(), SessionError>)> + Send>>;
type Release = Pin<Box<dyn Future<Output = Result<(), ControlLost>> + Send>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    Coordinator(Error),
    CoreUnavailable,
    Unsynchronized,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Native runtime: {self:?}")
    }
}
impl std::error::Error for RuntimeError {}
impl From<Error> for RuntimeError {
    fn from(error: Error) -> Self {
        Self::Coordinator(error)
    }
}

/// Failed preparation returns the unchanged core to its owner.
pub struct RejectedRuntime {
    pub error: RuntimeError,
    pub core: Box<RQS>,
}
impl fmt::Debug for RejectedRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RejectedRuntime")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for RejectedRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for RejectedRuntime {}

struct Startup {
    receiver: OutboundReceiver,
    discovery_sender: broadcast::Sender<EndpointInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndpointSnapshot {
    pub id: String,
    pub name: String,
}

struct Route {
    fullname: String,
    name: String,
    addr: SocketAddr,
}
struct Binding {
    transfer: TransferId,
    control: SessionControl,
    consent: Option<ConsentToken>,
}
struct Pending {
    transfer: TransferId,
    endpoint: String,
    request: SendInfo,
}

pub struct NativeRuntime {
    core: RQS,
    coordinator: Coordinator,
    sender: ManagedSender,
    events: broadcast::Receiver<SessionEvent>,
    discovery: broadcast::Receiver<EndpointInfo>,
    routes: BTreeMap<String, Route>,
    endpoint_sequence: u64,
    bindings: BTreeMap<SessionId, Binding>,
    pending: Option<Pending>,
    receipts: FuturesUnordered<Receipt>,
    inflight: BTreeMap<TransferId, u64>,
    deferred: BTreeMap<TransferId, u64>,
    releases: FuturesUnordered<Release>,
    core_lost: bool,
    startup: Option<Startup>,
    started: bool,
}

impl NativeRuntime {
    /// Prepare synchronously, subscribing before transports can create sessions.
    pub fn new(epoch: u128, limits: Limits, core: RQS) -> Result<Self, RejectedRuntime> {
        let prepared = (|| {
            if core.tracker.is_some() {
                return Err(RuntimeError::CoreUnavailable);
            }
            let coordinator = Coordinator::new(epoch, limits)?;
            let (sender, receiver) =
                ManagedSender::channel().map_err(|_| RuntimeError::CoreUnavailable)?;
            let events = sender
                .sessions()
                .subscribe_fresh()
                .map_err(|_| RuntimeError::CoreUnavailable)?;
            Ok::<_, RuntimeError>((coordinator, sender, receiver, events))
        })();
        let (coordinator, sender, receiver, events) = match prepared {
            Ok(parts) => parts,
            Err(error) => {
                return Err(RejectedRuntime {
                    error,
                    core: Box::new(core),
                });
            }
        };
        let (discovery_sender, discovery) = broadcast::channel(DISCOVERY_EVENTS);
        let mut runtime = Self::assemble(core, coordinator, sender, events, discovery);
        runtime.startup = Some(Startup {
            receiver,
            discovery_sender,
        });
        runtime.started = false;
        Ok(runtime)
    }

    /// Keep the runtime owned by the caller across this cancellable startup wait.
    /// After abandonment or failure, await shutdown and construct a fresh runtime.
    pub async fn start(&mut self) -> Result<(), RuntimeError> {
        let startup = self.startup.take().ok_or(RuntimeError::CoreUnavailable)?;
        if self.core.start(startup.receiver).await.is_err() {
            self.core.stop().await;
            return Err(RuntimeError::CoreUnavailable);
        }
        if self.core.discovery(startup.discovery_sender).is_err() {
            self.core.stop().await;
            return Err(RuntimeError::CoreUnavailable);
        }
        self.started = true;
        Ok(())
    }

    fn assemble(
        core: RQS,
        coordinator: Coordinator,
        sender: ManagedSender,
        events: broadcast::Receiver<SessionEvent>,
        discovery: broadcast::Receiver<EndpointInfo>,
    ) -> Self {
        Self {
            core,
            coordinator,
            sender,
            events,
            discovery,
            routes: BTreeMap::new(),
            endpoint_sequence: 0,
            bindings: BTreeMap::new(),
            pending: None,
            receipts: FuturesUnordered::new(),
            inflight: BTreeMap::new(),
            deferred: BTreeMap::new(),
            releases: FuturesUnordered::new(),
            core_lost: false,
            startup: None,
            started: true,
        }
    }

    pub fn connect(&mut self) -> Result<ClientId, Error> {
        if !self.started {
            return Err(Error::NotRunning);
        }
        self.coordinator.connect()
    }
    pub fn disconnect(&mut self, client: ClientId) -> Result<(), Error> {
        self.coordinator.disconnect(client)
    }
    pub fn snapshot(&mut self, client: ClientId) -> Result<Snapshot, Error> {
        self.coordinator.snapshot(client)
    }
    pub fn create_selection(
        &mut self,
        client: ClientId,
        selection: OpenedSelection,
    ) -> Result<SelectionId, Error> {
        self.coordinator.create_selection(client, selection)
    }
    pub fn begin_selection(
        &mut self,
        client: ClientId,
        request: &str,
    ) -> Result<super::UploadId, Error> {
        self.coordinator.begin_selection(client, request)
    }
    pub fn append_files(
        &mut self,
        client: ClientId,
        upload: super::UploadId,
        sequence: u32,
        files: Vec<crate::file_source::OpenedFile>,
    ) -> Result<u32, Error> {
        self.coordinator
            .append_files(client, upload, sequence, files)
    }
    pub fn get_upload(
        &self,
        client: ClientId,
        upload: super::UploadId,
    ) -> Result<super::uploads::UploadSnapshot, Error> {
        self.coordinator.get_upload(client, upload)
    }
    pub fn commit_selection(
        &mut self,
        client: ClientId,
        upload: super::UploadId,
    ) -> Result<SelectionId, Error> {
        self.coordinator.commit_selection(client, upload)
    }
    pub fn abort_selection(
        &mut self,
        client: ClientId,
        upload: super::UploadId,
    ) -> Result<(), Error> {
        self.coordinator.abort_selection(client, upload)
    }
    pub(super) fn selection_coordinator(&mut self) -> &mut Coordinator {
        &mut self.coordinator
    }
    pub fn discard_selection(
        &mut self,
        client: ClientId,
        selection: SelectionId,
    ) -> Result<(), Error> {
        self.coordinator.discard_selection(client, selection)
    }
    pub fn send_selection(
        &mut self,
        client: ClientId,
        selection: SelectionId,
        endpoint: &str,
        request: &str,
    ) -> Result<TransferId, Error> {
        self.coordinator
            .send_selection(client, selection, endpoint, request)
    }
    pub fn respond(
        &mut self,
        client: ClientId,
        transfer: TransferId,
        consent_revision: u64,
        accept: bool,
    ) -> Result<(), Error> {
        self.coordinator
            .respond(client, transfer, consent_revision, accept)
    }
    pub fn cancel(&mut self, client: ClientId, transfer: TransferId) -> Result<(), Error> {
        self.coordinator.cancel(client, transfer)
    }
    pub fn endpoints(&self) -> Vec<EndpointSnapshot> {
        self.routes
            .iter()
            .map(|(id, route)| EndpointSnapshot {
                id: id.clone(),
                name: route.name.clone(),
            })
            .collect()
    }

    fn fault(&mut self) -> Result<(), RuntimeError> {
        self.coordinator.mark_desynchronized()?;
        Err(RuntimeError::Unsynchronized)
    }

    // Invalid/unusable advertisements are ignored; discovery cannot inject hostnames into connect.
    fn discovered(&mut self, info: EndpointInfo) -> Result<(), RuntimeError> {
        if info.fullname.is_empty() || info.fullname.len() > 1024 {
            return Ok(());
        }
        let existing = self
            .routes
            .iter()
            .find_map(|(id, route)| (route.fullname == info.fullname).then(|| id.clone()));
        if info.present == Some(false) {
            if let Some(id) = existing {
                self.routes.remove(&id);
            }
        } else if info.present == Some(true) {
            let Some(addr) = info
                .ip
                .as_deref()
                .and_then(|ip| ip.parse::<IpAddr>().ok())
                .zip(
                    info.port
                        .as_deref()
                        .and_then(|port| port.parse::<u16>().ok())
                        .filter(|port| *port != 0),
                )
                .map(|(ip, port)| SocketAddr::new(ip, port))
            else {
                return Ok(());
            };
            let name = info.name.unwrap_or_else(|| "Nearby device".into());
            if name.len() > 256 || name.chars().any(char::is_control) {
                return Ok(());
            }
            let id = match existing {
                Some(id) => id,
                None => {
                    if self.routes.len() >= self.coordinator.limits.endpoints {
                        return Ok(());
                    }
                    self.endpoint_sequence = self
                        .endpoint_sequence
                        .checked_add(1)
                        .ok_or(Error::Exhausted)?;
                    format!(
                        "ep-{:032x}-{:016x}",
                        self.coordinator.epoch, self.endpoint_sequence
                    )
                }
            };
            self.routes.insert(
                id,
                Route {
                    fullname: info.fullname,
                    name,
                    addr,
                },
            );
        }
        self.coordinator
            .set_endpoints(self.routes.keys().cloned().collect())?;
        Ok(())
    }

    fn observed(&mut self, event: SessionEvent) -> Result<(), RuntimeError> {
        let snapshot = match event {
            SessionEvent::Opened(control) => {
                let snapshot = control.snapshot();
                if self.bindings.contains_key(&snapshot.id) {
                    return Ok(());
                }
                // Outbound bindings are installed atomically after successful queue admission.
                if snapshot.direction != crate::channel::TransferType::Inbound {
                    control.cancel();
                    return self.fault();
                }
                let transfer = match self.coordinator.register_inbound() {
                    Ok(id) => id,
                    Err(_) => {
                        control.cancel();
                        return self.fault();
                    }
                };
                self.bindings.insert(
                    snapshot.id,
                    Binding {
                        transfer,
                        control,
                        consent: None,
                    },
                );
                // Open carries a live capability; later ordered Changed events carry historical state.
                return Ok(());
            }
            SessionEvent::Changed(snapshot) => snapshot,
            SessionEvent::Closed(snapshot) => {
                let Some(binding) = self.bindings.remove(&snapshot.id) else {
                    return if self.coordinator.synchronized {
                        self.fault()
                    } else {
                        Ok(())
                    };
                };
                self.deferred.remove(&binding.transfer);
                if !crate::sessions::terminal(&snapshot.state) {
                    return self.fault();
                }
                self.coordinator.observe(
                    binding.transfer,
                    phase(&snapshot.state, &snapshot.direction),
                )?;
                return Ok(());
            }
        };
        let Some(binding) = self.bindings.get_mut(&snapshot.id) else {
            return if self.coordinator.synchronized {
                self.fault()
            } else {
                Ok(())
            };
        };
        self.deferred.remove(&binding.transfer);
        binding.consent = snapshot.consent;
        let observed = self.coordinator.observe(
            binding.transfer,
            phase(&snapshot.state, &snapshot.direction),
        );
        if observed.is_err() {
            return self.fault();
        }
        if snapshot.direction == crate::channel::TransferType::Inbound
            && snapshot.state == State::WaitingForUserConsent
            && snapshot.consent.is_none()
        {
            return self.fault();
        }
        Ok(())
    }

    fn drain_events(&mut self) -> Result<bool, RuntimeError> {
        let mut progressed = false;
        for _ in 0..crate::sessions::EVENT_LIMIT {
            match self.events.try_recv() {
                Ok(event) => {
                    self.observed(event)?;
                    progressed = true;
                }
                Err(broadcast::error::TryRecvError::Empty) => return Ok(progressed),
                Err(_) => return self.fault().map(|_| false),
            }
        }
        Ok(progressed)
    }

    fn drain_discovery(&mut self) -> Result<bool, RuntimeError> {
        let mut progressed = false;
        for _ in 0..DISCOVERY_EVENTS {
            match self.discovery.try_recv() {
                Ok(info) => {
                    self.discovered(info)?;
                    progressed = true;
                }
                Err(broadcast::error::TryRecvError::Empty) => return Ok(progressed),
                Err(_) => return self.fault().map(|_| false),
            }
        }
        Ok(progressed)
    }

    fn dispatch(&mut self) -> Result<bool, RuntimeError> {
        if !self.coordinator.synchronized
            || self.coordinator.lifecycle != Lifecycle::Running
            || self.releases.len() >= OWNED_DISPATCHES
        {
            return Ok(false);
        }
        if self.pending.is_none() {
            let Some(dispatch) = self.coordinator.next_send()? else {
                return Ok(false);
            };
            let Some(route) = self.routes.get(&dispatch.endpoint) else {
                self.coordinator.observe(dispatch.transfer, Phase::Failed)?;
                return Ok(true);
            };
            self.pending = Some(Pending {
                transfer: dispatch.transfer,
                endpoint: dispatch.endpoint,
                request: SendInfo {
                    id: format!(
                        "send-{:032x}-{:016x}",
                        dispatch.transfer.0, dispatch.transfer.1
                    ),
                    name: route.name.clone(),
                    addr: route.addr.to_string(),
                    ob: OutboundPayload::OpenedFiles(dispatch.selection),
                },
            });
        }
        let mut pending = self.pending.take().ok_or(Error::UnknownTransfer)?;
        let Some(route) = self.routes.get(&pending.endpoint) else {
            self.coordinator.observe(pending.transfer, Phase::Failed)?;
            return Ok(true);
        };
        pending.request.addr = route.addr.to_string();
        pending.request.name = route.name.clone();
        match self.sender.try_send(pending.request) {
            Ok(control) => {
                let session = control.session();
                self.bindings.insert(
                    session.snapshot().id,
                    Binding {
                        transfer: pending.transfer,
                        control: session,
                        consent: None,
                    },
                );
                self.releases
                    .push(Box::pin(async move { control.wait_released().await }));
                Ok(true)
            }
            Err(rejected) => {
                self.pending = Some(Pending {
                    transfer: pending.transfer,
                    endpoint: pending.endpoint,
                    request: *rejected.request,
                });
                match rejected.kind {
                    RejectionKind::Full => Ok(false),
                    _ => self.fault().map(|_| false),
                }
            }
        }
    }

    fn controls(&mut self) -> Result<bool, RuntimeError> {
        let intents: Vec<_> = self
            .coordinator
            .transfers
            .values()
            .filter_map(|transfer| transfer.view.pending_control)
            .collect();
        let mut progressed = false;
        for intent in intents {
            if intent.action != ControlAction::Cancel
                && (!self.coordinator.synchronized
                    || self.inflight.contains_key(&intent.transfer)
                    || self.deferred.get(&intent.transfer) == Some(&intent.revision))
            {
                continue;
            }
            if intent.action == ControlAction::Cancel {
                if self
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.transfer == intent.transfer)
                {
                    self.pending.take();
                    self.coordinator
                        .observe(intent.transfer, Phase::Cancelled)?;
                } else if let Some(binding) = self
                    .bindings
                    .values()
                    .find(|binding| binding.transfer == intent.transfer)
                {
                    binding.control.cancel();
                    self.coordinator.acknowledge_control(intent)?;
                } else {
                    return self.fault().map(|_| false);
                }
                progressed = true;
                continue;
            }
            let Some(binding) = self
                .bindings
                .values()
                .find(|binding| binding.transfer == intent.transfer)
            else {
                return self.fault().map(|_| false);
            };
            let Some(token) = binding.consent else {
                return self.fault().map(|_| false);
            };
            match binding.control.respond(
                token,
                intent.revision,
                intent.action == ControlAction::Accept,
            ) {
                Ok(receipt) => {
                    self.inflight.insert(intent.transfer, intent.revision);
                    self.receipts
                        .push(Box::pin(async move { (intent, receipt.wait().await) }));
                    progressed = true;
                }
                Err(SessionError::Stale | SessionError::Cancelled | SessionError::Closed) => {
                    self.deferred.insert(intent.transfer, intent.revision);
                }
                Err(_) => return self.fault().map(|_| false),
            }
        }
        Ok(progressed)
    }

    fn receipt(
        &mut self,
        intent: ControlIntent,
        result: Result<(), SessionError>,
    ) -> Result<(), RuntimeError> {
        self.inflight.remove(&intent.transfer);
        // A superseded or already terminal intent cannot clear newer state.
        if self
            .coordinator
            .transfers
            .get(&intent.transfer)
            .and_then(|transfer| transfer.view.pending_control)
            != Some(intent)
        {
            return Ok(());
        }
        match result {
            Ok(()) => self
                .coordinator
                .acknowledge_control(intent)
                .map_err(Into::into),
            Err(SessionError::Stale | SessionError::Cancelled | SessionError::Closed) => {
                self.deferred.insert(intent.transfer, intent.revision);
                Ok(())
            }
            Err(_) => self.fault(),
        }
    }

    /// Process work or wait for one condition. The caller multiplexes IPC with this future.
    /// Cancellation of this wait does not detach tasks or acknowledge pending commands.
    pub async fn step(&mut self) -> Result<(), RuntimeError> {
        if !self.started || self.coordinator.lifecycle == Lifecycle::Stopped {
            return Err(Error::NotRunning.into());
        }
        let events = self.drain_events()?;
        let discovery = self.drain_discovery()?;
        let mut completions = false;
        for _ in 0..OWNED_DISPATCHES {
            match self.receipts.next().now_or_never().flatten() {
                Some((intent, result)) => {
                    self.receipt(intent, result)?;
                    completions = true;
                }
                None => break,
            }
        }
        for _ in 0..OWNED_DISPATCHES {
            match self.releases.next().now_or_never().flatten() {
                Some(Ok(())) => completions = true,
                Some(Err(_)) => return self.fault(),
                None => break,
            }
        }
        let controls = self.controls()?;
        let dispatch = self.dispatch()?;
        if events || discovery || completions || controls || dispatch {
            return Ok(());
        }
        tokio::select! {
            result = self.events.recv() => match result { Ok(event) => self.observed(event), Err(_) => self.fault() },
            result = self.discovery.recv() => match result { Ok(info) => self.discovered(info), Err(_) => self.fault() },
            Some((intent, result)) = self.receipts.next(), if !self.receipts.is_empty() => self.receipt(intent, result),
            Some(result) = self.releases.next(), if !self.releases.is_empty() => if result.is_err() { self.fault() } else { Ok(()) },
            _ = self.sender.closed(), if !self.core_lost => {
                self.core_lost = true;
                self.coordinator.mark_desynchronized()?;
                Err(RuntimeError::CoreUnavailable)
            },
            result = self.sender.ready(), if self.pending.is_some() && self.coordinator.synchronized && self.releases.len() < OWNED_DISPATCHES => {
                match result { Ok(()) => Ok(()), Err(_) => self.fault() }
            },
        }
    }

    /// Always joins the core, including after a lost observation. Failure never means delivery.
    pub async fn shutdown(&mut self) -> Result<(), RuntimeError> {
        self.startup.take();
        // Close native admission before adopting buffered opens. No new session can
        // appear after this cutoff, even while transport shutdown yields.
        self.sender.sessions().close();
        let mut failed = self.drain_events().is_err();
        let begin = self.coordinator.begin_shutdown();
        failed |= begin.is_err();
        if begin.is_ok() {
            failed |= self.controls().is_err();
        }
        if let Some(pending) = self.pending.take() {
            failed |= self
                .coordinator
                .observe(pending.transfer, Phase::Cancelled)
                .is_err();
        }
        for binding in self.bindings.values() {
            binding.control.cancel();
        }
        self.core.stop().await;
        self.started = false;
        failed |= self.drain_events().is_err();
        while let Some((intent, result)) = self.receipts.next().await {
            failed |= self.receipt(intent, result).is_err();
        }
        while let Some(result) = self.releases.next().await {
            if result.is_err() {
                self.coordinator.mark_desynchronized()?;
                failed = true;
            }
        }
        if failed || !self.coordinator.synchronized {
            return Err(RuntimeError::Unsynchronized);
        }
        self.coordinator.finish_shutdown()?;
        Ok(())
    }
}

impl Drop for NativeRuntime {
    fn drop(&mut self) {
        // Explicit shutdown is required to await the barrier; Drop only requests cancellation.
        self.core.stop_discovery();
        self.sender.sessions().close();
        if let Some(lifetime) = &self.core.ctoken {
            lifetime.cancel();
        }
        for binding in self.bindings.values() {
            binding.control.cancel();
        }
    }
}

fn phase(state: &State, direction: &crate::channel::TransferType) -> Phase {
    match state {
        State::WaitingForUserConsent if *direction == crate::channel::TransferType::Inbound => {
            Phase::AwaitingConsent
        }
        State::ReceivingFiles => Phase::Receiving,
        State::SendingFiles => Phase::Sending,
        State::Finished => Phase::Finished,
        State::Rejected => Phase::Rejected,
        State::Cancelled => Phase::Cancelled,
        State::Disconnected => Phase::Disconnected,
        _ => Phase::Active,
    }
}

#[cfg(test)]
mod tests;
