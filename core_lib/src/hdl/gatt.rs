use std::sync::Arc;

use anyhow::anyhow;
use bluer::gatt::local::{
    Application, Characteristic, CharacteristicNotifier, CharacteristicNotify,
    CharacteristicNotifyMethod, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod,
    ReqError, Service,
};
use bluer::{Adapter, Uuid, UuidExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc::{channel, Receiver};
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

use crate::channel::{ChannelDirection, ChannelMessage, TransferType};
use crate::errors::AppError;
use crate::hdl::{BwuRouter, InboundRequest, MigratableStream, ReceiverAdvertisingRefresh, State};
use crate::sessions::{tasks::SessionTasks, SessionHub};

const INNER_NAME: &str = "ReceiverGattServer";

const QS_GATT_SERVICE: u16 = 0xFEF3;
const QS_ADV_SLOT0_UUID: &str = "00000000-0000-3000-8000-000000000000";
const QS_WEAVE_TO_PERIPHERAL: &str = "00000100-0004-1000-8000-001a11000101";
const QS_WEAVE_FROM_PERIPHERAL: &str = "00000100-0004-1000-8000-001a11000102";

const WEAVE_CONTROL: u8 = 0b1000_0000;
const WEAVE_CMD_MASK: u8 = 0b0000_1111;
const WEAVE_FIRST_BIT: u8 = 0b0000_1000;
const WEAVE_LAST_BIT: u8 = 0b0000_0100;
const WEAVE_CMD_CONN_REQUEST: u8 = 0;
const WEAVE_CMD_CONN_CONFIRM: u8 = 1;
const WEAVE_CMD_ERROR: u8 = 2;
const WEAVE_PROTOCOL_VERSION: u16 = 1;

const QS_SVC_HASH: [u8; 3] = [0xfc, 0x9f, 0x5e];
const SOCKET_CTRL_INTRODUCTION: u8 = 1;
const SOCKET_CTRL_DISCONNECTION: u8 = 2;

const MIN_WEAVE_PACKET_SIZE: u16 = 20;
const MAX_WEAVE_PACKET_SIZE: u16 = 509;
const MAX_INBOUND_FRAME_SIZE: usize = 5 * 1024 * 1024;
const MAX_WEAVE_MESSAGE_SIZE: usize = MAX_INBOUND_FRAME_SIZE + 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReceiverDiscoveryRefresh {
    resend_mdns: bool,
    advertising: Option<ReceiverAdvertisingRefresh>,
}

// The A/B override is restricted to debug builds. Release always retains the
// normal deferred FEF3 recovery after a slot-0 read.
fn slot0_deferred_advertising_enabled_for(
    debug_build: bool,
    diagnostic_value: Option<&str>,
) -> bool {
    !debug_build || diagnostic_value != Some("0")
}

fn slot0_deferred_advertising_enabled() -> bool {
    #[cfg(debug_assertions)]
    let diagnostic_value = std::env::var("RQS_DIAG_SLOT0_ADV_REFRESH").ok();

    #[cfg(not(debug_assertions))]
    let diagnostic_value: Option<String> = None;

    slot0_deferred_advertising_enabled_for(cfg!(debug_assertions), diagnostic_value.as_deref())
}

fn receiver_discovery_refresh(
    offset: usize,
    deferred_advertising_enabled: bool,
) -> Option<ReceiverDiscoveryRefresh> {
    (offset == 0).then_some(ReceiverDiscoveryRefresh {
        resend_mdns: true,
        advertising: deferred_advertising_enabled.then_some(ReceiverAdvertisingRefresh::Deferred),
    })
}

fn parse_connection_request(packet: &[u8]) -> Result<u16, anyhow::Error> {
    if packet.len() < 7 {
        return Err(anyhow!(
            "weave connection request is too short: {} bytes",
            packet.len()
        ));
    }

    let header = packet[0];
    if header & WEAVE_CONTROL == 0 || header & WEAVE_CMD_MASK != WEAVE_CMD_CONN_REQUEST {
        return Err(anyhow!("not a weave connection request"));
    }

    let min_version = u16::from_be_bytes([packet[1], packet[2]]);
    let max_version = u16::from_be_bytes([packet[3], packet[4]]);
    if !(min_version..=max_version).contains(&WEAVE_PROTOCOL_VERSION) {
        return Err(anyhow!(
            "unsupported weave protocol range {min_version}..={max_version}"
        ));
    }

    let requested = u16::from_be_bytes([packet[5], packet[6]]);
    Ok(requested.clamp(MIN_WEAVE_PACKET_SIZE, MAX_WEAVE_PACKET_SIZE))
}

fn connection_confirm(selected_packet_size: u16) -> [u8; 5] {
    [
        WEAVE_CONTROL | WEAVE_CMD_CONN_CONFIRM,
        (WEAVE_PROTOCOL_VERSION >> 8) as u8,
        (WEAVE_PROTOCOL_VERSION & 0xff) as u8,
        (selected_packet_size >> 8) as u8,
        (selected_packet_size & 0xff) as u8,
    ]
}

fn complete_framed_message_len(buffer: &[u8]) -> Result<Option<usize>, anyhow::Error> {
    if buffer.len() < 4 {
        return Ok(None);
    }

    let frame_len = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
    if frame_len == 0 || frame_len > MAX_INBOUND_FRAME_SIZE {
        return Err(anyhow!("invalid framed message length: {frame_len}"));
    }

    let total = 4_usize
        .checked_add(frame_len)
        .ok_or_else(|| anyhow!("framed message length overflow"))?;
    Ok((buffer.len() >= total).then_some(total))
}

fn inbound_disconnected_message(id: &str, error: impl Into<String>) -> ChannelMessage {
    ChannelMessage {
        id: id.to_owned(),
        direction: ChannelDirection::LibToFront,
        rtype: Some(TransferType::Inbound),
        state: Some(State::Disconnected),
        error: Some(error.into()),
        ..Default::default()
    }
}

pub struct ReceiverGattServer {
    adapter: Arc<Adapter>,
    advertisement: Vec<u8>,
    sender: Sender<ChannelMessage>,
    tcp_port: u16,
    bwu_router: BwuRouter,
    receiver_advertiser_refresh: tokio::sync::broadcast::Sender<ReceiverAdvertisingRefresh>,
    mdns_refresh_sender: tokio::sync::broadcast::Sender<()>,
    sessions: Option<SessionHub>,
}

impl ReceiverGattServer {
    pub async fn new(
        advertisement: Vec<u8>,
        sender: Sender<ChannelMessage>,
        tcp_port: u16,
        bwu_router: BwuRouter,
        receiver_advertiser_refresh: tokio::sync::broadcast::Sender<ReceiverAdvertisingRefresh>,
        mdns_refresh_sender: tokio::sync::broadcast::Sender<()>,
        sessions: Option<SessionHub>,
    ) -> Result<Self, anyhow::Error> {
        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;

        Ok(Self {
            adapter: Arc::new(adapter),
            advertisement,
            sender,
            tcp_port,
            bwu_router,
            receiver_advertiser_refresh,
            mdns_refresh_sender,
            sessions,
        })
    }

    pub async fn run(&self, ctk: CancellationToken) -> Result<(), anyhow::Error> {
        if ctk.is_cancelled() {
            return Ok(());
        }
        let service_uuid = Uuid::from_u16(QS_GATT_SERVICE);
        let slot0: Uuid = QS_ADV_SLOT0_UUID.parse()?;
        let weave_write: Uuid = QS_WEAVE_TO_PERIPHERAL.parse()?;
        let weave_notify: Uuid = QS_WEAVE_FROM_PERIPHERAL.parse()?;
        let advertisement = self.advertisement.clone();
        let deferred_slot0_advertising = slot0_deferred_advertising_enabled();
        #[cfg(debug_assertions)]
        info!(
            "{INNER_NAME}: diagnostic slot0 deferred advertising refresh enabled={deferred_slot0_advertising}"
        );

        const MAX_PENDING_GATT_WRITES: usize = 64;
        let (packet_sender, packet_receiver) = channel::<Vec<u8>>(MAX_PENDING_GATT_WRITES);
        let packet_receiver: Arc<Mutex<Option<Receiver<Vec<u8>>>>> =
            Arc::new(Mutex::new(Some(packet_receiver)));
        let channel_sender = self.sender.clone();
        let tcp_port = self.tcp_port;
        let bwu_router = self.bwu_router.clone();
        let slot_advertiser_refresh_sender = self.receiver_advertiser_refresh.clone();
        let slot_mdns_refresh_sender = self.mdns_refresh_sender.clone();
        let session_refresh_sender = self.receiver_advertiser_refresh.clone();
        let tasks = SessionTasks::new(&ctk, 33)?;
        let callback_tasks = tasks.clone();
        let bridge_slots = Arc::new(Semaphore::new(1));
        let sessions = self.sessions.clone();

        let app = Application {
            services: vec![Service {
                uuid: service_uuid,
                primary: true,
                characteristics: vec![
                    Characteristic {
                        uuid: slot0,
                        read: Some(CharacteristicRead {
                            read: true,
                            fun: Box::new(move |request| {
                                let advertisement = advertisement.clone();
                                let advertiser_refresh_sender =
                                    slot_advertiser_refresh_sender.clone();
                                let mdns_refresh_sender = slot_mdns_refresh_sender.clone();
                                Box::pin(async move {
                                    let offset = usize::from(request.offset);
                                    let response = if offset >= advertisement.len() {
                                        Vec::new()
                                    } else {
                                        advertisement[offset..].to_vec()
                                    };
                                    trace!(
                                        "{INNER_NAME}: slot0 read offset={offset}, returned={} bytes",
                                        response.len()
                                    );
                                    if let Some(refresh) = receiver_discovery_refresh(
                                        offset,
                                        deferred_slot0_advertising,
                                    ) {
                                        if refresh.resend_mdns {
                                            let _ = mdns_refresh_sender.send(());
                                        }
                                        if let Some(advertising) = refresh.advertising {
                                            let _ = advertiser_refresh_sender.send(advertising);
                                            debug!(
                                                "{INNER_NAME}: slot0 requested mDNS resend and deferred advertising recovery"
                                            );
                                        } else {
                                            debug!(
                                                "{INNER_NAME}: slot0 requested mDNS resend; deferred advertising recovery skipped (debug A/B)"
                                            );
                                        }
                                    }
                                    Ok(response)
                                })
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    Characteristic {
                        uuid: weave_write,
                        write: Some(CharacteristicWrite {
                            write: true,
                            write_without_response: false,
                            method: CharacteristicWriteMethod::Fun(Box::new(
                                move |value, _request| {
                                    let packet_sender = packet_sender.clone();
                                    Box::pin(async move {
                                        packet_sender.try_send(value).map_err(|error| {
                                            warn!(
                                                "{INNER_NAME}: rejecting GATT write because the weave queue is full or closed: {error}"
                                            );
                                            ReqError::Failed
                                        })?;
                                        Ok(())
                                    })
                                },
                            )),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    Characteristic {
                        uuid: weave_notify,
                        notify: Some(CharacteristicNotify {
                            notify: true,
                            indicate: true,
                            method: CharacteristicNotifyMethod::Fun(Box::new(move |notifier| {
                                let packet_receiver = packet_receiver.clone();
                                let channel_sender = channel_sender.clone();
                                let bwu_router = bwu_router.clone();
                                let refresh_sender = session_refresh_sender.clone();
                                let tasks = callback_tasks.clone();
                                let sessions = sessions.clone();
                                let bridge = Arc::clone(&bridge_slots).try_acquire_owned();
                                Box::pin(async move {
                                    let Ok(bridge) = bridge else {
                                        return;
                                    };
                                    debug!("{INNER_NAME}: weave notify subscription started");
                                    let worker_tasks = tasks.clone();
                                    let lifetime = tasks.lifetime();
                                    let started = tasks.spawn(async move {
                                        let _bridge = bridge;
                                        let result = crate::manager::session::until_stopped(
                                            &lifetime,
                                            weave_session(
                                                notifier,
                                                packet_receiver,
                                                channel_sender,
                                                bwu_router,
                                                tcp_port,
                                                worker_tasks,
                                                sessions,
                                            ),
                                        )
                                        .await;
                                        if let Some(Err(error)) = result {
                                            warn!(
                                            "{INNER_NAME}: weave session ended with error: {error}"
                                        );
                                        }
                                        let _ = refresh_sender
                                            .send(ReceiverAdvertisingRefresh::Immediate);
                                    });
                                    if let Ok(started) = started {
                                        let _ = started.await;
                                    }
                                })
                            })),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        };

        info!(
            "{INNER_NAME}: registering GATT 0x{QS_GATT_SERVICE:04X} (slot0 {} bytes)",
            self.advertisement.len()
        );
        let registration_lifetime = tasks.lifetime();
        let registration = tokio::select! {
            biased;
            _ = registration_lifetime.cancelled() => { tasks.stop().await?; return Ok(()); },
            registration = self.adapter.serve_gatt_application(app) => registration,
        };
        let handle = match registration {
            Ok(handle) => handle,
            Err(error) => {
                tasks.stop().await?;
                return Err(error.into());
            }
        };
        let lifetime = tasks.lifetime();
        tokio::select! {
            _ = ctk.cancelled() => {},
            _ = lifetime.cancelled() => {},
        }
        drop(handle);
        tasks.stop().await?;
        info!("{INNER_NAME}: stopped");
        Ok(())
    }
}

async fn weave_session(
    mut notifier: CharacteristicNotifier,
    packet_receiver: Arc<Mutex<Option<Receiver<Vec<u8>>>>>,
    sender: Sender<ChannelMessage>,
    bwu_router: BwuRouter,
    tcp_port: u16,
    tasks: SessionTasks,
    sessions: Option<SessionHub>,
) -> Result<(), anyhow::Error> {
    let notification_stopped = notifier.stopped();
    tokio::pin!(notification_stopped);
    let mut receiver_guard = tokio::select! {
        _ = &mut notification_stopped => return Ok(()),
        receiver = packet_receiver.lock() => receiver,
    };
    let receiver = receiver_guard
        .as_mut()
        .ok_or_else(|| anyhow!("weave packet receiver is unavailable"))?;

    info!("{INNER_NAME}: weave notify session opened");

    let selected_packet_size = loop {
        let packet = tokio::select! {
            _ = &mut notification_stopped => return Ok(()),
            packet = receiver.recv() => packet.ok_or_else(|| anyhow!("weave packet channel closed during handshake"))?,
        };

        match parse_connection_request(&packet) {
            Ok(selected) => break selected,
            Err(error) => {
                debug!("{INNER_NAME}: ignoring pre-handshake packet: {error}");
            }
        }
    };

    info!("{INNER_NAME}: negotiated BLE Weave packet size {selected_packet_size}");
    notifier
        .notify(connection_confirm(selected_packet_size).to_vec())
        .await?;
    let max_payload = usize::from(selected_packet_size.saturating_sub(1)).max(19);

    let (inbound_side, weave_side) = tokio::io::duplex(64 * 1024);
    let (mut weave_read, mut weave_write) = tokio::io::split(weave_side);
    let inbound_sender = sender.clone();
    let inbound_bwu_router = bwu_router.clone();
    let mut request = InboundRequest::new(
        MigratableStream::Ble(inbound_side),
        "ble-weave".to_owned(),
        inbound_sender,
    );
    let native = sessions.is_some();
    if let Some(sessions) = sessions {
        let session = sessions.open(TransferType::Inbound, None, CancellationToken::new())?;
        request = request.attach_session(session);
    }
    request.enable_bandwidth_upgrade();
    let lifetime = tasks.lifetime();
    tasks.spawn(run_inbound_worker(
        request,
        sender,
        inbound_bwu_router,
        tcp_port,
        lifetime,
        native,
    ))?;

    let mut reassembly = Vec::new();
    let mut outbound_buffer = Vec::new();
    let mut send_counter: u8 = 1;
    let mut read_buffer = [0_u8; 2048];

    // A peer can disappear without sending a final weave control packet. Observe
    // the GATT subscription lifecycle explicitly so the session mutex is released
    // and a later transfer can start without restarting the application.
    let result = loop {
        tokio::select! {
            _ = &mut notification_stopped => {
                debug!("{INNER_NAME}: peer stopped GATT notifications");
                break Ok(());
            }
            maybe_packet = receiver.recv() => {
                let Some(packet) = maybe_packet else {
                    break Ok(());
                };
                if packet.is_empty() {
                    continue;
                }

                let header = packet[0];
                if header & WEAVE_CONTROL != 0 {
                    if header & WEAVE_CMD_MASK == WEAVE_CMD_ERROR {
                        break Err(anyhow!("peer sent weave ERROR"));
                    }
                    continue;
                }

                if header & WEAVE_FIRST_BIT != 0 {
                    reassembly.clear();
                }

                let new_len = reassembly
                    .len()
                    .checked_add(packet.len().saturating_sub(1))
                    .ok_or_else(|| anyhow!("weave reassembly length overflow"))?;
                if new_len > MAX_WEAVE_MESSAGE_SIZE {
                    break Err(anyhow!("weave message exceeds safety limit"));
                }
                reassembly.extend_from_slice(&packet[1..]);

                if header & WEAVE_LAST_BIT == 0 {
                    continue;
                }

                if reassembly.len() < 3 {
                    reassembly.clear();
                    continue;
                }

                if reassembly[..3] == [0, 0, 0] {
                    let control_type = if reassembly.len() >= 5 && reassembly[3] == 0x08 {
                        reassembly[4]
                    } else {
                        0
                    };
                    reassembly.clear();

                    match control_type {
                        SOCKET_CTRL_INTRODUCTION => {
                            debug!("{INNER_NAME}: BLE socket introduction");
                            continue;
                        }
                        SOCKET_CTRL_DISCONNECTION => break Ok(()),
                        _ => continue,
                    }
                }

                if reassembly[..3] != QS_SVC_HASH {
                    warn!("{INNER_NAME}: ignoring packet for unknown service hash");
                    reassembly.clear();
                    continue;
                }

                let framed = &reassembly[3..];
                if framed.len() < 4 {
                    break Err(anyhow!("BLE data message is missing frame length"));
                }
                let declared = u32::from_be_bytes([framed[0], framed[1], framed[2], framed[3]]) as usize;
                if declared == 0 || declared > MAX_INBOUND_FRAME_SIZE {
                    break Err(anyhow!("BLE frame length is invalid: {declared}"));
                }
                if framed.len() != 4 + declared {
                    break Err(anyhow!(
                        "BLE frame length mismatch: declared {declared}, got {}",
                        framed.len().saturating_sub(4)
                    ));
                }

                weave_write.write_all(framed).await?;
                reassembly.clear();
            }
            read_result = weave_read.read(&mut read_buffer) => {
                let count = read_result?;
                if count == 0 {
                    break Ok(());
                }

                let new_len = outbound_buffer
                    .len()
                    .checked_add(count)
                    .ok_or_else(|| anyhow!("outbound BLE buffer length overflow"))?;
                if new_len > MAX_WEAVE_MESSAGE_SIZE {
                    break Err(anyhow!("outbound BLE buffer exceeds safety limit"));
                }
                outbound_buffer.extend_from_slice(&read_buffer[..count]);

                while let Some(total) = complete_framed_message_len(&outbound_buffer)? {
                    let framed: Vec<u8> = outbound_buffer.drain(..total).collect();

                    let mut message = Vec::with_capacity(3 + framed.len());
                    message.extend_from_slice(&QS_SVC_HASH);
                    message.extend_from_slice(&framed);

                    let mut offset = 0;
                    while offset < message.len() {
                        let end = (offset + max_payload).min(message.len());
                        let mut header = (send_counter & 0x07) << 4;
                        if offset == 0 {
                            header |= WEAVE_FIRST_BIT;
                        }
                        if end == message.len() {
                            header |= WEAVE_LAST_BIT;
                        }

                        let mut packet = Vec::with_capacity(1 + end - offset);
                        packet.push(header);
                        packet.extend_from_slice(&message[offset..end]);
                        notifier.notify(packet).await?;

                        send_counter = send_counter.wrapping_add(1);
                        offset = end;
                    }
                }
            }
        }
    };

    // The group owns the worker after this bridge closes, including after BWU.
    info!("{INNER_NAME}: weave session closed");
    result
}

async fn run_inbound_worker(
    mut request: InboundRequest<MigratableStream>,
    sender: Sender<ChannelMessage>,
    router: BwuRouter,
    tcp_port: u16,
    lifetime: CancellationToken,
    native: bool,
) {
    let cancellation = request.cancellation();
    let result: Option<Result<(), anyhow::Error>> = crate::manager::session::until_either_stops(&lifetime, &cancellation, async {
        loop {
            request.handle().await?;
            if request.take_bwu_pending() {
                if let Err(error) = request.do_bandwidth_upgrade(&router, tcp_port).await {
                    warn!("{INNER_NAME}: Wi-Fi bandwidth upgrade failed; retaining current transport: {error}");
                }
            }
        }
    }).await;
    let message = match result {
        None => crate::manager::cancellation_message(
            &request.state.id,
            TransferType::Inbound,
            &request.state.state,
            request.state.transfer_metadata.clone(),
        ),
        Some(Err(error))
            if error.downcast_ref::<AppError>().is_none()
                && !crate::sessions::terminal(&request.state.state)
                && (!native || request.state.state != State::Initial) =>
        {
            let mut message = inbound_disconnected_message(
                &request.state.id,
                error.to_string().chars().take(512).collect::<String>(),
            );
            message.meta = request.state.transfer_metadata.clone();
            Some(message)
        }
        _ => None,
    };
    if let Some(message) = message {
        if let Some(state) = message.state.clone() {
            request.record_termination(state);
        }
        let _ = sender.send(message);
    } else if lifetime.is_cancelled() || cancellation.is_cancelled() {
        request.record_termination(State::Cancelled);
    }
}

#[cfg(test)]
mod managed_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot0_zero_offset_preserves_mdns_and_deferred_advertising_by_default() {
        let refresh = receiver_discovery_refresh(0, true).expect("slot0 offset zero must refresh");
        assert!(refresh.resend_mdns);
        assert_eq!(
            refresh.advertising,
            Some(ReceiverAdvertisingRefresh::Deferred)
        );
    }

    #[test]
    fn slot0_debug_ab_skips_only_deferred_advertising() {
        let refresh = receiver_discovery_refresh(0, false).expect("slot0 offset zero must refresh");
        assert!(refresh.resend_mdns);
        assert_eq!(refresh.advertising, None);
    }

    #[test]
    fn slot0_nonzero_offset_does_not_refresh_discovery_in_either_variant() {
        for enabled in [true, false] {
            assert_eq!(receiver_discovery_refresh(1, enabled), None);
            assert_eq!(receiver_discovery_refresh(usize::MAX, enabled), None);
        }
    }

    #[test]
    fn slot0_ab_flag_is_debug_only_and_opt_out_only() {
        assert!(slot0_deferred_advertising_enabled_for(true, None));
        assert!(slot0_deferred_advertising_enabled_for(true, Some("1")));
        assert!(slot0_deferred_advertising_enabled_for(true, Some("false")));
        assert!(!slot0_deferred_advertising_enabled_for(true, Some("0")));
        assert!(slot0_deferred_advertising_enabled_for(false, Some("0")));
    }

    #[test]
    fn inbound_error_message_surfaces_disconnected_state() {
        let message = inbound_disconnected_message("ble-weave", "protocol failure");

        assert_eq!(message.id, "ble-weave");
        assert_eq!(message.direction, ChannelDirection::LibToFront);
        assert_eq!(message.rtype, Some(TransferType::Inbound));
        assert_eq!(message.state, Some(State::Disconnected));
        assert_eq!(message.error.as_deref(), Some("protocol failure"));
    }

    #[test]
    fn parses_supported_connection_request_and_clamps_packet_size() {
        let packet = [0x80, 0x00, 0x01, 0x00, 0x01, 0x10, 0x00];
        assert_eq!(
            parse_connection_request(&packet).unwrap(),
            MAX_WEAVE_PACKET_SIZE
        );

        let packet = [0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01];
        assert_eq!(
            parse_connection_request(&packet).unwrap(),
            MIN_WEAVE_PACKET_SIZE
        );
    }

    #[test]
    fn rejects_unsupported_or_malformed_connection_requests() {
        assert!(parse_connection_request(&[0x80]).is_err());
        assert!(parse_connection_request(&[0x80, 0x00, 0x02, 0x00, 0x02, 0x01, 0xfd]).is_err());
        assert!(parse_connection_request(&[0x00, 0x00, 0x01, 0x00, 0x01, 0x01, 0xfd]).is_err());
    }

    #[test]
    fn reports_complete_framed_messages_safely() {
        assert_eq!(complete_framed_message_len(&[0, 0, 0]).unwrap(), None);

        let mut frame = 3_u32.to_be_bytes().to_vec();
        frame.extend_from_slice(&[1, 2, 3]);
        assert_eq!(complete_framed_message_len(&frame).unwrap(), Some(7));

        assert!(complete_framed_message_len(&0_u32.to_be_bytes()).is_err());
    }
    #[test]
    fn connection_request_preserves_supported_boundary_packet_sizes() {
        let min = [0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x14];
        let max = [0x80, 0x00, 0x01, 0x00, 0x01, 0x01, 0xfd];

        assert_eq!(
            parse_connection_request(&min).unwrap(),
            MIN_WEAVE_PACKET_SIZE
        );
        assert_eq!(
            parse_connection_request(&max).unwrap(),
            MAX_WEAVE_PACKET_SIZE
        );
    }

    #[test]
    fn connection_request_rejects_wrong_command_and_inverted_version_range() {
        let wrong_command = [0x81, 0x00, 0x01, 0x00, 0x01, 0x00, 0x14];
        let inverted_range = [0x80, 0x00, 0x02, 0x00, 0x01, 0x00, 0x14];

        assert!(parse_connection_request(&wrong_command).is_err());
        assert!(parse_connection_request(&inverted_range).is_err());
    }

    #[test]
    fn connection_confirm_encodes_version_and_packet_size() {
        assert_eq!(
            connection_confirm(0x01fd),
            [
                WEAVE_CONTROL | WEAVE_CMD_CONN_CONFIRM,
                0x00,
                0x01,
                0x01,
                0xfd
            ]
        );
    }

    #[test]
    fn framed_message_length_handles_boundaries_and_trailing_bytes() {
        let max_prefix = (MAX_INBOUND_FRAME_SIZE as u32).to_be_bytes();
        assert_eq!(complete_framed_message_len(&max_prefix).unwrap(), None);

        let too_large = ((MAX_INBOUND_FRAME_SIZE + 1) as u32).to_be_bytes();
        assert!(complete_framed_message_len(&too_large).is_err());

        let incomplete = [0_u8, 0, 0, 3, 1, 2];
        assert_eq!(complete_framed_message_len(&incomplete).unwrap(), None);

        let with_trailing = [0_u8, 0, 0, 1, 0xaa, 0xbb, 0xcc];
        assert_eq!(
            complete_framed_message_len(&with_trailing).unwrap(),
            Some(5)
        );
    }
}
