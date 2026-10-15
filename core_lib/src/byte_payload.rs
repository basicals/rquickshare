//! Request-owned reassembly. Reservations bound sparse/interleaved peer input.

use std::collections::HashMap;
use std::fmt;

use anyhow::{anyhow, Result};

use crate::location_nearby_connections::payload_transfer_frame::{PayloadChunk, PayloadHeader};
use crate::protocol::{checked_payload_buffer_size, SANE_FRAME_LENGTH};

const MAX_PENDING: usize = 16;
const MAX_RESERVED_BYTES: usize = SANE_FRAME_LENGTH as usize;

struct Pending {
    size: usize,
    bytes: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct BytePayloads {
    pending: HashMap<i64, Pending>,
    reserved: usize,
}

impl fmt::Debug for BytePayloads {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BytePayloads")
            .field("pending", &self.pending.len())
            .field("reserved", &self.reserved)
            .finish()
    }
}

impl BytePayloads {
    /// Errors terminate the handler; release all incomplete ownership immediately.
    pub(crate) fn push(
        &mut self,
        header: &PayloadHeader,
        chunk: &PayloadChunk,
    ) -> Result<Option<Vec<u8>>> {
        let result = self.push_checked(header, chunk);
        if result.is_err() {
            self.pending.clear();
            self.reserved = 0;
        }
        result
    }

    fn push_checked(
        &mut self,
        header: &PayloadHeader,
        chunk: &PayloadChunk,
    ) -> Result<Option<Vec<u8>>> {
        let id = header
            .id
            .ok_or_else(|| anyhow!("Missing byte payload id"))?;
        let size = checked_payload_buffer_size(
            header
                .total_size
                .ok_or_else(|| anyhow!("Missing byte payload size"))?,
        )?;
        let offset = chunk
            .offset
            .and_then(|offset| usize::try_from(offset).ok())
            .ok_or_else(|| anyhow!("Missing or invalid byte payload offset"))?;
        let existing = self.pending.get(&id);
        if existing.is_some_and(|pending| pending.size != size) {
            return Err(anyhow!("Byte payload size changed"));
        }
        let previous_len = existing.map_or(0, |pending| pending.bytes.len());
        if offset != previous_len {
            return Err(anyhow!("Unexpected byte payload offset"));
        }
        let new_len = previous_len
            .checked_add(chunk.body().len())
            .filter(|&len| len <= size)
            .ok_or_else(|| anyhow!("Byte payload exceeds declared size"))?;
        let complete = chunk.flags() & 1 == 1;
        if complete && new_len != size {
            return Err(anyhow!("Byte payload is incomplete"));
        }

        if existing.is_none() {
            let reserved = self
                .reserved
                .checked_add(size)
                .filter(|&bytes| bytes <= MAX_RESERVED_BYTES)
                .ok_or_else(|| anyhow!("Byte payload reservation limit exceeded"))?;
            if self.pending.len() >= MAX_PENDING {
                return Err(anyhow!("Too many incomplete byte payloads"));
            }
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(size)
                .map_err(|_| anyhow!("Cannot reserve byte payload buffer"))?;
            self.pending.insert(id, Pending { size, bytes });
            self.reserved = reserved;
        }

        let pending = self.pending.get_mut(&id).expect("admitted byte payload");
        pending.bytes.extend_from_slice(chunk.body());
        if complete {
            let pending = self.pending.remove(&id).expect("completed byte payload");
            self.reserved -= pending.size;
            return Ok(Some(pending.bytes));
        }
        Ok(None)
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.reserved == 0
    }
}

#[cfg(test)]
pub(crate) mod tests;
