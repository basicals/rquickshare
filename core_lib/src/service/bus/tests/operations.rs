use super::*;

async fn ordered<B: Serialize + Type>(
    client: &Connection,
    method: &str,
    body: &B,
) -> zbus::Result<Message> {
    client
        .call_method(
            Some(SERVICE),
            PATH,
            Some(OPERATIONS_INTERFACE),
            method,
            body,
        )
        .await
}
async fn id(reply: zbus::Result<Message>) -> String {
    let reply = reply.unwrap();
    assert_eq!(reply.body().signature().to_string_no_parens(), "s");
    assert!(state_message_bounded(&reply));
    reply.body().deserialize().unwrap()
}

#[tokio::test]
async fn given_ordered_bus_operations_when_replies_are_reconciled_then_receipts_and_legacy_contracts_remain_separate(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
        coordinator.set_endpoints(vec!["peer".into()]).unwrap();
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            coordinator,
        );
        let client = private.client().await;
        let foreign = private.client().await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        // Act / Assert
        let upload = id(ordered(&client, "BeginSelection", &1u64).await).await;
        assert_eq!(
            id(ordered(&client, "BeginSelection", &1u64).await).await,
            upload
        );
        let legacy = begin(&client, "1").await;
        assert_ne!(legacy, upload);
        rejected(
            ordered(&foreign, "CommitSelection", &(&upload, 1u64)).await,
            "WrongOwner",
        );
        rejected(
            call(&client, "CommitSelection", &upload).await,
            "InvalidState",
        );
        rejected(
            call(&client, "AbortSelection", &upload).await,
            "InvalidState",
        );
        ordered(&client, "AcknowledgeThrough", &1u64).await.unwrap();
        append(&client, &upload, 0, &[(&file, "a")]).await.unwrap();
        let selected = id(ordered(&client, "CommitSelection", &(&upload, 2u64)).await).await;
        assert_eq!(
            id(ordered(&client, "CommitSelection", &(&upload, 2u64)).await).await,
            selected
        );
        let receipt = ordered(&client, "GetReceipt", &2u64).await.unwrap();
        assert_eq!(receipt.body().signature().to_string_no_parens(), "ss");
        assert_eq!(
            receipt.body().deserialize::<(String, String)>().unwrap(),
            ("commit_selection".into(), selected.clone())
        );
        let transfer =
            id(ordered(&client, "SendSelection", &(&selected, "peer", 3u64)).await).await;
        assert_eq!(
            id(ordered(&client, "SendSelection", &(&selected, "peer", 3u64)).await).await,
            transfer
        );
        rejected(
            ordered(&client, "SendSelection", &(&selected, "other", 3u64)).await,
            "ReplayConflict",
        );
        let second = id(ordered(&client, "BeginSelection", &4u64).await).await;
        ordered(&client, "AbortSelection", &(&second, 5u64))
            .await
            .unwrap();
        let receipt: (String, String) = ordered(&client, "GetReceipt", &5u64)
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!(receipt, ("abort_selection".into(), String::new()));
        ordered(&client, "AcknowledgeThrough", &5u64).await.unwrap();
        let state = ordered(&client, "GetOperationState", &()).await.unwrap();
        assert_eq!(state.body().signature().to_string_no_parens(), "tt");
        assert_eq!(state.body().deserialize::<(u64, u64)>().unwrap(), (5, 5));
        rejected(
            ordered(&client, "GetReceipt", &2u64).await,
            "RequestExpired",
        );
        rejected(
            ordered(&client, "SendSelection", &(&selected, "peer", 3u64)).await,
            "RequestExpired",
        );
        assert_eq!(begin(&client, "1").await, legacy);
        call(&client, "Cancel", &transfer).await.unwrap();
        assert!(source.path().is_file());
        let (coordinator, result) = host.finish().await;
        assert_eq!(result, Ok(()));
        assert_eq!(coordinator.transfers.len(), 1);
        assert_eq!(
            coordinator.transfers.values().next().unwrap().view.phase,
            crate::service::Phase::Cancelled
        );
    })
    .await;
}

#[tokio::test]
async fn given_operation_wire_abuse_when_dispatched_then_bad_bodies_fds_sequences_and_methods_are_rejected(
) {
    scenario(async {
        // Arrange
        let private=PrivateBus::new();
        let host=Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let client=private.client().await;
        let source=tempfile::NamedTempFile::new().unwrap();
        let methods=["BeginSelection","CommitSelection","AbortSelection","SendSelection","GetOperationState","GetReceipt","AcknowledgeThrough"];
        // Act / Assert
        for method in methods {
            rejected(ordered(&client,method,&"wrong-signature").await,"InvalidInput");
            rejected(ordered(&client,method,&Fd::from(source.as_file())).await,"InvalidInput");
        }
        for seq in [0,2,u64::MAX] { rejected(ordered(&client,"BeginSelection",&seq).await,"InvalidInput"); }
        rejected(ordered(&client,"AcknowledgeThrough",&1u64).await,"InvalidInput");
        let error=ordered(&client,"NotAMethod",&()).await.unwrap_err();
        assert!(matches!(error,zbus::Error::MethodError(ref name,_,_) if name.as_str()=="org.freedesktop.DBus.Error.UnknownMethod"));
        assert_eq!(ordered(&client,"GetOperationState",&()).await.unwrap().body().deserialize::<(u64,u64)>().unwrap(),(0,0));
        assert_eq!(host.finish().await.1,Ok(()));
    }).await;
}

#[tokio::test]
async fn given_extension_wire_with_invalid_signatures_or_foreign_tokens_when_called_then_no_source_is_changed(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let host = Harness::with_coordinator(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            Coordinator::new(123, Limits::default()).unwrap(),
        );
        let client = private.client().await;
        let foreign = private.client().await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        let upload = id(ordered(&client, "BeginSelection", &1u64).await).await;
        append(&client, &upload, 0, &[(&file, "old")])
            .await
            .unwrap();
        let base = id(ordered(&client, "CommitSelection", &(&upload, 2u64)).await).await;
        let delta = id(ordered(&client, "BeginSelection", &3u64).await).await;
        append(&client, &delta, 0, &[(&file, "new")]).await.unwrap();
        // Act / Assert
        rejected(
            ordered(&client, "CommitExtension", &(&delta, 4u64)).await,
            "InvalidInput",
        );
        rejected(
            ordered(&client, "CommitExtension", &(Fd::from(&file), &base, 4u64)).await,
            "InvalidInput",
        );
        rejected(
            ordered(&client, "CommitExtension", &(&delta, "invalid", 4u64)).await,
            "InvalidInput",
        );
        rejected(
            ordered(&foreign, "CommitExtension", &(&delta, &base, 1u64)).await,
            "WrongOwner",
        );
        let updated = id(ordered(&client, "CommitExtension", &(&delta, &base, 4u64)).await).await;
        assert_ne!(updated, base);
        assert_eq!(
            id(ordered(&client, "CommitExtension", &(&delta, &base, 4u64)).await).await,
            updated
        );
        drop(client);
        drop(foreign);
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert!(coordinator.transfers.is_empty());
    })
    .await;
}
