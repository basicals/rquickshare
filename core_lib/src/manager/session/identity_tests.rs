use prost::Message;
use tokio::io::AsyncReadExt;
use tokio::sync::Notify;

use super::*;
use crate::identity::{DeviceName, IdentityPolicy};
use crate::location_nearby_connections::OfflineFrame;

struct GatedConnector {
    started: Arc<Notify>,
    release: Arc<Notify>,
}
impl Connector for GatedConnector {
    async fn connect<'a>(&'a self, address: &'a str) -> std::io::Result<TcpStream> {
        self.started.notify_one();
        self.release.notified().await;
        TcpStream::connect(address).await
    }
}

#[tokio::test]
async fn given_admitted_session_when_name_changes_during_connect_then_both_handshake_names_keep_admission_snapshot(
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let identity = IdentityPolicy::new(DeviceName::new("Before").unwrap(), &[]);
    let connector = Arc::new(GatedConnector {
        started: started.clone(),
        release: release.clone(),
    });
    let (sender, _) = tokio::sync::broadcast::channel(16);
    let context = SessionContext::new(*b"TEST", sender, connector, None)
        .with_identity(Some(identity.clone()));
    let stop = CancellationToken::new();
    let lifetime = stop.clone();
    let info = SendInfo {
        id: "identity-test".into(),
        addr: listener.local_addr().unwrap().to_string(),
        name: "Peer".into(),
        ob: crate::OutboundPayload::Text("Payload".into()),
    };
    let task = tokio::spawn(async move { context.execute_outbound(lifetime, info).await });
    started.notified().await;
    identity
        .set_override(Some(&DeviceName::new("After").unwrap()))
        .unwrap();
    release.notify_one();
    let (mut peer, _) = listener.accept().await.unwrap();
    let length = peer.read_u32().await.unwrap() as usize;
    assert!(length < 4096);
    let mut bytes = vec![0; length];
    peer.read_exact(&mut bytes).await.unwrap();
    let request = OfflineFrame::decode(bytes.as_slice())
        .unwrap()
        .v1
        .unwrap()
        .connection_request
        .unwrap();
    assert_eq!(request.endpoint_name.as_deref(), Some(b"Before".as_slice()));
    let info = request.endpoint_info.unwrap();
    assert_eq!(usize::from(info[17]), "Before".len());
    assert_eq!(&info[18..], b"Before");
    assert_eq!(identity.snapshot().name.as_str(), "After");
    stop.cancel();
    task.await.unwrap();
}
