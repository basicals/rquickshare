//! Immutable, connection-owned read projections. No payload ownership or history eviction.

use std::collections::BTreeMap;
use std::mem::size_of;
use std::sync::Arc;

use super::{token, ClientId, Coordinator, Error};
use crate::service::{ControlAction, Direction, Lifecycle, Phase};
use crate::sessions::details::{DetailsStatus, DeviceKind, TransferDetails};

pub(super) const PAGE: usize = 16;
const ENDPOINTS: usize = 128;
const SELECTIONS: usize = 128;
const TRANSFERS: usize = 256;
const BASE_BYTES: usize = 512 * 1024;
const CAPTURE_BYTES: usize = 20 * 1024 * 1024;
const CAPTURES: usize = 16;
const CACHE_BYTES: usize = 40 * 1024 * 1024;

pub(super) type Metadata = (
    String,
    String,
    u64,
    &'static str,
    bool,
    u32,
    u32,
    u64,
    u32,
    u32,
    u32,
);
pub(super) type EndpointRow = (String, String);
pub(super) type TransferRow = (
    String,
    &'static str,
    &'static str,
    bool,
    u64,
    bool,
    &'static str,
    u64,
);

pub(super) type DetailRow = (
    String,
    &'static str,
    String,
    &'static str,
    String,
    u64,
    u64,
    u32,
);
pub(super) type FileRow = (String, u64);
pub(super) type NoticeRow = (String, String, u64, u64);

struct Capture {
    text: Vec<super::text::MetadataRow>,
    notices: Vec<NoticeRow>,
    charge: usize,
    details: Vec<(String, Arc<TransferDetails>)>,
    metadata: Metadata,
    endpoints: Vec<EndpointRow>,
    selections: Vec<String>,
    transfers: Vec<TransferRow>,
}

#[derive(Default)]
pub(super) struct StateCache {
    captures: BTreeMap<ClientId, Capture>,
    sequence: u64,
}

fn lifecycle(value: Lifecycle) -> &'static str {
    match value {
        Lifecycle::Running => "running",
        Lifecycle::Draining => "draining",
        Lifecycle::Stopped => "stopped",
    }
}
fn direction(value: Direction) -> &'static str {
    match value {
        Direction::Inbound => "inbound",
        Direction::Outbound => "outbound",
    }
}
fn phase(value: Phase) -> &'static str {
    match value {
        Phase::Queued => "queued",
        Phase::Active => "active",
        Phase::AwaitingConsent => "awaiting_consent",
        Phase::Receiving => "receiving",
        Phase::Sending => "sending",
        Phase::CancelRequested => "cancel_requested",
        Phase::Finished => "finished",
        Phase::Rejected => "rejected",
        Phase::Cancelled => "cancelled",
        Phase::Failed => "failed",
        Phase::Disconnected => "disconnected",
    }
}
fn action(value: ControlAction) -> &'static str {
    match value {
        ControlAction::Accept => "accept",
        ControlAction::Reject => "reject",
        ControlAction::Cancel => "cancel",
    }
}

impl StateCache {
    pub(super) fn capture<'a>(
        &mut self,
        coordinator: &mut Coordinator,
        client: ClientId,
        endpoints: impl ExactSizeIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Metadata, Error> {
        coordinator.client(client)?;
        if (!self.captures.contains_key(&client) && self.captures.len() >= CAPTURES)
            || endpoints.len() > ENDPOINTS
            || coordinator
                .selections
                .values()
                .filter(|entry| entry.owner == client)
                .count()
                > SELECTIONS
            || coordinator.transfers.len() > TRANSFERS
        {
            return Err(Error::Capacity);
        }
        // Charge immutable referenced data, including old captures, before projection.
        let charge = coordinator
            .transfers
            .values()
            .try_fold(BASE_BYTES, |bytes, transfer| {
                bytes
                    .checked_add(transfer.view.details.retained_bytes())
                    .ok_or(Error::Capacity)
            })?;
        let retained = self
            .captures
            .iter()
            .filter(|(owner, _)| **owner != client)
            .try_fold(0usize, |bytes, (_, capture)| {
                bytes.checked_add(capture.charge).ok_or(Error::Capacity)
            })?;
        if charge > CAPTURE_BYTES
            || retained
                .checked_add(charge)
                .is_none_or(|bytes| bytes > CACHE_BYTES)
        {
            return Err(Error::Capacity);
        }
        let sequence = self.sequence.checked_add(1).ok_or(Error::Exhausted)?;
        let resources = coordinator.usage();
        u32::try_from(resources.selections).map_err(|_| Error::Capacity)?;
        u32::try_from(resources.reserved_handles).map_err(|_| Error::Capacity)?;
        let mut endpoint_rows = Vec::with_capacity(endpoints.len());
        for (id, name) in endpoints {
            crate::service::identifier(id, 256)?;
            if name.len() > 256 || name.chars().any(char::is_control) {
                return Err(Error::InvalidInput);
            }
            endpoint_rows.push((id.to_owned(), name.to_owned()));
        }
        endpoint_rows.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        if endpoint_rows.windows(2).any(|rows| rows[0].0 == rows[1].0) {
            return Err(Error::InvalidInput);
        }
        // The count guards precede the coordinator's bounded allocations.
        let snapshot = coordinator.snapshot(client)?;
        let selected: Vec<_> = snapshot
            .selections
            .into_iter()
            .map(|id| token(id.0, id.1))
            .collect();
        let details = snapshot
            .transfers
            .iter()
            .map(|view| (token(view.id.0, view.id.1), view.details.clone()))
            .collect();
        let text = snapshot
            .transfers
            .iter()
            .map(|view| {
                let resource = &coordinator.transfers[&view.id];
                (
                    token(view.id.0, view.id.1),
                    view.details.kind.wire(),
                    resource.received_text.is_some(),
                    if resource.received_text.is_some() {
                        resource.terminal_revision.unwrap_or(0)
                    } else {
                        0
                    },
                )
            })
            .collect();
        let transfers: Vec<_> = snapshot
            .transfers
            .into_iter()
            .map(|view| {
                let consent = view.consent_revision;
                let pending = view.pending_control;
                (
                    token(view.id.0, view.id.1),
                    direction(view.direction),
                    phase(view.phase),
                    consent.is_some(),
                    consent.unwrap_or(0),
                    pending.is_some(),
                    pending.map_or("", |intent| action(intent.action)),
                    pending.map_or(0, |intent| intent.revision),
                )
            })
            .collect();
        let metadata = (
            token(snapshot.epoch, sequence),
            format!("{:032x}", snapshot.epoch),
            snapshot.revision,
            lifecycle(snapshot.lifecycle),
            snapshot.synchronized,
            u32::try_from(snapshot.resources.selections).map_err(|_| Error::Capacity)?,
            u32::try_from(snapshot.resources.reserved_handles).map_err(|_| Error::Capacity)?,
            snapshot.resources.selected_bytes,
            endpoint_rows.len() as u32,
            selected.len() as u32,
            transfers.len() as u32,
        );
        let notices = snapshot
            .notices
            .into_iter()
            .map(|notice| {
                (
                    token(notice.transfer.0, notice.transfer.1),
                    notice.kind.wire().to_owned(),
                    notice.round,
                    notice.revision,
                )
            })
            .collect();
        let capture = Capture {
            text,
            notices,
            charge,
            details,
            metadata,
            endpoints: endpoint_rows,
            selections: selected,
            transfers,
        };
        // Fixed reservation includes record storage, owned buffers and conservative map overhead.
        let bytes = size_of::<Capture>()
            + 1024
            + capture.text.capacity() * size_of::<super::text::MetadataRow>()
            + capture
                .text
                .iter()
                .map(|row| row.0.capacity())
                .sum::<usize>()
            + capture.notices.capacity() * size_of::<NoticeRow>()
            + capture
                .notices
                .iter()
                .map(|row| row.0.capacity() + row.1.capacity())
                .sum::<usize>()
            + capture.metadata.0.capacity()
            + capture.metadata.1.capacity()
            + capture.endpoints.capacity() * size_of::<EndpointRow>()
            + capture.selections.capacity() * size_of::<String>()
            + capture.transfers.capacity() * size_of::<TransferRow>()
            + capture
                .endpoints
                .iter()
                .map(|(id, name)| id.capacity() + name.capacity())
                .sum::<usize>()
            + capture
                .selections
                .iter()
                .map(String::capacity)
                .sum::<usize>()
            + capture
                .transfers
                .iter()
                .map(|row| row.0.capacity())
                .sum::<usize>();
        if bytes > BASE_BYTES {
            return Err(Error::Capacity);
        }
        let result = capture.metadata.clone();
        self.sequence = sequence;
        self.captures.insert(client, capture);
        Ok(result)
    }

    fn get(&self, client: ClientId, id: &str) -> Result<&Capture, Error> {
        let capture = self
            .captures
            .iter()
            .find(|(_, capture)| capture.metadata.0 == id)
            .ok_or(Error::UnknownSnapshot)?;
        if *capture.0 != client {
            return Err(Error::WrongOwner);
        }
        Ok(capture.1)
    }
    pub(super) fn text_page(
        &self,
        client: ClientId,
        id: &str,
        offset: u32,
    ) -> Result<(u32, Vec<super::text::MetadataRow>), Error> {
        page(&self.get(client, id)?.text, offset)
    }
    pub(super) fn notice_count(&self, client: ClientId, id: &str) -> Result<u32, Error> {
        Ok(self.get(client, id)?.notices.len() as u32)
    }
    pub(super) fn notice_page(
        &self,
        client: ClientId,
        id: &str,
        offset: u32,
    ) -> Result<(u32, Vec<NoticeRow>), Error> {
        page(&self.get(client, id)?.notices, offset)
    }
    pub(super) fn endpoint_page(
        &self,
        client: ClientId,
        id: &str,
        offset: u32,
    ) -> Result<(u32, Vec<EndpointRow>), Error> {
        page(&self.get(client, id)?.endpoints, offset)
    }
    pub(super) fn selection_page(
        &self,
        client: ClientId,
        id: &str,
        offset: u32,
    ) -> Result<(u32, Vec<String>), Error> {
        page(&self.get(client, id)?.selections, offset)
    }
    pub(super) fn transfer_page(
        &self,
        client: ClientId,
        id: &str,
        offset: u32,
    ) -> Result<(u32, Vec<TransferRow>), Error> {
        page(&self.get(client, id)?.transfers, offset)
    }
    pub(super) fn detail_page(
        &self,
        client: ClientId,
        id: &str,
        offset: u32,
    ) -> Result<(u32, Vec<DetailRow>), Error> {
        let (next, rows) = page(&self.get(client, id)?.details, offset)?;
        Ok((
            next,
            rows.into_iter()
                .map(|(id, details)| {
                    (
                        id,
                        match details.status {
                            DetailsStatus::Unavailable => "unavailable",
                            DetailsStatus::Ready => "ready",
                            DetailsStatus::Unsupported => "unsupported",
                        },
                        details.sender.clone(),
                        match details.device {
                            DeviceKind::Unknown => "unknown",
                            DeviceKind::Phone => "phone",
                            DeviceKind::Tablet => "tablet",
                            DeviceKind::Laptop => "laptop",
                        },
                        details.pin.clone(),
                        details.total_bytes,
                        details.transferred_bytes,
                        details.files.len() as u32,
                    )
                })
                .collect(),
        ))
    }
    pub(super) fn file_page(
        &self,
        client: ClientId,
        id: &str,
        transfer: &str,
        offset: u32,
    ) -> Result<(u32, Vec<FileRow>), Error> {
        let capture = self.get(client, id)?;
        let details = &capture
            .details
            .iter()
            .find(|(id, _)| id == transfer)
            .ok_or(Error::UnknownTransfer)?
            .1;
        let (next, rows) = page(&details.files, offset)?;
        Ok((
            next,
            rows.into_iter()
                .map(|file| (file.name, file.size))
                .collect(),
        ))
    }
    pub(super) fn end(&mut self, client: ClientId, id: &str) -> Result<(), Error> {
        match self.get(client, id) {
            Ok(_) => {
                self.captures.remove(&client);
                Ok(())
            }
            Err(Error::UnknownSnapshot) => Ok(()),
            Err(error) => Err(error),
        }
    }
    pub(super) fn disconnect(&mut self, client: ClientId) {
        self.captures.remove(&client);
    }
    pub(super) fn clear(&mut self) {
        self.captures.clear();
    }
}

fn page<T: Clone>(rows: &[T], offset: u32) -> Result<(u32, Vec<T>), Error> {
    let start = offset as usize;
    if start > rows.len() {
        return Err(Error::InvalidInput);
    }
    let end = (start + PAGE).min(rows.len());
    Ok((end as u32, rows[start..end].to_vec()))
}

#[cfg(test)]
mod tests;
