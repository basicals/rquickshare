//! Explicit installed-daemon activation. Never owns or stops the shared process.
use tokio::net::UnixStream;
use tokio_util::sync::CancellationToken;
use zbus::{zvariant::OwnedValue, Connection, Message};

use super::bus::{connection_builder, SERVICE};
const BROKER: &str = "org.freedesktop.DBus";
const PATH: &str = "/org/freedesktop/DBus";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationError {
    Unavailable,
    Unsupported,
    Protocol,
    Cancelled,
}
trait Broker: Sync {
    fn owner(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<String>, ActivationError>> + Send;
    fn uid(
        &self,
        owner: &str,
    ) -> impl std::future::Future<Output = Result<u32, ActivationError>> + Send;
    fn supported(&self) -> impl std::future::Future<Output = Result<bool, ActivationError>> + Send;
    fn start(&self) -> impl std::future::Future<Output = Result<u32, ActivationError>> + Send;
}
struct Bus<'a> {
    connection: &'a Connection,
    target: &'static str,
}
fn checked(reply: &Message) -> Result<(), ActivationError> {
    if reply
        .header()
        .sender()
        .is_none_or(|name| name.as_str() != BROKER)
        || !reply.data().fds().is_empty()
    {
        return Err(ActivationError::Protocol);
    }
    Ok(())
}
impl Bus<'_> {
    async fn call<B: serde::Serialize + zbus::zvariant::Type>(
        &self,
        interface: &str,
        member: &str,
        body: &B,
    ) -> Result<Message, ActivationError> {
        match self
            .connection
            .call_method(Some(BROKER), PATH, Some(interface), member, body)
            .await
        {
            Ok(reply) => {
                checked(&reply)?;
                Ok(reply)
            }
            Err(zbus::Error::MethodError(_, _, reply)) => {
                checked(&reply)?;
                Err(ActivationError::Unavailable)
            }
            Err(_) => Err(ActivationError::Unavailable),
        }
    }
}
impl Broker for Bus<'_> {
    async fn owner(&self) -> Result<Option<String>, ActivationError> {
        match self
            .connection
            .call_method(
                Some(BROKER),
                PATH,
                Some(BROKER),
                "GetNameOwner",
                &self.target,
            )
            .await
        {
            Ok(reply) => {
                checked(&reply)?;
                reply
                    .body()
                    .deserialize()
                    .map(Some)
                    .map_err(|_| ActivationError::Protocol)
            }
            Err(zbus::Error::MethodError(name, _, reply)) => {
                checked(&reply)?;
                if name.as_str() == "org.freedesktop.DBus.Error.NameHasNoOwner" {
                    Ok(None)
                } else {
                    Err(ActivationError::Unavailable)
                }
            }
            Err(_) => Err(ActivationError::Unavailable),
        }
    }
    async fn uid(&self, owner: &str) -> Result<u32, ActivationError> {
        self.call(BROKER, "GetConnectionUnixUser", &owner)
            .await?
            .body()
            .deserialize()
            .map_err(|_| ActivationError::Protocol)
    }
    async fn supported(&self) -> Result<bool, ActivationError> {
        let reply = self
            .call(
                "org.freedesktop.DBus.Properties",
                "Get",
                &(BROKER, "Features"),
            )
            .await?;
        let value: OwnedValue = reply
            .body()
            .deserialize()
            .map_err(|_| ActivationError::Protocol)?;
        let features = Vec::<String>::try_from(value).map_err(|_| ActivationError::Protocol)?;
        Ok(features
            .iter()
            .any(|feature| feature == "SystemdActivation"))
    }
    async fn start(&self) -> Result<u32, ActivationError> {
        self.call(BROKER, "StartServiceByName", &(self.target, 0u32))
            .await?
            .body()
            .deserialize()
            .map_err(|_| ActivationError::Protocol)
    }
}
async fn ensure(broker: &impl Broker) -> Result<(), ActivationError> {
    let owner = match broker.owner().await? {
        Some(owner) => owner,
        None => {
            if !broker.supported().await? {
                return Err(ActivationError::Unsupported);
            }
            if !(1..=2).contains(&broker.start().await?) {
                return Err(ActivationError::Protocol);
            }
            broker.owner().await?.ok_or(ActivationError::Unavailable)?
        }
    };
    zbus::names::UniqueName::try_from(owner.as_str()).map_err(|_| ActivationError::Protocol)?;
    if broker.uid(&owner).await? != rustix::process::geteuid().as_raw() {
        return Err(ActivationError::Protocol);
    }
    if broker.owner().await?.as_deref() != Some(owner.as_str()) {
        return Err(ActivationError::Unavailable);
    }
    Ok(())
}
async fn owned(
    connection: Connection,
    broker: &impl Broker,
    cancel: CancellationToken,
) -> Result<(), ActivationError> {
    let result = tokio::select! { biased;
        _ = cancel.cancelled() => Err(ActivationError::Cancelled),
        result = ensure(broker) => result,
    };
    let closed = connection.close().await;
    if closed.is_err() && result.is_ok() {
        Err(ActivationError::Unavailable)
    } else {
        result
    }
}
/// Call only for explicit application launch/reconnect. Afterwards use the passive
/// NativeClient handshake; accepted activation is not readiness or transfer replay.
pub async fn activate_daemon(
    stream: UnixStream,
    cancel: CancellationToken,
) -> Result<(), ActivationError> {
    activate(stream, cancel, SERVICE).await
}
pub async fn activate_desktop(
    stream: UnixStream,
    cancel: CancellationToken,
) -> Result<(), ActivationError> {
    activate(stream, cancel, super::client::desktop::SERVICE).await
}
pub(crate) async fn ensure_desktop(connection: &Connection) -> Result<(), ActivationError> {
    ensure(&Bus {
        connection,
        target: super::client::desktop::SERVICE,
    })
    .await
}
async fn activate(
    stream: UnixStream,
    cancel: CancellationToken,
    target: &'static str,
) -> Result<(), ActivationError> {
    let connection = tokio::select! { biased;
        _ = cancel.cancelled() => return Err(ActivationError::Cancelled),
        result = connection_builder(stream).build() => result.map_err(|_| ActivationError::Unavailable)?,
    };
    let broker_connection = connection.clone();
    owned(
        connection,
        &Bus {
            connection: &broker_connection,
            target,
        },
        cancel,
    )
    .await
}
#[cfg(test)]
mod tests;
