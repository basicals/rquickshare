use futures::FutureExt;

use super::*;
use crate::service::Phase;

type Header = (
    String,
    String,
    u64,
    String,
    bool,
    u32,
    u32,
    u64,
    u32,
    u32,
    u32,
);
type TransferRow = (String, String, String, bool, u64, bool, String, u64);

async fn capture(client: &Connection) -> Header {
    let reply = call(client, "BeginSnapshot", &()).await.unwrap();
    assert_eq!(
        reply.body().signature().to_string_no_parens(),
        "sstsbuutuuu"
    );
    assert!(state_message_bounded(&reply));
    reply.body().deserialize().unwrap()
}
async fn read_pages<T: serde::de::DeserializeOwned + Type>(
    client: &Connection,
    method: &str,
    id: &str,
) -> Vec<T> {
    let mut records = Vec::new();
    let mut offset = 0u32;
    loop {
        let reply = call(client, method, &(id, offset)).await.unwrap();
        assert!(state_message_bounded(&reply));
        assert_eq!(
            reply.body().signature().to_string_no_parens(),
            match method {
                "GetEndpointPage" => "ua(ss)",
                "GetSelectionPage" => "uas",
                "GetTransferPage" => "ua(sssbtbst)",
                _ => panic!("Unknown page method"),
            }
        );
        let (next, page): (u32, Vec<T>) = reply.body().deserialize().unwrap();
        assert!(page.len() <= 16);
        assert_eq!(next, offset + page.len() as u32);
        if page.is_empty() {
            break;
        }
        records.extend(page);
        offset = next;
    }
    records
}

#[tokio::test]
async fn given_maximum_native_state_when_captured_then_all_pages_fit_wire_limits_without_private_metadata(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        let client = private.client().await;
        let owner = bus
            .authenticated_client(&mut coordinator, client.unique_name().unwrap().as_str())
            .await
            .unwrap();
        let mut selected = Vec::new();
        let source = tempfile::NamedTempFile::new().unwrap();
        source.as_file().set_len(3).unwrap();
        for n in 0..128 {
            let upload = coordinator
                .begin_selection(owner, &format!("upload-{n}"))
                .unwrap();
            coordinator
                .append_files(
                    owner,
                    upload,
                    0,
                    vec![OpenedFile::adopt(
                        File::open(source.path()).unwrap(),
                        "private-source-name".into(),
                    )
                    .unwrap()],
                )
                .unwrap();
            selected.push(coordinator.commit_selection(owner, upload).unwrap());
        }
        let mut transfers = Vec::new();
        for _ in 0..256 {
            transfers.push(coordinator.register_inbound().unwrap());
        }
        coordinator
            .observe(transfers[0], Phase::AwaitingConsent)
            .unwrap();
        let names: Vec<_> = (0..128)
            .map(|n| (format!("ep-{n:03}"), "é".repeat(128)))
            .collect();
        let host = Harness::with_endpoints(bus, coordinator, names.clone());
        // Act
        let header = capture(&client).await;
        let endpoints: Vec<(String, String)> =
            read_pages(&client, "GetEndpointPage", &header.0).await;
        let selections: Vec<String> = read_pages(&client, "GetSelectionPage", &header.0).await;
        let views: Vec<TransferRow> = read_pages(&client, "GetTransferPage", &header.0).await;
        // Assert
        assert_eq!(
            (header.1.as_str(), header.3.as_str(), header.4),
            ("0000000000000000000000000000007b", "running", true)
        );
        assert_eq!(
            (header.5, header.6, header.7, header.8, header.9, header.10),
            (128, 256, 384, 128, 128, 256)
        );
        assert_eq!(endpoints, names);
        assert_eq!(
            selections,
            selected
                .iter()
                .map(|id| token(id.0, id.1))
                .collect::<Vec<_>>()
        );
        assert_eq!(views.len(), 256);
        assert_eq!(views[0].0, token(transfers[0].0, transfers[0].1));
        assert_eq!(views[0].1, "inbound");
        assert_eq!(views[0].2, "awaiting_consent");
        assert!(views[0].3 && views[0].4 > 0);
        assert_eq!(
            (
                views[1].3,
                views[1].4,
                views[1].5,
                views[1].6.as_str(),
                views[1].7
            ),
            (false, 0, false, "", 0)
        );
        assert!(!format!("{header:?}{endpoints:?}{selections:?}{views:?}")
            .contains("private-source-name"));
        assert!(!format!("{header:?}{endpoints:?}{selections:?}{views:?}")
            .contains(source.path().to_str().unwrap()));
        call(&client, "EndSnapshot", &header.0).await.unwrap();
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert_eq!(coordinator.transfers.len(), 256);
        assert_eq!(coordinator.usage(), ResourceUsage::default());
        assert!(source.path().is_file());
    })
    .await;
}

#[tokio::test]
async fn given_live_selection_change_when_old_snapshot_is_read_then_pages_keep_one_revision_and_replacement_is_explicit(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let client = private.client().await;
        let first = capture(&client).await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        let (upload_id, selected) = super::controls::committed(&client, &file).await;
        // Act / Assert
        assert!(read_pages::<String>(&client, "GetSelectionPage", &first.0)
            .await
            .is_empty());
        let second = capture(&client).await;
        assert_ne!(first.0, second.0);
        assert!(second.2 > first.2);
        assert_eq!(second.9, 1);
        assert_eq!(
            read_pages::<String>(&client, "GetSelectionPage", &second.0).await,
            vec![selected.clone()]
        );
        rejected(
            call(&client, "GetSelectionPage", &(&first.0, 0u32)).await,
            "UnknownSnapshot",
        );
        call(&client, "EndSnapshot", &first.0).await.unwrap(); // old absent capture is harmless
        for _ in 0..2 {
            call(&client, "EndSnapshot", &second.0).await.unwrap();
        }
        assert_eq!(upload(&client, &upload_id).await.2, selected); // read cleanup never discards source selection
        let third = capture(&client).await;
        assert_eq!(third.2, second.2);
        assert_eq!(third.9, 1);
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_foreign_disconnected_or_restarted_client_when_view_is_used_then_no_old_read_authority_survives(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let first = private.client().await;
        let second = private.client().await;
        let header = capture(&first).await;
        // Act / Assert
        for method in ["GetEndpointPage", "GetSelectionPage", "GetTransferPage"] {
            rejected(
                call(&second, method, &(&header.0, 0u32)).await,
                "WrongOwner",
            );
        }
        rejected(call(&second, "EndSnapshot", &header.0).await, "WrongOwner");
        first.close().await.unwrap();
        host.clients.wait_for(|count| *count == 1).await.unwrap();
        rejected(
            call(&second, "GetSelectionPage", &(&header.0, 0u32)).await,
            "UnknownSnapshot",
        );
        call(&second, "EndSnapshot", &header.0).await.unwrap();
        let reconnect = private.client().await;
        let replacement = capture(&reconnect).await;
        assert_ne!(replacement.0, header.0);
        assert_eq!(replacement.1, header.1);
        assert_eq!(host.finish().await.1, Ok(()));
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            Coordinator::new(124, Limits::default()).unwrap(),
        );
        let new = capture(&second).await;
        assert_ne!(new.1, replacement.1);
        rejected(
            call(&second, "GetSelectionPage", &(&replacement.0, 0u32)).await,
            "InvalidInput",
        );
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_bad_state_calls_when_received_then_fixed_errors_preserve_the_current_capture() {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let client = private.client().await;
        let header = capture(&client).await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        // Act / Assert
        for method in [
            "BeginSnapshot",
            "GetEndpointPage",
            "GetSelectionPage",
            "GetTransferPage",
            "EndSnapshot",
        ] {
            rejected(call(&client, method, &42u32).await, "InvalidInput");
            rejected(
                call(&client, method, &(Fd::from(&file), &header.0)).await,
                "InvalidInput",
            );
        }
        for method in ["GetEndpointPage", "GetSelectionPage", "GetTransferPage"] {
            for id in [
                String::new(),
                token(124, 1),
                token(123, 0),
                header.0.to_uppercase(),
            ] {
                rejected(call(&client, method, &(id, 0u32)).await, "InvalidInput");
            }
            rejected(
                call(&client, method, &(&header.0, 1u32)).await,
                "InvalidInput",
            );
            rejected(
                call(&client, method, &(&header.0, u32::MAX)).await,
                "InvalidInput",
            );
            rejected(
                call(&client, method, &(token(123, 9999), 0u32)).await,
                "UnknownSnapshot",
            );
        }
        for id in [String::new(), token(124, 1), token(123, 0)] {
            rejected(call(&client, "EndSnapshot", &id).await, "InvalidInput");
        }
        assert!(read_pages::<String>(&client, "GetSelectionPage", &header.0)
            .await
            .is_empty());
        let next = capture(&client).await;
        assert_eq!(next.2, header.2);
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_full_snapshot_budget_when_one_connection_leaves_then_another_capture_can_be_admitted(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let mut clients = Vec::new();
        let mut headers = Vec::new();
        for _ in 0..16 {
            let client = private.client().await;
            headers.push(capture(&client).await);
            clients.push(client);
        }
        let blocked = private.client().await;
        // Act / Assert
        rejected(call(&blocked, "BeginSnapshot", &()).await, "Capacity");
        let replacement = capture(&clients[0]).await;
        assert_ne!(replacement.0, headers[0].0);
        clients.remove(0).close().await.unwrap();
        host.clients.wait_for(|count| *count == 16).await.unwrap();
        let admitted = capture(&blocked).await;
        assert_eq!((admitted.8, admitted.9, admitted.10), (0, 0, 0));
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_registered_client_when_state_changes_then_hints_are_directed_coalesced_and_not_replayed_for_unchanged_reads(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        let client = private.client().await;
        let stranger = private.client().await;
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        let authenticated = bus
            .authenticated_client(&mut coordinator, client.unique_name().unwrap().as_str())
            .await
            .unwrap();
        let rule = zbus::MatchRule::builder()
            .msg_type(MessageType::Signal)
            .interface(INTERFACE)
            .unwrap()
            .member("StateChanged")
            .unwrap()
            .build();
        let mut hints = MessageStream::for_match_rule(rule.clone(), &client, Some(8))
            .await
            .unwrap();
        let mut other_hints = MessageStream::for_match_rule(rule, &stranger, Some(8))
            .await
            .unwrap();
        // Act
        coordinator.register_inbound().unwrap();
        coordinator.register_inbound().unwrap();
        bus.publish_changes(&coordinator).await.unwrap();
        let hint = hints.next().await.unwrap().unwrap();
        let (epoch, revision): (String, u64) = hint.body().deserialize().unwrap();
        // Assert
        assert_eq!(epoch, format!("{:032x}", coordinator.epoch));
        assert_eq!(revision, coordinator.revision);
        assert_eq!(
            hint.header().destination().unwrap().as_str(),
            client.unique_name().unwrap().as_str()
        );
        let before = bus.notified.clone();
        let captured = bus
            .state
            .capture(&mut coordinator, authenticated, std::iter::empty())
            .unwrap();
        assert_eq!(captured.2, revision);
        bus.publish_changes(&coordinator).await.unwrap();
        assert_eq!(bus.notified, before);
        // A broker round-trip plus all stream message polling is an ordering fence,
        // not a timed guess about whether another hint was sent.
        bus.broker_call("GetId", &()).await.unwrap();
        client
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "GetId",
                &(),
            )
            .await
            .unwrap();
        stranger
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "GetId",
                &(),
            )
            .await
            .unwrap();
        assert!(hints.next().now_or_never().is_none());
        assert!(other_hints.next().now_or_never().is_none());
        coordinator.mark_desynchronized().unwrap();
        bus.publish_changes(&coordinator).await.unwrap();
        let hint = hints.next().await.unwrap().unwrap();
        assert_eq!(
            hint.body().deserialize::<(String, u64)>().unwrap().1,
            coordinator.revision
        );
        assert_eq!(
            bus.state
                .transfer_page(authenticated, &captured.0, 0)
                .unwrap()
                .1
                .len(),
            2
        );
        bus.disconnect_clients(&mut coordinator).unwrap();
        assert!(bus.notified.is_empty());
        bus.close_connection().await;
    })
    .await;
}

#[test]
fn given_serialized_state_reply_limits_when_checked_then_exact_body_and_frame_caps_are_enforced() {
    // Arrange / Act / Assert: serialized string body = 4-byte length + bytes + NUL.
    for (size, accepted) in [(48 * 1024 - 5, true), (48 * 1024 - 4, false)] {
        let message = Message::signal(PATH, INTERFACE, "StateChanged")
            .unwrap()
            .build(&"x".repeat(size))
            .unwrap();
        assert_eq!(state_message_bounded(&message), accepted);
    }
    // Large headers alone must not evade the complete-frame cap.
    let message = Message::method_call(format!("/{}", "x".repeat(65 * 1024)), "BeginSnapshot")
        .unwrap()
        .build(&())
        .unwrap();
    assert!(message.body().is_empty());
    assert!(!state_message_bounded(&message));
}
