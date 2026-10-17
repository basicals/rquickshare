use super::*;
use crate::test_bus::Broker;
use tokio::sync::mpsc;
use zbus::{fdo, message::Header};
struct Request {
    closed: mpsc::Sender<()>,
}
#[zbus::interface(name = "org.freedesktop.portal.Request")]
impl Request {
    fn close(&self) {
        self.closed.try_send(()).unwrap();
    }
}
struct Portal {
    version: u32,
    code: u32,
    admitted: mpsc::Sender<OwnedObjectPath>,
    closed: mpsc::Sender<()>,
}
#[zbus::interface(name = "org.freedesktop.portal.OpenURI")]
impl Portal {
    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        self.version
    }
    async fn open_file(
        &self,
        parent: &str,
        fd: Fd<'_>,
        options: HashMap<String, OwnedValue>,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        self.open_directory(parent, fd, options, header, connection)
            .await
    }
    async fn open_directory(
        &self,
        parent: &str,
        fd: Fd<'_>,
        options: HashMap<String, OwnedValue>,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        let invalid = || fdo::Error::InvalidArgs("Portal contract mismatch".into());
        if !parent.is_empty() || options.len() != 1 {
            return Err(invalid());
        }
        let token = <&str>::try_from(options.get("handle_token").ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        let suffix = token.strip_prefix("rquickshare_").ok_or_else(invalid)?;
        if suffix.len() != 32 || !suffix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        let sender = header.sender().unwrap().as_str();
        let path: OwnedObjectPath = format!(
            "{PATH}/request/{}/{token}",
            sender.trim_start_matches(':').replace('.', "_")
        )
        .try_into()
        .unwrap();
        let file = File::from(fd.as_fd().try_clone_to_owned().unwrap());
        if rustix::fs::fcntl_getfl(&file).map_err(|_| invalid())? & rustix::fs::OFlags::ACCMODE
            != rustix::fs::OFlags::RDONLY
        {
            return Err(invalid());
        }
        if header
            .member()
            .is_some_and(|member| member.as_str() == "OpenFile")
        {
            if !file.metadata().map_err(|_| invalid())?.is_dir() {
                return Err(invalid());
            }
        } else {
            use std::os::unix::fs::FileExt;
            let mut bytes = [0; 8];
            file.read_exact_at(&mut bytes, 0).map_err(|_| invalid())?;
            if &bytes != b"received" {
                return Err(invalid());
            }
        }
        connection
            .object_server()
            .at(
                path.clone(),
                Request {
                    closed: self.closed.clone(),
                },
            )
            .await
            .unwrap();
        self.admitted.try_send(path.clone()).unwrap();
        // Signal before method reply deliberately exercises subscription ordering.
        if self.code == 93 {
            connection
                .emit_signal(
                    Some(sender),
                    path.as_str(),
                    REQUEST,
                    "Response",
                    &"malformed",
                )
                .await
                .unwrap();
        } else if self.code != 99 && self.code != 92 {
            connection
                .emit_signal(
                    Some(sender),
                    path.as_str(),
                    REQUEST,
                    "Response",
                    &(self.code, HashMap::<String, OwnedValue>::new()),
                )
                .await
                .unwrap();
        }
        Ok(if self.code == 92 {
            "/org/freedesktop/portal/desktop/request/wrong"
                .try_into()
                .unwrap()
        } else {
            path
        })
    }
}
fn file() -> (tempfile::NamedTempFile, Arc<File>) {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"received").unwrap();
    let reader = Arc::new(File::open(file.path()).unwrap());
    (file, reader)
}
async fn server(
    broker: &Broker,
    version: u32,
    code: u32,
) -> (
    Connection,
    mpsc::Receiver<OwnedObjectPath>,
    mpsc::Receiver<()>,
) {
    let connection = broker.connection().await;
    let (admitted, receive) = mpsc::channel(4);
    let (closed, end) = mpsc::channel(4);
    connection
        .object_server()
        .at(
            PATH,
            Portal {
                version,
                code,
                admitted,
                closed,
            },
        )
        .await
        .unwrap();
    connection.request_name(SERVICE).await.unwrap();
    (connection, receive, end)
}
#[tokio::test]
async fn given_private_portal_when_response_precedes_reply_or_is_invalid_then_only_valid_success_is_acknowledged_and_owned_requests_close(
) {
    // Arrange / Act / Assert: external portal fake by its real D-Bus contract; no live desktop.
    for (version, code, expected) in [
        (3, 0, Ok(())),
        (3, 1, Err(Error::Cancelled)),
        (3, 2, Err(Error::Rejected)),
        (3, 3, Err(Error::Protocol)),
        (3, 92, Err(Error::Protocol)),
        (3, 93, Err(Error::Protocol)),
        (2, 0, Err(Error::Unsupported)),
    ] {
        let broker = Broker::new();
        let (server, mut admitted, mut closed) = server(&broker, version, code).await;
        let (_source, file) = file();
        assert_eq!(
            show_on(broker.connection().await, file, CancellationToken::new()).await,
            expected
        );
        if version == 2 {
            assert!(admitted.try_recv().is_err());
        }
        if matches!(code, 3 | 92 | 93) {
            closed.recv().await.unwrap();
        }
        server.close().await.unwrap();
    }
    let broker = Broker::new();
    let (_source, file) = file();
    assert_eq!(
        show_on(broker.connection().await, file, CancellationToken::new()).await,
        Err(Error::Unavailable)
    );
}
#[tokio::test]
async fn given_pending_portal_when_gui_wait_drops_or_owner_disappears_then_close_is_owned_and_ingress_shutdown_joins(
) {
    // Arrange / Act / Assert: event-controlled waiting, no sleeps or real portal.
    for lost in [false, true] {
        let broker = Broker::new();
        let (server, mut admitted, mut closed) = server(&broker, 3, 99).await;
        let (_source, file) = file();
        let owner = crate::ingress::Ingress::new();
        let permit = owner.gate.try_acquire().unwrap();
        let cancel = CancellationToken::new();
        let weak_file = Arc::downgrade(&file);
        let work = show_on(broker.connection().await, file, cancel.clone());
        let mut receive = owner.reveal_job(&tokio::runtime::Handle::current(), permit, work);
        tokio::select! {
            path = admitted.recv() => { path.unwrap(); }
            result = &mut receive => panic!("Portal ended before request admission: {result:?}"),
        }
        assert!(owner.gate.try_acquire().is_none());
        if lost {
            server.release_name(SERVICE).await.unwrap();
            assert_eq!(receive.await.unwrap(), Err(Error::Unavailable));
        } else {
            drop(receive);
            cancel.cancel();
        }
        owner.close().await;
        closed.recv().await.unwrap();
        assert!(weak_file.upgrade().is_none());
        assert!(!owner.subscribe().borrow().busy);
        assert!(owner.gate.try_acquire().is_none());
        server.close().await.unwrap();
    }
}
#[test]
fn given_portal_message_when_sender_or_descriptors_differ_then_reply_validation_fails_closed() {
    // Arrange / Act / Assert
    let valid = Message::signal(PATH, REQUEST, "Response")
        .unwrap()
        .sender(":1.23")
        .unwrap()
        .build(&(0u32, HashMap::<String, OwnedValue>::new()))
        .unwrap();
    assert_eq!(checked(&valid, ":1.23"), Ok(()));
    assert_eq!(checked(&valid, ":1.24"), Err(Error::Protocol));
    let (_source, file) = file();
    let invalid = Message::signal(PATH, REQUEST, "Response")
        .unwrap()
        .sender(":1.23")
        .unwrap()
        .build(&Fd::from(file.as_fd()))
        .unwrap();
    assert_eq!(checked(&invalid, ":1.23"), Err(Error::Protocol));
}

#[tokio::test]
async fn given_directory_contents_action_when_fd_is_regular_then_it_is_rejected_before_any_desktop_connection(
) {
    let (_source, file) = file();
    assert_eq!(
        show_directory_contents(file, CancellationToken::new()).await,
        Err(Error::Protocol)
    );
}
#[tokio::test]
async fn given_checked_directory_when_contents_are_requested_then_portal_receives_directory_only_and_acknowledges_response(
) {
    let broker = Broker::new();
    let (server, mut admitted, _) = server(&broker, 3, 0).await;
    let directory = tempfile::tempdir().unwrap();
    let file = Arc::new(File::open(directory.path()).unwrap());
    assert_eq!(
        show_on_mode(
            broker.connection().await,
            file,
            CancellationToken::new(),
            true
        )
        .await,
        Ok(())
    );
    assert!(admitted.try_recv().is_ok());
    server.close().await.unwrap();
}
