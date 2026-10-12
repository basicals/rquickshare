use super::*;
use crate::file_source::{OpenedSelection, SelectionLimits};
use crate::service::{ControlAction, Phase};

fn message<B: Serialize + Type>(method: &str, body: &B) -> Message {
    Message::method_call(PATH, method)
        .unwrap()
        .interface(INTERFACE)
        .unwrap()
        .build(body)
        .unwrap()
}

fn setup(limits: Limits) -> (Coordinator, ClientId, SelectionId) {
    let mut coordinator = Coordinator::new(123, limits).unwrap();
    coordinator.set_endpoints(vec!["nearby".into()]).unwrap();
    let client = coordinator.connect().unwrap();
    let source = tempfile::NamedTempFile::new().unwrap();
    source.as_file().set_len(7).unwrap();
    let file = OpenedFile::adopt(File::open(source.path()).unwrap(), "document".into()).unwrap();
    let selection = OpenedSelection::from_files(vec![file], SelectionLimits::default()).unwrap();
    let id = coordinator.create_selection(client, selection).unwrap();
    (coordinator, client, id)
}

pub(super) async fn committed(client: &Connection, source: &File) -> (String, String) {
    let upload = begin(client, "choose").await;
    append(client, &upload, 0, &[(source, "folder/document")])
        .await
        .unwrap();
    let selected = call(client, "CommitSelection", &upload)
        .await
        .unwrap()
        .body()
        .deserialize()
        .unwrap();
    (upload, selected)
}

#[tokio::test]
async fn given_committed_selection_when_sent_and_replayed_then_one_service_owned_send_survives_disconnect(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        coordinator.set_endpoints(vec!["nearby".into()]).unwrap();
        let mut host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let client = private.client().await;
        let source = tempfile::NamedTempFile::new().unwrap();
        source.as_file().set_len(7).unwrap();
        let file = File::open(source.path()).unwrap();
        let (_, selected) = committed(&client, &file).await;
        // Act
        let args = (&selected, "nearby", "send");
        let sent: String = call(&client, "SendSelection", &args)
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        let replay: String = call(&client, "SendSelection", &args)
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        rejected(
            call(&client, "SendSelection", &(&selected, "other", "send")).await,
            "ReplayConflict",
        );
        client.close().await.unwrap();
        host.clients.wait_for(|count| *count == 0).await.unwrap();
        let (mut coordinator, outcome) = host.finish().await;
        // Assert
        assert_eq!(outcome, Ok(()));
        assert_eq!(sent, replay);
        assert!(coordinator.clients.is_empty() && coordinator.selections.is_empty());
        assert_eq!(coordinator.transfers.len(), 1);
        assert_eq!(
            coordinator.usage(),
            ResourceUsage {
                selections: 1,
                reserved_handles: 2,
                selected_bytes: 7
            }
        );
        let dispatch = coordinator.next_send().unwrap().unwrap();
        assert_eq!(token(dispatch.transfer.0, dispatch.transfer.1), sent);
        assert_eq!(dispatch.endpoint, "nearby");
        assert_eq!(dispatch.selection.total_bytes(), 7);
        assert!(coordinator.next_send().unwrap().is_none());
        assert!(source.path().is_file());
    })
    .await;
}

#[tokio::test]
async fn given_foreign_selection_when_sent_or_discarded_then_owner_retains_it_and_can_discard_once()
{
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        coordinator.set_endpoints(vec!["nearby".into()]).unwrap();
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let owner = private.client().await;
        let foreign = private.client().await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        let (upload_id, selected) = committed(&owner, &file).await;
        // Act / Assert
        rejected(
            call(&foreign, "SendSelection", &(&selected, "nearby", "send")).await,
            "WrongOwner",
        );
        rejected(
            call(&foreign, "DiscardSelection", &selected).await,
            "WrongOwner",
        );
        assert_eq!(upload(&owner, &upload_id).await.2, selected);
        for _ in 0..2 {
            call(&owner, "DiscardSelection", &selected)
                .await
                .unwrap()
                .body()
                .deserialize::<()>()
                .unwrap();
        }
        assert_eq!(upload(&owner, &upload_id).await.2, selected);
        rejected(
            call(&owner, "SendSelection", &(&selected, "nearby", "send")).await,
            "UnknownSelection",
        );
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert_eq!(coordinator.usage(), ResourceUsage::default());
        assert!(coordinator.transfers.is_empty() && coordinator.queue.is_empty());
        assert!(source.path().is_file());
    })
    .await;
}

#[tokio::test]
async fn given_current_consent_when_answered_over_bus_then_only_exact_replays_preserve_pending_intent(
) {
    scenario(async {
        for accept in [false, true] {
            // Arrange
            let private = PrivateBus::new();
            let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
            let inbound = coordinator.register_inbound().unwrap();
            coordinator
                .observe_details(
                    inbound,
                    Phase::AwaitingConsent,
                    std::sync::Arc::new(crate::sessions::details::TransferDetails {
                        status: crate::sessions::details::DetailsStatus::Ready,
                        sender: "phone".into(),
                        pin: "0123".into(),
                        total_bytes: 3,
                        files: vec![crate::sessions::details::FileDetail {
                            name: "item".into(),
                            size: 3,
                        }]
                        .into(),
                        ..Default::default()
                    }),
                )
                .unwrap();
            let revision = coordinator.transfers[&inbound]
                .view
                .consent_revision
                .unwrap();
            let id = token(inbound.0, inbound.1);
            let host = Harness::with_coordinator(
                SelectionBus::connect(private.socket().await).await.unwrap(),
                coordinator,
            );
            let client = private.client().await;
            // Act
            rejected(
                call(&client, "Respond", &(&id, revision - 1, accept)).await,
                "StaleConsent",
            );
            for _ in 0..2 {
                call(&client, "Respond", &(&id, revision, accept))
                    .await
                    .unwrap()
                    .body()
                    .deserialize::<()>()
                    .unwrap();
            }
            rejected(
                call(&client, "Respond", &(&id, revision, !accept)).await,
                "StaleConsent",
            );
            let (coordinator, outcome) = host.finish().await;
            // Assert: admission is not processing acknowledgement or completion.
            assert_eq!(outcome, Ok(()));
            let view = &coordinator.transfers[&inbound].view;
            assert_eq!(view.phase, Phase::AwaitingConsent);
            assert_eq!(view.consent_revision, Some(revision));
            let pending = view.pending_control.unwrap();
            assert_eq!(pending.transfer, inbound);
            assert_eq!(
                pending.action,
                if accept {
                    ControlAction::Accept
                } else {
                    ControlAction::Reject
                }
            );
            assert_eq!(pending.revision, revision + 2); // one authenticated client, one answer
        }
    })
    .await;
}

#[tokio::test]
async fn given_service_owned_queue_when_other_client_cancels_then_no_dispatch_or_source_reservation_remains(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        coordinator.set_endpoints(vec!["nearby".into()]).unwrap();
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let owner = private.client().await;
        let panel = private.client().await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        let (upload_id, selected) = committed(&owner, &file).await;
        let transfer: String = call(&owner, "SendSelection", &(&selected, "nearby", "send"))
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        // Act
        for _ in 0..2 {
            call(&panel, "Cancel", &transfer)
                .await
                .unwrap()
                .body()
                .deserialize::<()>()
                .unwrap();
        }
        let replay: String = call(&owner, "SendSelection", &(&selected, "nearby", "send"))
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!(upload(&owner, &upload_id).await.2, selected);
        let (mut coordinator, outcome) = host.finish().await;
        // Assert
        assert_eq!(outcome, Ok(()));
        assert_eq!(replay, transfer);
        assert_eq!(coordinator.transfers.len(), 1);
        assert_eq!(
            coordinator.transfers.values().next().unwrap().view.phase,
            Phase::Cancelled
        );
        assert_eq!(coordinator.usage(), ResourceUsage::default());
        assert!(coordinator.next_send().unwrap().is_none());
        assert!(source.path().is_file());
    })
    .await;
}

#[test]
fn given_invalid_control_bodies_or_tokens_when_decoded_then_every_rejection_preserves_snapshot() {
    // Arrange
    let (mut coordinator, client, selection) = setup(Limits::default());
    let id = token(selection.0, selection.1);
    let before = coordinator.snapshot(client).unwrap();
    let source = tempfile::NamedTempFile::new().unwrap();
    let file = File::open(source.path()).unwrap();
    // Act / Assert
    for method in ["DiscardSelection", "SendSelection", "Respond", "Cancel"] {
        for invalid in [
            message(method, &()),
            message(method, &42u32),
            message(method, &(&id, "extra")),
        ] {
            assert_eq!(
                execute(&mut coordinator, client, method, &invalid).err(),
                Some(Error::InvalidInput)
            );
            assert_eq!(coordinator.snapshot(client).unwrap(), before);
        }
        for invalid in [
            "".into(),
            token(124, 1),
            token(123, 0),
            id.to_uppercase(),
            "é".repeat(25),
        ] {
            let request = control_message(method, &invalid);
            assert_eq!(
                execute(&mut coordinator, client, method, &request).err(),
                Some(Error::InvalidInput)
            );
            assert_eq!(coordinator.snapshot(client).unwrap(), before);
        }
        let valid = control_message(method, &id);
        let bytes = valid.body();
        let descriptor: std::os::fd::OwnedFd = file.try_clone().unwrap().into();
        // SAFETY: the SDK already encoded this body/signature; an extra unused
        // ancillary descriptor is valid framing but forbidden control policy.
        let with_fd = unsafe {
            Message::method_call(PATH, method)
                .unwrap()
                .build_raw_body(bytes.data(), bytes.signature(), vec![descriptor.into()])
                .unwrap()
        };
        assert_eq!(
            execute(&mut coordinator, client, method, &with_fd).err(),
            Some(Error::InvalidInput)
        );
        assert_eq!(coordinator.snapshot(client).unwrap(), before);
    }
}

fn control_message(method: &str, id: &str) -> Message {
    match method {
        "SendSelection" => message(method, &(id, "nearby", "send")),
        "Respond" => message(method, &(id, 1u64, true)),
        _ => message(method, &id),
    }
}

#[test]
fn given_failed_send_admission_when_retried_with_valid_route_and_capacity_then_selection_is_consumed_once(
) {
    // Arrange
    let (mut coordinator, client, selection) = setup(Limits {
        queued_sends: 1,
        ..Limits::default()
    });
    let id = token(selection.0, selection.1);
    let before = coordinator.snapshot(client).unwrap();
    // Act / Assert
    for (endpoint, request, error) in [
        ("missing".into(), "send".into(), Error::UnknownEndpoint),
        (String::new(), "send".into(), Error::InvalidInput),
        ("x".repeat(257), "send".into(), Error::InvalidInput),
        ("bad endpoint".into(), "send".into(), Error::InvalidInput),
        ("nearby".into(), String::new(), Error::InvalidInput),
        ("nearby".into(), "x".repeat(129), Error::InvalidInput),
        ("nearby".into(), "bad request".into(), Error::InvalidInput),
    ] {
        let request = message("SendSelection", &(&id, endpoint, request));
        assert_eq!(
            execute(&mut coordinator, client, "SendSelection", &request).err(),
            Some(error)
        );
        assert_eq!(coordinator.snapshot(client).unwrap(), before);
    }
    // Exact maximum identifier lengths are valid if the endpoint was discovered.
    let endpoint = "e".repeat(256);
    coordinator.set_endpoints(vec![endpoint.clone()]).unwrap();
    let request = message("SendSelection", &(&id, &endpoint, "r".repeat(128)));
    let first = match execute(&mut coordinator, client, "SendSelection", &request).unwrap() {
        Reply::Token(id) => id,
        _ => panic!("Expected transfer ID"),
    };
    let upload = coordinator.begin_selection(client, "second").unwrap();
    let source = tempfile::NamedTempFile::new().unwrap();
    coordinator
        .append_files(
            client,
            upload,
            0,
            vec![OpenedFile::adopt(File::open(source.path()).unwrap(), "second".into()).unwrap()],
        )
        .unwrap();
    let second = coordinator.commit_selection(client, upload).unwrap();
    let second_id = token(second.0, second.1);
    let second_request = message("SendSelection", &(&second_id, &endpoint, "retry"));
    let before = coordinator.snapshot(client).unwrap();
    assert_eq!(
        execute(&mut coordinator, client, "SendSelection", &second_request).err(),
        Some(Error::Capacity)
    );
    assert_eq!(coordinator.snapshot(client).unwrap(), before);
    execute(
        &mut coordinator,
        client,
        "Cancel",
        &message("Cancel", &first),
    )
    .unwrap();
    assert!(matches!(
        execute(&mut coordinator, client, "SendSelection", &second_request),
        Ok(Reply::Token(_))
    ));
    assert_eq!(coordinator.queue.len(), 1);
    assert!(!coordinator.selections.contains_key(&second));
}

#[test]
fn given_uncertain_active_transfer_when_cancel_supersedes_answer_then_late_response_cannot_acknowledge_it(
) {
    // Arrange
    let (mut coordinator, client, _) = setup(Limits::default());
    let inbound = coordinator.register_inbound().unwrap();
    coordinator
        .observe_details(
            inbound,
            Phase::AwaitingConsent,
            std::sync::Arc::new(crate::sessions::details::TransferDetails {
                status: crate::sessions::details::DetailsStatus::Ready,
                sender: "phone".into(),
                pin: "0123".into(),
                total_bytes: 3,
                files: vec![crate::sessions::details::FileDetail {
                    name: "item".into(),
                    size: 3,
                }]
                .into(),
                ..Default::default()
            }),
        )
        .unwrap();
    let revision = coordinator.transfers[&inbound]
        .view
        .consent_revision
        .unwrap();
    let id = token(inbound.0, inbound.1);
    execute(
        &mut coordinator,
        client,
        "Respond",
        &message("Respond", &(&id, revision, true)),
    )
    .unwrap();
    let answer = coordinator.next_control().unwrap();
    coordinator.mark_desynchronized().unwrap();
    // Act
    let before = coordinator.snapshot(client).unwrap();
    assert_eq!(
        execute(
            &mut coordinator,
            client,
            "Respond",
            &message("Respond", &(&id, revision, true))
        )
        .err(),
        Some(Error::Unsynchronized)
    );
    let selected = token(before.selections[0].0, before.selections[0].1);
    assert_eq!(
        execute(
            &mut coordinator,
            client,
            "SendSelection",
            &control_message("SendSelection", &selected)
        )
        .err(),
        Some(Error::Unsynchronized)
    );
    assert_eq!(coordinator.snapshot(client).unwrap(), before);
    for _ in 0..2 {
        execute(&mut coordinator, client, "Cancel", &message("Cancel", &id)).unwrap();
    }
    // Assert
    let view = &coordinator.transfers[&inbound].view;
    assert_eq!(view.phase, Phase::CancelRequested);
    assert_eq!(view.consent_revision, None);
    let cancellation = view.pending_control.unwrap();
    assert_eq!(cancellation.action, ControlAction::Cancel);
    assert!(cancellation.revision > answer.revision);
    assert_eq!(
        coordinator.acknowledge_control(answer),
        Err(Error::StaleControl)
    );
    assert_eq!(coordinator.next_control(), Some(cancellation));
}

#[test]
fn given_unknown_or_inapplicable_transfer_when_controlled_then_no_false_consent_or_completion_is_created(
) {
    // Arrange
    let (mut coordinator, client, selection) = setup(Limits::default());
    let unknown = token(123, 9999);
    // Act / Assert
    assert!(matches!(
        execute(
            &mut coordinator,
            client,
            "DiscardSelection",
            &message("DiscardSelection", &unknown)
        ),
        Ok(Reply::Empty)
    ));
    for method in ["SendSelection", "Respond", "Cancel"] {
        let expected = if method == "SendSelection" {
            Error::UnknownSelection
        } else {
            Error::UnknownTransfer
        };
        assert_eq!(
            execute(
                &mut coordinator,
                client,
                method,
                &control_message(method, &unknown)
            )
            .err(),
            Some(expected)
        );
    }
    let outbound = coordinator
        .send_selection(client, selection, "nearby", "send")
        .unwrap();
    let id = token(outbound.0, outbound.1);
    let before = coordinator.snapshot(client).unwrap();
    assert_eq!(
        execute(
            &mut coordinator,
            client,
            "Respond",
            &message("Respond", &(&id, 1u64, true))
        )
        .err(),
        Some(Error::StaleConsent)
    );
    assert_eq!(coordinator.snapshot(client).unwrap(), before);
    let dispatch = coordinator.next_send().unwrap().unwrap();
    execute(&mut coordinator, client, "Cancel", &message("Cancel", &id)).unwrap();
    assert_eq!(
        coordinator.transfers[&outbound].view.phase,
        Phase::CancelRequested
    );
    assert_eq!(coordinator.usage().selections, 1); // active owner still retains the source
    coordinator.observe(outbound, Phase::Finished).unwrap();
    drop(dispatch);
    let before = coordinator.snapshot(client).unwrap();
    execute(&mut coordinator, client, "Cancel", &message("Cancel", &id)).unwrap();
    assert_eq!(coordinator.snapshot(client).unwrap(), before);
    assert_eq!(before.transfers[0].phase, Phase::Finished);
    assert_eq!(before.resources, ResourceUsage::default());
}

#[test]
fn given_incomplete_native_consent_when_accepted_then_no_control_is_admitted_and_rejection_remains_available(
) {
    // Arrange / Act / Assert
    for status in [
        crate::sessions::details::DetailsStatus::Unavailable,
        crate::sessions::details::DetailsStatus::Unsupported,
    ] {
        let (mut coordinator, client, _) = setup(Limits::default());
        let inbound = coordinator.register_inbound().unwrap();
        coordinator
            .observe_details(
                inbound,
                Phase::AwaitingConsent,
                std::sync::Arc::new(crate::sessions::details::TransferDetails {
                    status,
                    ..Default::default()
                }),
            )
            .unwrap();
        let revision = coordinator.transfers[&inbound]
            .view
            .consent_revision
            .unwrap();
        let id = token(inbound.0, inbound.1);
        let before = coordinator.snapshot(client).unwrap();
        assert!(matches!(
            execute(
                &mut coordinator,
                client,
                "Respond",
                &message("Respond", &(&id, revision, true))
            ),
            Err(Error::InvalidState)
        ));
        assert_eq!(coordinator.snapshot(client).unwrap(), before);
        assert!(coordinator.next_control().is_none());
        assert!(execute(
            &mut coordinator,
            client,
            "Respond",
            &message("Respond", &(&id, revision, false))
        )
        .is_ok());
        assert_eq!(
            coordinator.next_control().unwrap().action,
            ControlAction::Reject
        );
    }
}
