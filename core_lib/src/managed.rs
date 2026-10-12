//! Owned dispatch controls for the native core adapter; no IPC or authentication API.

use std::fmt;

use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

use crate::SendInfo;

const QUEUE_CAPACITY: usize = 10;

#[derive(Clone)]
pub struct ManagedSender {
    sender: mpsc::Sender<OutboundJob>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionKind {
    Full,
    Closed,
}

/// Admission failed: the adapter still owns the unsent input.
pub struct RejectedSend {
    pub kind: RejectionKind,
    pub request: Box<SendInfo>,
}

impl fmt::Debug for RejectedSend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RejectedSend")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for RejectedSend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Managed dispatch rejected: {:?}", self.kind)
    }
}

impl std::error::Error for RejectedSend {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlLost;

impl fmt::Display for ControlLost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Managed dispatch owner disappeared before release acknowledgement")
    }
}

impl std::error::Error for ControlLost {}

/// Dropping this handle leaves service-owned work running.
#[derive(Clone)]
pub struct TransferControl {
    cancellation: CancellationToken,
    released: watch::Receiver<bool>,
}

impl TransferControl {
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// A release acknowledgement is not a transfer outcome or delivery receipt.
    pub async fn wait_released(&self) -> Result<(), ControlLost> {
        let mut released = self.released.clone();
        released
            .wait_for(|released| *released)
            .await
            .map(|_| ())
            .map_err(|_| ControlLost)
    }
}

impl ManagedSender {
    pub(crate) fn channel() -> (Self, OutboundReceiver) {
        let (sender, receiver) = mpsc::channel(QUEUE_CAPACITY);
        (Self { sender }, OutboundReceiver::Managed(receiver))
    }

    pub fn try_send(&self, info: SendInfo) -> Result<TransferControl, RejectedSend> {
        let cancellation = CancellationToken::new();
        let (completion, released) = watch::channel(false);
        let control = TransferControl {
            cancellation: cancellation.clone(),
            released,
        };
        let job = OutboundJob {
            info,
            cancellation,
            completion: Some(completion),
        };
        self.sender.try_send(job).map_err(|error| {
            let (kind, job) = match error {
                mpsc::error::TrySendError::Full(job) => (RejectionKind::Full, job),
                mpsc::error::TrySendError::Closed(job) => (RejectionKind::Closed, job),
            };
            RejectedSend {
                kind,
                request: Box::new(job.info),
            }
        })?;
        Ok(control)
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    /// Unexpected closure is a health-loss input, never successful delivery.
    pub async fn closed(&self) {
        self.sender.closed().await;
    }

    #[cfg(test)]
    pub(crate) async fn wait_queue_empty(&self) {
        drop(
            self.sender
                .reserve_many(QUEUE_CAPACITY)
                .await
                .expect("test queue closed"),
        );
    }
}

pub(crate) struct OutboundJob {
    pub info: SendInfo,
    pub cancellation: CancellationToken,
    pub completion: Option<watch::Sender<bool>>,
}

impl OutboundJob {
    pub fn release(self) {
        let Self {
            info, completion, ..
        } = self;
        drop(info);
        acknowledge_release(completion);
    }
}

pub(crate) fn acknowledge_release(completion: Option<watch::Sender<bool>>) {
    if let Some(completion) = completion {
        completion.send_replace(true);
    }
}

impl From<SendInfo> for OutboundJob {
    fn from(info: SendInfo) -> Self {
        Self {
            info,
            cancellation: CancellationToken::new(),
            completion: None,
        }
    }
}

pub(crate) enum OutboundReceiver {
    Legacy(mpsc::Receiver<SendInfo>),
    Managed(mpsc::Receiver<OutboundJob>),
}

impl From<mpsc::Receiver<SendInfo>> for OutboundReceiver {
    fn from(receiver: mpsc::Receiver<SendInfo>) -> Self {
        Self::Legacy(receiver)
    }
}

impl OutboundReceiver {
    pub async fn recv(&mut self) -> Option<OutboundJob> {
        match self {
            Self::Legacy(receiver) => receiver.recv().await.map(Into::into),
            Self::Managed(receiver) => receiver.recv().await,
        }
    }

    pub fn close(&mut self) {
        match self {
            Self::Legacy(receiver) => receiver.close(),
            Self::Managed(receiver) => receiver.close(),
        }
    }

    pub fn try_recv(&mut self) -> Result<OutboundJob, mpsc::error::TryRecvError> {
        match self {
            Self::Legacy(receiver) => receiver.try_recv().map(Into::into),
            Self::Managed(receiver) => receiver.try_recv(),
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
