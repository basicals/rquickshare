//! Checked immutable complete client projections of Manager2 captures.

use std::sync::Arc;

use serde::de::DeserializeOwned;

use super::*;
use crate::service::{ControlAction, Direction, Lifecycle, Phase, ResourceUsage};
pub use crate::sessions::details::{DetailsStatus, DeviceKind, FileDetail, TransferDetails};

pub(super) type Metadata = (
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
pub(super) type DetailRow = (String, String, String, String, String, u64, u64, u32);

pub(super) type TransferRow = (String, String, String, bool, u64, bool, String, u64);

#[derive(Clone, Eq, PartialEq)]
pub struct Endpoint {
    pub id: String,
    pub name: String,
}
impl fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Endpoint").finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferView {
    pub content_revision: Option<u64>,
    pub transfer: Transfer,
    pub direction: Direction,
    pub phase: Phase,
    pub consent_revision: Option<u64>,
    pub pending_control: Option<(ControlAction, u64)>,
    pub details: Arc<TransferDetails>,
}
impl TransferView {
    /// Presentation readiness only; the daemon revalidates the live round.
    pub fn can_accept(&self) -> bool {
        self.phase == Phase::AwaitingConsent
            && self.pending_control.is_none()
            && self.consent_revision.is_some()
            && self.details.status == DetailsStatus::Ready
            && self.details.valid(true)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupWarning {
    VisibilityRecovered,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateView {
    pub epoch: u128,
    pub revision: u64,
    pub lifecycle: Lifecycle,
    pub synchronized: bool,
    pub resources: ResourceUsage,
    pub endpoints: Vec<Endpoint>,
    pub selections: Vec<Selection>,
    pub transfers: Vec<TransferView>,
    pub warnings: Vec<StartupWarning>,
    pub notices: Vec<super::notices::Notice>,
}

fn lifecycle(value: &str) -> Result<Lifecycle> {
    match value {
        "running" => Ok(Lifecycle::Running),
        "draining" => Ok(Lifecycle::Draining),
        "stopped" => Ok(Lifecycle::Stopped),
        _ => Err(ClientError::Protocol),
    }
}
fn direction(value: &str) -> Result<Direction> {
    match value {
        "inbound" => Ok(Direction::Inbound),
        "outbound" => Ok(Direction::Outbound),
        _ => Err(ClientError::Protocol),
    }
}
fn phase(value: &str) -> Result<Phase> {
    match value {
        "queued" => Ok(Phase::Queued),
        "active" => Ok(Phase::Active),
        "awaiting_consent" => Ok(Phase::AwaitingConsent),
        "receiving" => Ok(Phase::Receiving),
        "sending" => Ok(Phase::Sending),
        "cancel_requested" => Ok(Phase::CancelRequested),
        "finished" => Ok(Phase::Finished),
        "rejected" => Ok(Phase::Rejected),
        "cancelled" => Ok(Phase::Cancelled),
        "failed" => Ok(Phase::Failed),
        "disconnected" => Ok(Phase::Disconnected),
        _ => Err(ClientError::Protocol),
    }
}
fn action(value: &str) -> Result<ControlAction> {
    match value {
        "accept" => Ok(ControlAction::Accept),
        "reject" => Ok(ControlAction::Reject),
        "cancel" => Ok(ControlAction::Cancel),
        _ => Err(ClientError::Protocol),
    }
}
fn optional_revision(present: bool, value: u64, revision: u64) -> Result<Option<u64>> {
    match (present, value) {
        (false, 0) => Ok(None),
        (true, value) if value > 0 && value <= revision => Ok(Some(value)),
        _ => Err(ClientError::Protocol),
    }
}
fn sorted<'a>(ids: impl IntoIterator<Item = &'a str>) -> Result<()> {
    let mut previous = None;
    for id in ids {
        if previous.is_some_and(|previous| previous >= id) {
            return Err(ClientError::Protocol);
        }
        previous = Some(id);
    }
    Ok(())
}

fn decode_details(
    row: DetailRow,
    kind: crate::sessions::details::PayloadKind,
) -> Result<(TransferDetails, u32)> {
    let status = match row.1.as_str() {
        "unavailable" => DetailsStatus::Unavailable,
        "ready" => DetailsStatus::Ready,
        "unsupported" => DetailsStatus::Unsupported,
        _ => return Err(ClientError::Protocol),
    };
    let device = match row.3.as_str() {
        "unknown" => DeviceKind::Unknown,
        "phone" => DeviceKind::Phone,
        "tablet" => DeviceKind::Tablet,
        "laptop" => DeviceKind::Laptop,
        _ => return Err(ClientError::Protocol),
    };
    if row.7 > 256
        || row.2.len() > 256
        || row.2.chars().any(char::is_control)
        || row.4.len() > 4
        || !row.4.bytes().all(|b| b.is_ascii_digit())
        || row.6 > row.5
        || (status == DetailsStatus::Ready
            && (row.2.is_empty()
                || (kind == crate::sessions::details::PayloadKind::Files && row.7 == 0)))
        || (status != DetailsStatus::Ready
            && (!row.2.is_empty()
                || device != DeviceKind::Unknown
                || !row.4.is_empty()
                || row.5 != 0
                || row.6 != 0
                || row.7 != 0))
    {
        return Err(ClientError::Protocol);
    }
    Ok((
        TransferDetails {
            kind,
            status,
            sender: row.2,
            device,
            pin: row.4,
            total_bytes: row.5,
            transferred_bytes: row.6,
            files: Arc::default(),
        },
        row.7,
    ))
}

impl NativeClient {
    fn empty_view(&self, metadata: &Metadata) -> Result<StateView> {
        let current = epoch(&metadata.1)?;
        if current != self.epoch
            || metadata.8 > 128
            || metadata.9 > 128
            || metadata.10 > 256
            || metadata.2 == 0
        {
            return Err(ClientError::Protocol);
        }
        canonical(&metadata.0, current)?;
        Ok(StateView {
            epoch: current,
            revision: metadata.2,
            lifecycle: lifecycle(&metadata.3)?,
            synchronized: metadata.4,
            resources: ResourceUsage {
                selections: metadata.5 as usize,
                reserved_handles: metadata.6 as usize,
                selected_bytes: metadata.7,
            },
            endpoints: Vec::with_capacity(metadata.8 as usize),
            selections: Vec::with_capacity(metadata.9 as usize),
            transfers: Vec::with_capacity(metadata.10 as usize),
            warnings: Vec::new(),
            notices: Vec::new(),
        })
    }
    fn transfer_view(&self, row: TransferRow, revision: u64) -> Result<TransferView> {
        canonical(&row.0, self.epoch)?;
        let direction = direction(&row.1)?;
        let phase = phase(&row.2)?;
        let consent_revision = optional_revision(row.3, row.4, revision)?;
        let pending_revision = optional_revision(row.5, row.7, revision)?;
        let pending_control = if let Some(revision) = pending_revision {
            Some((action(&row.6)?, revision))
        } else {
            if !row.6.is_empty() {
                return Err(ClientError::Protocol);
            }
            None
        };
        if (phase == Phase::Queued || phase == Phase::Sending) && direction != Direction::Outbound
            || matches!(phase, Phase::AwaitingConsent | Phase::Receiving)
                && direction != Direction::Inbound
            || (phase == Phase::AwaitingConsent) != consent_revision.is_some()
        {
            return Err(ClientError::Protocol);
        }
        if let Some((action, pending_revision)) = pending_control {
            if (action != ControlAction::Cancel
                && consent_revision.is_some_and(|consent| pending_revision <= consent))
                || (action == ControlAction::Cancel && phase != Phase::CancelRequested)
                || (action != ControlAction::Cancel
                    && (direction != Direction::Inbound || phase != Phase::AwaitingConsent))
            {
                return Err(ClientError::Protocol);
            }
        }
        Ok(TransferView {
            content_revision: None,
            transfer: Transfer {
                id: row.0,
                connection: self.identity.clone(),
            },
            direction,
            phase,
            consent_revision,
            pending_control,
            details: Arc::default(),
        })
    }
    async fn pages<T: DeserializeOwned + Type>(
        &mut self,
        method: &'static str,
        id: &str,
        count: u32,
    ) -> Result<Vec<T>> {
        self.pages_for(method, id, count, None).await
    }
    async fn pages_for<T: DeserializeOwned + Type>(
        &mut self,
        method: &'static str,
        id: &str,
        count: u32,
        transfer: Option<&str>,
    ) -> Result<Vec<T>> {
        self.pages_for_on(INTERFACE, method, id, count, transfer)
            .await
    }
    pub(super) async fn pages_for_on<T: DeserializeOwned + Type>(
        &mut self,
        interface: &'static str,
        method: &'static str,
        id: &str,
        count: u32,
        transfer: Option<&str>,
    ) -> Result<Vec<T>> {
        let mut rows = Vec::with_capacity(count as usize);
        let mut offset: u32 = 0;
        loop {
            let reply = if let Some(transfer) = transfer {
                self.wire(interface, method, &(id, transfer, offset))
                    .await?
            } else {
                self.wire(interface, method, &(id, offset)).await?
            };
            let (next, page): (u32, Vec<T>) = reply
                .body()
                .deserialize()
                .map_err(|_| ClientError::Protocol)?;
            if page.len() > 16
                || next
                    != offset
                        .checked_add(page.len() as u32)
                        .ok_or(ClientError::Protocol)?
                || next > count
            {
                return Err(ClientError::Protocol);
            }
            if page.is_empty() {
                if offset != count {
                    return Err(ClientError::Protocol);
                }
                break;
            }
            rows.extend(page);
            offset = next;
        }
        Ok(rows)
    }
    pub async fn read_state(&mut self) -> Result<StateView> {
        self.view_incomplete = true;
        let metadata: Metadata = self
            .wire(INTERFACE, "BeginSnapshot", &())
            .await?
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)?;
        let mut view = self.empty_view(&metadata)?;
        self.capture = Some(metadata.0.clone());
        let endpoints: Vec<(String, String)> = self
            .pages("GetEndpointPage", &metadata.0, metadata.8)
            .await?;
        for (id, name) in endpoints {
            if id.is_empty()
                || id.len() > 256
                || !id.bytes().all(|b| b.is_ascii_graphic())
                || name.len() > 256
                || name.chars().any(char::is_control)
            {
                return Err(ClientError::Protocol);
            }
            view.endpoints.push(Endpoint { id, name });
        }
        sorted(view.endpoints.iter().map(|v| v.id.as_str()))?;
        let selections: Vec<String> = self
            .pages("GetSelectionPage", &metadata.0, metadata.9)
            .await?;
        for id in selections {
            canonical(&id, self.epoch)?;
            view.selections.push(Selection {
                id,
                connection: self.identity.clone(),
            });
        }
        sorted(view.selections.iter().map(|v| v.id()))?;
        let transfers: Vec<TransferRow> = self
            .pages("GetTransferPage", &metadata.0, metadata.10)
            .await?;
        for row in transfers {
            view.transfers.push(self.transfer_view(row, view.revision)?);
        }
        sorted(view.transfers.iter().map(|v| v.transfer.id()))?;
        let headers: Vec<DetailRow> = self
            .pages("GetTransferDetailsPage", &metadata.0, metadata.10)
            .await?;
        let text_rows: Vec<super::text::MetadataRow> = self
            .pages_for_on(
                bus::text::INTERFACE,
                "GetMetadataPage",
                &metadata.0,
                metadata.10,
                None,
            )
            .await?;
        for ((transfer, header), text) in view.transfers.iter_mut().zip(headers).zip(text_rows) {
            if transfer.transfer.id() != header.0 || header.7 > 256 {
                return Err(ClientError::Protocol);
            }
            if text.0 != transfer.transfer.id() {
                return Err(ClientError::Protocol);
            }
            let mut details = decode_details(header, super::text::kind(&text.1)?)?;
            transfer.content_revision = optional_revision(text.2, text.3, view.revision)?;
            if text.2
                && (transfer.direction != Direction::Inbound
                    || transfer.phase != Phase::Finished
                    || details.0.kind == crate::sessions::details::PayloadKind::Files
                    || details.0.status != DetailsStatus::Ready
                    || details.0.transferred_bytes != details.0.total_bytes)
            {
                return Err(ClientError::Protocol);
            }
            let rows: Vec<(String, u64)> = self
                .pages_for(
                    "GetFilePage",
                    &metadata.0,
                    details.1,
                    Some(transfer.transfer.id()),
                )
                .await?;
            details.0.files = rows
                .into_iter()
                .map(|(name, size)| FileDetail { name, size })
                .collect::<Vec<_>>()
                .into();
            if !details.0.valid(transfer.phase == Phase::AwaitingConsent)
                || details
                    .0
                    .files
                    .windows(2)
                    .any(|files| (&files[0].name, files[0].size) > (&files[1].name, files[1].size))
            {
                return Err(ClientError::Protocol);
            }
            transfer.details = Arc::new(details.0);
        }
        self.read_notices(&metadata.0, &mut view).await?;
        let warnings: Vec<String> = self
            .wire(INTERFACE, "GetStartupWarnings", &())
            .await?
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)?;
        if warnings.len() > 1 {
            return Err(ClientError::Protocol);
        }
        for warning in warnings {
            match warning.as_str() {
                "visibility_recovered" => view.warnings.push(StartupWarning::VisibilityRecovered),
                _ => return Err(ClientError::Protocol),
            }
        }
        self.wire(INTERFACE, "EndSnapshot", &metadata.0)
            .await?
            .body()
            .deserialize::<()>()
            .map_err(|_| ClientError::Protocol)?;
        self.capture = None;
        self.view_incomplete = false;
        self.last_view = Some(view.clone());
        Ok(view)
    }
    pub fn cached_state(&self) -> Option<&StateView> {
        self.last_view.as_ref()
    }
    pub fn state_is_stale(&self) -> bool {
        let Some(view) = self.last_view.as_ref() else {
            return true;
        };
        if self.view_incomplete {
            return true;
        }
        self.watcher.as_ref().is_none_or(|watcher| {
            let hint = watcher.receiver.borrow();
            hint.failure.is_some()
                || hint.epoch.is_some_and(|epoch| epoch != view.epoch)
                || hint.revision > view.revision
        })
    }
    pub async fn next_change(&mut self) -> Result<u64> {
        loop {
            let watcher = self.watcher.as_mut().ok_or(ClientError::Closed)?;
            if let Some(error) = watcher.receiver.borrow().failure {
                return Err(error);
            }
            watcher
                .receiver
                .changed()
                .await
                .map_err(|_| ClientError::Transport)?;
            let hint = *watcher.receiver.borrow_and_update();
            if let Some(error) = hint.failure {
                return Err(error);
            }
            if hint.epoch.is_some_and(|epoch| epoch != self.epoch) {
                return Err(ClientError::DaemonChanged);
            }
            if hint.settings_generation > self.settings_seen {
                self.settings_seen = hint.settings_generation;
                return Ok(hint.revision);
            }
            if self
                .last_view
                .as_ref()
                .is_none_or(|view| hint.revision > view.revision)
            {
                return Ok(hint.revision);
            }
        }
    }
    pub async fn discard_selection(&mut self, selection: &Selection) -> Result<()> {
        if self.pending.is_some() {
            return Err(ClientError::PendingOperation);
        }
        self.owned(selection.id(), &selection.connection)?;
        self.wire(INTERFACE, "DiscardSelection", &selection.id())
            .await?
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)
    }
    pub async fn respond(
        &mut self,
        transfer: &Transfer,
        consent_revision: u64,
        accept: bool,
    ) -> Result<()> {
        canonical(transfer.id(), self.epoch).map_err(|_| ClientError::ForeignHandle)?;
        if consent_revision == 0 {
            return Err(ClientError::InvalidInput);
        }
        self.wire(
            INTERFACE,
            "Respond",
            &(transfer.id(), consent_revision, accept),
        )
        .await?
        .body()
        .deserialize()
        .map_err(|_| ClientError::Protocol)
    }
    pub async fn cancel(&mut self, transfer: &Transfer) -> Result<()> {
        canonical(transfer.id(), self.epoch).map_err(|_| ClientError::ForeignHandle)?;
        self.wire(INTERFACE, "Cancel", &transfer.id())
            .await?
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)
    }
}

#[cfg(test)]
mod tests;
