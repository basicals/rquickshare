//! Protocol workers owned by the TCP manager's join barrier.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

use super::{
    cancellation_message, error_for_ui, outbound_failure, route_bandwidth_upgrade_if_pending,
    terminal, ChannelDirection, ChannelMessage, SendInfo, Sender, State, TransferType,
};
use crate::errors::AppError;
use crate::hdl::{BwuRouter, InboundRequest, OutboundRequest};
use crate::sessions::{SessionHub, SessionLease};
use crate::utils::RemoteDeviceInfo;

/// Cancellation encloses the complete operation, including its initial I/O.
pub(crate) async fn until_stopped<T>(
    token: &CancellationToken,
    operation: impl Future<Output = T>,
) -> Option<T> {
    tokio::select! {
        biased;
        _ = token.cancelled() => None,
        result = operation => Some(result),
    }
}

pub(crate) trait Connector: Send + Sync + 'static {
    fn connect<'a>(
        &'a self,
        address: &'a str,
    ) -> impl Future<Output = std::io::Result<TcpStream>> + Send + 'a;
}

pub(crate) struct TcpConnector;

impl Connector for TcpConnector {
    fn connect<'a>(
        &'a self,
        address: &'a str,
    ) -> impl Future<Output = std::io::Result<TcpStream>> + Send + 'a {
        TcpStream::connect(address)
    }
}

pub(super) struct SessionContext<C> {
    endpoint_id: [u8; 4],
    sender: Sender<ChannelMessage>,
    connector: Arc<C>,
    sessions: Option<SessionHub>,
    receive_policy: crate::receive::ReceivePolicy,
}

impl<C> Clone for SessionContext<C> {
    fn clone(&self) -> Self {
        Self {
            endpoint_id: self.endpoint_id,
            sender: self.sender.clone(),
            connector: Arc::clone(&self.connector),
            sessions: self.sessions.clone(),
            receive_policy: self.receive_policy.clone(),
        }
    }
}

impl<C: Connector> SessionContext<C> {
    pub(super) fn new(
        endpoint_id: [u8; 4],
        sender: Sender<ChannelMessage>,
        connector: Arc<C>,
        sessions: Option<SessionHub>,
    ) -> Self {
        Self {
            endpoint_id,
            sender,
            connector,
            sessions,
            receive_policy: crate::receive::ReceivePolicy::new(None),
        }
    }

    pub(super) fn with_receive_policy(mut self, policy: crate::receive::ReceivePolicy) -> Self {
        self.receive_policy = policy;
        self
    }

    pub(super) fn close_sessions(&self) {
        if let Some(sessions) = &self.sessions {
            sessions.close();
        }
    }

    pub(super) fn cancel_queued(&self, id: &str) {
        if let Some(message) =
            cancellation_message(id, TransferType::Outbound, &State::Initial, None)
        {
            let _ = self.sender.send(message);
        }
    }

    #[cfg(test)]
    pub(super) async fn execute_outbound(&self, token: CancellationToken, info: SendInfo) {
        self.execute_session(token, CancellationToken::new(), info, None)
            .await;
    }

    pub(super) async fn execute_session(
        &self,
        token: CancellationToken,
        cancellation: CancellationToken,
        info: SendInfo,
        mut session: Option<SessionLease>,
    ) {
        let id = info.id.clone();
        if let Err(error) = self
            .connect(&token, &cancellation, info, &mut session)
            .await
        {
            if let Some(session) = session.as_mut() {
                session.observe(&State::Disconnected);
            }
            error!("TcpServer: outbound worker failed: {error}");
            if let Some(message) = outbound_failure(&id, &State::Initial, None, &error) {
                let _ = self.sender.send(message);
            }
        }
    }

    async fn connect(
        &self,
        token: &CancellationToken,
        cancellation: &CancellationToken,
        info: SendInfo,
        session: &mut Option<SessionLease>,
    ) -> Result<(), anyhow::Error> {
        let Some(socket) =
            until_either_stops(token, cancellation, self.connector.connect(&info.addr)).await
        else {
            self.cancel_queued(&info.id);
            if let Some(session) = session.as_mut() {
                session.observe(&State::Cancelled);
            }
            return Ok(());
        };
        let mut request = OutboundRequest::new(
            self.endpoint_id,
            socket?,
            info.id,
            self.sender.clone(),
            info.ob,
            RemoteDeviceInfo {
                device_type: crate::DeviceType::Unknown,
                name: info.name,
            },
        )?;
        if let Some(session) = session.take() {
            request = request.attach_session(session);
        }

        let result: Option<Result<(), anyhow::Error>> =
            until_either_stops(token, cancellation, async {
                request.send_connection_request().await?;
                request.send_ukey2_client_init().await?;
                loop {
                    request.handle().await?;
                }
            })
            .await;

        let message = match result {
            None => cancellation_message(
                &request.state.id,
                TransferType::Outbound,
                &request.state.state,
                request.state.transfer_metadata.clone(),
            ),
            Some(Err(error)) if error.downcast_ref::<AppError>().is_none() => outbound_failure(
                &request.state.id,
                &request.state.state,
                request.state.transfer_metadata.clone(),
                &error,
            ),
            _ => None,
        };
        if let Some(message) = message {
            if let Some(state) = message.state.clone() {
                request.record_termination(state);
            }
            let _ = self.sender.send(message);
        }
        Ok(())
    }

    pub(super) async fn classify_inbound(
        &self,
        token: CancellationToken,
        socket: TcpStream,
        remote_addr: SocketAddr,
        router: BwuRouter,
        sessions: Arc<Semaphore>,
        classification: OwnedSemaphorePermit,
    ) {
        let classified =
            until_stopped(&token, route_bandwidth_upgrade_if_pending(socket, &router)).await;
        let socket = match classified {
            Some(Ok(Some(socket))) => socket,
            Some(Err(error)) => {
                warn!("TcpServer: inbound classification failed: {error}");
                return;
            }
            None | Some(Ok(None)) => return,
        };
        let Ok(permit) = sessions.try_acquire_owned() else {
            // A probe at capacity has no established transfer or user-facing identity.
            return;
        };
        drop(classification);
        let _permit = permit;
        self.execute_inbound(&token, socket, remote_addr).await;
    }

    async fn execute_inbound(
        &self,
        token: &CancellationToken,
        socket: TcpStream,
        remote: SocketAddr,
    ) {
        let mut request = InboundRequest::new(socket, remote.to_string(), self.sender.clone())
            .with_receive_directory(self.receive_policy.snapshot());
        if let Some(sessions) = self.sessions.as_ref() {
            let Ok(session) = sessions.open(TransferType::Inbound, None, CancellationToken::new())
            else {
                return;
            };
            request = request.attach_session(session);
        }
        let cancellation = request.cancellation();
        let result: Option<Result<(), anyhow::Error>> =
            until_either_stops(token, &cancellation, async {
                loop {
                    request.handle().await?;
                }
            })
            .await;
        let message = match result {
            None => cancellation_message(
                &request.state.id,
                TransferType::Inbound,
                &request.state.state,
                request.state.transfer_metadata.clone(),
            ),
            Some(Err(error))
                if error.downcast_ref::<AppError>().is_none()
                    && request.state.state != State::Initial
                    && !terminal(&request.state.state) =>
            {
                Some(ChannelMessage {
                    id: request.state.id.clone(),
                    direction: ChannelDirection::LibToFront,
                    rtype: Some(TransferType::Inbound),
                    state: Some(State::Disconnected),
                    meta: request.state.transfer_metadata.clone(),
                    error: Some(error_for_ui(&error)),
                    ..Default::default()
                })
            }
            _ => None,
        };
        if let Some(message) = message {
            if let Some(state) = message.state.clone() {
                request.record_termination(state);
            }
            let _ = self.sender.send(message);
        } else if result_is_cancelled(token, &cancellation) {
            request.record_termination(State::Cancelled);
        }
    }
}

pub(crate) async fn until_either_stops<T>(
    service: &CancellationToken,
    request: &CancellationToken,
    operation: impl Future<Output = T>,
) -> Option<T> {
    tokio::select! {
        biased;
        _ = service.cancelled() => None,
        _ = request.cancelled() => None,
        result = operation => Some(result),
    }
}

fn result_is_cancelled(service: &CancellationToken, request: &CancellationToken) -> bool {
    service.is_cancelled() || request.is_cancelled()
}
