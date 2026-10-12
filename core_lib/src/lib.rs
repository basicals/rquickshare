#[macro_use]
extern crate log;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::anyhow;
use channel::ChannelMessage;
use hdl::MDnsDiscovery;
#[cfg(all(feature = "experimental", target_os = "linux"))]
use hdl::{receiver_service_data, BleAdvertiser, ReceiverAdvertiser, ReceiverGattServer};
use once_cell::sync::Lazy;
use rand::distr::Alphanumeric;
use rand::Rng;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[cfg(feature = "experimental")]
use crate::hdl::BleListener;
use crate::hdl::MDnsServer;
use crate::manager::TcpServer;

pub mod channel;
mod errors;
mod file_reader;
#[cfg(target_os = "linux")]
pub mod file_source;
mod hdl;
mod manager;
mod protocol;
mod utils;

pub use hdl::{
    EndpointInfo, ManagedEphemeralFile, OutboundPayload, State, Visibility,
    MANAGED_EPHEMERAL_FILE_PREFIX,
};
pub use manager::SendInfo;
pub use utils::DeviceType;

pub mod sharing_nearby {
    include!(concat!(env!("OUT_DIR"), "/sharing.nearby.rs"));
}

pub mod securemessage {
    include!(concat!(env!("OUT_DIR"), "/securemessage.rs"));
}

pub mod securegcm {
    include!(concat!(env!("OUT_DIR"), "/securegcm.rs"));
}

pub mod location_nearby_connections {
    include!(concat!(env!("OUT_DIR"), "/location.nearby.connections.rs"));
}

static CUSTOM_DOWNLOAD: Lazy<RwLock<Option<PathBuf>>> = Lazy::new(|| RwLock::new(None));

#[derive(Debug)]
pub struct RQS {
    tracker: Option<TaskTracker>,
    ctoken: Option<CancellationToken>,
    // Discovery token is different than ctoken because he is on his own
    // - can be cancelled while the ctoken is still active
    discovery_ctk: Option<CancellationToken>,

    // Used to trigger a change in the mDNS visibility (and later on, BLE)
    pub visibility_sender: Arc<Mutex<watch::Sender<Visibility>>>,
    visibility_receiver: watch::Receiver<Visibility>,

    // Only used to send the info "a nearby device is sharing"
    ble_sender: broadcast::Sender<()>,

    pub foreground_sender: watch::Sender<bool>,
    foreground_receiver: watch::Receiver<bool>,

    port_number: Option<u32>,

    pub message_sender: broadcast::Sender<ChannelMessage>,
}

impl Default for RQS {
    fn default() -> Self {
        Self::new(Visibility::Visible, None, None)
    }
}

impl RQS {
    pub fn new(
        visibility: Visibility,
        port_number: Option<u32>,
        download_path: Option<PathBuf>,
    ) -> Self {
        match CUSTOM_DOWNLOAD.write() {
            Ok(mut guard) => *guard = download_path,
            Err(_) => error!("CUSTOM_DOWNLOAD lock is poisoned during initialization"),
        }

        let (message_sender, _) = broadcast::channel(50);
        let (ble_sender, _) = broadcast::channel(5);

        // Define default visibility as per the args inside the new()
        let (visibility_sender, visibility_receiver) = watch::channel(Visibility::Invisible);
        let _ = visibility_sender.send(visibility);
        let (foreground_sender, foreground_receiver) = watch::channel(true);

        Self {
            tracker: None,
            ctoken: None,
            discovery_ctk: None,
            visibility_sender: Arc::new(Mutex::new(visibility_sender)),
            visibility_receiver,
            ble_sender,
            foreground_sender,
            foreground_receiver,
            port_number,
            message_sender,
        }
    }

    pub async fn run(
        &mut self,
    ) -> Result<(mpsc::Sender<SendInfo>, broadcast::Receiver<()>), anyhow::Error> {
        let tracker = TaskTracker::new();
        let ctoken = CancellationToken::new();
        self.tracker = Some(tracker.clone());
        self.ctoken = Some(ctoken.clone());

        let endpoint_id: Vec<u8> = rand::rng().sample_iter(Alphanumeric).take(4).collect();
        let tcp_listener =
            TcpListener::bind(format!("0.0.0.0:{}", self.port_number.unwrap_or(0))).await?;
        let binded_addr = tcp_listener.local_addr()?;
        info!("TcpListener on: {}", binded_addr);

        // MPSC for the TcpServer
        let send_channel = mpsc::channel(10);
        let bwu_router = crate::hdl::BwuRouter::new();

        // Start TcpServer in own "task". The same listener also accepts
        // Wi-Fi bandwidth-upgrade sockets routed from active BLE sessions.
        let mut server = TcpServer::new(
            endpoint_id[..4].try_into()?,
            tcp_listener,
            self.message_sender.clone(),
            send_channel.1,
            bwu_router.clone(),
        )?;
        let ctk = ctoken.clone();
        tracker.spawn(async move { server.run(ctk).await });

        #[cfg(feature = "experimental")]
        {
            // Don't threat BleListener error as fatal, it's a nice to have.
            if let Ok(ble) = BleListener::new(self.ble_sender.clone()).await {
                let ctk = ctoken.clone();
                let foreground_rx = self.foreground_receiver.clone();
                tracker.spawn(async move {
                    if let Err(error) = ble.run(ctk, foreground_rx).await {
                        error!("BleListener stopped with error: {error:#}");
                    }
                });
            }
        }

        // Start MDnsServer in own "task"
        let (receiver_mdns_refresh_sender, receiver_mdns_refresh_receiver) =
            broadcast::channel::<()>(8);
        let mut mdns = MDnsServer::new(
            endpoint_id[..4].try_into()?,
            binded_addr.port(),
            self.ble_sender.subscribe(),
            receiver_mdns_refresh_receiver,
            self.visibility_sender.clone(),
            self.visibility_receiver.clone(),
        )?;
        let ctk = ctoken.clone();
        tracker.spawn(async move { mdns.run(ctk).await });

        #[cfg(all(feature = "experimental", target_os = "linux"))]
        {
            let endpoint_id: [u8; 4] = endpoint_id[..4].try_into()?;
            let hostname = sys_metrics::host::get_hostname()?;
            let receiver_advertisement = receiver_service_data(
                endpoint_id,
                crate::utils::DeviceType::Laptop as u8,
                &hostname,
            );

            let visibility_rx = self.visibility_receiver.clone();
            let ctk = ctoken.clone();
            tracker.spawn(async move {
                let blea = match BleAdvertiser::new(visibility_rx).await {
                    Ok(advertiser) => advertiser,
                    Err(error) => {
                        error!("Couldn't init BleAdvertiser: {error}");
                        return;
                    }
                };

                if let Err(error) = blea.run(ctk).await {
                    error!("BleAdvertiser stopped with error: {error}");
                }
            });

            let (receiver_adv_refresh_sender, _) =
                broadcast::channel::<crate::hdl::ReceiverAdvertisingRefresh>(8);

            let visibility_rx = self.visibility_receiver.clone();
            let receiver_ctk = ctoken.clone();
            let advertiser_refresh_sender = receiver_adv_refresh_sender.clone();
            tracker.spawn(async move {
                match ReceiverAdvertiser::new(visibility_rx, advertiser_refresh_sender).await {
                    Ok(advertiser) => {
                        if let Err(error) = advertiser.run(receiver_ctk).await {
                            error!("ReceiverAdvertiser stopped with error: {error}");
                        }
                    }
                    Err(error) => error!("Couldn't init ReceiverAdvertiser: {error}"),
                }
            });

            let gatt_ctk = ctoken.clone();
            let gatt_sender = self.message_sender.clone();
            let gatt_bwu_router = bwu_router.clone();
            let gatt_tcp_port = binded_addr.port();
            tracker.spawn(async move {
                match ReceiverGattServer::new(
                    receiver_advertisement,
                    gatt_sender,
                    gatt_tcp_port,
                    gatt_bwu_router,
                    receiver_adv_refresh_sender,
                    receiver_mdns_refresh_sender,
                )
                .await
                {
                    Ok(server) => {
                        if let Err(error) = server.run(gatt_ctk).await {
                            error!("ReceiverGattServer stopped with error: {error}");
                        }
                    }
                    Err(error) => error!("Couldn't init ReceiverGattServer: {error}"),
                }
            });
        }

        tracker.close();

        Ok((send_channel.0, self.ble_sender.subscribe()))
    }

    pub fn discovery(
        &mut self,
        sender: broadcast::Sender<EndpointInfo>,
    ) -> Result<(), anyhow::Error> {
        let tracker = self
            .tracker
            .as_ref()
            .ok_or_else(|| anyhow!("The service wasn't first started"))?;

        let ctk = CancellationToken::new();
        self.discovery_ctk = Some(ctk.clone());

        let discovery = MDnsDiscovery::new(sender)?;
        tracker.spawn(async move { discovery.run(ctk.clone()).await });

        Ok(())
    }

    pub fn stop_discovery(&mut self) {
        if let Some(discovert_ctk) = &self.discovery_ctk {
            discovert_ctk.cancel();
            self.discovery_ctk = None;
        }
    }

    pub fn change_visibility(&mut self, nv: Visibility) {
        match self.visibility_sender.lock() {
            Ok(sender) => sender.send_modify(|state| *state = nv),
            Err(_) => error!("visibility sender lock is poisoned"),
        }
    }

    pub fn set_foreground(&self, foreground: bool) {
        let _ = self.foreground_sender.send(foreground);
    }

    pub async fn stop(&mut self) {
        self.stop_discovery();

        if let Some(ctoken) = &self.ctoken {
            ctoken.cancel();
        }

        if let Some(tracker) = &self.tracker {
            tracker.wait().await;
        }

        self.ctoken = None;
        self.tracker = None;
    }

    // Setting None here will resume the default settings
    pub fn set_download_path(&self, p: Option<PathBuf>) {
        debug!("Setting the download path to {:?}", p);
        match CUSTOM_DOWNLOAD.write() {
            Ok(mut guard) => *guard = p,
            Err(_) => error!("CUSTOM_DOWNLOAD lock is poisoned while changing path"),
        }
    }
}
