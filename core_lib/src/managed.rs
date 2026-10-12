//! Owned dispatch controls for the native core adapter; no IPC or authentication API.

use std::fmt;

use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

use crate::channel::TransferType;
use crate::sessions::{SessionControl, SessionHub, SessionLease};
use crate::SendInfo;

const QUEUE_CAPACITY: usize = 10;

#[derive(Clone)]
pub struct ManagedSender {
    sender: mpsc::Sender<OutboundJob>,
    sessions: SessionHub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionKind {
    Full,
    Closed,
    Unavailable,
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
    session: SessionControl,
}

impl TransferControl {
    pub fn session(&self) -> SessionControl {
        self.session.clone()
    }
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
    pub(crate) fn channel() -> Result<(Self, OutboundReceiver), crate::sessions::SessionError> {
        let (sender, receiver) = mpsc::channel(QUEUE_CAPACITY);
        // RQS creates one fresh run epoch; tests can inject epochs directly into the hub.
        let epoch = rand::random::<u128>().max(1);
        let sessions = SessionHub::new(epoch)?;
        Ok((
            Self {
                sender,
                sessions: sessions.clone(),
            },
            OutboundReceiver::Managed { receiver, sessions },
        ))
    }

    pub fn sessions(&self) -> SessionHub {
        self.sessions.clone()
    }

    pub fn try_send(&self, info: SendInfo) -> Result<TransferControl, RejectedSend> {
        let cancellation = CancellationToken::new();
        let session = match self.sessions.open(
            TransferType::Outbound,
            Some(info.id.clone()),
            cancellation.clone(),
        ) {
            Ok(session) => session,
            Err(error) => {
                return Err(RejectedSend {
                    kind: match error {
                        crate::sessions::SessionError::Full => RejectionKind::Full,
                        crate::sessions::SessionError::Closed => RejectionKind::Closed,
                        _ => RejectionKind::Unavailable,
                    },
                    request: Box::new(info),
                })
            }
        };
        let (completion, released) = watch::channel(false);
        let control = TransferControl {
            cancellation: cancellation.clone(),
            released,
            session: session.control(),
        };
        let job = OutboundJob {
            info,
            cancellation,
            completion: Some(completion),
            session: Some(session),
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
    pub session: Option<SessionLease>,
}

impl OutboundJob {
    pub fn release(self) {
        let Self {
            info,
            completion,
            session,
            ..
        } = self;
        drop(info);
        if let Some(mut session) = session {
            session.observe(&crate::State::Cancelled);
        }
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
            session: None,
        }
    }
}

pub(crate) enum OutboundReceiver {
    Legacy(mpsc::Receiver<SendInfo>),
    Managed {
        receiver: mpsc::Receiver<OutboundJob>,
        sessions: SessionHub,
    },
}

impl From<mpsc::Receiver<SendInfo>> for OutboundReceiver {
    fn from(receiver: mpsc::Receiver<SendInfo>) -> Self {
        Self::Legacy(receiver)
    }
}

impl OutboundReceiver {
    pub fn sessions(&self) -> Option<SessionHub> {
        match self {
            Self::Legacy(_) => None,
            Self::Managed { sessions, .. } => Some(sessions.clone()),
        }
    }
    pub async fn recv(&mut self) -> Option<OutboundJob> {
        match self {
            Self::Legacy(receiver) => receiver.recv().await.map(Into::into),
            Self::Managed { receiver, .. } => receiver.recv().await,
        }
    }

    pub fn close(&mut self) {
        match self {
            Self::Legacy(receiver) => receiver.close(),
            Self::Managed { receiver, .. } => receiver.close(),
        }
    }

    pub fn try_recv(&mut self) -> Result<OutboundJob, mpsc::error::TryRecvError> {
        match self {
            Self::Legacy(receiver) => receiver.try_recv().map(Into::into),
            Self::Managed { receiver, .. } => receiver.try_recv(),
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
