//! Bounded, authenticated immutable text read capabilities; no display projection owns a body.
use std::{collections::BTreeMap, sync::Arc};

use super::{token, ClientId, Coordinator, Error, TransferId};
use crate::{
    service::{Direction, Phase},
    sessions::text::ReceivedText,
};

pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Text1";
pub const CHUNK: usize = 16 * 1024;
const READERS: usize = 32;
pub(super) type MetadataRow = (String, &'static str, bool, u64);

struct Reader {
    id: String,
    target: TransferId,
    revision: u64,
    body: Arc<ReceivedText>,
}
#[derive(Default)]
pub(super) struct TextCache {
    readers: BTreeMap<ClientId, Reader>,
    ended: BTreeMap<ClientId, String>,
    sequence: u64,
}
impl TextCache {
    pub(super) fn begin(
        &mut self,
        coordinator: &Coordinator,
        client: ClientId,
        target: TransferId,
        revision: u64,
    ) -> Result<(String, &'static str, u32), Error> {
        coordinator.client(client)?;
        if let Some(reader) = self.readers.get(&client) {
            return if reader.target == target && reader.revision == revision {
                Ok((
                    reader.id.clone(),
                    reader.body.kind().wire(),
                    reader.body.text().len() as u32,
                ))
            } else {
                Err(Error::Busy)
            };
        }
        if self.readers.len() >= READERS {
            return Err(Error::Capacity);
        }
        let transfer = coordinator
            .transfers
            .get(&target)
            .ok_or(Error::UnknownTransfer)?;
        if revision == 0 || transfer.terminal_revision != Some(revision) {
            return Err(Error::StaleControl);
        }
        if transfer.view.direction != Direction::Inbound || transfer.view.phase != Phase::Finished {
            return Err(Error::InvalidState);
        }
        let body = transfer.received_text.clone().ok_or(Error::InvalidState)?;
        let sequence = self.sequence.checked_add(1).ok_or(Error::Exhausted)?;
        let reader = Reader {
            id: token(coordinator.epoch, sequence),
            target,
            revision,
            body,
        };
        let result = (
            reader.id.clone(),
            reader.body.kind().wire(),
            reader.body.text().len() as u32,
        );
        self.sequence = sequence;
        self.readers.insert(client, reader);
        Ok(result)
    }
    pub(super) fn chunk(
        &self,
        client: ClientId,
        id: &str,
        offset: u32,
    ) -> Result<(u32, Vec<u8>), Error> {
        let (owner, reader) = self
            .readers
            .iter()
            .find(|(_, reader)| reader.id == id)
            .ok_or(Error::UnknownSnapshot)?;
        if *owner != client {
            return Err(Error::WrongOwner);
        }
        let bytes = reader.body.text().as_bytes();
        let offset = offset as usize;
        if offset > bytes.len() || (offset != bytes.len() && !offset.is_multiple_of(CHUNK)) {
            return Err(Error::InvalidInput);
        }
        let next = (offset + CHUNK).min(bytes.len());
        Ok((next as u32, bytes[offset..next].to_vec()))
    }
    pub(super) fn end(&mut self, client: ClientId, id: &str) -> Result<(), Error> {
        if self.ended.get(&client).is_some_and(|ended| ended == id) {
            return Ok(());
        }
        if self.ended.values().any(|ended| ended == id) {
            return Err(Error::WrongOwner);
        }
        let (owner, _) = self
            .readers
            .iter()
            .find(|(_, reader)| reader.id == id)
            .ok_or(Error::UnknownSnapshot)?;
        if *owner != client {
            return Err(Error::WrongOwner);
        }
        self.readers.remove(&client);
        self.ended.insert(client, id.to_owned());
        Ok(())
    }
    pub(super) fn disconnect(&mut self, client: ClientId) {
        self.readers.remove(&client);
        self.ended.remove(&client);
    }
}

#[cfg(test)]
pub(crate) mod tests;
