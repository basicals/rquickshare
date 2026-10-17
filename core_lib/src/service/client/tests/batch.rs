use super::*;
use crate::service::{
    batch::INTERFACE as BATCH,
    bus::{INTERFACE as MANAGER, PATH, SERVICE},
};

async fn fixture_many() -> (PrivateBus, Harness, NativeClient) {
    let private = PrivateBus::new();
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    coordinator
        .set_endpoints(vec!["peer".into(), "second".into()])
        .unwrap();
    let host = Harness::with_endpoints(
        SelectionBus::connect(private.socket().await).await.unwrap(),
        coordinator,
        vec![
            ("peer".into(), "First phone".into()),
            ("second".into(), "Second phone".into()),
        ],
    );
    let client = NativeClient::connect(private.socket().await).await.unwrap();
    (private, host, client)
}
async fn prepare(client: &mut NativeClient) -> (tempfile::NamedTempFile, Selection) {
    let (file, files) = source();
    let upload = client.begin_selection().await.unwrap();
    client.append_batch(&upload, 0, &files).await.unwrap();
    let selected = client.commit_selection(&upload).await.unwrap();
    (file, selected)
}
fn endpoints() -> Vec<String> {
    vec!["peer".into(), "second".into()]
}

#[tokio::test]
async fn given_many_recipient_command_when_sdk_and_controller_send_then_captured_names_and_daemon_queue_outlive_each_client(
) {
    scenario(async {
        // Arrange
        let (private, host, mut client) = fixture_many().await;
        let (_file, selected) = prepare(&mut client).await;
        let capture: state::Metadata = client
            .wire(MANAGER, "BeginSnapshot", &())
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        // Act
        let transfers = client.send_many(&selected, endpoints()).await.unwrap();
        assert_eq!(transfers.len(), 2);
        let (next, old): (u32, Vec<(String, String)>) = client
            .wire(BATCH, "GetRecipientPage", &(capture.0.as_str(), 0u32))
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!((next, old), (0, vec![]));
        let (empty_next, empty_links): (u32, Vec<(String, String)>) = client
            .wire(
                BATCH,
                "GetTransferEndpointPage",
                &(capture.0.as_str(), 0u32),
            )
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!((empty_next, empty_links), (0, vec![]));
        client
            .wire(MANAGER, "EndSnapshot", &capture.0)
            .await
            .unwrap();
        let view = client.read_state().await.unwrap();
        // Assert
        assert!(view.selections.is_empty());
        assert_eq!(view.transfers.len(), 2);
        assert_eq!(view.resources.selections, 1);
        assert_eq!(view.resources.reserved_handles, 3);
        assert_eq!(view.transfers[0].recipient.as_deref(), Some("First phone"));
        assert_eq!(view.transfers[1].recipient.as_deref(), Some("Second phone"));
        assert_eq!(
            view.transfers[0].associated_endpoint.as_deref(),
            Some("peer")
        );
        assert_eq!(
            view.transfers[1].associated_endpoint.as_deref(),
            Some("second")
        );
        assert!(!format!("{view:?}").contains("First phone"));
        client.close().await.unwrap();
        let second = NativeClient::connect(private.socket().await).await.unwrap();
        let controller = controller::PanelController::spawn(second);
        let handle = controller.handle();
        let mut updates = handle.subscribe();
        updates
            .wait_for(|state| state.view.is_some())
            .await
            .unwrap();
        let controller::ActionResult::Selection(selection) = handle
            .choose_text(crate::prepared_source::NativeText::new("Batch text".into()).unwrap())
            .unwrap()
            .wait()
            .await
            .unwrap()
        else {
            panic!("expected prepared text")
        };
        let result = handle
            .send_many(selection, endpoints())
            .unwrap()
            .wait()
            .await
            .unwrap();
        let controller::ActionResult::Transfers(ids) = result else {
            panic!("expected one batch result")
        };
        assert_eq!(ids.len(), 2);
        assert_eq!(updates.borrow().view.as_ref().unwrap().transfers.len(), 4);
        controller.close().await.unwrap();
        let (core, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert_eq!(core.queue.len(), 4);
        assert!(core.clients.is_empty());
        assert_eq!(core.usage().selections, 2);
    })
    .await;
}

#[tokio::test]
async fn given_lost_batch_reply_or_ack_when_recovered_then_member_receipts_replay_without_duplicate_transfers(
) {
    for method in ["Send", "AcknowledgeThrough"] {
        scenario(async {
            // Arrange
            let (_private, host, mut client) = fixture_many().await;
            let (_file, selected) = prepare(&mut client).await;
            let sequence = client.last + 1;
            let reached = hook(&mut client, method, true);
            // Act
            interrupted(client.send_many(&selected, endpoints()), reached).await;
            assert!(client.has_pending());
            if method == "Send" {
                assert!(matches!(
                    client
                        .wire(OPERATIONS_INTERFACE, "GetReceipt", &sequence)
                        .await,
                    Err(ClientError::Rejected(crate::service::Error::InvalidState))
                ));
                let mut reversed = endpoints();
                reversed.reverse();
                assert!(matches!(
                    client
                        .wire(BATCH, "Send", &(selected.id(), reversed, sequence))
                        .await,
                    Err(ClientError::Rejected(crate::service::Error::ReplayConflict))
                ));
            }
            let outcome = client.recover_pending().await.unwrap().unwrap();
            let Outcome::Operation(receipt) = outcome else {
                panic!("expected batch receipt")
            };
            let OperationValue::Transfers(ids) = receipt.result else {
                panic!("expected members")
            };
            // Assert
            assert_eq!(ids.len(), 2);
            assert!(!client.has_pending());
            let view = client.read_state().await.unwrap();
            assert_eq!(view.transfers.len(), 2);
            assert_eq!(
                ids,
                view.transfers
                    .iter()
                    .map(|view| view.transfer.clone())
                    .collect::<Vec<_>>()
            );
            assert!(matches!(
                client
                    .wire(BATCH, "Send", &(selected.id(), endpoints(), sequence))
                    .await,
                Err(ClientError::Rejected(crate::service::Error::RequestExpired))
            ));
            client.close().await.unwrap();
            let (core, result) = host.finish().await;
            assert_eq!(result, Ok(()));
            assert_eq!(core.queue.len(), 2);
        })
        .await;
    }
}

#[tokio::test]
async fn given_invalid_batch_wire_or_foreign_source_when_requested_then_no_member_or_prepared_source_is_consumed(
) {
    use std::os::fd::AsFd;

    use zbus::zvariant::Fd;
    scenario(async {
        // Arrange
        let (private,host,mut client)=fixture_many().await;let (file,selected)=prepare(&mut client).await;
        let raw=private.client().await;
        // Act / Assert
        for peers in [vec![],vec!["peer".to_owned();11],vec!["peer".into(),"peer".into()],vec!["bad id".into()],vec!["a".repeat(257)]] {
            let result=raw.call_method(Some(SERVICE),PATH,Some(BATCH),"Send",&(selected.id(),peers,1u64)).await;
            assert!(matches!(result,Err(zbus::Error::MethodError(name,_,_))if name.as_str()==crate::service::bus::error_name(crate::service::Error::InvalidInput)));
        }
        let result=raw.call_method(Some(SERVICE),PATH,Some(BATCH),"Send",&(selected.id(),endpoints(),1u64)).await;
        assert!(matches!(result,Err(zbus::Error::MethodError(name,_,_))if name.as_str()==crate::service::bus::error_name(crate::service::Error::WrongOwner)));
        for method in ["Send","GetReceipt","GetRecipientPage"] {
            let result=raw.call_method(Some(SERVICE),PATH,Some(BATCH),method,&vec![Fd::from(file.as_fd())]).await;
            assert!(matches!(result,Err(zbus::Error::MethodError(name,_,_))if name.as_str()==crate::service::bus::error_name(crate::service::Error::InvalidInput)));
        }
        assert!(client.send_many(&selected,vec![]).await.is_err());assert!(!client.has_pending());
        assert!(matches!(client.send_many(&selected,vec!["peer".into(),"missing".into()]).await,Err(ClientError::Rejected(crate::service::Error::UnknownEndpoint))));
        assert!(!client.has_pending());let view=client.read_state().await.unwrap();assert_eq!(view.selections,vec![selected]);assert!(view.transfers.is_empty());
        raw.close().await.unwrap();client.close().await.unwrap();let (core,result)=host.finish().await;assert_eq!(result,Ok(()));assert!(core.queue.is_empty());
    }).await;
}

#[tokio::test]
async fn given_queue_api_when_multiple_clicks_reuse_source_then_order_and_preparation_are_preserved(
) {
    scenario(async {
        // Arrange
        let (_private, host, mut client) = fixture_many().await;
        let (_source, selection) = prepare(&mut client).await;
        // Act
        let first = client
            .queue_recipient(&selection, "second".into())
            .await
            .unwrap();
        let second = client
            .queue_recipient(&selection, "peer".into())
            .await
            .unwrap();
        // Assert
        let state = client.read_state().await.unwrap();
        assert_eq!(state.selections, vec![selection]);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert!(first[0].id() < second[0].id());
        assert_eq!(
            state
                .transfers
                .iter()
                .map(|t| t.associated_endpoint.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("second"), Some("peer")]
        );
        assert!(state
            .transfers
            .iter()
            .all(|t| t.phase == crate::service::Phase::Queued));
        client.close().await.unwrap();
        host.finish().await.1.unwrap();
    })
    .await;
}

#[tokio::test]
async fn given_saved_journal_when_read_and_cleared_then_metadata_pages_are_immutable_and_sharing_stays_available(
) {
    scenario(async {
        // Arrange
        use std::os::unix::fs::PermissionsExt;

        use crate::service::journal::{Record, INTERFACE};
        let private = PrivateBus::new();
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let row = Record {
            id: format!("{:032x}-{:016x}", 7, 1),
            peer: "Untrusted peer".into(),
            inbound: true,
            status: "finished".into(),
            kind: "text".into(),
            files: 0,
            total: 4,
            transferred: 4,
            started: 4_000_000_000,
            updated: 4_000_000_000,
        };
        let file = directory.path().join("native-transfers.json");
        std::fs::write(
            &file,
            serde_json::to_vec(
                &serde_json::json!({"schema":1,"last_clear":"","records":[row.clone()]}),
            )
            .unwrap(),
        )
        .unwrap();
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_journal(directory.path().into());
        let host =
            Harness::with_coordinator(bus, Coordinator::new(123, Limits::default()).unwrap());
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        client.read_state().await.unwrap();
        // Act / Assert
        let saved = client.read_journal(None).await.unwrap();
        assert_eq!(saved.error, 0);
        assert_eq!(saved.records, vec![row.clone()]);
        client.clear_journal(&saved.token).await.unwrap();
        let immutable = client
            .wire(INTERFACE, "GetPage", &(saved.token.as_str(), 0u32))
            .await
            .unwrap();
        let (total, rows): (u32, Vec<Record>) = immutable.body().deserialize().unwrap();
        assert_eq!(total, 1);
        assert_eq!(rows, vec![row]);
        let empty = client.retry_journal().await.unwrap();
        assert_eq!(empty.error, 0);
        assert!(empty.records.is_empty());
        client.clear_journal(&saved.token).await.unwrap();
        assert_eq!(
            client
                .wire(INTERFACE, "GetPage", &(saved.token.as_str(), 0u32))
                .await
                .unwrap_err(),
            ClientError::Rejected(crate::service::Error::UnknownSnapshot)
        );
        assert!(client.read_state().await.is_ok());
        client.close().await.unwrap();
        host.finish().await.1.unwrap();
    })
    .await;
}
