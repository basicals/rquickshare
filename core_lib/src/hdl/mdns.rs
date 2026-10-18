use mdns_sd::{AddrType, ServiceDaemon, ServiceInfo};
use tokio::sync::broadcast::Receiver;
use tokio_util::sync::CancellationToken;

use crate::utils::{gen_mdns_endpoint_info, gen_mdns_name, DeviceType};
use crate::Visibility;

const INNER_NAME: &str = "MDnsServer";

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
    visibility_receiver: crate::VisibilityReader,
    endpoint_id: [u8; 4],
    identity: Option<crate::identity::IdentityConsumer>,
    shutdown_requested: bool,
}

impl MDnsServer {
    fn identity_visible(visibility: &crate::VisibilityReader) -> bool {
        visibility.current() != Visibility::Invisible
    }

    pub(crate) fn with_identity(mut self, identity: crate::identity::IdentityConsumer) -> Self {
        self.identity = Some(identity);
        self
    }

    pub fn new(
        endpoint_id: [u8; 4],
        service_port: u16,
        ble_receiver: Receiver<()>,
        receiver_refresh_receiver: Receiver<()>,
        visibility_receiver: crate::VisibilityReader,
    ) -> Result<Self, anyhow::Error> {
        let service_info = Self::build_service(endpoint_id, service_port, DeviceType::Laptop)?;

        Ok(Self {
            daemon: ServiceDaemon::new()?,
            service_info,
            ble_receiver,
            receiver_refresh_receiver,
            visibility_receiver,
            endpoint_id,
            identity: None,
            shutdown_requested: false,
        })
    }

    pub async fn run(&mut self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        let outcome = self.run_inner(ctk).await;
        self.shutdown_requested = true;
        complete_lifetime(outcome, async {
            let completion = self.daemon.shutdown()?;
            completion.recv_async().await?;
            Ok(())
        })
        .await
    }

    async fn run_inner(&mut self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        info!("{INNER_NAME}: service starting");
        let monitor = self.daemon.monitor()?;
        let ble_receiver = &mut self.ble_receiver;
        let receiver_refresh_receiver = &mut self.receiver_refresh_receiver;
        self.visibility_receiver.current_and_update();
        let mut receiver_refresh_open = true;
        let mut identity = self.identity.take();
        if let Some(consumer) = identity.as_mut() {
            let snapshot = consumer.snapshot();
            self.service_info = Self::renamed_service(
                &self.service_info,
                self.endpoint_id,
                snapshot.name.as_str(),
            )?;
            if Self::identity_visible(&self.visibility_receiver) {
                self.daemon.register(self.service_info.clone())?;
                crate::receiver_trace::event(crate::receiver_trace::Event::MdnsRegister);
            }
            consumer.acknowledge(snapshot.revision)?;
        } else if Self::identity_visible(&self.visibility_receiver) {
            self.daemon.register(self.service_info.clone())?;
            crate::receiver_trace::event(crate::receiver_trace::Event::MdnsRegister);
        }

        loop {
            tokio::select! {
                _ = ctk.cancelled() => {
                    info!("{INNER_NAME}: tracker cancelled, breaking");
                    break;
                }
                changed = async {
                    match identity.as_mut() {
                        Some(consumer) => consumer.changed().await,
                        None => std::future::pending().await,
                    }
                } => {
                    changed?;
                    let consumer = identity.as_mut().ok_or_else(|| anyhow::anyhow!("Identity consumer unavailable"))?;
                    let snapshot = consumer.snapshot();
                    self.service_info = Self::renamed_service(&self.service_info, self.endpoint_id, snapshot.name.as_str())?;
                    if Self::identity_visible(&self.visibility_receiver) {
                        self.daemon.register(self.service_info.clone())?;
                crate::receiver_trace::event(crate::receiver_trace::Event::MdnsRegister);
                    }
                    consumer.acknowledge(snapshot.revision)?;
                }
                r = monitor.recv_async() => {
                    match r {
                        Ok(_) => continue,
                        Err(err) => return Err(err.into()),
                    }
                },
                changed = self.visibility_receiver.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let visibility = self.visibility_receiver.current_and_update();
                    debug!("{INNER_NAME}: visibility changed: {visibility:?}");
                    crate::receiver_trace::event(if visibility==Visibility::Invisible {crate::receiver_trace::Event::Hidden}else{crate::receiver_trace::Event::Visible});
                    if visibility == Visibility::Invisible {
                        let receiver = self.daemon.unregister(self.service_info.get_fullname())?;
                        let _ = receiver.recv_async().await;
                    } else {
                        self.daemon.register(self.service_info.clone())?;
                crate::receiver_trace::event(crate::receiver_trace::Event::MdnsRegister);
                    }
                }
                _ = ble_receiver.recv() => {
                    let visibility = self.visibility_receiver.current();
                    if visibility == Visibility::Invisible {
                        continue;
                    }

                    debug!("{INNER_NAME}: ble_receiver: got event");
                    if visibility == Visibility::Visible || visibility == Visibility::Temporarily {
                        // Android can sometime not see the mDNS service if the service
                        // was running BEFORE Android started the Discovery phase for QuickShare.
                        // So resend a broadcast if there's a android device sending.
                        self.daemon.register_resend(self.service_info.get_fullname())?;
                        crate::receiver_trace::event(crate::receiver_trace::Event::MdnsResend);
                    } else {
                        self.daemon.register(self.service_info.clone())?;
                crate::receiver_trace::event(crate::receiver_trace::Event::MdnsRegister);
                    }
                },
                refresh = receiver_refresh_receiver.recv(), if receiver_refresh_open => {
                    match receiver_refresh_action(refresh) {
                        ReceiverRefreshAction::Resend => {
                            let visibility = self.visibility_receiver.current();
                            if visibility == Visibility::Invisible {
                                continue;
                            }

                            debug!("{INNER_NAME}: receiver slot0 requested mDNS resend");
                            self.daemon.register_resend(self.service_info.get_fullname())?;
                        crate::receiver_trace::event(crate::receiver_trace::Event::MdnsResend);
                        }
                        ReceiverRefreshAction::Disable => {
                            receiver_refresh_open = false;
                            debug!("{INNER_NAME}: receiver refresh channel closed; disabling slot0-triggered mDNS resends");
                        }
                    }
                },
            }
        }

        // Unregister the mDNS service - we're shutting down
        let receiver = self.daemon.unregister(self.service_info.get_fullname())?;
        if let Ok(event) = receiver.recv_async().await {
            info!("MDnsServer: service unregistered: {:?}", &event);
        }

        Ok(())
    }

    fn build_service(
        endpoint_id: [u8; 4],
        service_port: u16,
        device_type: DeviceType,
    ) -> Result<ServiceInfo, anyhow::Error> {
        let hostname = sys_metrics::host::get_hostname()?;
        Self::named_service(endpoint_id, service_port, device_type, &hostname, &hostname)
    }

    fn renamed_service(
        original: &ServiceInfo,
        endpoint_id: [u8; 4],
        display_name: &str,
    ) -> Result<ServiceInfo, anyhow::Error> {
        Self::named_service(
            endpoint_id,
            original.get_port(),
            DeviceType::Laptop,
            original.get_hostname(),
            display_name,
        )
    }

    fn named_service(
        endpoint_id: [u8; 4],
        service_port: u16,
        device_type: DeviceType,
        hostname: &str,
        display_name: &str,
    ) -> Result<ServiceInfo, anyhow::Error> {
        let name = gen_mdns_name(endpoint_id);
        let endpoint_info = gen_mdns_endpoint_info(device_type as u8, display_name);

        let properties = [("n", endpoint_info)];
        let si = ServiceInfo::new(
            "_FC9F5ED42C8A._tcp.local.",
            &name,
            hostname,
            "",
            service_port,
            &properties[..],
        )?
        .enable_addr_auto(AddrType::V4);

        Ok(si)
    }
}

impl Drop for MDnsServer {
    fn drop(&mut self) {
        if !self.shutdown_requested && self.daemon.shutdown().is_err() {
            error!("mDNS SDK shutdown admission failed during owner drop");
        }
    }
}

async fn complete_lifetime(
    outcome: Result<(), anyhow::Error>,
    shutdown: impl std::future::Future<Output = Result<(), anyhow::Error>>,
) -> Result<(), anyhow::Error> {
    let shutdown = shutdown.await;
    if shutdown.is_err() {
        error!("mDNS SDK shutdown failed");
    }
    outcome.and(shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn given_worker_and_sdk_outcomes_when_completing_then_shutdown_ack_is_required_and_primary_error_survives(
    ) {
        for worker_failed in [false, true] {
            for shutdown_failed in [false, true] {
                // Arrange
                let (release, acknowledgement) = tokio::sync::oneshot::channel();
                let outcome = if worker_failed {
                    Err(anyhow::anyhow!("worker failure"))
                } else {
                    Ok(())
                };
                let mut completion = Box::pin(complete_lifetime(outcome, async {
                    acknowledgement.await.unwrap();
                    if shutdown_failed {
                        Err(anyhow::anyhow!("SDK failure"))
                    } else {
                        Ok(())
                    }
                }));
                // Act / Assert
                assert!(futures::poll!(completion.as_mut()).is_pending());
                release.send(()).unwrap();
                let result = completion.await;
                assert_eq!(
                    result.err().map(|error| error.to_string()),
                    if worker_failed {
                        Some("worker failure".into())
                    } else if shutdown_failed {
                        Some("SDK failure".into())
                    } else {
                        None
                    }
                );
            }
        }
    }

    #[test]
    fn given_display_name_change_when_service_is_rebuilt_then_dns_identity_and_endpoint_stay_fixed()
    {
        let policy = crate::visibility::VisibilityPolicy::new(Visibility::Temporarily);
        let receiver = policy.subscribe();
        assert!(MDnsServer::identity_visible(&receiver));
        policy.set(Visibility::Invisible, true).unwrap();
        assert!(receiver.has_changed().unwrap());
        assert!(!MDnsServer::identity_visible(&receiver));
        policy.set(Visibility::Visible, true).unwrap();
        assert!(MDnsServer::identity_visible(&receiver));
        let endpoint = *b"TEST";
        let original = MDnsServer::named_service(
            endpoint,
            2345,
            DeviceType::Laptop,
            "machine.local.",
            "Original",
        )
        .unwrap();
        let renamed = MDnsServer::renamed_service(&original, endpoint, "Рабочий ПК").unwrap();
        assert_eq!(renamed.get_fullname(), original.get_fullname());
        assert_eq!(renamed.get_hostname(), original.get_hostname());
        assert_eq!(renamed.get_port(), original.get_port());
        let value = renamed.get_properties().get("n").unwrap().val_str();
        let (kind, name) = crate::utils::parse_mdns_endpoint_info(value).unwrap();
        assert_eq!(kind, DeviceType::Laptop);
        assert_eq!(name, "Рабочий ПК");
        let max = "a".repeat(crate::identity::MAX_NAME_BYTES);
        let maximum = MDnsServer::renamed_service(&original, endpoint, &max).unwrap();
        assert_eq!(
            crate::utils::parse_mdns_endpoint_info(
                maximum.get_properties().get("n").unwrap().val_str()
            )
            .unwrap()
            .1,
            max
        );
    }

    #[test]
    fn given_receiver_mdns_services_when_built_then_encoded_endpoint_port_and_txt_identity_agree(
    ) {
        // Arrange: exercise production ServiceInfo with bounded public names.
        let endpoints = [[0_u8; 4], *b"TEST", [u8::MAX; 4]];
        let ports = [1_u16, 2345, u16::MAX];
        let names = [
            String::new(),
            "A".to_owned(),
            "Cafe".to_owned(),
            "x".repeat(crate::identity::MAX_NAME_BYTES),
        ];
        let kinds = [
            DeviceType::Unknown,
            DeviceType::Phone,
            DeviceType::Tablet,
            DeviceType::Laptop,
        ];

        for endpoint in endpoints {
            let fullname = format!(
                "{}._FC9F5ED42C8A._tcp.local.",
                crate::utils::gen_mdns_name(endpoint)
            );
            for port in ports {
                for kind in &kinds {
                    for name in &names {
                        // Act: call the real mDNS service builder.
                        let service = MDnsServer::named_service(
                            endpoint, port, kind.clone(), "machine.local.", name,
                        )
                        .unwrap();

                        // Assert: no mismatched DNS endpoint, port or TXT owner.
                        assert_eq!(service.get_fullname(), fullname);
                        assert_eq!(service.get_hostname(), "machine.local.");
                        assert_eq!(service.get_port(), port);
                        let txt = service.get_properties().get("n").unwrap().val_str();
                        let (found_kind, found_name) =
                            crate::utils::parse_mdns_endpoint_info(txt).unwrap();
                        assert_eq!(found_kind, *kind);
                        assert_eq!(found_name, *name);
                    }
                }
            }
        }
    }

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
