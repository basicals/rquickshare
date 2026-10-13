use super::*;
use crate::service::bus::text::tests::completed;
use crate::service::client::tests::{hook, interrupted, scenario};
use crate::service::{
    bus::{
        tests::{Harness, PrivateBus},
        SelectionBus,
    },
    Coordinator, Limits,
};

pub(crate) async fn fixture(body: &str, ty: PayloadKind) -> (PrivateBus, Harness, NativeClient) {
    let bus = PrivateBus::new();
    let mut coordinator = Coordinator::new(123, Limits::default()).unwrap();
    completed(&mut coordinator, ty, body);
    let host = Harness::with_coordinator(
        SelectionBus::connect(bus.socket().await).await.unwrap(),
        coordinator,
    );
    let client = NativeClient::connect(bus.socket().await).await.unwrap();
    (bus, host, client)
}
#[tokio::test]
async fn given_real_text_wire_when_read_or_interrupted_then_exact_resource_is_returned_without_snapshot_leak(
) {
    scenario(async {
        // Arrange / Act / Assert
        for (ty, body) in [
            (PayloadKind::Text, String::new()),
            (PayloadKind::Url, "https://example.invalid/?private".into()),
            (PayloadKind::Text, "é".repeat(65536)),
        ] {
            let (_bus, host, mut client) = fixture(&body, ty).await;
            let view = client.read_state().await.unwrap();
            let row = &view.transfers[0];
            assert_eq!(row.details.kind, ty);
            assert!(row.details.files.is_empty());
            let revision = row.content_revision.unwrap();
            if !body.is_empty() {
                assert!(!format!("{view:?}").contains(&body));
            }
            let read = client
                .read_received_text(&row.transfer, revision)
                .await
                .unwrap();
            assert_eq!(read.body.text(), body);
            assert_eq!(read.body.kind(), ty);
            assert!(!client.has_pending_text());
            if !body.is_empty() {
                assert!(!format!("{read:?}").contains(&body));
            }
            client.close().await.unwrap();
            assert_eq!(host.finish().await.1, Ok(()));
        }
        for method in ["BeginRead", "GetChunk", "EndRead"] {
            for after in [false, true] {
                let body = "private".repeat(5000);
                let (_bus, host, mut client) = fixture(&body, PayloadKind::Text).await;
                let row = client.read_state().await.unwrap().transfers.remove(0);
                let reached = hook(&mut client, method, after);
                interrupted(
                    client.read_received_text(&row.transfer, row.content_revision.unwrap()),
                    reached,
                )
                .await;
                assert!(client.has_pending_text());
                let read = client.retry_text().await.unwrap();
                assert_eq!(read.body.text(), body);
                assert_eq!(read.transfer, row.transfer);
                assert!(!client.has_pending_text());
                client.close().await.unwrap();
                assert_eq!(host.finish().await.1, Ok(()));
            }
        }
    })
    .await;
}
#[tokio::test]
async fn given_authenticated_text_routes_when_foreign_stale_or_descriptor_arguments_arrive_then_authority_is_rejected(
) {
    scenario(async {
        // Arrange
        let (bus, host, mut owner) = fixture("private-text", PayloadKind::Text).await;
        let mut other = NativeClient::connect(bus.socket().await).await.unwrap();
        let row = owner.read_state().await.unwrap().transfers.remove(0);
        let (reader, _, _): (String, String, u32) = owner
            .wire(
                INTERFACE,
                "BeginRead",
                &(row.transfer.id(), row.content_revision.unwrap()),
            )
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        // Act / Assert
        assert_eq!(
            other
                .read_received_text(&row.transfer, row.content_revision.unwrap())
                .await,
            Err(ClientError::ForeignHandle)
        );
        assert_eq!(
            other
                .wire(INTERFACE, "GetChunk", &(reader.as_str(), 0u32))
                .await
                .unwrap_err(),
            ClientError::Rejected(DomainError::WrongOwner)
        );
        assert_eq!(
            other.wire(INTERFACE, "EndRead", &reader).await.unwrap_err(),
            ClientError::Rejected(DomainError::WrongOwner)
        );
        for id in ["bad".into(), format!("{:032x}-{:016x}", 124, 1)] {
            assert_eq!(
                owner
                    .wire(INTERFACE, "GetChunk", &(id, 0u32))
                    .await
                    .unwrap_err(),
                ClientError::Rejected(DomainError::InvalidInput)
            );
        }
        let file = tempfile::tempfile().unwrap();
        for method in ["GetMetadataPage", "BeginRead", "GetChunk", "EndRead"] {
            assert_eq!(
                owner
                    .wire(INTERFACE, method, &Fd::from(file.as_fd()))
                    .await
                    .unwrap_err(),
                ClientError::Rejected(DomainError::InvalidInput)
            );
        }
        owner.wire(INTERFACE, "EndRead", &reader).await.unwrap();
        owner.wire(INTERFACE, "EndRead", &reader).await.unwrap();
        assert_eq!(
            owner
                .wire(INTERFACE, "GetChunk", &(reader, 0u32))
                .await
                .unwrap_err(),
            ClientError::Rejected(DomainError::UnknownSnapshot)
        );
        other.close().await.unwrap();
        owner.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_controller_read_when_completed_then_private_result_never_enters_watched_panel_state()
{
    scenario(async {
        // Arrange
        use super::super::controller::{ActionResult, PanelController};
        let (_bus, host, mut client) = fixture("private-body", PayloadKind::Text).await;
        let row = client.read_state().await.unwrap().transfers.remove(0);
        let controller = PanelController::spawn(client);
        let handle = controller.handle();
        // Act / Assert
        let ActionResult::Text(read) = handle
            .read_text(row.transfer.clone(), row.content_revision.unwrap())
            .unwrap()
            .wait()
            .await
            .unwrap()
        else {
            panic!("Missing text result")
        };
        assert_eq!(read.body.text(), "private-body");
        assert_eq!(read.transfer, row.transfer);
        let state = handle.subscribe().borrow().clone();
        assert!(!state.text_pending);
        assert!(!format!("{state:?}").contains("private-body"));
        controller.close().await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}
