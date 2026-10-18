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

    // Actual isolated TCP sockets, not a fake BwuRouter or byte channel.
    async fn bwu_tcp_pair() -> (TcpStream, TcpStream) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let peer = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (inbound, _) = listener.accept().await.unwrap();
        (peer, inbound)
    }

    #[tokio::test]
    async fn given_untrusted_bwu_alias_when_routing_then_socket_and_other_owners_remain_intact() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Arrange: two unrelated live peer registrations and a third party.
        let router = BwuRouter::new();
        let mut alice = router.register("alice".into()).await.unwrap();
        let mut bob = router.register("bob".into()).await.unwrap();
        let (mut outside, incoming) = bwu_tcp_pair().await;

        // Act: neither the current id nor claimed prior id belongs to a waiter.
        let mut returned = router
            .route_with_alias("untrusted", Some("forged"), incoming)
            .await
            .unwrap_err();

        // Assert: a rejected route retains the exact usable socket and owners.
        outside.write_all(b"SAFE").await.unwrap();
        let mut original = [0_u8; 4];
        returned.read_exact(&mut original).await.unwrap();
        assert_eq!(&original, b"SAFE");
        assert_eq!(returned.peer_addr().unwrap(), outside.local_addr().unwrap());
        assert!(matches!(
            alice.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        assert!(matches!(
            bob.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        assert!(router.has_pending().await);
        assert_eq!(router.pending.lock().await.len(), 2);
        router.cancel("alice").await;
        router.cancel("bob").await;
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn given_two_live_bwu_owners_when_alias_conflicts_then_exact_and_previous_bytes_never_mix()
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Arrange: exact current owner and alternate old-id owner both wait.
        let router = BwuRouter::new();
        let previous = router.register("previous".into()).await.unwrap();
        let current = router.register("current".into()).await.unwrap();
        let (mut current_peer, current_socket) = bwu_tcp_pair().await;
        let (mut previous_peer, previous_socket) = bwu_tcp_pair().await;

        // Act: exact id must win, without consuming the alias owner.
        assert_eq!(
            router
                .route_with_alias("current", Some("previous"), current_socket)
                .await
                .unwrap(),
            "current"
        );
        assert_eq!(router.pending.lock().await.len(), 1);
        assert!(router.pending.lock().await.contains_key("previous"));

        // The second peer can still route only to its original owner.
        assert_eq!(
            router
                .route_with_alias("previous", None, previous_socket)
                .await
                .unwrap(),
            "previous"
        );
        let mut owned_current = current.await.unwrap();
        let mut owned_previous = previous.await.unwrap();
        current_peer.write_all(b"CUR").await.unwrap();
        previous_peer.write_all(b"OLD").await.unwrap();
        let mut current_bytes = [0_u8; 3];
        let mut previous_bytes = [0_u8; 3];
        owned_current.read_exact(&mut current_bytes).await.unwrap();
        owned_previous
            .read_exact(&mut previous_bytes)
            .await
            .unwrap();
        assert_eq!(&current_bytes, b"CUR");
        assert_eq!(&previous_bytes, b"OLD");
        assert_eq!(owned_current.peer_addr().unwrap(), current_peer.local_addr().unwrap());
        assert_eq!(
            owned_previous.peer_addr().unwrap(),
            previous_peer.local_addr().unwrap()
        );
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn given_consumed_bwu_route_when_another_socket_arrives_then_one_shot_owner_cannot_replay()
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let router = BwuRouter::new();
        let initial_owner = router.register("session".into()).await.unwrap();
        let (mut first_peer, first_socket) = bwu_tcp_pair().await;
        assert_eq!(
            router
                .route_with_alias("session", None, first_socket)
                .await
                .unwrap(),
            "session"
        );

        // Act: the session's ownership token was already consumed.
        let (mut late_peer, late_socket) = bwu_tcp_pair().await;
        let mut returned_late = router
            .route_with_alias("session", None, late_socket)
            .await
            .unwrap_err();
        late_peer.write_all(b"LATE").await.unwrap();
        let mut late_bytes = [0_u8; 4];
        returned_late.read_exact(&mut late_bytes).await.unwrap();
        assert_eq!(&late_bytes, b"LATE");
        assert!(!router.has_pending().await);

        // Assert: only the original accepted TCP socket reaches that owner.
        let mut owned_first = initial_owner.await.unwrap();
        first_peer.write_all(b"FIRST").await.unwrap();
        let mut original = [0_u8; 5];
        owned_first.read_exact(&mut original).await.unwrap();
        assert_eq!(&original, b"FIRST");

        // A subsequent generation requires a fresh explicit registration.
        let next_owner = router.register("session".into()).await.unwrap();
        let (mut fresh_peer, fresh_socket) = bwu_tcp_pair().await;
        assert_eq!(
            router
                .route_with_alias("session", None, fresh_socket)
                .await
                .unwrap(),
            "session"
        );
        let mut fresh = next_owner.await.unwrap();
        fresh_peer.write_all(b"NEXT").await.unwrap();
        let mut next = [0_u8; 4];
        fresh.read_exact(&mut next).await.unwrap();
        assert_eq!(&next, b"NEXT");
        assert!(!router.has_pending().await);
    }

    #[tokio::test]
    async fn given_cancelled_bwu_generation_when_late_socket_arrives_then_new_owner_only_accepts_fresh_route()
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let router = BwuRouter::new();
        let old_receiver = router.register("reused".into()).await.unwrap();

        // Act: user/worker closes this specific admission generation.
        router.cancel("reused").await;
        assert!(old_receiver.await.is_err());
        assert!(!router.has_pending().await);

        // A late TCP socket remains usable by caller but cannot be admitted.
        let (mut late_peer, late_socket) = bwu_tcp_pair().await;
        let mut returned = router
            .route_with_alias("reused", None, late_socket)
            .await
            .unwrap_err();
        late_peer.write_all(b"STALE").await.unwrap();
        let mut stale = [0_u8; 5];
        returned.read_exact(&mut stale).await.unwrap();
        assert_eq!(&stale, b"STALE");

        // A new session explicitly registers and receives ONLY its new socket.
        let fresh_owner = router.register("reused".into()).await.unwrap();
        let (mut fresh_peer, fresh_socket) = bwu_tcp_pair().await;
        assert_eq!(
            router
                .route_with_alias("reused", None, fresh_socket)
                .await
                .unwrap(),
            "reused"
        );
        let mut delivered = fresh_owner.await.unwrap();
        fresh_peer.write_all(b"FRESH").await.unwrap();
        let mut fresh = [0_u8; 5];
        delivered.read_exact(&mut fresh).await.unwrap();
        assert_eq!(&fresh, b"FRESH");
        assert_eq!(delivered.peer_addr().unwrap(), fresh_peer.local_addr().unwrap());
        assert!(!router.has_pending().await);
    }

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
