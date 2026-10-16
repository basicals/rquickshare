//! Private read-only received-file completion. Ordinary state never owns file handles.
use std::{fs::File, sync::Arc};

use super::*;
use crate::service::Direction;
use crate::sessions::details::{DetailsStatus, PayloadKind};

#[derive(Clone)]
pub struct ReceivedFile {
    pub epoch: u128,
    pub transfer: Transfer,
    pub revision: u64,
    pub index: u32,
    file: Arc<File>,
}
impl ReceivedFile {
    pub fn file(&self) -> &File {
        &self.file
    }
}
impl fmt::Debug for ReceivedFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceivedFile")
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}
impl PartialEq for ReceivedFile {
    fn eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch
            && self.transfer == other.transfer
            && self.revision == other.revision
            && self.index == other.index
            && Arc::ptr_eq(&self.file, &other.file)
    }
}
impl Eq for ReceivedFile {}
impl NativeClient {
    pub async fn acquire_received_file(
        &mut self,
        target: &Transfer,
        revision: u64,
        index: u32,
    ) -> Result<ReceivedFile> {
        if target.connection != self.identity {
            return Err(ClientError::ForeignHandle);
        }
        canonical(target.id(), self.epoch)?;
        let view = self
            .last_view
            .as_ref()
            .filter(|_| !self.view_incomplete)
            .ok_or(ClientError::Protocol)?;
        let transfer = view
            .transfers
            .iter()
            .find(|row| &row.transfer == target)
            .ok_or(ClientError::Rejected(DomainError::UnknownTransfer))?;
        if transfer.direction != Direction::Inbound
            || !transfer.phase.is_terminal()
            || transfer.reveal_revision != Some(revision)
            || revision == 0
            || transfer.details.kind != PayloadKind::Files
            || transfer.details.status != DetailsStatus::Ready
            || index as usize >= transfer.completed_files.len()
        {
            return Err(ClientError::Rejected(DomainError::InvalidState));
        }
        let reply = self
            .wire(
                bus::files::INTERFACE,
                "AcquireFile",
                &(target.id(), revision, index),
            )
            .await?;
        let body = reply.body();
        let fd: Fd<'_> = body.deserialize().map_err(|_| ClientError::Protocol)?;
        let file = File::from(
            fd.as_fd()
                .try_clone_to_owned()
                .map_err(|_| ClientError::Source)?,
        );
        let flags = rustix::fs::fcntl_getfl(&file).map_err(|_| ClientError::Protocol)?;
        if flags.contains(rustix::fs::OFlags::PATH)
            || flags & rustix::fs::OFlags::ACCMODE != rustix::fs::OFlags::RDONLY
            || !file
                .metadata()
                .map_err(|_| ClientError::Protocol)?
                .is_file()
        {
            return Err(ClientError::Protocol);
        }
        rustix::io::fcntl_setfd(&file, rustix::io::FdFlags::CLOEXEC)
            .map_err(|_| ClientError::Protocol)?;
        Ok(ReceivedFile {
            epoch: self.epoch,
            transfer: target.clone(),
            revision,
            index,
            file: Arc::new(file),
        })
    }
}

#[cfg(test)]
mod tests;
