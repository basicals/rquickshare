//! Owned KDE tray registration. Coordinates are presentation hints only.

use futures::StreamExt;
use rqs_lib::service::process::BusAddress;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use zbus::{fdo, message::Header, Connection};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const PATH: &str = "/StatusNotifierItem";
type Pixmaps = Vec<(i32, i32, Vec<u8>)>;
type ToolTip<'a> = (&'a str, Pixmaps, &'a str, &'a str);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TrayState {
    pub activation: Option<(i32, i32)>,
    pub failed: bool,
}

struct Item {
    updates: watch::Sender<TrayState>,
}
impl Item {
    async fn activate_checked(
        &self,
        x: i32,
        y: i32,
        connection: &Connection,
        header: Header<'_>,
    ) -> fdo::Result<()> {
        let sender = header
            .sender()
            .ok_or_else(|| fdo::Error::AccessDenied("Missing sender".into()))?;
        let uid = fdo::DBusProxy::new(connection)
            .await?
            .get_connection_unix_user(sender.clone().into())
            .await?;
        if !same_user(uid, rustix::process::geteuid().as_raw()) {
            return Err(fdo::Error::AccessDenied("Different user".into()));
        }
        self.updates.send_modify(|state| {
            if !state.failed {
                state.activation = Some((x, y));
            }
        });
        Ok(())
    }
}

fn same_user(actual: u32, expected: u32) -> bool {
    actual == expected
}

#[zbus::interface(name = "org.kde.StatusNotifierItem", spawn = false)]
impl Item {
    async fn activate(
        &self,
        x: i32,
        y: i32,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        self.activate_checked(x, y, connection, header).await
    }
    async fn secondary_activate(
        &self,
        x: i32,
        y: i32,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        self.activate_checked(x, y, connection, header).await
    }
    async fn context_menu(
        &self,
        x: i32,
        y: i32,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        self.activate_checked(x, y, connection, header).await
    }
    #[zbus(property)]
    fn category(&self) -> &str {
        "Communications"
    }
    #[zbus(property)]
    fn id(&self) -> &str {
        "rquickshare"
    }
    #[zbus(property)]
    fn title(&self) -> &str {
        "rQuickShare"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "Active"
    }
    #[zbus(property)]
    fn icon_name(&self) -> &str {
        "network-wireless-symbolic"
    }
    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn tool_tip(&self) -> ToolTip<'_> {
        (
            "network-wireless-symbolic",
            Vec::new(),
            "rQuickShare",
            "Send and receive nearby files",
        )
    }
}

pub struct Tray {
    stop: CancellationToken,
    task: Option<JoinHandle<zbus::Result<()>>>,
    pub updates: watch::Receiver<TrayState>,
}
impl Tray {
    pub async fn connect(address: &BusAddress) -> zbus::Result<Self> {
        let stream = address
            .connect()
            .await
            .map_err(|_| zbus::Error::Failure("Tray bus unavailable".into()))?;
        let (updates, receiver) = watch::channel(TrayState::default());
        let connection = rqs_lib::service::bus::connection_builder(stream)
            .serve_at(
                PATH,
                Item {
                    updates: updates.clone(),
                },
            )?
            .build()
            .await?;
        let setup = async {
            let broker = fdo::DBusProxy::new(&connection).await?;
            // Subscribe before resolving/registering; the owner is pinned until restart.
            let changes = broker
                .receive_name_owner_changed_with_args(&[(0, WATCHER)])
                .await?;
            let owner = broker.get_name_owner(WATCHER.try_into()?).await?;
            if !same_user(
                broker
                    .get_connection_unix_user(owner.clone().into())
                    .await?,
                rustix::process::geteuid().as_raw(),
            ) {
                return Err(zbus::Error::Failure(
                    "Tray watcher belongs to another user".into(),
                ));
            }
            let watcher = zbus::Proxy::new(
                &connection,
                owner.as_str(),
                "/StatusNotifierWatcher",
                WATCHER,
            )
            .await?;
            watcher
                .call::<_, _, ()>(
                    "RegisterStatusNotifierItem",
                    &connection
                        .unique_name()
                        .ok_or_else(|| zbus::Error::Failure("Missing tray identity".into()))?
                        .as_str(),
                )
                .await?;
            if broker.get_name_owner(WATCHER.try_into()?).await? != owner {
                return Err(zbus::Error::Failure("Tray watcher changed".into()));
            }
            Ok(changes)
        }
        .await;
        let mut changes = match setup {
            Ok(changes) => changes,
            Err(error) => {
                let _ = connection.close().await;
                return Err(error);
            }
        };
        let stop = CancellationToken::new();
        let task_stop = stop.clone();
        let task = tokio::spawn(async move {
            tokio::select! {
                _ = task_stop.cancelled() => (),
                _ = changes.next() => { updates.send_modify(|state| state.failed = true); }
            }
            drop(changes);
            // Disconnecting the registered unique name unregisters the item.
            connection.close().await
        });
        Ok(Self {
            stop,
            task: Some(task),
            updates: receiver,
        })
    }
    pub async fn close(mut self) -> zbus::Result<()> {
        self.stop.cancel();
        self.task
            .take()
            .expect("owned tray task")
            .await
            .map_err(|_| zbus::Error::Failure("Tray task failed".into()))?
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(test)]
mod tests;
