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
use crate::utils::RemoteDeviceInfo;

/// Cancellation encloses the complete operation, including its initial I/O.
pub(super) async fn until_stopped<T>(
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
}

impl<C> Clone for SessionContext<C> {
    fn clone(&self) -> Self {
        Self {
            endpoint_id: self.endpoint_id,
            sender: self.sender.clone(),
            connector: Arc::clone(&self.connector),
        }
    }
}

impl<C: Connector> SessionContext<C> {
    pub(super) fn new(
        endpoint_id: [u8; 4],
        sender: Sender<ChannelMessage>,
        connector: Arc<C>,
    ) -> Self {
        Self {
            endpoint_id,
            sender,
            connector,
        }
    }

    pub(super) fn cancel_queued(&self, id: &str) {
        if let Some(message) =
            cancellation_message(id, TransferType::Outbound, &State::Initial, None)
        {
            let _ = self.sender.send(message);
        }
    }

    pub(super) async fn execute_outbound(&self, token: CancellationToken, info: SendInfo) {
        let id = info.id.clone();
        if let Err(error) = self.connect(&token, info).await {
            error!("TcpServer: outbound worker failed: {error}");
            if let Some(message) = outbound_failure(&id, &State::Initial, None, &error) {
                let _ = self.sender.send(message);
            }
        }
    }

    async fn connect(
        &self,
        token: &CancellationToken,
        info: SendInfo,
    ) -> Result<(), anyhow::Error> {
        let Some(socket) = until_stopped(token, self.connector.connect(&info.addr)).await else {
            self.cancel_queued(&info.id);
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

        let result: Option<Result<(), anyhow::Error>> = until_stopped(token, async {
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
        let mut request = InboundRequest::new(socket, remote.to_string(), self.sender.clone());
        let result: Option<Result<(), anyhow::Error>> = until_stopped(token, async {
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
            let _ = self.sender.send(message);
        }
    }
}
