use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex};

/// Routes a Wi-Fi bandwidth-upgrade TCP connection from the application's
/// primary listener to the BLE session that requested it.
#[derive(Clone, Default)]
pub struct BwuRouter {
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<TcpStream>>>>,
}

impl BwuRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(
        &self,
        endpoint_id: String,
    ) -> Result<oneshot::Receiver<TcpStream>, anyhow::Error> {
        if endpoint_id.is_empty() {
            return Err(anyhow!("cannot register an empty BWU endpoint id"));
        }

        let (sender, receiver) = oneshot::channel();
        let mut pending = self.pending.lock().await;
        pending.retain(|_, sender| !sender.is_closed());
        if pending.contains_key(&endpoint_id) {
            return Err(anyhow!(
                "bandwidth upgrade is already pending for endpoint {endpoint_id}"
            ));
        }
        pending.insert(endpoint_id, sender);
        Ok(receiver)
    }

    /// Route a modern BWU connection by its current endpoint id, or by the
    /// previous endpoint id explicitly carried by CLIENT_INTRODUCTION during a
    /// dynamic role switch. Exact current-id ownership always takes precedence.
    pub async fn route_with_alias(
        &self,
        endpoint_id: &str,
        last_endpoint_id: Option<&str>,
        socket: TcpStream,
    ) -> Result<String, TcpStream> {
        let mut pending = self.pending.lock().await;
        pending.retain(|_, sender| !sender.is_closed());
        let route_id = if pending.contains_key(endpoint_id) {
            endpoint_id.to_owned()
        } else if let Some(last_endpoint_id) =
            last_endpoint_id.filter(|id| pending.contains_key(*id))
        {
            last_endpoint_id.to_owned()
        } else {
            return Err(socket);
        };

        let Some(sender) = pending.remove(&route_id) else {
            return Err(socket);
        };
        drop(pending);

        sender.send(socket).map(|()| route_id)
    }

    pub async fn cancel(&self, endpoint_id: &str) {
        self.pending.lock().await.remove(endpoint_id);
    }

    pub async fn has_pending(&self) -> bool {
        let mut pending = self.pending.lock().await;
        pending.retain(|_, sender| !sender.is_closed());
        !pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn route_delivers_the_registered_socket() {
        let router = BwuRouter::new();
        let receiver = router.register("peer-route".to_owned()).await.unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = tokio::net::TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        let routed_as = router
            .route_with_alias("peer-route", None, server)
            .await
            .unwrap();
        assert_eq!(routed_as, "peer-route");
        let pending_after_route = router.has_pending().await;
        assert!(
            !pending_after_route,
            "successful BWU routing must consume the pending endpoint"
        );

        let delivered = tokio::time::timeout(std::time::Duration::from_millis(250), receiver)
            .await
            .expect("BWU route did not deliver the socket")
            .expect("BWU route sender dropped without a socket");

        assert_eq!(delivered.local_addr().unwrap(), client.peer_addr().unwrap());
    }

    #[tokio::test]
    async fn registration_is_unique_and_cancellable() {
        let router = BwuRouter::new();

        let receiver = router.register("peer-1234".to_owned()).await.unwrap();
        assert!(router.has_pending().await);
        assert!(router.register("peer-1234".to_owned()).await.is_err());

        router.cancel("peer-1234").await;
        assert!(!router.has_pending().await);
        drop(receiver);

        assert!(router.register("peer-1234".to_owned()).await.is_ok());
    }
    #[tokio::test]
    async fn empty_endpoint_registration_is_rejected() {
        let router = BwuRouter::new();

        assert!(router.register(String::new()).await.is_err());
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn routing_unknown_endpoint_returns_socket() {
        let router = BwuRouter::new();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let _client = tokio::net::TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        assert!(router
            .route_with_alias("missing-peer", None, server)
            .await
            .is_err());
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn dropped_route_receiver_returns_socket_and_consumes_registration() {
        let router = BwuRouter::new();
        let receiver = router.register("peer-dropped".to_owned()).await.unwrap();
        drop(receiver);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let _client = tokio::net::TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        assert!(router
            .route_with_alias("peer-dropped", None, server)
            .await
            .is_err());
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn concurrent_duplicate_registration_allows_only_one_owner() {
        let router = BwuRouter::new();
        let first_router = router.clone();
        let second_router = router.clone();

        let (first, second) = tokio::join!(
            first_router.register("peer-race".to_owned()),
            second_router.register("peer-race".to_owned())
        );

        assert_ne!(first.is_ok(), second.is_ok());
        assert!(router.has_pending().await);
        router.cancel("peer-race").await;
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn dynamic_alias_routes_to_last_endpoint_id() {
        let router = BwuRouter::new();
        let receiver = router.register("peer-old".to_owned()).await.unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let _client = tokio::net::TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        let routed_as = router
            .route_with_alias("peer-new", Some("peer-old"), server)
            .await
            .unwrap();
        assert_eq!(routed_as, "peer-old");
        assert!(!router.has_pending().await);
        drop(receiver);
    }

    #[tokio::test]
    async fn exact_endpoint_id_wins_over_dynamic_alias() {
        let router = BwuRouter::new();
        let current_receiver = router.register("peer-new".to_owned()).await.unwrap();
        let old_receiver = router.register("peer-old".to_owned()).await.unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let _client = tokio::net::TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();

        let routed_as = router
            .route_with_alias("peer-new", Some("peer-old"), server)
            .await
            .unwrap();
        assert_eq!(routed_as, "peer-new");
        assert!(router.pending.lock().await.contains_key("peer-old"));
        assert!(!router.pending.lock().await.contains_key("peer-new"));

        router.cancel("peer-old").await;
        drop(current_receiver);
        drop(old_receiver);
    }

    #[tokio::test]
    async fn cancelling_unknown_endpoint_is_idempotent() {
        let router = BwuRouter::new();

        router.cancel("missing-peer").await;
        router.cancel("missing-peer").await;

        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn given_abandoned_bwu_waiter_when_checked_or_registered_then_closed_route_is_pruned() {
        // Arrange
        let router = BwuRouter::new();
        let old = router.register("peer".into()).await.unwrap();
        // Act
        drop(old);
        // Assert
        assert!(!router.has_pending().await);
        let current = router.register("peer".into()).await.unwrap();
        assert!(router.has_pending().await);
        router.cancel("peer").await;
        let replacement = router.register("peer".into()).await.unwrap();
        drop(current);
        assert!(router.has_pending().await);
        drop(replacement);
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn given_closed_current_route_when_alias_is_live_then_socket_routes_only_to_live_waiter()
    {
        // Arrange
        let router = BwuRouter::new();
        let current = router.register("current".into()).await.unwrap();
        let previous = router.register("previous".into()).await.unwrap();
        drop(current);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (socket, _) = listener.accept().await.unwrap();
        // Act
        let route = router
            .route_with_alias("current", Some("previous"), socket)
            .await
            .unwrap();
        // Assert
        assert_eq!(route, "previous");
        assert_eq!(
            previous.await.unwrap().peer_addr().unwrap(),
            client.local_addr().unwrap()
        );
        assert!(!router.has_pending().await);
    }
}
