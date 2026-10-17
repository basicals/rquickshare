//! Read-only KDE safety observations. Missing or replaced providers are unknown.
use futures::StreamExt;
use rqs_lib::service::process::BusAddress;
use tokio::{sync::watch, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use zbus::{
    fdo, message::Type, zvariant::OwnedValue, Connection, MatchRule, Message, MessageStream,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SafetyState {
    pub locked: Option<bool>,
    pub inhibited: Option<bool>,
    pub closed: bool,
}
impl SafetyState {
    pub fn manual_allowed(self) -> bool {
        !self.closed && self.locked == Some(false)
    }
    pub fn automatic_allowed(self) -> bool {
        self.manual_allowed() && self.inhibited == Some(false)
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Lock,
    Inhibition,
}
impl Kind {
    fn service(self) -> &'static str {
        match self {
            Self::Lock => "org.freedesktop.ScreenSaver",
            Self::Inhibition => "org.freedesktop.Notifications",
        }
    }
    fn path(self) -> &'static str {
        match self {
            Self::Lock => "/ScreenSaver",
            Self::Inhibition => "/org/freedesktop/Notifications",
        }
    }
    fn interface(self) -> &'static str {
        match self {
            Self::Lock => self.service(),
            Self::Inhibition => "org.freedesktop.DBus.Properties",
        }
    }
    fn member(self) -> &'static str {
        match self {
            Self::Lock => "ActiveChanged",
            Self::Inhibition => "PropertiesChanged",
        }
    }
    fn publish(self, updates: &watch::Sender<SafetyState>, value: Option<bool>) {
        updates.send_modify(|state| match self {
            Self::Lock => state.locked = value,
            Self::Inhibition => state.inhibited = value,
        });
    }
}
struct Provider {
    owner: String,
    signals: MessageStream,
}

fn sender(message: &Message, owner: &str) -> bool {
    message
        .header()
        .sender()
        .is_some_and(|sender| sender.as_str() == owner)
}
fn relevant_signal(message: &Message, owner: &str, kind: Kind) -> Result<bool, ()> {
    let header = message.header();
    if message.message_type() != Type::Signal
        || !sender(message, owner)
        || header
            .path()
            .is_none_or(|path| path.as_str() != kind.path())
        || header
            .interface()
            .is_none_or(|interface| interface.as_str() != kind.interface())
        || header
            .member()
            .is_none_or(|member| member.as_str() != kind.member())
    {
        return Err(());
    }
    match kind {
        Kind::Lock => message
            .body()
            .deserialize::<bool>()
            .map(|_| true)
            .map_err(|_| ()),
        Kind::Inhibition => {
            let (interface, changed, invalidated) = message
                .body()
                .deserialize::<(
                    String,
                    std::collections::BTreeMap<String, OwnedValue>,
                    Vec<String>,
                )>()
                .map_err(|_| ())?;
            if interface != kind.service() {
                return Ok(false);
            }
            if let Some(value) = changed.get("Inhibited") {
                bool::try_from(value).map_err(|_| ())?;
                return Ok(true);
            }
            Ok(invalidated.iter().any(|name| name == "Inhibited"))
        }
    }
}
async fn current_owner(connection: &Connection, kind: Kind) -> Option<String> {
    let broker = fdo::DBusProxy::new(connection).await.ok()?;
    Some(
        broker
            .get_name_owner(kind.service().try_into().ok()?)
            .await
            .ok()?
            .to_string(),
    )
}
async fn read(connection: &Connection, provider: &Provider, kind: Kind) -> Option<bool> {
    let reply = match kind {
        Kind::Lock => connection
            .call_method(
                Some(provider.owner.as_str()),
                kind.path(),
                Some(kind.service()),
                "GetActive",
                &(),
            )
            .await
            .ok()?,
        Kind::Inhibition => connection
            .call_method(
                Some(provider.owner.as_str()),
                kind.path(),
                Some(kind.interface()),
                "Get",
                &(kind.service(), "Inhibited"),
            )
            .await
            .ok()?,
    };
    if !sender(&reply, &provider.owner) {
        return None;
    }
    let value = match kind {
        Kind::Lock => reply.body().deserialize::<bool>().ok()?,
        Kind::Inhibition => bool::try_from(reply.body().deserialize::<OwnedValue>().ok()?).ok()?,
    };
    (current_owner(connection, kind).await.as_deref() == Some(provider.owner.as_str()))
        .then_some(value)
}
async fn resolve(connection: &Connection, kind: Kind, uid: u32) -> Option<Provider> {
    let owner = current_owner(connection, kind).await?;
    let broker = fdo::DBusProxy::new(connection).await.ok()?;
    if broker
        .get_connection_unix_user(owner.as_str().try_into().ok()?)
        .await
        .ok()?
        != uid
    {
        return None;
    }
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(owner.as_str())
        .ok()?
        .path(kind.path())
        .ok()?
        .interface(kind.interface())
        .ok()?
        .member(kind.member())
        .ok()?
        .build();
    let signals = MessageStream::for_match_rule(rule, connection, Some(8))
        .await
        .ok()?;
    Some(Provider { owner, signals })
}
async fn next(provider: &mut Option<Provider>) -> Option<zbus::Result<Message>> {
    match provider {
        Some(provider) => provider.signals.next().await,
        None => std::future::pending().await,
    }
}

async fn monitor(
    connection: Connection,
    kind: Kind,
    uid: u32,
    updates: watch::Sender<SafetyState>,
    stop: CancellationToken,
) {
    let run = async {
        let broker = fdo::DBusProxy::new(&connection).await?;
        let mut owners = broker
            .receive_name_owner_changed_with_args(&[(0, kind.service())])
            .await?;
        let mut provider = resolve(&connection, kind, uid).await;
        if let Some(provider) = &provider {
            kind.publish(&updates, read(&connection, provider, kind).await);
        }
        loop {
            tokio::select! {
                biased;
                _=stop.cancelled()=>break,
                owner=owners.next()=>{
                    let Some(owner)=owner else {break};
                    if !sender(owner.message(),"org.freedesktop.DBus") {break;}
                    kind.publish(&updates,None);
                    provider=resolve(&connection,kind,uid).await;
                    if let Some(provider)=&provider {kind.publish(&updates,read(&connection,provider,kind).await);}
                },
                signal=next(&mut provider)=>{
                    match (signal,provider.as_ref()) {
                        (Some(Ok(message)),Some(current)) => match relevant_signal(&message,&current.owner,kind) {
                            Ok(true)=>{kind.publish(&updates,None);kind.publish(&updates,read(&connection,current,kind).await);},
                            Ok(false)=>(),
                            Err(())=>{kind.publish(&updates,None);provider=None;},
                        },
                        _=>{kind.publish(&updates,None);provider=None;},
                    }
                }
            }
        }
        Ok::<_, zbus::Error>(())
    };
    // Dropping an in-flight read on shutdown does not mutate any desktop state.
    tokio::select! {biased;_=stop.cancelled()=>(),_=run=>()}
    kind.publish(&updates, None);
    let _ = connection.close().await;
}

pub struct DesktopGuard {
    pub updates: watch::Receiver<SafetyState>,
    state: watch::Sender<SafetyState>,
    stop: CancellationToken,
    tasks: Vec<JoinHandle<()>>,
}
impl DesktopGuard {
    pub async fn connect(address: &BusAddress) -> zbus::Result<Self> {
        Self::connect_with_uid(address, rustix::process::geteuid().as_raw()).await
    }
    async fn connect_with_uid(address: &BusAddress, uid: u32) -> zbus::Result<Self> {
        let mut connections = Vec::new();
        for _ in 0..2 {
            let connection =
                async {
                    rqs_lib::service::bus::connection_builder(address.connect().await.map_err(
                        |_| zbus::Error::Failure("Desktop safety bus unavailable".into()),
                    )?)
                    .build()
                    .await
                }
                .await;
            match connection {
                Ok(connection) => connections.push(connection),
                Err(error) => {
                    for connection in connections {
                        let _ = connection.close().await;
                    }
                    return Err(error);
                }
            }
        }
        let (state, updates) = watch::channel(SafetyState::default());
        let stop = CancellationToken::new();
        let tasks = connections
            .into_iter()
            .zip([Kind::Lock, Kind::Inhibition])
            .map(|(connection, kind)| {
                tokio::spawn(monitor(connection, kind, uid, state.clone(), stop.clone()))
            })
            .collect();
        Ok(Self {
            updates,
            state,
            stop,
            tasks,
        })
    }
    pub async fn close(mut self) {
        self.state.send_modify(|state| state.closed = true);
        self.stop.cancel();
        for task in self.tasks.drain(..) {
            let _ = task.await;
        }
    }
}
impl Drop for DesktopGuard {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(test)]
mod tests;
