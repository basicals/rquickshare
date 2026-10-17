//! Typed receipt lookup; only private actual receive resources can authorize file IO.
use std::{fs::File, sync::Arc};

use super::*;
use crate::{received_files::ReceivedFiles, service::Direction};
pub const INTERFACE: &str = "io.github.morozhenko_apps.RQuickShare.Files1";
pub(crate) type Resolver = dyn Fn(Arc<ReceivedFiles>, u32) -> Result<File, Error> + Send + Sync;
pub(super) struct Request {
    pub message: Message,
    pub client: ClientId,
    pub target: TransferId,
    pub revision: u64,
    pub index: u32,
}
/// Cancellation-safe polling: the bus retains the handle until completion or close.
pub(super) async fn next(job: &mut Option<JoinHandle<Result<File, Error>>>) -> Result<File, Error> {
    match job {
        Some(job) => job.await.unwrap_or(Err(Error::InvalidState)),
        None => std::future::pending().await,
    }
}
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
    if transfer.view.direction != Direction::Inbound || !transfer.view.phase.is_terminal() {
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
