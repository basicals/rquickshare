//! Ordered connection-local receipts. Retired sequences never authorize new work.

use uploads::UploadState;

use super::*;

pub const RECEIPTS: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Call {
    Begin,
    BeginText(u32),
    Commit(UploadId),
    Abort(UploadId),
    Send(SelectionId, String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationResult {
    Upload(UploadId),
    Selection(SelectionId),
    Transfer(TransferId),
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationReceipt {
    pub kind: &'static str,
    pub result: OperationResult,
}

#[derive(Default)]
pub(super) struct Ledger {
    last: u64,
    acknowledged: u64,
    pub(super) receipts: BTreeMap<u64, (Call, OperationReceipt)>,
}

impl Call {
    fn kind(&self) -> &'static str {
        match self {
            Self::Begin => "begin_selection",
            Self::BeginText(_) => "begin_text",
            Self::Commit(_) => "commit_selection",
            Self::Abort(_) => "abort_selection",
            Self::Send(_, _) => "send_selection",
        }
    }

    fn upload(&self) -> Option<UploadId> {
        match self {
            Self::Commit(id) | Self::Abort(id) => Some(*id),
            _ => None,
        }
    }
}

impl Coordinator {
    fn operation(
        &mut self,
        client: ClientId,
        sequence: u64,
        call: Call,
    ) -> Result<OperationReceipt, Error> {
        self.client(client)?;
        let ledger = &self.operations[&client];
        if sequence == 0 {
            return Err(Error::InvalidInput);
        }
        if sequence <= ledger.acknowledged {
            return Err(Error::RequestExpired);
        }
        if sequence <= ledger.last {
            let (previous, receipt) = &ledger.receipts[&sequence];
            return if previous == &call {
                Ok(receipt.clone())
            } else {
                Err(Error::ReplayConflict)
            };
        }
        if sequence != ledger.last.checked_add(1).ok_or(Error::Exhausted)? {
            return Err(Error::InvalidInput);
        }
        if ledger.receipts.len() >= RECEIPTS {
            return Err(Error::Capacity);
        }
        self.revision.checked_add(1).ok_or(Error::Exhausted)?;
        let result = match &call {
            Call::Begin => OperationResult::Upload(self.begin_upload(client, None)?),
            Call::BeginText(count) => {
                OperationResult::Upload(self.begin_text_upload(client, *count)?)
            }
            Call::Commit(id) => {
                self.check_upload_origin(client, *id, true)?;
                OperationResult::Selection(self.commit_upload(client, *id)?)
            }
            Call::Abort(id) => {
                self.check_upload_origin(client, *id, true)?;
                self.abort_upload(client, *id)?;
                OperationResult::Empty
            }
            Call::Send(id, endpoint) => {
                OperationResult::Transfer(self.enqueue_selection(client, *id, endpoint, None)?)
            }
        };
        let receipt = OperationReceipt {
            kind: call.kind(),
            result,
        };
        // All fallible admission is complete. No await or further ownership check here.
        let ledger = self
            .operations
            .get_mut(&client)
            .ok_or(Error::UnknownClient)?;
        ledger.last = sequence;
        ledger.receipts.insert(sequence, (call, receipt.clone()));
        Ok(receipt)
    }

    pub fn ordered_begin(&mut self, client: ClientId, sequence: u64) -> Result<UploadId, Error> {
        match self.operation(client, sequence, Call::Begin)?.result {
            OperationResult::Upload(id) => Ok(id),
            _ => Err(Error::InvalidState),
        }
    }
    pub fn ordered_begin_text(
        &mut self,
        client: ClientId,
        count: u32,
        sequence: u64,
    ) -> Result<UploadId, Error> {
        match self
            .operation(client, sequence, Call::BeginText(count))?
            .result
        {
            OperationResult::Upload(id) => Ok(id),
            _ => Err(Error::InvalidState),
        }
    }
    pub fn ordered_commit(
        &mut self,
        client: ClientId,
        id: UploadId,
        sequence: u64,
    ) -> Result<SelectionId, Error> {
        match self.operation(client, sequence, Call::Commit(id))?.result {
            OperationResult::Selection(id) => Ok(id),
            _ => Err(Error::InvalidState),
        }
    }
    pub fn ordered_abort(
        &mut self,
        client: ClientId,
        id: UploadId,
        sequence: u64,
    ) -> Result<(), Error> {
        self.operation(client, sequence, Call::Abort(id))
            .map(|_| ())
    }
    pub fn ordered_send(
        &mut self,
        client: ClientId,
        id: SelectionId,
        endpoint: &str,
        sequence: u64,
    ) -> Result<TransferId, Error> {
        match self
            .operation(client, sequence, Call::Send(id, endpoint.to_owned()))?
            .result
        {
            OperationResult::Transfer(id) => Ok(id),
            _ => Err(Error::InvalidState),
        }
    }
    pub fn operation_state(&self, client: ClientId) -> Result<(u64, u64), Error> {
        self.client(client)?;
        let ledger = &self.operations[&client];
        Ok((ledger.last, ledger.acknowledged))
    }
    pub fn operation_receipt(
        &self,
        client: ClientId,
        sequence: u64,
    ) -> Result<OperationReceipt, Error> {
        self.client(client)?;
        let ledger = &self.operations[&client];
        if sequence == 0 || sequence > ledger.last {
            return Err(Error::InvalidInput);
        }
        if sequence <= ledger.acknowledged {
            return Err(Error::RequestExpired);
        }
        Ok(ledger.receipts[&sequence].1.clone())
    }
    pub fn acknowledge_operations(&mut self, client: ClientId, sequence: u64) -> Result<(), Error> {
        self.client(client)?;
        let ledger = &self.operations[&client];
        if sequence > ledger.last {
            return Err(Error::InvalidInput);
        }
        if sequence <= ledger.acknowledged {
            return Ok(());
        }
        self.advance()?;
        let ledger = self
            .operations
            .get_mut(&client)
            .ok_or(Error::UnknownClient)?;
        ledger.acknowledged = sequence;
        ledger.receipts.retain(|id, _| *id > sequence);
        self.collect_ordered_uploads();
        Ok(())
    }

    fn collect_ordered_uploads(&mut self) {
        let pinned: BTreeSet<_> = self
            .operations
            .values()
            .flat_map(|ledger| ledger.receipts.values())
            .filter_map(|(call, receipt)| {
                call.upload().or(match receipt.result {
                    OperationResult::Upload(id) => Some(id),
                    _ => None,
                })
            })
            .collect();
        self.uploads.retain(|id, upload| {
            upload.request.is_some()
                || upload.state == UploadState::Preparing
                || pinned.contains(id)
        });
    }

    pub(super) fn operation_pins(&self, transfer: TransferId) -> bool {
        self.operations.values().any(|ledger| {
            ledger
                .receipts
                .values()
                .any(|(_, receipt)| receipt.result == OperationResult::Transfer(transfer))
        })
    }
}

#[cfg(test)]
pub(crate) mod tests;
