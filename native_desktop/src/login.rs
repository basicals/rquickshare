//! One bounded owner of explicit, persistent user-manager login enablement.
use futures::StreamExt;
use rqs_lib::service::{bus::connection_builder, process::BusAddress};
use std::collections::HashMap;
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use zbus::{
    zvariant::{OwnedObjectPath, OwnedValue},
    Connection, MatchRule, Message, MessageStream,
};
const BROKER: &str = "org.freedesktop.DBus";
const SERVICE: &str = "org.freedesktop.systemd1";
const PATH: &str = "/org/freedesktop/systemd1";
const MANAGER: &str = "org.freedesktop.systemd1.Manager";
pub const UNIT: &str = "rquickshare-desktop.service";
const UNIT_PATH: &str = "/org/freedesktop/systemd1/unit/rquickshare_2ddesktop_2eservice";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Unavailable,
    Unsupported,
    Protocol,
    Conflict,
    Denied,
    Uncertain,
    Closed,
    Busy,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct View {
    pub enabled: Option<bool>,
    pub busy: bool,
    pub error: Option<Error>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            enabled: None,
            busy: false,
            error: Some(Error::Unavailable),
        }
    }
}
type Result<T> = std::result::Result<T, Error>;
fn checked(message: &Message, owner: &str) -> Result<()> {
    if message
        .header()
        .sender()
        .is_none_or(|sender| sender.as_str() != owner)
        || !message.data().fds().is_empty()
    {
        Err(Error::Protocol)
    } else {
        Ok(())
    }
}
async fn broker<B: serde::Serialize + zbus::zvariant::Type>(
    connection: &Connection,
    member: &str,
    body: &B,
) -> Result<Message> {
    match connection
        .call_method(
            Some(BROKER),
            "/org/freedesktop/DBus",
            Some(BROKER),
            member,
            body,
        )
        .await
    {
        Ok(reply) => {
            checked(&reply, BROKER)?;
            Ok(reply)
        }
        Err(zbus::Error::MethodError(_, _, reply)) => {
            checked(&reply, BROKER)?;
            Err(Error::Unavailable)
        }
        Err(_) => Err(Error::Unavailable),
    }
}
async fn owner(connection: &Connection) -> Result<String> {
    let reply = broker(connection, "GetNameOwner", &SERVICE).await?;
    let name: String = reply.body().deserialize().map_err(|_| Error::Protocol)?;
    zbus::names::UniqueName::try_from(name.as_str()).map_err(|_| Error::Protocol)?;
    Ok(name)
}
struct Manager<'a> {
    connection: &'a Connection,
    owner: &'a str,
}
impl Manager<'_> {
    async fn call<B: serde::Serialize + zbus::zvariant::Type>(
        &self,
        path: &str,
        interface: &str,
        member: &str,
        body: &B,
    ) -> Result<Message> {
        if owner(self.connection).await?.as_str() != self.owner {
            return Err(Error::Closed);
        }
        let reply = match self
            .connection
            .call_method(Some(self.owner), path, Some(interface), member, body)
            .await
        {
            Ok(reply) => reply,
            Err(zbus::Error::MethodError(name, _, reply)) => {
                checked(&reply, self.owner)?;
                return Err(match name.as_str() {
                    "org.freedesktop.DBus.Error.AccessDenied"
                    | "org.freedesktop.systemd1.AccessDenied" => Error::Denied,
                    "org.freedesktop.systemd1.NoSuchUnit"
                    | "org.freedesktop.systemd1.UnitMasked" => Error::Unsupported,
                    _ => Error::Unavailable,
                });
            }
            Err(_) => return Err(Error::Unavailable),
        };
        checked(&reply, self.owner)?;
        if owner(self.connection).await?.as_str() != self.owner {
            return Err(Error::Closed);
        }
        Ok(reply)
    }
    async fn read(&self) -> Result<bool> {
        let reply = self.call(PATH, MANAGER, "LoadUnit", &UNIT).await?;
        let unit: OwnedObjectPath = reply.body().deserialize().map_err(|_| Error::Protocol)?;
        if unit.as_str() != UNIT_PATH {
            return Err(Error::Protocol);
        }
        let reply = self
            .call(
                UNIT_PATH,
                "org.freedesktop.DBus.Properties",
                "GetAll",
                &"org.freedesktop.systemd1.Unit",
            )
            .await?;
        let mut properties: HashMap<String, OwnedValue> =
            reply.body().deserialize().map_err(|_| Error::Protocol)?;
        let fragment = String::try_from(properties.remove("FragmentPath").ok_or(Error::Protocol)?)
            .map_err(|_| Error::Protocol)?;
        let dropins =
            Vec::<String>::try_from(properties.remove("DropInPaths").ok_or(Error::Protocol)?)
                .map_err(|_| Error::Protocol)?;
        let loaded = String::try_from(properties.remove("LoadState").ok_or(Error::Protocol)?)
            .map_err(|_| Error::Protocol)?;
        if !matches!(
            fragment.as_str(),
            "/usr/lib/systemd/user/rquickshare-desktop.service"
                | "/lib/systemd/user/rquickshare-desktop.service"
        ) || !dropins.is_empty()
            || loaded != "loaded"
        {
            return Err(Error::Unsupported);
        }
        let reply = self.call(PATH, MANAGER, "GetUnitFileState", &UNIT).await?;
        let state: String = reply.body().deserialize().map_err(|_| Error::Protocol)?;
        match state.as_str() {
            "enabled" => Ok(true),
            "disabled" => Ok(false),
            _ => Err(Error::Unsupported),
        }
    }
    async fn set(&self, expected: bool, desired: bool) -> Result<bool> {
        let current = self.read().await?;
        if current != expected {
            return Err(Error::Conflict);
        }
        if current == desired {
            return Ok(current);
        }
        let changes = if desired {
            let reply = self
                .call(
                    PATH,
                    MANAGER,
                    "EnableUnitFiles",
                    &(vec![UNIT], false, false),
                )
                .await
                .map_err(uncertain)?;
            let (install, changes): (bool, Vec<(String, String, String)>) =
                reply.body().deserialize().map_err(|_| Error::Uncertain)?;
            if !install {
                return Err(Error::Unsupported);
            }
            changes
        } else {
            let reply = self
                .call(PATH, MANAGER, "DisableUnitFiles", &(vec![UNIT], false))
                .await
                .map_err(uncertain)?;
            reply
                .body()
                .deserialize::<Vec<(String, String, String)>>()
                .map_err(|_| Error::Uncertain)?
        };
        if changes.len() > 256
            || changes.iter().any(|(kind, path, target)| {
                !matches!(kind.as_str(), "symlink" | "unlink")
                    || path.len() > 4096
                    || target.len() > 4096
            })
        {
            return Err(Error::Uncertain);
        }
        let actual = self.read().await.map_err(uncertain)?;
        if actual != desired {
            return Err(Error::Conflict);
        }
        Ok(actual)
    }
}
fn uncertain(error: Error) -> Error {
    match error {
        Error::Denied | Error::Closed => error,
        _ => Error::Uncertain,
    }
}
enum Action {
    Read,
    Set(bool, bool),
}
struct Request {
    action: Action,
    reply: oneshot::Sender<Result<bool>>,
}
#[derive(Clone)]
pub struct Handle {
    commands: mpsc::Sender<Request>,
    state: watch::Receiver<View>,
}
pub struct Completion(oneshot::Receiver<Result<bool>>);
impl Completion {
    pub async fn wait(self) -> Result<bool> {
        self.0.await.map_err(|_| Error::Closed)?
    }
}
impl Handle {
    pub fn subscribe(&self) -> watch::Receiver<View> {
        self.state.clone()
    }
    pub fn read(&self) -> Result<Completion> {
        self.submit(Action::Read)
    }
    pub fn set(&self, expected: bool, desired: bool) -> Result<Completion> {
        self.submit(Action::Set(expected, desired))
    }
    fn submit(&self, action: Action) -> Result<Completion> {
        let (reply, receive) = oneshot::channel();
        self.commands
            .try_send(Request { action, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Error::Busy,
                mpsc::error::TrySendError::Closed(_) => Error::Closed,
            })?;
        Ok(Completion(receive))
    }
}
pub struct Owner {
    stop: CancellationToken,
    task: Option<JoinHandle<()>>,
    handle: Handle,
}
impl Owner {
    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }
    pub async fn connect(address: &BusAddress, cancel: CancellationToken) -> Self {
        let stop = cancel.child_token();
        let (commands, mut receive) = mpsc::channel::<Request>(8);
        let (updates, state) = watch::channel(View::default());
        let handle = Handle { commands, state };
        let connected = tokio::select! {biased;
            _=stop.cancelled()=>Err(Error::Closed),
            result=async {connection_builder(address.connect().await.map_err(|_|Error::Unavailable)?).build().await.map_err(|_|Error::Unavailable)}=>result,
        };
        let Ok(connection) = connected else {
            return Self {
                stop,
                task: None,
                handle,
            };
        };
        let setup = async {
            let name = owner(&connection).await?;
            let reply = broker(&connection, "GetConnectionUnixUser", &name).await?;
            let uid: u32 = reply.body().deserialize().map_err(|_| Error::Protocol)?;
            if uid != rustix::process::geteuid().as_raw() {
                return Err(Error::Denied);
            }
            let changes_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .sender(name.as_str())
                .map_err(|_| Error::Protocol)?
                .path(PATH)
                .map_err(|_| Error::Protocol)?
                .interface(MANAGER)
                .map_err(|_| Error::Protocol)?
                .member("UnitFilesChanged")
                .map_err(|_| Error::Protocol)?
                .build();
            let changes = MessageStream::for_match_rule(changes_rule, &connection, Some(8))
                .await
                .map_err(|_| Error::Unavailable)?;
            let owner_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .sender(BROKER)
                .map_err(|_| Error::Protocol)?
                .path("/org/freedesktop/DBus")
                .map_err(|_| Error::Protocol)?
                .interface(BROKER)
                .map_err(|_| Error::Protocol)?
                .member("NameOwnerChanged")
                .map_err(|_| Error::Protocol)?
                .arg(0, SERVICE)
                .map_err(|_| Error::Protocol)?
                .build();
            let owners = MessageStream::for_match_rule(owner_rule, &connection, Some(8))
                .await
                .map_err(|_| Error::Unavailable)?;
            let manager = Manager {
                connection: &connection,
                owner: &name,
            };
            manager
                .call(PATH, MANAGER, "Subscribe", &())
                .await?
                .body()
                .deserialize::<()>()
                .map_err(|_| Error::Protocol)?;
            Ok::<_, Error>((name, changes, owners))
        };
        let setup =
            tokio::select! {biased;_=stop.cancelled()=>Err(Error::Closed),result=setup=>result};
        let (name, mut changes, mut owners) = match setup {
            Ok(value) => value,
            Err(error) => {
                updates.send_replace(View {
                    error: Some(error),
                    ..View::default()
                });
                let _ = connection.close().await;
                return Self {
                    stop,
                    task: None,
                    handle,
                };
            }
        };
        let end = stop.clone();
        let task = tokio::spawn(async move {
            let manager = Manager {
                connection: &connection,
                owner: &name,
            };
            let mut refresh = true;
            loop {
                let request = if refresh {
                    refresh = false;
                    None
                } else {
                    tokio::select! {biased;
                        _=end.cancelled()=>break,
                        _=owners.next()=>break,
                        signal=changes.next()=>{
                            if !signal.as_ref().is_some_and(|signal|signal.as_ref().is_ok_and(|message|checked(message,&name).is_ok()&&message.body().deserialize::<()>().is_ok())){break}
                            refresh=true;continue;
                        }
                        request=receive.recv()=>match request {Some(request)=>Some(request),None=>break},
                    }
                };
                updates.send_modify(|state| state.busy = true);
                let work = async {
                    match request.as_ref().map(|r| &r.action) {
                        Some(Action::Set(expected, desired)) => {
                            manager.set(*expected, *desired).await
                        }
                        _ => manager.read().await,
                    }
                };
                let result = tokio::select! {biased;
                    _=end.cancelled()=>Err(Error::Closed),
                    _=owners.next()=>Err(Error::Closed),
                    result=work=>result,
                };
                updates.send_replace(match result {
                    Ok(enabled) => View {
                        enabled: Some(enabled),
                        busy: false,
                        error: None,
                    },
                    Err(error) => View {
                        enabled: None,
                        busy: false,
                        error: Some(error),
                    },
                });
                if let Some(request) = request {
                    let _ = request.reply.send(result);
                }
                if result == Err(Error::Closed) {
                    break;
                }
            }
            receive.close();
            while let Ok(request) = receive.try_recv() {
                let _ = request.reply.send(Err(Error::Closed));
            }
            updates.send_replace(View {
                error: Some(Error::Closed),
                ..View::default()
            });
            drop(changes);
            drop(owners);
            let _ = connection.close().await;
        });
        Self {
            stop,
            task: Some(task),
            handle,
        }
    }
    pub async fn close(mut self) {
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
#[cfg(test)]
mod tests;
