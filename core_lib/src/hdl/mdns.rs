use std::sync::{Arc, Mutex};
use std::time::Duration;

use mdns_sd::{AddrType, ServiceDaemon, ServiceInfo};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Receiver;
use tokio::sync::watch;
use tokio::time::{interval_at, Instant};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::utils::{gen_mdns_endpoint_info, gen_mdns_name, DeviceType};

const INNER_NAME: &str = "MDnsServer";
const TICK_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Visibility {
    Visible = 0,
    Invisible = 1,
    Temporarily = 2,
}

#[allow(dead_code)]
impl Visibility {
    pub fn from_raw_value(value: u64) -> Self {
        Self::try_from_raw_value(value).unwrap_or(Self::Invisible)
    }

    pub fn try_from_raw_value(value: u64) -> Option<Self> {
        match value {
            0 => Some(Visibility::Visible),
            1 => Some(Visibility::Invisible),
            2 => Some(Visibility::Temporarily),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReceiverRefreshAction {
    Resend,
    Disable,
}

fn receiver_refresh_action(
    result: Result<(), tokio::sync::broadcast::error::RecvError>,
) -> ReceiverRefreshAction {
    match result {
        Ok(()) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
            ReceiverRefreshAction::Resend
        }
        Err(tokio::sync::broadcast::error::RecvError::Closed) => ReceiverRefreshAction::Disable,
    }
}

pub struct MDnsServer {
    daemon: ServiceDaemon,
    service_info: ServiceInfo,
    ble_receiver: Receiver<()>,
    receiver_refresh_receiver: Receiver<()>,
    visibility_sender: Arc<Mutex<watch::Sender<Visibility>>>,
    visibility_receiver: watch::Receiver<Visibility>,
}

impl MDnsServer {
    pub fn new(
        endpoint_id: [u8; 4],
        service_port: u16,
        ble_receiver: Receiver<()>,
        receiver_refresh_receiver: Receiver<()>,
        visibility_sender: Arc<Mutex<watch::Sender<Visibility>>>,
        visibility_receiver: watch::Receiver<Visibility>,
    ) -> Result<Self, anyhow::Error> {
        let service_info = Self::build_service(endpoint_id, service_port, DeviceType::Laptop)?;

        Ok(Self {
            daemon: ServiceDaemon::new()?,
            service_info,
            ble_receiver,
            receiver_refresh_receiver,
            visibility_sender,
            visibility_receiver,
        })
    }

    pub async fn run(&mut self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        info!("{INNER_NAME}: service starting");
        let monitor = self.daemon.monitor()?;
        let ble_receiver = &mut self.ble_receiver;
        let receiver_refresh_receiver = &mut self.receiver_refresh_receiver;
        let mut visibility = *self.visibility_receiver.borrow();
        let mut interval = interval_at(Instant::now() + TICK_INTERVAL, TICK_INTERVAL);
        let mut receiver_refresh_open = true;

        loop {
            tokio::select! {
                _ = ctk.cancelled() => {
                    info!("{INNER_NAME}: tracker cancelled, breaking");
                    break;
                }
                r = monitor.recv_async() => {
                    match r {
                        Ok(_) => continue,
                        Err(err) => return Err(err.into()),
                    }
                },
                _ = self.visibility_receiver.changed() => {
                    visibility = *self.visibility_receiver.borrow_and_update();

                    debug!("{INNER_NAME}: visibility changed: {visibility:?}");
                    if visibility == Visibility::Visible {
                        self.daemon.register(self.service_info.clone())?;
                    } else if visibility == Visibility::Invisible {
                        let receiver = self.daemon.unregister(self.service_info.get_fullname())?;
                        let _ = receiver.recv();
                    } else if visibility == Visibility::Temporarily {
                        self.daemon.register(self.service_info.clone())?;
                        interval.reset();
                    }
                }
                _ = ble_receiver.recv() => {
                    if visibility == Visibility::Invisible {
                        continue;
                    }

                    debug!("{INNER_NAME}: ble_receiver: got event");
                    if visibility == Visibility::Visible || visibility == Visibility::Temporarily {
                        // Android can sometime not see the mDNS service if the service
                        // was running BEFORE Android started the Discovery phase for QuickShare.
                        // So resend a broadcast if there's a android device sending.
                        self.daemon.register_resend(self.service_info.get_fullname())?;
                    } else {
                        self.daemon.register(self.service_info.clone())?;
                    }
                },
                refresh = receiver_refresh_receiver.recv(), if receiver_refresh_open => {
                    match receiver_refresh_action(refresh) {
                        ReceiverRefreshAction::Resend => {
                            if visibility == Visibility::Invisible {
                                continue;
                            }

                            debug!("{INNER_NAME}: receiver slot0 requested mDNS resend");
                            self.daemon.register_resend(self.service_info.get_fullname())?;
                        }
                        ReceiverRefreshAction::Disable => {
                            receiver_refresh_open = false;
                            debug!("{INNER_NAME}: receiver refresh channel closed; disabling slot0-triggered mDNS resends");
                        }
                    }
                },
                _ = interval.tick() => {
                    if visibility != Visibility::Temporarily {
                        continue;
                    }

                    let receiver = self.daemon.unregister(self.service_info.get_fullname())?;
                    let _ = receiver.recv();
                    match self.visibility_sender.lock() {
                        Ok(sender) => {
                            let _ = sender.send(Visibility::Invisible);
                        }
                        Err(_) => error!("{INNER_NAME}: visibility sender lock is poisoned"),
                    }
                }
            }
        }

        // Unregister the mDNS service - we're shutting down
        let receiver = self.daemon.unregister(self.service_info.get_fullname())?;
        if let Ok(event) = receiver.recv() {
            info!("MDnsServer: service unregistered: {:?}", &event);
        }

        Ok(())
    }

    fn build_service(
        endpoint_id: [u8; 4],
        service_port: u16,
        device_type: DeviceType,
    ) -> Result<ServiceInfo, anyhow::Error> {
        let name = gen_mdns_name(endpoint_id);
        let hostname = sys_metrics::host::get_hostname()?;
        info!("Broadcasting with: {hostname}");
        let endpoint_info = gen_mdns_endpoint_info(device_type as u8, &hostname);

        let properties = [("n", endpoint_info)];
        let si = ServiceInfo::new(
            "_FC9F5ED42C8A._tcp.local.",
            &name,
            &hostname,
            "",
            service_port,
            &properties[..],
        )?
        .enable_addr_auto(AddrType::V4);

        Ok(si)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_refresh_ok_and_lagged_request_resend() {
        assert_eq!(
            receiver_refresh_action(Ok(())),
            ReceiverRefreshAction::Resend
        );
        assert_eq!(
            receiver_refresh_action(Err(tokio::sync::broadcast::error::RecvError::Lagged(2))),
            ReceiverRefreshAction::Resend
        );
    }

    #[test]
    fn receiver_refresh_closed_disables_slot0_resends() {
        assert_eq!(
            receiver_refresh_action(Err(tokio::sync::broadcast::error::RecvError::Closed)),
            ReceiverRefreshAction::Disable
        );
    }
}
