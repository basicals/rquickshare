use futures::FutureExt;

use super::*;
use crate::service::bus::tests::{Harness, PrivateBus};
use crate::service::bus::SelectionBus;
use crate::service::client::tests::{fixture, hook, interrupted, scenario, source};
use crate::service::{Coordinator, Limits};

fn metadata() -> Metadata {
    (
        format!("{:032x}-{:016x}", 123, 1),
        format!("{:032x}", 123),
        10,
        "running".into(),
        true,
        0,
        0,
        0,
        0,
        0,
        0,
    )
}
fn row() -> TransferRow {
    (
        format!("{:032x}-{:016x}", 123, 1),
        "inbound".into(),
        "active".into(),
        false,
        0,
        false,
        String::new(),
        0,
    )
}
async fn seeded(count: usize, waiting: bool) -> (PrivateBus, Harness, NativeClient) {
    let bus = PrivateBus::new();
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    for _ in 0..count {
        let id = coordinator.register_inbound().unwrap();
        if waiting {
            coordinator
                .observe_details(
                    id,
                    Phase::AwaitingConsent,
                    Arc::new(TransferDetails {
                        status: DetailsStatus::Ready,
                        sender: "phone".into(),
                        pin: "0123".into(),
                        total_bytes: 3,
                        files: vec![FileDetail {
                            name: "item".into(),
                            size: 3,
                        }]
                        .into(),
                        ..Default::default()
                    }),
                )
                .unwrap();
        }
    }
    let endpoints: Vec<_> = (0..count.min(128))
        .map(|n| (format!("ep-{n:03}"), "Nearby 界".into()))
        .collect();
    coordinator
        .set_endpoints(endpoints.iter().map(|(id, _)| id.clone()).collect())
        .unwrap();
    let host = Harness::with_endpoints(
        SelectionBus::connect(bus.socket().await).await.unwrap(),
        coordinator,
        endpoints,
    );
    let client = NativeClient::connect(bus.socket().await).await.unwrap();
    (bus, host, client)
}

#[tokio::test]
async fn given_frozen_enums_when_projected_then_every_known_value_and_direction_constraint_is_checked(
) {
    // Arrange
    let (_bus, host, client) = fixture().await;
    // Act / Assert
    for (code, value) in [
        ("running", Lifecycle::Running),
        ("draining", Lifecycle::Draining),
        ("stopped", Lifecycle::Stopped),
    ] {
        assert_eq!(lifecycle(code), Ok(value));
    }
    for (code, value) in [
        ("inbound", Direction::Inbound),
        ("outbound", Direction::Outbound),
    ] {
        assert_eq!(direction(code), Ok(value));
    }
    for (code, value) in [
        ("accept", ControlAction::Accept),
        ("reject", ControlAction::Reject),
        ("cancel", ControlAction::Cancel),
    ] {
        assert_eq!(action(code), Ok(value));
    }
    for (code, value) in [
        ("queued", Phase::Queued),
        ("active", Phase::Active),
        ("awaiting_consent", Phase::AwaitingConsent),
        ("receiving", Phase::Receiving),
        ("sending", Phase::Sending),
        ("cancel_requested", Phase::CancelRequested),
        ("finished", Phase::Finished),
        ("rejected", Phase::Rejected),
        ("cancelled", Phase::Cancelled),
        ("failed", Phase::Failed),
        ("disconnected", Phase::Disconnected),
    ] {
        assert_eq!(phase(code), Ok(value));
        for direction in ["inbound", "outbound"] {
            let mut input = row();
            input.1 = direction.into();
            input.2 = code.into();
            if value == Phase::AwaitingConsent {
                input.3 = true;
                input.4 = 2;
            }
            let invalid = (matches!(value, Phase::Queued | Phase::Sending)
                && direction == "inbound")
                || (matches!(value, Phase::AwaitingConsent | Phase::Receiving)
                    && direction == "outbound");
            let result = client.transfer_view(input, 10);
            if invalid {
                assert_eq!(result, Err(ClientError::Protocol));
            } else {
                assert_eq!(result.unwrap().phase, value);
            }
        }
    }
    for value in ["", "unknown", "Running"] {
        assert_eq!(lifecycle(value), Err(ClientError::Protocol));
        assert_eq!(direction(value), Err(ClientError::Protocol));
        assert_eq!(phase(value), Err(ClientError::Protocol));
        assert_eq!(action(value), Err(ClientError::Protocol));
    }
    client.close().await.unwrap();
    assert_eq!(host.finish().await.1, Ok(()));
}

#[tokio::test]
async fn given_optional_flags_and_revision_edges_when_decoded_then_absence_and_current_intents_are_unambiguous(
) {
    // Arrange
    let (_bus, host, client) = fixture().await;
    // Act / Assert
    for present in [false, true] {
        for value in [0, 1, 10, 11, u64::MAX] {
            let result = optional_revision(present, value, 10);
            if !present && value == 0 {
                assert_eq!(result, Ok(None));
            } else if present && value > 0 && value <= 10 {
                assert_eq!(result, Ok(Some(value)));
            } else {
                assert_eq!(result, Err(ClientError::Protocol));
            }
        }
    }
    for invalid in 0..8 {
        let mut input = row();
        match invalid {
            0 => {
                input.3 = true;
                input.4 = 1;
            }
            1 => input.2 = "awaiting_consent".into(),
            2 => input.4 = 1,
            3 => input.6 = "cancel".into(),
            4 => input.7 = 1,
            5 => {
                input.5 = true;
                input.7 = 1;
            }
            6 => {
                input.5 = true;
                input.6 = "cancel".into();
                input.7 = 1;
            }
            _ => {
                input.2 = "awaiting_consent".into();
                input.3 = true;
                input.4 = 2;
                input.5 = true;
                input.6 = "accept".into();
                input.7 = 2;
            }
        }
        assert_eq!(client.transfer_view(input, 10), Err(ClientError::Protocol));
    }
    for action in ["accept", "reject"] {
        let mut input = row();
        input.2 = "awaiting_consent".into();
        input.3 = true;
        input.4 = 2;
        input.5 = true;
        input.6 = action.into();
        input.7 = 3;
        assert!(client.transfer_view(input, 10).is_ok());
    }
    let mut input = row();
    input.2 = "cancel_requested".into();
    input.5 = true;
    input.6 = "cancel".into();
    input.7 = 3;
    assert!(client.transfer_view(input, 10).is_ok());
    client.close().await.unwrap();
    assert_eq!(host.finish().await.1, Ok(()));
}

#[tokio::test]
async fn given_collection_caps_or_invalid_order_when_projected_then_oversized_or_duplicate_views_are_rejected(
) {
    // Arrange
    let (_bus, host, client) = fixture().await;
    // Act / Assert
    for (endpoints, selections, transfers) in [(0, 0, 0), (128, 128, 256)] {
        let mut meta = metadata();
        meta.8 = endpoints;
        meta.9 = selections;
        meta.10 = transfers;
        assert!(client.empty_view(&meta).is_ok());
    }
    for index in 0..6 {
        let mut meta = metadata();
        match index {
            0 => meta.8 = 129,
            1 => meta.9 = 129,
            2 => meta.10 = 257,
            3 => meta.8 = u32::MAX,
            4 => meta.1 = format!("{:032x}", 456),
            _ => meta.2 = 0,
        };
        assert_eq!(client.empty_view(&meta), Err(ClientError::Protocol));
    }
    assert_eq!(sorted(["a", "b"]), Ok(()));
    assert_eq!(sorted(std::iter::empty()), Ok(()));
    assert_eq!(sorted(["a", "a"]), Err(ClientError::Protocol));
    assert_eq!(sorted(["b", "a"]), Err(ClientError::Protocol));
    assert!(!format!(
        "{:?}",
        Endpoint {
            id: "id".into(),
            name: "Private device label".into()
        }
    )
    .contains("Private"));
    client.close().await.unwrap();
    assert_eq!(host.finish().await.1, Ok(()));
}

#[tokio::test]
async fn given_real_collection_boundaries_when_all_pages_are_read_then_complete_views_are_bounded_and_typed(
) {
    scenario(async {
        // Arrange / Act / Assert
        for count in [0, 1, 16, 17, 128, 256] {
            let (_bus, host, mut client) = seeded(count, false).await;
            let view = client.read_state().await.unwrap();
            assert_eq!(view.transfers.len(), count);
            assert_eq!(view.endpoints.len(), count.min(128));
            assert!(view.selections.is_empty());
            assert!(view
                .transfers
                .iter()
                .all(|v| v.direction == Direction::Inbound
                    && v.phase == Phase::Active
                    && v.consent_revision.is_none()));
            assert_eq!(client.cached_state(), Some(&view));
            assert!(client.capture.is_none());
            assert!(!client.state_is_stale());
            client.close().await.unwrap();
            assert_eq!(host.finish().await.1, Ok(()));
        }
    })
    .await;
}

#[tokio::test]
async fn given_interrupted_read_when_capture_is_replaced_then_previous_view_is_never_partially_published(
) {
    scenario(async {
        // Arrange
        let (_bus, host, mut client) = seeded(17, false).await;
        let old = client.read_state().await.unwrap();
        let upload = client.begin_selection().await.unwrap();
        let reached = hook(&mut client, "GetTransferPage", true);
        interrupted(client.read_state(), reached).await;
        // Act / Assert
        assert_eq!(client.cached_state(), Some(&old));
        assert!(client.capture.is_some());
        assert!(client.state_is_stale());
        let view = client.read_state().await.unwrap();
        assert!(view.revision > old.revision);
        assert_eq!(view.transfers.len(), 17);
        assert_eq!(view.resources.selections, 1);
        assert!(client.capture.is_none());
        client.abort_selection(&upload).await.unwrap();
        client.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_unconsumed_invalidation_hints_when_many_operations_complete_then_socket_replies_keep_flowing_and_latest_view_reconciles(
) {
    scenario(async {
        // Arrange
        let (_bus, host, mut client) = fixture().await;
        let old = client.read_state().await.unwrap();
        // Act: exceed both subscription budgets without polling next_change.
        for _ in 0..40 {
            let upload = client.begin_selection().await.unwrap();
            client.abort_selection(&upload).await.unwrap();
        }
        // Assert
        let revision = client.next_change().await.unwrap();
        assert!(revision > old.revision);
        assert!(client.state_is_stale());
        let view = client.read_state().await.unwrap();
        assert!(view.revision >= revision);
        assert_eq!(view.resources, ResourceUsage::default());
        assert!(!client.state_is_stale());
        assert!(client.next_change().now_or_never().is_none());
        client.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_owner_loss_when_waiting_for_changes_then_failure_is_sticky_and_does_not_require_another_method_call(
) {
    scenario(async {
        // Arrange
        let (_bus, host, mut client) = fixture().await;
        client.read_state().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
        // Act / Assert
        assert_eq!(client.next_change().await, Err(ClientError::DaemonChanged));
        assert_eq!(client.next_change().await, Err(ClientError::DaemonChanged));
        assert!(client.state_is_stale());
        assert_eq!(client.read_state().await, Err(ClientError::DaemonChanged));
        client.close().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn given_service_owned_consent_when_other_same_uid_client_responds_then_exact_intent_is_visible_without_false_completion(
) {
    scenario(async {
        // Arrange / Act / Assert
        for accept in [false, true] {
            let (bus, host, mut client) = seeded(1, true).await;
            let old = client.read_state().await.unwrap();
            let transfer = &old.transfers[0];
            let mut other = NativeClient::connect(bus.socket().await).await.unwrap();
            other
                .respond(
                    &transfer.transfer,
                    transfer.consent_revision.unwrap(),
                    accept,
                )
                .await
                .unwrap();
            let first = other.read_state().await.unwrap();
            other
                .respond(
                    &transfer.transfer,
                    transfer.consent_revision.unwrap(),
                    accept,
                )
                .await
                .unwrap();
            let current = other.read_state().await.unwrap();
            assert_eq!(current.revision, first.revision);
            assert_eq!(current.transfers[0].phase, Phase::AwaitingConsent);
            assert_eq!(
                current.transfers[0].pending_control.unwrap().0,
                if accept {
                    ControlAction::Accept
                } else {
                    ControlAction::Reject
                }
            );
            client.cancel(&current.transfers[0].transfer).await.unwrap();
            let cancelled = client.read_state().await.unwrap();
            assert_eq!(cancelled.transfers[0].phase, Phase::CancelRequested);
            assert_eq!(
                cancelled.transfers[0].pending_control.unwrap().0,
                ControlAction::Cancel
            );
            other.close().await.unwrap();
            client.close().await.unwrap();
            assert_eq!(host.finish().await.1, Ok(()));
        }
    })
    .await;
}

#[tokio::test]
async fn given_stale_consent_or_repeated_cancel_when_requested_then_old_rounds_are_rejected_and_cancel_is_idempotent(
) {
    scenario(async {
        // Arrange
        let (_bus, host, mut client) = seeded(1, true).await;
        let view = client.read_state().await.unwrap();
        let transfer = &view.transfers[0];
        let revision = transfer.consent_revision.unwrap();
        // Act / Assert
        assert_eq!(
            client.respond(&transfer.transfer, 0, true).await,
            Err(ClientError::InvalidInput)
        );
        assert_eq!(
            client.respond(&transfer.transfer, revision + 1, true).await,
            Err(ClientError::Rejected(DomainError::StaleConsent))
        );
        client.cancel(&transfer.transfer).await.unwrap();
        let first = client.read_state().await.unwrap();
        client.cancel(&transfer.transfer).await.unwrap();
        let second = client.read_state().await.unwrap();
        assert_eq!(second, first);
        assert_eq!(
            client.respond(&transfer.transfer, revision, true).await,
            Err(ClientError::Rejected(DomainError::StaleConsent))
        );
        let unknown = Transfer {
            id: format!("{:032x}-{:016x}", 123, u64::MAX),
            connection: "different client".into(),
        };
        assert_eq!(
            client.cancel(&unknown).await,
            Err(ClientError::Rejected(DomainError::UnknownTransfer))
        );
        let stale = Transfer {
            id: format!("{:032x}-{:016x}", 456, 1),
            connection: client.identity.clone(),
        };
        assert_eq!(client.cancel(&stale).await, Err(ClientError::ForeignHandle));
        client.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_foreign_or_pending_selection_when_discarded_then_only_owner_idle_selection_can_be_released(
) {
    scenario(async {
        // Arrange
        let (bus, host, mut client) = fixture().await;
        let (original, files) = source();
        let upload = client.begin_selection().await.unwrap();
        client.append_batch(&upload, 0, &files).await.unwrap();
        let selection = client.commit_selection(&upload).await.unwrap();
        let mut other = NativeClient::connect(bus.socket().await).await.unwrap();
        // Act / Assert
        assert_eq!(
            other.discard_selection(&selection).await,
            Err(ClientError::ForeignHandle)
        );
        let reached = hook(&mut client, "SendSelection", false);
        interrupted(client.send_selection(&selection, "peer"), reached).await;
        assert_eq!(
            client.discard_selection(&selection).await,
            Err(ClientError::PendingOperation)
        );
        client.send_selection(&selection, "peer").await.unwrap();
        client.discard_selection(&selection).await.unwrap();
        let view = client.read_state().await.unwrap();
        assert!(view.selections.is_empty());
        assert_eq!(view.transfers.len(), 1);
        assert_eq!(view.transfers[0].phase, Phase::Queued);
        other.cancel(&view.transfers[0].transfer).await.unwrap();
        assert_eq!(
            client.read_state().await.unwrap().transfers[0].phase,
            Phase::Cancelled
        );
        assert!(original.path().is_file());
        other.close().await.unwrap();
        client.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_interrupted_control_reply_when_exact_action_is_retried_then_no_new_consent_round_or_control_intent_is_created(
) {
    scenario(async {
        // Arrange / Act / Assert
        for method in ["Respond", "Cancel", "DiscardSelection"] {
            for after in [false, true] {
                let (_bus, host, mut client) = seeded(1, true).await;
                let view = client.read_state().await.unwrap();
                let transfer = &view.transfers[0];
                let (source, files) = source();
                let upload = client.begin_selection().await.unwrap();
                client.append_batch(&upload, 0, &files).await.unwrap();
                let selection = client.commit_selection(&upload).await.unwrap();
                let reached = hook(&mut client, method, after);
                match method {
                    "Respond" => {
                        interrupted(
                            client.respond(
                                &transfer.transfer,
                                transfer.consent_revision.unwrap(),
                                true,
                            ),
                            reached,
                        )
                        .await
                    }
                    "Cancel" => interrupted(client.cancel(&transfer.transfer), reached).await,
                    _ => interrupted(client.discard_selection(&selection), reached).await,
                }
                match method {
                    "Respond" => client
                        .respond(&transfer.transfer, transfer.consent_revision.unwrap(), true)
                        .await
                        .unwrap(),
                    "Cancel" => client.cancel(&transfer.transfer).await.unwrap(),
                    _ => client.discard_selection(&selection).await.unwrap(),
                }
                let first = client.read_state().await.unwrap();
                match method {
                    "Respond" => client
                        .respond(&transfer.transfer, transfer.consent_revision.unwrap(), true)
                        .await
                        .unwrap(),
                    "Cancel" => client.cancel(&transfer.transfer).await.unwrap(),
                    _ => client.discard_selection(&selection).await.unwrap(),
                }
                assert_eq!(client.read_state().await.unwrap(), first);
                assert!(source.path().is_file());
                client.close().await.unwrap();
                assert_eq!(host.finish().await.1, Ok(()));
            }
        }
    })
    .await;
}

#[test]
fn given_detail_header_and_consent_readiness_boundaries_when_decoded_then_malformed_or_partial_descriptions_cannot_enable_acceptance(
) {
    // Arrange
    let header: DetailRow = (
        format!("{:032x}-{:016x}", 123, 1),
        "ready".into(),
        "phone".into(),
        "phone".into(),
        "0123".into(),
        3,
        0,
        1,
    );
    // Act / Assert
    for device in ["unknown", "phone", "tablet", "laptop"] {
        let mut row = header.clone();
        row.3 = device.into();
        assert!(decode_details(row, crate::sessions::details::PayloadKind::Files).is_ok());
    }
    for count in [1, 255, 256, 257] {
        let mut row = header.clone();
        row.7 = count;
        assert_eq!(
            decode_details(row, crate::sessions::details::PayloadKind::Files).is_ok(),
            count <= 256
        );
    }
    for field in 0..12 {
        let mut row = header.clone();
        match field {
            0 => row.1 = "future".into(),
            1 => row.2.clear(),
            2 => row.2 = "a".repeat(257),
            3 => row.2 = "phone\n".into(),
            4 => row.3 = "future".into(),
            5 => row.4 = "12345".into(),
            6 => row.4 = "1a34".into(),
            7 => row.6 = 4,
            8 => row.7 = 0,
            9 => row.1 = "unsupported".into(),
            10 => row.1 = "unavailable".into(),
            _ => row.4 = "１２３４".into(),
        }
        assert!(
            matches!(
                decode_details(row, crate::sessions::details::PayloadKind::Files),
                Err(ClientError::Protocol)
            ),
            "{field}"
        );
    }
    for status in ["unavailable", "unsupported"] {
        let row = (
            header.0.clone(),
            status.into(),
            String::new(),
            "unknown".into(),
            String::new(),
            0,
            0,
            0,
        );
        assert!(decode_details(row, crate::sessions::details::PayloadKind::Files).is_ok());
    }
    let (mut details, _) =
        decode_details(header.clone(), crate::sessions::details::PayloadKind::Files).unwrap();
    details.files = vec![FileDetail {
        name: "item".into(),
        size: 3,
    }]
    .into();
    let mut transfer = TransferView {
        content_revision: None,
        transfer: Transfer {
            id: header.0,
            connection: "test".into(),
        },
        direction: Direction::Inbound,
        phase: Phase::AwaitingConsent,
        consent_revision: Some(1),
        pending_control: None,
        details: Arc::new(details),
    };
    assert!(transfer.can_accept());
    transfer.pending_control = Some((ControlAction::Accept, 2));
    assert!(!transfer.can_accept());
    transfer.pending_control = None;
    transfer.phase = Phase::Receiving;
    assert!(!transfer.can_accept());
    transfer.phase = Phase::AwaitingConsent;
    Arc::make_mut(&mut transfer.details).pin.clear();
    assert!(!transfer.can_accept());
    transfer.details = Arc::default();
    assert!(!transfer.can_accept());
}

#[tokio::test]
async fn given_real_complete_manager2_view_when_read_then_all_files_pin_and_progress_share_one_revision(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        for count in [1, 16, 17, 256] {
            let id = coordinator.register_inbound().unwrap();
            coordinator
                .observe_details(
                    id,
                    Phase::AwaitingConsent,
                    Arc::new(TransferDetails {
                        status: DetailsStatus::Ready,
                        sender: "phone".into(),
                        device: DeviceKind::Phone,
                        pin: "0123".into(),
                        total_bytes: count as u64,
                        files: (0..count)
                            .map(|n| FileDetail {
                                name: format!("{n:03}{}", "a".repeat(253)),
                                size: 1,
                            })
                            .collect::<Vec<_>>()
                            .into(),
                        ..Default::default()
                    }),
                )
                .unwrap();
        }
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        // Act
        let view = client.read_state().await.unwrap();
        // Assert
        assert_eq!(view.transfers.len(), 4);
        assert!(!client.state_is_stale());
        for (transfer, count) in view.transfers.iter().zip([1, 16, 17, 256]) {
            assert!(transfer.can_accept());
            assert_eq!(transfer.details.files.len(), count);
            assert_eq!(
                (
                    transfer.details.pin.as_str(),
                    transfer.details.total_bytes,
                    transfer.details.transferred_bytes
                ),
                ("0123", count as u64, 0)
            );
            assert!(transfer
                .details
                .files
                .iter()
                .all(|file| file.name.len() == 256 && file.size == 1));
        }
        let xml = client
            .wire("org.freedesktop.DBus.Introspectable", "Introspect", &())
            .await
            .unwrap()
            .body()
            .deserialize::<String>()
            .unwrap();
        assert!(xml.contains("RQuickShare.Manager2"));
        assert!(!xml.contains("RQuickShare.Manager1"));
        assert!(client
            .wire(
                "io.github.morozhenko_apps.RQuickShare.Manager1",
                "BeginSnapshot",
                &()
            )
            .await
            .is_err());
        client.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}
