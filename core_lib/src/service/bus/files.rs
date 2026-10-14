//! Typed receipt lookup; only private actual receive resources can authorize file IO.
use std::sync::Arc;

use super::*;
use crate::{
    received_files::ReceivedFiles,
    service::{Direction, Phase},
};
pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Files1";
pub(super) type MetadataRow = (String, u64, u32);
pub(super) fn resource(
    coordinator: &Coordinator,
    client: ClientId,
    target: TransferId,
    revision: u64,
    index: u32,
) -> Result<Arc<ReceivedFiles>, Error> {
    coordinator.client(client)?;
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
    let files = transfer.received_files.clone().ok_or(Error::InvalidState)?;
    if index as usize >= files.len() {
        return Err(Error::InvalidInput);
    }
    Ok(files)
}
#[cfg(test)]
pub(crate) mod tests;
