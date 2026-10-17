use std::sync::Arc;

use bluer::adv::{Advertisement, AdvertisementHandle};
use bluer::UuidExt;
use bytes::Bytes;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::Visibility;

const SERVICE_DATA: Bytes = Bytes::from_static(&[
    252, 18, 142, 1, 66, 0, 0, 0, 0, 0, 0, 0, 0, 0, 191, 45, 91, 160, 225, 216, 117, 36, 202, 0,
]);

const INNER_NAME: &str = "BleAdvertiser";

#[derive(Debug, Clone)]
pub struct BleAdvertiser {
    adapter: Arc<bluer::Adapter>,
    visibility_receiver: crate::VisibilityReader,
}

impl BleAdvertiser {
    pub async fn new(visibility_receiver: crate::VisibilityReader) -> Result<Self, anyhow::Error> {
        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;

        Ok(Self {
            adapter: Arc::new(adapter),
            visibility_receiver,
        })
    }

    fn should_advertise(visibility: Visibility) -> bool {
        matches!(visibility, Visibility::Visible | Visibility::Temporarily)
    }

    pub async fn run(mut self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        info!(
            "{INNER_NAME}: using Bluetooth adapter {} with address {}",
            self.adapter.name(),
            self.adapter.address().await?
        );

        let service_uuid = Uuid::from_u16(0xFE2C);
        let mut handle: Option<AdvertisementHandle> = None;
        let mut visibility = self.visibility_receiver.current_and_update();

        if Self::should_advertise(visibility) {
            handle = Some(
                self.adapter
                    .advertise(self.get_advertisement(service_uuid, SERVICE_DATA))
                    .await?,
            );
            info!("{INNER_NAME}: started advertising");
        }

        loop {
            tokio::select! {
                _ = ctk.cancelled() => {
                    info!("{INNER_NAME}: tracker cancelled, returning");
                    break;
                }
                changed = self.visibility_receiver.changed() => {
                    if changed.is_err() {
                        debug!("{INNER_NAME}: visibility channel closed, stopping advertiser");
                        break;
                    }

                    visibility = self.visibility_receiver.current_and_update();
                    debug!("{INNER_NAME}: visibility changed: {visibility:?}");

                    if Self::should_advertise(visibility) {
                        if handle.is_none() {
                            handle = Some(
                                self.adapter
                                    .advertise(self.get_advertisement(service_uuid, SERVICE_DATA))
                                    .await?,
                            );
                            info!("{INNER_NAME}: started advertising");
                        }
                    } else if handle.take().is_some() {
                        info!("{INNER_NAME}: stopped advertising");
                    }
                }
            }
        }

        drop(handle);
        Ok(())
    }

    fn get_advertisement(&self, service_uuid: Uuid, adv_data: Bytes) -> Advertisement {
        Advertisement {
            advertisement_type: bluer::adv::Type::Broadcast,
            service_data: [(service_uuid, adv_data.into())].into(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_only_when_discoverable() {
        assert!(BleAdvertiser::should_advertise(Visibility::Visible));
        assert!(BleAdvertiser::should_advertise(Visibility::Temporarily));
        assert!(!BleAdvertiser::should_advertise(Visibility::Invisible));
    }

    #[test]
    fn receiver_advertisement_has_valid_full_layout() {
        let endpoint = [0x11, 0x22, 0x33, 0x44];
        let data = receiver_service_data(endpoint, 3, "Alcotester");

        assert_eq!(data[0], 0x48);
        assert_eq!(&data[1..4], &QS_SVC_HASH);

        let connection_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let connection_end = 8 + connection_len;
        assert_eq!(data.len(), connection_end + 2);

        let connection = &data[8..connection_end];
        assert_eq!(connection[0], 0x23);
        assert_eq!(&connection[1..4], &QS_SVC_HASH);
        assert_eq!(&connection[4..8], &endpoint);

        let info_len = usize::from(connection[8]);
        let info_end = 9 + info_len;
        let endpoint_info = &connection[9..info_end];

        assert_eq!(endpoint_info[0], (1 << 5) | (3 << 1));
        let name_len = usize::from(endpoint_info[17]);
        assert_eq!(&endpoint_info[18..18 + name_len], b"Alcotester");
        assert_eq!(&connection[info_end..info_end + 8], &[0_u8; 8]);
    }

    #[test]
    fn receiver_advertisement_truncates_long_names_to_endpoint_info_limit() {
        let data = receiver_service_data([1, 2, 3, 4], 3, &"x".repeat(300));

        let connection_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let connection = &data[8..8 + connection_len];
        let info_len = usize::from(connection[8]);
        let endpoint_info = &connection[9..9 + info_len];

        assert_eq!(info_len, u8::MAX as usize);
        assert_eq!(usize::from(endpoint_info[17]), 237);
        assert_eq!(&endpoint_info[18..], vec![b'x'; 237]);
    }

    #[test]
    fn receiver_refresh_modes_remain_distinct() {
        assert_ne!(
            ReceiverAdvertisingRefresh::Immediate,
            ReceiverAdvertisingRefresh::Deferred
        );
    }

    #[test]
    fn receiver_refresh_policy_requires_visible_registered_advertising() {
        assert!(ReceiverAdvertiser::should_refresh(
            Visibility::Visible,
            true
        ));
        assert!(ReceiverAdvertiser::should_refresh(
            Visibility::Temporarily,
            true
        ));
        assert!(!ReceiverAdvertiser::should_refresh(
            Visibility::Invisible,
            true
        ));
        assert!(!ReceiverAdvertiser::should_refresh(
            Visibility::Visible,
            false
        ));
    }

    #[test]
    fn receiver_periodic_refresh_default_and_release_are_fixed() {
        assert_eq!(receiver_periodic_refresh_secs_for(true, None), 30);
        assert_eq!(receiver_periodic_refresh_secs_for(false, None), 30);
        assert_eq!(receiver_periodic_refresh_secs_for(false, Some("10")), 30);
        assert_eq!(receiver_periodic_refresh_secs_for(false, Some("120")), 30);
    }

    #[test]
    fn receiver_periodic_refresh_debug_accepts_only_bounded_seconds() {
        assert_eq!(receiver_periodic_refresh_secs_for(true, Some("5")), 5);
        assert_eq!(receiver_periodic_refresh_secs_for(true, Some("10")), 10);
        assert_eq!(receiver_periodic_refresh_secs_for(true, Some("30")), 30);
        assert_eq!(receiver_periodic_refresh_secs_for(true, Some("120")), 120);
        for invalid in [
            "",
            "0",
            "1",
            "4",
            "121",
            "-1",
            "10s",
            " 10",
            "18446744073709551616",
        ] {
            assert_eq!(receiver_periodic_refresh_secs_for(true, Some(invalid)), 30);
        }
    }

    #[test]
    fn receiver_advertisement_truncates_multibyte_names_on_utf8_boundary() {
        let data = receiver_service_data([1, 2, 3, 4], 3, &"é".repeat(200));

        let connection_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let connection = &data[8..8 + connection_len];
        let info_len = usize::from(connection[8]);
        let endpoint_info = &connection[9..9 + info_len];
        let name_len = usize::from(endpoint_info[17]);
        let name = std::str::from_utf8(&endpoint_info[18..18 + name_len]).unwrap();

        assert_eq!(info_len, 254);
        assert_eq!(name_len, 236);
        assert_eq!(name, "é".repeat(118));
    }
}

// Quick Share receiver discovery over BLE (service UUID 0xFEF3).
const RX_INNER_NAME: &str = "ReceiverAdvertiser";
const QS_SERVICE_UUID: u16 = 0xFEF3;
const QS_SVC_HASH: [u8; 3] = [0xfc, 0x9f, 0x5e];
const RX_PERIODIC_REFRESH_DEFAULT_SECS: u64 = 30;

// Only debug builds may override the safety refresh frequency. Keep the
// release schedule unchanged while diagnosing BlueZ/Android cold discovery.
fn receiver_periodic_refresh_secs_for(debug_build: bool, raw: Option<&str>) -> u64 {
    if !debug_build {
        return RX_PERIODIC_REFRESH_DEFAULT_SECS;
    }

    raw.and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| (5..=120).contains(seconds))
        .unwrap_or(RX_PERIODIC_REFRESH_DEFAULT_SECS)
}

fn receiver_periodic_refresh_secs() -> u64 {
    #[cfg(debug_assertions)]
    let raw = std::env::var("RQS_DIAG_RX_PERIODIC_ADV_SECS").ok();
    #[cfg(not(debug_assertions))]
    let raw: Option<String> = None;

    receiver_periodic_refresh_secs_for(cfg!(debug_assertions), raw.as_deref())
}

/// Build the Nearby Connections receiver advertisement carried as 0xFEF3
/// service data. The endpoint id must match the id used by mDNS.
pub fn receiver_service_data(endpoint_id: [u8; 4], device_type: u8, device_name: &str) -> Vec<u8> {
    let mut endpoint_info = Vec::new();
    endpoint_info.push((1 << 5) | ((device_type & 0x7) << 1));

    let identity: [u8; 16] = rand::random();
    endpoint_info.extend_from_slice(&identity);

    const MAX_ENDPOINT_INFO_BYTES: usize = u8::MAX as usize;
    const MAX_RECEIVER_NAME_BYTES: usize = crate::identity::MAX_NAME_BYTES;

    let name = device_name.as_bytes();
    let mut name_len = name.len().min(MAX_RECEIVER_NAME_BYTES);
    while !device_name.is_char_boundary(name_len) {
        name_len -= 1;
    }
    endpoint_info.push(name_len as u8);
    endpoint_info.extend_from_slice(&name[..name_len]);

    let mut connection_advertisement = Vec::new();
    connection_advertisement.push(0x23);
    connection_advertisement.extend_from_slice(&QS_SVC_HASH);
    connection_advertisement.extend_from_slice(&endpoint_id);
    connection_advertisement.push(endpoint_info.len().min(MAX_ENDPOINT_INFO_BYTES) as u8);
    connection_advertisement.extend_from_slice(&endpoint_info);

    // Reserved Bluetooth MAC, UWB-address length and extra-field byte.
    connection_advertisement.extend_from_slice(&[0_u8; 8]);

    let mut service_data = Vec::new();
    service_data.push(0x48);
    service_data.extend_from_slice(&QS_SVC_HASH);
    service_data.extend_from_slice(&(connection_advertisement.len() as u32).to_be_bytes());
    service_data.extend_from_slice(&connection_advertisement);

    let device_token: [u8; 2] = rand::random();
    service_data.extend_from_slice(&device_token);
    service_data
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverAdvertisingRefresh {
    Immediate,
    Deferred,
}

#[derive(Default)]
struct ReceiverSchedule {
    periodic: Option<tokio::time::Instant>,
    deferred: Option<tokio::time::Instant>,
}
impl ReceiverSchedule {
    fn started(&mut self, now: tokio::time::Instant, seconds: u64) {
        self.periodic = Some(now + std::time::Duration::from_secs(seconds));
        self.deferred = None;
    }
    fn defer(&mut self, now: tokio::time::Instant) {
        self.deferred
            .get_or_insert(now + std::time::Duration::from_secs(2));
    }
    fn stopped(&mut self) {
        self.periodic = None;
        self.deferred = None;
    }
}

#[derive(Debug, Clone)]
pub struct ReceiverAdvertiser {
    adapter: Arc<bluer::Adapter>,
    visibility_receiver: crate::VisibilityReader,
    refresh_sender: broadcast::Sender<ReceiverAdvertisingRefresh>,
}

impl ReceiverAdvertiser {
    pub async fn new(
        visibility_receiver: crate::VisibilityReader,
        refresh_sender: broadcast::Sender<ReceiverAdvertisingRefresh>,
    ) -> Result<Self, anyhow::Error> {
        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;

        Ok(Self {
            adapter: Arc::new(adapter),
            visibility_receiver,
            refresh_sender,
        })
    }

    fn should_refresh(visibility: Visibility, has_handle: bool) -> bool {
        has_handle && BleAdvertiser::should_advertise(visibility)
    }

    pub async fn run(mut self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        let service_uuid = Uuid::from_u16(QS_SERVICE_UUID);
        let mut handle: Option<AdvertisementHandle> = None;
        let mut refresh_receiver = self.refresh_sender.subscribe();
        let mut schedule = ReceiverSchedule::default();
        let periodic_refresh_secs = receiver_periodic_refresh_secs();
        #[cfg(debug_assertions)]
        info!(
            "{RX_INNER_NAME}: diagnostic periodic advertising refresh interval={periodic_refresh_secs}s"
        );

        info!(
            "{RX_INNER_NAME}: prepared Quick Share receiver advertisement on {} ({})",
            self.adapter.name(),
            self.adapter.address().await?
        );

        loop {
            let visibility = self.visibility_receiver.current_and_update();
            if BleAdvertiser::should_advertise(visibility) && handle.is_none() {
                // Keep the on-air packet legacy-sized: advertise only the 0xFEF3
                // service UUID and serve the full Nearby receiver advertisement
                // through GATT slot 0. Putting the full service data on air can
                // force BlueZ into Extended Advertising, which some Android
                // scanners and kernel/adapter combinations do not discover reliably.
                let advertisement = Advertisement {
                    advertisement_type: bluer::adv::Type::Peripheral,
                    service_uuids: [service_uuid].into(),
                    discoverable: Some(true),
                    min_interval: Some(std::time::Duration::from_millis(100)),
                    max_interval: Some(std::time::Duration::from_millis(150)),
                    ..Default::default()
                };
                match self.adapter.advertise(advertisement).await {
                    Ok(advertisement_handle) => {
                        handle = Some(advertisement_handle);
                        schedule.started(tokio::time::Instant::now(), periodic_refresh_secs);
                        crate::receiver_trace::event(
                            crate::receiver_trace::Event::AdvertisementStarted,
                        );
                        info!("{RX_INNER_NAME}: started advertising");
                    }
                    Err(error) => {
                        crate::receiver_trace::event(
                            crate::receiver_trace::Event::AdvertisementFailed,
                        );
                        warn!("{RX_INNER_NAME}: advertise failed ({error}); retrying");
                        tokio::select! {
                            _ = ctk.cancelled() => break,
                            _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {}
                        }
                        continue;
                    }
                }
            } else if !BleAdvertiser::should_advertise(visibility) && handle.take().is_some() {
                schedule.stopped();
                crate::receiver_trace::event(crate::receiver_trace::Event::AdvertisementStopped);
                info!("{RX_INNER_NAME}: stopped advertising");
            }

            tokio::select! {
                _ = ctk.cancelled() => {
                    info!("{RX_INNER_NAME}: tracker cancelled, returning");
                    break;
                }
                changed = self.visibility_receiver.changed() => {
                    if changed.is_err() {
                        debug!("{RX_INNER_NAME}: visibility channel closed, stopping advertiser");
                        break;
                    }
                }
                refresh = refresh_receiver.recv() => {
                    match refresh {
                        Ok(ReceiverAdvertisingRefresh::Immediate) => {
                            let visibility = self.visibility_receiver.current();
                            if Self::should_refresh(visibility, handle.is_some()) {
                                debug!("{RX_INNER_NAME}: immediate advertising refresh requested");
                                schedule.stopped();
                                crate::receiver_trace::event(crate::receiver_trace::Event::AdvertisementImmediate);
                                handle.take();
                            }
                        }
                        Ok(ReceiverAdvertisingRefresh::Deferred) | Err(broadcast::error::RecvError::Lagged(_)) => {
                            let visibility = self.visibility_receiver.current();
                            if Self::should_refresh(visibility, handle.is_some()) {
                                // Repeated reads must not postpone the first scheduled recovery.
                                schedule.defer(tokio::time::Instant::now());
                                debug!("{RX_INNER_NAME}: deferred advertising refresh scheduled");
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("{RX_INNER_NAME}: refresh channel closed");
                        }
                    }
                }
                _ = tokio::time::sleep_until(schedule.deferred.unwrap_or_else(tokio::time::Instant::now)), if schedule.deferred.is_some() => {
                    let visibility = self.visibility_receiver.current();
                    schedule.deferred = None;
                    if Self::should_refresh(visibility, handle.is_some()) {
                        debug!("{RX_INNER_NAME}: applying deferred advertising refresh");
                        schedule.stopped();
                        crate::receiver_trace::event(crate::receiver_trace::Event::AdvertisementDeferred);
                        handle.take();
                    }
                }
                _ = tokio::time::sleep_until(schedule.periodic.unwrap_or_else(tokio::time::Instant::now)), if handle.is_some() => {
                    // Safety fallback. BlueZ consumes connectable advertising sets on some
                    // adapters; normal recovery is event-driven from GATT activity.
                    debug!("{RX_INNER_NAME}: periodic advertising refresh");
                    schedule.stopped();
                    crate::receiver_trace::event(crate::receiver_trace::Event::AdvertisementPeriodic);
                    handle.take();
                }
            }
        }

        drop(handle);
        Ok(())
    }
}

#[cfg(test)]
mod schedule_tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn given_continuous_refresh_events_when_scheduled_then_periodic_and_first_deferred_deadlines_never_slide(
    ) {
        // Arrange
        let mut schedule = ReceiverSchedule::default();
        let start = tokio::time::Instant::now();
        schedule.started(start, 30);
        schedule.defer(start);
        let periodic = schedule.periodic;
        let deferred = schedule.deferred;
        // Act / Assert
        for _ in 0..31 {
            tokio::time::advance(std::time::Duration::from_secs(1)).await;
            schedule.defer(tokio::time::Instant::now());
            assert_eq!(schedule.periodic, periodic);
            assert_eq!(schedule.deferred, deferred);
        }
        assert!(schedule.periodic.unwrap() <= tokio::time::Instant::now());
        assert!(schedule.deferred.unwrap() <= tokio::time::Instant::now());
        schedule.stopped();
        assert!(schedule.periodic.is_none() && schedule.deferred.is_none());
        let restarted = tokio::time::Instant::now();
        schedule.started(restarted, 30);
        assert_eq!(
            schedule.periodic,
            Some(restarted + std::time::Duration::from_secs(30))
        );
        assert!(schedule.deferred.is_none());
    }
}
