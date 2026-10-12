//! Safe current-run history; internal barriers never imply payload success.

use super::*;

pub const RECENT_TERMINALS: usize = 64;

impl Coordinator {
    pub(super) fn owner_closed(&mut self, id: TransferId) -> Result<(), Error> {
        let transfer = self.transfers.get_mut(&id).ok_or(Error::UnknownTransfer)?;
        if !transfer.view.phase.is_terminal() {
            return Err(Error::InvalidState);
        }
        transfer.owner_closed = true;
        Ok(())
    }
    pub(super) fn source_released(&mut self, id: TransferId) -> Result<(), Error> {
        self.transfers
            .get_mut(&id)
            .ok_or(Error::UnknownTransfer)?
            .released = true;
        Ok(())
    }
    pub(super) fn control_receipt(&mut self, id: TransferId, pending: bool) -> Result<(), Error> {
        self.transfers
            .get_mut(&id)
            .ok_or(Error::UnknownTransfer)?
            .control_receipt = pending;
        Ok(())
    }
    pub(super) fn maintain_history(&mut self) -> Result<bool, Error> {
        self.refresh_resources()?;
        if !self.synchronized {
            return Ok(false);
        }
        let mut eligible: Vec<_> = self
            .transfers
            .iter()
            .filter_map(|(id, transfer)| {
                let terminal_revision = transfer.terminal_revision?;
                (transfer.view.phase.is_terminal()
                    && transfer.owner_closed
                    && transfer.released
                    && transfer.source.strong_count() == 0
                    && !transfer.control_receipt
                    && transfer.view.pending_control.is_none()
                    && !self.operation_pins(*id)
                    && !self
                        .clients
                        .values()
                        .any(|ledger| ledger.values().any(|entry| entry.transfer == *id)))
                .then_some((terminal_revision, *id))
            })
            .collect();
        eligible.sort_unstable();
        let target = RECENT_TERMINALS.min(self.limits.transfers.saturating_sub(1));
        let remove = eligible.len().saturating_sub(target);
        if remove > 0 {
            self.advance()?;
            for (_, id) in eligible.into_iter().take(remove) {
                self.transfers.remove(&id);
            }
        }
        Ok(remove > 0)
    }
}

#[cfg(test)]
mod tests;
