//! One atomic local admission creates independent daemon-owned peer transfers.
use super::*;

pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Batch1";
pub const RECIPIENTS: usize = 10;

pub(crate) fn valid_recipients(endpoints: &[String]) -> Result<(), Error> {
    if endpoints.is_empty() || endpoints.len() > RECIPIENTS {
        return Err(Error::InvalidInput);
    }
    let mut seen = BTreeSet::new();
    for endpoint in endpoints {
        identifier(endpoint, 256)?;
        if !seen.insert(endpoint) {
            return Err(Error::InvalidInput);
        }
    }
    Ok(())
}

impl Coordinator {
    pub(super) fn enqueue_many(
        &mut self,
        client: ClientId,
        selection: SelectionId,
        endpoints: &[String],
        names: &[String],
    ) -> Result<Vec<TransferId>, Error> {
        self.running()?;
        self.client(client)?;
        self.synchronized()?;
        valid_recipients(endpoints)?;
        if names.len() != endpoints.len()
            || names
                .iter()
                .any(|name| name.len() > 256 || name.chars().any(char::is_control))
        {
            return Err(Error::InvalidInput);
        }
        let pending = self
            .selections
            .get(&selection)
            .ok_or(Error::UnknownSelection)?;
        if pending.owner != client {
            return Err(Error::WrongOwner);
        }
        if endpoints
            .iter()
            .any(|endpoint| !self.endpoints.contains(endpoint))
        {
            return Err(Error::UnknownEndpoint);
        }
        let count = endpoints.len();
        if self
            .queue
            .len()
            .checked_add(count)
            .is_none_or(|n| n > self.limits.queued_sends)
            || self
                .transfers
                .len()
                .checked_add(count)
                .is_none_or(|n| n > self.limits.transfers)
        {
            return Err(Error::Capacity);
        }
        let reserved = self
            .reservations
            .get(&selection)
            .ok_or(Error::InvalidState)?;
        let handles = pending
            .resource
            .files()
            .len()
            .checked_mul(count + 1)
            .ok_or(Error::Capacity)?
            .max(reserved.handles);
        let extra_handles = handles - reserved.handles;
        if self
            .usage()
            .reserved_handles
            .checked_add(extra_handles)
            .is_none_or(|n| n > self.limits.reserved_handles)
        {
            return Err(Error::Capacity);
        }
        let revision = self
            .revision
            .checked_add(count as u64)
            .ok_or(Error::Exhausted)?;
        let mut transfers = Vec::new();
        transfers
            .try_reserve_exact(count)
            .map_err(|_| Error::Capacity)?;
        for offset in 1..=count {
            transfers.push(TransferId(self.epoch, self.revision + offset as u64));
        }
        // All checks and the entire ID range precede source consumption; no await or partial result.
        let pending = self
            .selections
            .remove(&selection)
            .expect("validated batch selection");
        self.reservations
            .get_mut(&selection)
            .expect("validated source reservation")
            .handles = handles;
        self.revision = revision;
        for ((id, endpoint), name) in transfers.iter().zip(endpoints).zip(names) {
            let name = if name.is_empty() {
                "Unnamed nearby device"
            } else {
                name
            };
            self.queue_outbound(
                *id,
                pending.resource.clone(),
                endpoint.clone(),
                Some(name.to_owned()),
            );
        }
        Ok(transfers)
    }
}

#[cfg(test)]
mod tests;
