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
    use crate::DeviceType;

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
    fn given_receiver_service_data_name_boundaries_when_encoded_then_all_lengths_and_codepoints_remain_valid(
    ) {
        // Arrange: the public identity budget is 237 UTF-8 bytes; direct
        // callers may still submit longer strings, which the BLE encoder
        // must truncate without splitting a Unicode scalar.
        let cases = [
            ("empty", String::new()),
            ("one", "x".into()),
            ("below-maximum", "x".repeat(236)),
            ("maximum", "x".repeat(237)),
            ("one-above-maximum", "x".repeat(238)),
            ("byte-255", "x".repeat(255)),
            ("byte-256", "x".repeat(256)),
            ("large", "x".repeat(512)),
            ("two-byte-utf8-boundary", "é".repeat(120)),
            ("four-byte-utf8-boundary", "🧩".repeat(60)),
        ];

        for (case, name) in cases {
            // Act: test the real on-wire service data, not a copied encoder.
            let encoded = receiver_service_data(*b"TEST", DeviceType::Laptop as u8, &name);
            let announced = u32::from_be_bytes(encoded[4..8].try_into().unwrap()) as usize;
            assert_eq!(encoded.len(), 8 + announced + 2, "{case}");
            assert_eq!(&encoded[..4], &[0x48, 0xfc, 0x9f, 0x5e], "{case}");

            let connection = &encoded[8..8 + announced];
            assert_eq!(connection[0], 0x23, "{case}");
            assert_eq!(&connection[1..4], &QS_SVC_HASH, "{case}");
            assert_eq!(&connection[4..8], b"TEST", "{case}");

            let info_len = usize::from(connection[8]);
            assert_eq!(connection.len(), 9 + info_len + 8, "{case}");
            let info = &connection[9..9 + info_len];
            assert_eq!(info[0], (1 << 5) | (3 << 1), "{case}");
            assert_eq!(info.len(), 18 + usize::from(info[17]), "{case}");

            let mut expected_len = name.len().min(crate::identity::MAX_NAME_BYTES);
            while !name.is_char_boundary(expected_len) {
                expected_len -= 1;
            }
            assert_eq!(usize::from(info[17]), expected_len, "{case}");
            assert_eq!(&info[18..], &name.as_bytes()[..expected_len], "{case}");
            assert!(std::str::from_utf8(&info[18..]).is_ok(), "{case}");
            assert_eq!(&connection[9 + info_len..], &[0_u8; 8], "{case}");
        }
    }

    #[test]
    fn given_same_receiver_identity_when_encoded_for_ble_and_mdns_then_endpoint_and_name_agree() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

        // Arrange: compare two independent real production encoders and the
        // mDNS decoder for names accepted by the identity contract.
        let endpoints = [[0_u8; 4], *b"TEST", [0x01, 0x80, 0x3f, 0xfe], [u8::MAX; 4]];
        let names = [
            String::new(),
            "A".into(),
            "Alcotester".into(),
            "é".repeat(118),
            "x".repeat(crate::identity::MAX_NAME_BYTES),
        ];
        let devices = [
            (0_u8, DeviceType::Unknown),
            (1, DeviceType::Phone),
            (2, DeviceType::Tablet),
            (3, DeviceType::Laptop),
        ];

        for endpoint in endpoints {
            let service_name = crate::utils::gen_mdns_name(endpoint);
            let decoded = URL_SAFE_NO_PAD.decode(service_name).unwrap();
            let expected_service_name = [
                0x23,
                endpoint[0],
                endpoint[1],
                endpoint[2],
                endpoint[3],
                0xfc,
                0x9f,
                0x5e,
                0x00,
                0x00,
            ];
            assert_eq!(decoded.as_slice(), expected_service_name);

            for (kind, expected_device) in &devices {
                for name in &names {
                    // Act: generate the published data through both media.
                    let ble = receiver_service_data(endpoint, *kind, name);
                    let txt = crate::utils::gen_mdns_endpoint_info(*kind, name);
                    let (type_from_mdns, name_from_mdns) =
                        crate::utils::parse_mdns_endpoint_info(&txt).unwrap();

                    // Assert: the endpoint never changes across the BLE/mDNS
                    // boundary, nor does an allowed name or device class.
                    let connection_len = u32::from_be_bytes(ble[4..8].try_into().unwrap()) as usize;
                    let connection = &ble[8..8 + connection_len];
                    assert_eq!(connection[0], 0x23);
                    assert_eq!(&connection[1..4], &QS_SVC_HASH);
                    assert_eq!(&connection[4..8], &endpoint);
                    let info = &connection[9..9 + usize::from(connection[8])];
                    assert_eq!(info[0], (1 << 5) | (kind << 1));
                    assert_eq!(usize::from(info[17]), name.len());
                    assert_eq!(&info[18..], name.as_bytes());
                    assert_eq!(type_from_mdns, expected_device.clone());
                    assert_eq!(name_from_mdns, *name);
                }
            }
        }
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
    header: tokio::sync::watch::Receiver<Option<super::receiver_header::Header>>,
}

impl ReceiverAdvertiser {
    pub async fn new(
        visibility_receiver: crate::VisibilityReader,
        refresh_sender: broadcast::Sender<ReceiverAdvertisingRefresh>,
        header: tokio::sync::watch::Receiver<Option<super::receiver_header::Header>>,
    ) -> Result<Self, anyhow::Error> {
        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;

        Ok(Self {
            adapter: Arc::new(adapter),
            visibility_receiver,
            refresh_sender,
            header,
        })
    }

    fn packet(header: super::receiver_header::Header) -> Advertisement {
        let service_uuid = Uuid::from_u16(QS_SERVICE_UUID);
        Advertisement {
            advertisement_type: bluer::adv::Type::Peripheral,
            service_uuids: [service_uuid].into(),
            service_data: [(service_uuid, header.to_vec())].into(),
            discoverable: Some(true),
            min_interval: Some(std::time::Duration::from_millis(100)),
            max_interval: Some(std::time::Duration::from_millis(150)),
            ..Default::default()
        }
    }
    fn should_refresh(visibility: Visibility, has_handle: bool) -> bool {
        has_handle && BleAdvertiser::should_advertise(visibility)
    }

    pub async fn run(mut self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
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
            let header = *self.header.borrow_and_update();
            if let Some(header) =
                header.filter(|_| BleAdvertiser::should_advertise(visibility) && handle.is_none())
            {
                // The compact V2 header fits legacy advertising and points to slot0.
                // The full name/endpoint bytes stay with their GATT owner.
                let advertisement = Self::packet(header);
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
            } else if (!BleAdvertiser::should_advertise(visibility) || header.is_none())
                && handle.take().is_some()
            {
                schedule.stopped();
                crate::receiver_trace::event(crate::receiver_trace::Event::AdvertisementStopped);
                info!("{RX_INNER_NAME}: stopped advertising");
            }

            tokio::select! {
                changed = self.header.changed() => {
                    if changed.is_err() { break; }
                    handle.take();
                    schedule.stopped();
                }
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
    async fn given_repeated_gatt_refreshes_when_scheduled_then_first_deferred_and_periodic_deadlines_stay_fixed(
    ) {
        use std::time::Duration;

        // Arrange: all valid refresh intervals are checked with an isolated
        // fake Tokio clock; no BLE adapter or elapsed wall time is involved.
        for seconds in [5_u64, RX_PERIODIC_REFRESH_DEFAULT_SECS, 120] {
            let start = tokio::time::Instant::now();
            let mut schedule = ReceiverSchedule::default();
            schedule.started(start, seconds);
            let periodic = start + Duration::from_secs(seconds);
            assert_eq!(schedule.periodic, Some(periodic));
            assert!(schedule.deferred.is_none());

            // Act: the first slot-0 request starts the 2-second deadline.
            schedule.defer(start);
            let first_deferred = start + Duration::from_secs(2);
            assert_eq!(schedule.deferred, Some(first_deferred));

            tokio::time::advance(Duration::from_millis(1_999)).await;
            assert!(tokio::time::Instant::now() < first_deferred);
            schedule.defer(tokio::time::Instant::now());
            assert_eq!(schedule.deferred, Some(first_deferred));

            tokio::time::advance(Duration::from_millis(1)).await;
            assert!(tokio::time::Instant::now() >= first_deferred);
            for seconds_later in [1_u64, 2, 30, 120] {
                tokio::time::advance(Duration::from_secs(seconds_later)).await;
                schedule.defer(tokio::time::Instant::now());
                // Assert: neither repeated discovery traffic nor an overdue
                // timer may postpone the first deferred recovery or safety.
                assert_eq!(schedule.deferred, Some(first_deferred));
                assert_eq!(schedule.periodic, Some(periodic));
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn given_advertising_stop_and_restart_when_scheduled_then_old_deadlines_never_reappear() {
        use std::time::Duration;

        let mut schedule = ReceiverSchedule::default();

        // Arrange: stopping an inactive advertisement is an idempotent no-op.
        for _ in 0..2 {
            schedule.stopped();
            assert!(schedule.periodic.is_none());
            assert!(schedule.deferred.is_none());
        }

        // Act/Assert: across multiple independent advertising epochs, a
        // previous slot-0 deferred deadline must never reach the next epoch.
        for period_seconds in [5_u64, 30, 120] {
            let start = tokio::time::Instant::now();
            schedule.started(start, period_seconds);
            schedule.defer(start);
            let previous_periodic = schedule.periodic.unwrap();
            let previous_deferred = schedule.deferred.unwrap();
            assert_eq!(
                previous_periodic,
                start + Duration::from_secs(period_seconds)
            );
            assert_eq!(previous_deferred, start + Duration::from_secs(2));

            tokio::time::advance(Duration::from_millis(500)).await;
            schedule.stopped();
            assert!(schedule.periodic.is_none());
            assert!(schedule.deferred.is_none());

            tokio::time::advance(Duration::from_secs(period_seconds + 1)).await;
            assert!(schedule.periodic.is_none());
            assert!(schedule.deferred.is_none());

            // A fresh advertisement resets its periodic deadline and does
            // not inherit any deferred work until an actual new slot-0 read.
            let restarted = tokio::time::Instant::now();
            schedule.started(restarted, period_seconds);
            assert_eq!(
                schedule.periodic,
                Some(restarted + Duration::from_secs(period_seconds))
            );
            assert!(schedule.deferred.is_none());
            schedule.defer(restarted);
            assert_eq!(schedule.deferred, Some(restarted + Duration::from_secs(2)));
            assert!(schedule.deferred.unwrap() > previous_deferred);
            assert!(schedule.periodic.unwrap() > previous_periodic);
            schedule.stopped();
        }
    }

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

#[cfg(test)]
mod receiver_packet_tests {
    use super::*;
    #[test]
    fn given_legacy_receiver_headers_when_advertisement_is_built_then_bluez_contract_and_budget_are_preserved(
    ) {
        let patterned = std::array::from_fn(|index| (index as u8).wrapping_mul(17));
        let headers = [[0_u8; 17], patterned, [0xff_u8; 17]];

        for header in headers {
            // Arrange/Act: this is the real outgoing BlueZ advertisement
            // descriptor, with the 17-byte GATT header kept opaque.
            let packet = ReceiverAdvertiser::packet(header);
            let service = Uuid::from_u16(QS_SERVICE_UUID);

            // Assert: packet bounds/flags must be identical for any header
            // content and must stay within the legacy 31-byte budget.
            assert_eq!(packet.advertisement_type, bluer::adv::Type::Peripheral);
            assert_eq!(packet.service_uuids, [service].into());
            assert_eq!(packet.service_data.len(), 1);
            assert_eq!(packet.service_data[&service], header.to_vec());
            assert_eq!(packet.discoverable, Some(true));
            assert_eq!(
                packet.min_interval,
                Some(std::time::Duration::from_millis(100))
            );
            assert_eq!(
                packet.max_interval,
                Some(std::time::Duration::from_millis(150))
            );
            assert!(packet.secondary_channel.is_none());
            assert!(3 + 4 + 4 + packet.service_data[&service].len() <= 31);
        }
    }

    #[test]
    fn given_current_gatt_header_when_packet_is_built_then_exact_service_data_uses_the_connectable_legacy_budget(
    ) {
        let mut header = [0; 17];
        header[0] = 0x41;
        let packet = ReceiverAdvertiser::packet(header);
        let service = Uuid::from_u16(QS_SERVICE_UUID);
        assert_eq!(packet.advertisement_type, bluer::adv::Type::Peripheral);
        assert_eq!(packet.service_uuids, [service].into());
        assert_eq!(packet.service_data, [(service, header.to_vec())].into());
        assert_eq!(packet.discoverable, Some(true));
        assert!(packet.secondary_channel.is_none());
        assert!(3 + 4 + 4 + packet.service_data[&service].len() <= 31);
    }
}
