//! Explicit bounded text reads. Ordinary state contains no body and never touches clipboard.
use std::sync::Arc;

use super::*;
use crate::service::bus::text::{CHUNK, INTERFACE};
use crate::{
    service::{Direction, Phase},
    sessions::{
        details::PayloadKind,
        text::{ReceivedText, MAX_TEXT_BYTES},
    },
};

pub(super) type MetadataRow = (String, String, bool, u64);
pub(super) fn kind(value: &str) -> Result<PayloadKind> {
    match value {
        "files" => Ok(PayloadKind::Files),
        "text" => Ok(PayloadKind::Text),
        "url" => Ok(PayloadKind::Url),
        _ => Err(ClientError::Protocol),
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextRead {
    pub epoch: u128,
    pub transfer: Transfer,
    pub revision: u64,
    pub body: Arc<ReceivedText>,
}
pub(super) struct PendingText {
    target: Transfer,
    revision: u64,
    kind: PayloadKind,
    count: u32,
    reader: Option<String>,
    bytes: Vec<u8>,
}
impl NativeClient {
    pub fn has_pending_text(&self) -> bool {
        self.pending_text.is_some()
    }
    pub async fn retry_text(&mut self) -> Result<TextRead> {
        let pending = self
            .pending_text
            .as_ref()
            .ok_or(ClientError::InvalidInput)?;
        let target = pending.target.clone();
        let revision = pending.revision;
        self.read_received_text(&target, revision).await
    }
    pub async fn read_received_text(
        &mut self,
        target: &Transfer,
        revision: u64,
    ) -> Result<TextRead> {
        if target.connection != self.identity {
            return Err(ClientError::ForeignHandle);
        }
        canonical(target.id(), self.epoch)?;
        if let Some(pending) = &self.pending_text {
            if &pending.target != target || pending.revision != revision {
                return Err(ClientError::PendingOperation);
            }
        } else {
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
                || transfer.phase != Phase::Finished
                || transfer.content_revision != Some(revision)
                || transfer.details.kind == PayloadKind::Files
                || transfer.details.total_bytes > MAX_TEXT_BYTES as u64
            {
                return Err(ClientError::Rejected(DomainError::InvalidState));
            }
            self.pending_text = Some(PendingText {
                target: target.clone(),
                revision,
                kind: transfer.details.kind,
                count: transfer.details.total_bytes as u32,
                reader: None,
                bytes: Vec::new(),
            });
        }
        if self.pending_text.as_ref().unwrap().reader.is_none() {
            let reply = self
                .wire(INTERFACE, "BeginRead", &(target.id(), revision))
                .await?;
            let (reader, ty, count): (String, String, u32) = reply
                .body()
                .deserialize()
                .map_err(|_| ClientError::Protocol)?;
            canonical(&reader, self.epoch)?;
            let pending = self.pending_text.as_mut().unwrap();
            if kind(&ty)? != pending.kind
                || count != pending.count
                || count as usize > MAX_TEXT_BYTES
            {
                return Err(ClientError::Protocol);
            }
            pending.reader = Some(reader);
            pending.bytes.reserve_exact(count as usize);
        }
        loop {
            let pending = self.pending_text.as_ref().unwrap();
            let offset = pending.bytes.len() as u32;
            if offset == pending.count && pending.count != 0 {
                break;
            }
            let reader = pending.reader.clone().unwrap();
            let reply = self
                .wire(INTERFACE, "GetChunk", &(reader.as_str(), offset))
                .await?;
            let (next, bytes): (u32, Vec<u8>) = reply
                .body()
                .deserialize()
                .map_err(|_| ClientError::Protocol)?;
            let pending = self.pending_text.as_mut().unwrap();
            let expected = CHUNK.min((pending.count - offset) as usize);
            if bytes.len() != expected || next != offset + expected as u32 {
                return Err(ClientError::Protocol);
            }
            pending.bytes.extend(bytes);
            if next == pending.count {
                break;
            }
        }
        let pending = self.pending_text.as_ref().unwrap();
        let text = std::str::from_utf8(&pending.bytes)
            .map_err(|_| ClientError::Protocol)?
            .to_owned();
        let body = Arc::new(ReceivedText::new(pending.kind, text).ok_or(ClientError::Protocol)?);
        let reader = pending.reader.clone().unwrap();
        self.wire(INTERFACE, "EndRead", &reader)
            .await?
            .body()
            .deserialize::<()>()
            .map_err(|_| ClientError::Protocol)?;
        self.pending_text = None;
        Ok(TextRead {
            epoch: self.epoch,
            transfer: target.clone(),
            revision,
            body,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests;
