//! Bounded connection-owned preparation; successful commit never duplicates a selection.

use super::*;
use crate::file_source::{validate_files, OpenedFile, OpenedSelection, SelectionLimits};

pub const BATCH_FILES: usize = 16;
pub const PREPARATIONS_PER_CLIENT: usize = 4;
pub const RECEIPTS_PER_CLIENT: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UploadState {
    Preparing,
    Committed(SelectionId),
    Aborted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UploadSnapshot {
    pub next_sequence: u32,
    pub state: UploadState,
}

pub(super) struct Upload {
    pub(super) owner: ClientId,
    pub(super) request: Option<String>,
    next_sequence: u32,
    pub(super) state: UploadState,
    files: Vec<OpenedFile>,
    bytes: u64,
}

impl Coordinator {
    pub(super) fn upload_usage(&self) -> ResourceUsage {
        self.uploads
            .values()
            .filter(|upload| upload.state == UploadState::Preparing)
            .fold(ResourceUsage::default(), |mut usage, upload| {
                usage.selections += 1;
                usage.reserved_handles += upload.files.len() * 2;
                usage.selected_bytes += upload.bytes;
                usage
            })
    }

    fn owned_upload(&self, client: ClientId, id: UploadId) -> Result<&Upload, Error> {
        self.client(client)?;
        let upload = self.uploads.get(&id).ok_or(Error::UnknownUpload)?;
        if upload.owner != client {
            return Err(Error::WrongOwner);
        }
        Ok(upload)
    }

    pub fn begin_selection(&mut self, client: ClientId, request: &str) -> Result<UploadId, Error> {
        self.running()?;
        self.client(client)?;
        identifier(request, 128)?;
        self.begin_upload(client, Some(request))
    }

    pub(super) fn begin_upload(
        &mut self,
        client: ClientId,
        request: Option<&str>,
    ) -> Result<UploadId, Error> {
        self.running()?;
        self.client(client)?;
        let owned: Vec<_> = self
            .uploads
            .iter()
            .filter(|(_, upload)| upload.owner == client)
            .collect();
        if let Some(request) = request {
            if let Some((id, _)) = owned
                .iter()
                .find(|(_, upload)| upload.request.as_deref() == Some(request))
            {
                return Ok(**id);
            }
        }
        if (request.is_some()
            && owned
                .iter()
                .filter(|(_, upload)| upload.request.is_some())
                .count()
                >= RECEIPTS_PER_CLIENT)
            || owned
                .iter()
                .filter(|(_, upload)| upload.state == UploadState::Preparing)
                .count()
                >= PREPARATIONS_PER_CLIENT
        {
            return Err(Error::Capacity);
        }
        self.refresh_resources()?;
        reserve_usage(self.usage(), 0, 0, self.limits)?;
        let id = UploadId(self.epoch, self.advance()?);
        self.uploads.insert(
            id,
            Upload {
                owner: client,
                request: request.map(str::to_owned),
                next_sequence: 0,
                state: UploadState::Preparing,
                files: Vec::new(),
                bytes: 0,
            },
        );
        Ok(id)
    }

    /// Every rejection drops this batch and leaves the preceding preparation intact.
    pub fn append_files(
        &mut self,
        client: ClientId,
        id: UploadId,
        sequence: u32,
        files: Vec<OpenedFile>,
    ) -> Result<u32, Error> {
        self.running()?;
        let upload = self.owned_upload(client, id)?;
        if upload.state != UploadState::Preparing {
            return Err(Error::InvalidState);
        }
        if sequence != upload.next_sequence {
            return Err(Error::ReplayConflict);
        }
        if files.is_empty() || files.len() > BATCH_FILES {
            return Err(Error::InvalidInput);
        }
        let next = sequence.checked_add(1).ok_or(Error::Exhausted)?;
        let bytes = validate_files(
            upload.files.iter().chain(files.iter()),
            SelectionLimits::default(),
        )
        .map_err(|_| Error::InvalidInput)?;
        let added_bytes = bytes.checked_sub(upload.bytes).ok_or(Error::Capacity)?;
        self.refresh_resources()?;
        let usage = self.usage();
        if usage
            .reserved_handles
            .checked_add(files.len() * 2)
            .is_none_or(|n| n > self.limits.reserved_handles)
            || usage
                .selected_bytes
                .checked_add(added_bytes)
                .is_none_or(|n| n > self.limits.selected_bytes)
        {
            return Err(Error::Capacity);
        }
        self.advance()?;
        if let Some(upload) = self.uploads.get_mut(&id) {
            upload.files.extend(files);
            upload.bytes = bytes;
            upload.next_sequence = next;
        }
        Ok(next)
    }

    pub fn get_upload(&self, client: ClientId, id: UploadId) -> Result<UploadSnapshot, Error> {
        let upload = self.owned_upload(client, id)?;
        Ok(UploadSnapshot {
            next_sequence: upload.next_sequence,
            state: upload.state,
        })
    }

    pub fn commit_selection(
        &mut self,
        client: ClientId,
        id: UploadId,
    ) -> Result<SelectionId, Error> {
        self.running()?;
        self.check_upload_origin(client, id, false)?;
        self.commit_upload(client, id)
    }

    pub(super) fn check_upload_origin(
        &self,
        client: ClientId,
        id: UploadId,
        ordered: bool,
    ) -> Result<(), Error> {
        let upload = self.owned_upload(client, id)?;
        if upload.request.is_none() != ordered {
            return Err(Error::InvalidState);
        }
        Ok(())
    }

    pub(super) fn commit_upload(
        &mut self,
        client: ClientId,
        id: UploadId,
    ) -> Result<SelectionId, Error> {
        self.running()?;
        let upload = self.owned_upload(client, id)?;
        match upload.state {
            UploadState::Committed(selection) => return Ok(selection),
            UploadState::Aborted => return Err(Error::InvalidState),
            UploadState::Preparing => (),
        }
        if upload.files.is_empty() {
            return Err(Error::InvalidInput);
        }
        // Append already reserved both original handles and eventual worker clones.
        // Advance before taking ownership so exhaustion cannot consume a preparation.
        let selection = SelectionId(self.epoch, self.advance()?);
        if let Some(upload) = self.uploads.get_mut(&id) {
            let resource = Arc::new(OpenedSelection::from_validated_files(
                std::mem::take(&mut upload.files),
                upload.bytes,
            ));
            upload.state = UploadState::Committed(selection);
            self.reservations.insert(
                selection,
                Reservation {
                    resource: Arc::downgrade(&resource),
                    handles: resource.files().len() * 2,
                    bytes: resource.total_bytes(),
                },
            );
            self.selections.insert(
                selection,
                Selection {
                    owner: client,
                    resource,
                },
            );
        }
        Ok(selection)
    }

    pub fn abort_selection(&mut self, client: ClientId, id: UploadId) -> Result<(), Error> {
        self.client(client)?;
        if self.uploads.contains_key(&id) {
            self.check_upload_origin(client, id, false)?;
        }
        self.abort_upload(client, id)
    }

    pub(super) fn abort_upload(&mut self, client: ClientId, id: UploadId) -> Result<(), Error> {
        self.client(client)?;
        let Some(upload) = self.uploads.get(&id) else {
            return Ok(());
        };
        if upload.owner != client {
            return Err(Error::WrongOwner);
        }
        match upload.state {
            UploadState::Aborted => return Ok(()),
            UploadState::Committed(_) => return Err(Error::InvalidState),
            UploadState::Preparing => (),
        }
        self.advance()?;
        if let Some(upload) = self.uploads.get_mut(&id) {
            upload.files.clear();
            upload.bytes = 0;
            upload.state = UploadState::Aborted;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
