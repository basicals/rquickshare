use std::future::Future;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use serde::Serialize;
use tokio::sync::watch;

use super::*;
use crate::service::{Lifecycle, Limits, ResourceUsage};

pub(crate) struct PrivateBus {
    child: Child,
    directory: tempfile::TempDir,
    address: String,
}
impl PrivateBus {
    pub(crate) fn address_for_test(&self) -> String {
        self.address.clone()
    }
    pub(crate) fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let address = format!("unix:path={}", directory.path().join("bus").display());
        let child = Command::new("dbus-daemon")
            .args([
                "--session",
                "--nofork",
                "--print-address=1",
                "--address",
                &address,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut bus = Self {
            child,
            directory,
            address: String::new(),
        };
        BufReader::new(bus.child.stdout.take().unwrap())
            .read_line(&mut bus.address)
            .unwrap();
        bus.address = bus.address.trim().into();
        assert!(bus.address.starts_with("unix:path="));
        bus
    }
    pub(crate) async fn socket(&self) -> UnixStream {
        UnixStream::connect(self.directory.path().join("bus"))
            .await
            .unwrap()
    }
    pub(crate) async fn client(&self) -> Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .unwrap()
            .build()
            .await
            .unwrap()
    }
}
impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub(crate) struct Harness {
    stop: CancellationToken,
    task: Option<JoinHandle<(Coordinator, Result<(), BusError>)>>,
    clients: watch::Receiver<usize>,
}
impl Harness {
    pub(crate) fn start(bus: SelectionBus) -> Self {
        Self::with_limits(bus, Limits::default())
    }
    fn with_limits(bus: SelectionBus, limits: Limits) -> Self {
        Self::with_coordinator(bus, Coordinator::new(123, limits).unwrap())
    }
    pub(crate) fn with_coordinator(bus: SelectionBus, coordinator: Coordinator) -> Self {
        Self::with_endpoints(bus, coordinator, Vec::new())
    }
    pub(crate) fn with_endpoints(
        mut bus: SelectionBus,
        mut coordinator: Coordinator,
        endpoints: Vec<(String, String)>,
    ) -> Self {
        let stop = CancellationToken::new();
        let end = stop.clone();
        let failure = bus.failed.clone();
        let (observed, clients) = watch::channel(0);
        let task = tokio::spawn(async move {
            let outcome = loop {
                tokio::select! {
                    _ = end.cancelled() => break Ok(()),
                    _ = failure.cancelled() => break Err(BusError::Transport),
                    message = bus.inbox.recv() => {
                        let Some(message) = message else { break Err(BusError::Transport) };
                        let result = tokio::select! {
                            _ = end.cancelled() => break Ok(()),
                            _ = failure.cancelled() => break Err(BusError::Transport),
                            result = bus.dispatch(&mut coordinator, endpoints.iter().map(|(id, name)| (id.as_str(), name.as_str())), message) => result,
                        };
                        if let Err(error) = result { break Err(error); }
                        let result = tokio::select! {
                            _ = end.cancelled() => break Ok(()),
                            _ = failure.cancelled() => break Err(BusError::Transport),
                            result = bus.publish_changes(&coordinator) => result,
                        };
                        if let Err(error) = result { break Err(error); }
                        observed.send_replace(bus.clients.len());
                    }
                }
            };
            bus.disconnect_clients(&mut coordinator).unwrap();
            bus.close_connection().await;
            (coordinator, outcome)
        });
        Self {
            stop,
            task: Some(task),
            clients,
        }
    }
    pub(crate) async fn finish(mut self) -> (Coordinator, Result<(), BusError>) {
        self.stop.cancel();
        self.task.take().unwrap().await.unwrap()
    }
}

mod controls;
mod operations;
mod settings;
mod state;

#[tokio::test]
async fn given_startup_recovery_when_warning_is_read_then_fixed_wire_code_is_persistent_and_authenticated(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        bus.set_visibility_recovered(true);
        let host = Harness::start(bus);
        let client = private.client().await;
        // Act / Assert
        for _ in 0..2 {
            let warnings: Vec<String> = call(&client, "GetStartupWarnings", &())
                .await
                .unwrap()
                .body()
                .deserialize()
                .unwrap();
            assert_eq!(warnings, vec!["visibility_recovered"]);
        }
        rejected(
            call(&client, "GetStartupWarnings", &1u32).await,
            "InvalidInput",
        );
        assert_eq!(host.finish().await.1, Ok(()));
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let warnings: Vec<String> = call(&client, "GetStartupWarnings", &())
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_waiting_bus_authentication_when_stop_arrives_then_owned_socket_closes_without_a_pump(
) {
    scenario(async {
        // Arrange
        use tokio::io::AsyncReadExt;
        let (stream, mut peer) = UnixStream::pair().unwrap();
        let stop = CancellationToken::new();
        let connecting = SelectionBus::connect_stoppable(stream, stop.clone());
        tokio::pin!(connecting);
        // Act: handshake bytes are an event-driven startup fence.
        let mut bytes = [0; 64];
        tokio::select! {
            result = &mut connecting => panic!("Unexpected early connect: {}", result.is_ok()),
            bytes = peer.read(&mut bytes) => assert!(bytes.unwrap() > 0),
        }
        stop.cancel();
        // Assert
        assert!(connecting.await.unwrap().is_none());
        let mut remaining = Vec::new();
        peer.read_to_end(&mut remaining).await.unwrap();
        assert!(remaining.len() < 64 * 1024);
    })
    .await;
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.stop.cancel();
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

async fn scenario(test: impl Future<Output = ()>) {
    tokio::time::timeout(std::time::Duration::from_secs(30), test)
        .await
        .expect("Private bus scenario must finish");
}
async fn call<B: Serialize + Type>(
    connection: &Connection,
    method: &str,
    body: &B,
) -> zbus::Result<Message> {
    connection
        .call_method(Some(SERVICE), PATH, Some(INTERFACE), method, body)
        .await
}
async fn begin(connection: &Connection, request: &str) -> String {
    call(connection, "BeginSelection", &request)
        .await
        .unwrap()
        .body()
        .deserialize()
        .unwrap()
}
async fn append(
    connection: &Connection,
    id: &str,
    sequence: u32,
    files: &[(&File, &str)],
) -> zbus::Result<Message> {
    let entries: Vec<_> = files
        .iter()
        .map(|(file, name)| (Fd::from(*file), *name))
        .collect();
    call(connection, "AppendFiles", &(id, sequence, entries)).await
}
async fn upload(connection: &Connection, id: &str) -> (u32, String, String) {
    call(connection, "GetUpload", &id)
        .await
        .unwrap()
        .body()
        .deserialize()
        .unwrap()
}
fn rejected(result: zbus::Result<Message>, suffix: &str) {
    match result {
        Err(zbus::Error::MethodError(name, message, _)) => {
            assert!(name.as_str().ends_with(suffix), "Unexpected error: {name}");
            assert_eq!(message.as_deref(), Some("Native request rejected"));
        }
        _ => panic!("Expected a fixed application error"),
    }
}

#[tokio::test]
async fn given_private_bus_when_full_selection_flow_runs_then_wire_receipts_and_atomic_commit_match_fixture(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let client = private.client().await;
        let source = tempfile::NamedTempFile::new().unwrap();
        source.as_file().set_len(7).unwrap();
        let file = File::open(source.path()).unwrap();
        // Act / Assert
        let xml: String = client
            .call_method(
                Some(SERVICE),
                PATH,
                Some("org.freedesktop.DBus.Introspectable"),
                "Introspect",
                &(),
            )
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!(xml, INTROSPECTION);
        let id = begin(&client, "request").await;
        assert_eq!(begin(&client, "request").await, id);
        assert_eq!(
            append(&client, &id, 0, &[(&file, "folder/a")])
                .await
                .unwrap()
                .body()
                .deserialize::<u32>()
                .unwrap(),
            1
        );
        assert_eq!(
            upload(&client, &id).await,
            (1, "preparing".into(), String::new())
        );
        rejected(
            append(&client, &id, 0, &[(&file, "folder/a")]).await,
            "ReplayConflict",
        );
        append(&client, &id, 1, &[(&file, "folder/b")])
            .await
            .unwrap();
        let selected: String = call(&client, "CommitSelection", &id)
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!(
            upload(&client, &id).await,
            (2, "committed".into(), selected.clone())
        );
        assert_eq!(
            call(&client, "CommitSelection", &id)
                .await
                .unwrap()
                .body()
                .deserialize::<String>()
                .unwrap(),
            selected
        );
        rejected(call(&client, "AbortSelection", &id).await, "InvalidState");
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert_eq!(coordinator.usage(), ResourceUsage::default());
        assert!(coordinator.uploads.is_empty() && coordinator.selections.is_empty());
        assert!(source.path().is_file());
    })
    .await;
}

#[tokio::test]
async fn given_foreign_bus_connection_when_same_request_or_token_used_then_ownership_is_not_transferred(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let first = private.client().await;
        let second = private.client().await;
        let id = begin(&first, "request").await;
        // Act / Assert
        assert_ne!(begin(&second, "request").await, id);
        for method in ["GetUpload", "CommitSelection", "AbortSelection"] {
            rejected(call(&second, method, &id).await, "WrongOwner");
        }
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        rejected(append(&second, &id, 0, &[(&file, "a")]).await, "WrongOwner");
        assert_eq!(upload(&first, &id).await.0, 0);
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_disconnected_unique_name_when_observed_then_staged_files_and_receipts_are_released()
{
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let first = private.client().await;
        let id = begin(&first, "request").await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        append(&first, &id, 0, &[(&file, "a")]).await.unwrap();
        host.clients.wait_for(|count| *count == 1).await.unwrap();
        // Act
        first.close().await.unwrap();
        host.clients.wait_for(|count| *count == 0).await.unwrap();
        // Assert
        let second = private.client().await;
        rejected(call(&second, "GetUpload", &id).await, "UnknownUpload");
        assert!(source.path().is_file());
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert_eq!(coordinator.usage(), ResourceUsage::default());
    })
    .await;
}

#[tokio::test]
async fn given_wrong_broker_uid_when_called_then_resources_and_introspection_are_denied() {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let wrong_uid = rustix::process::geteuid().as_raw().wrapping_add(1);
        let host = Harness::start(
            SelectionBus::connect_with_uid(private.socket().await, wrong_uid)
                .await
                .unwrap(),
        );
        let client = private.client().await;
        // Act / Assert
        rejected(
            call(&client, "BeginSelection", &"request").await,
            "AccessDenied",
        );
        rejected(
            client
                .call_method(
                    Some(SERVICE),
                    PATH,
                    Some("org.freedesktop.DBus.Introspectable"),
                    "Introspect",
                    &(),
                )
                .await,
            "AccessDenied",
        );
        let (coordinator, outcome) = host.finish().await;
        assert_eq!(outcome, Ok(()));
        assert!(coordinator.clients.is_empty() && coordinator.uploads.is_empty());
    })
    .await;
}

#[tokio::test]
async fn given_invalid_signature_name_or_descriptor_when_appended_then_valid_prefix_is_not_retained(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let client = private.client().await;
        let id = begin(&client, "request").await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let good = File::open(source.path()).unwrap();
        let other = File::open(source.path()).unwrap();
        let device = File::open("/dev/null").unwrap();
        let directory = File::open(private.directory.path()).unwrap();
        let path_only: File = rustix::fs::open(
            source.path(),
            rustix::fs::OFlags::PATH | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .unwrap()
        .into();
        // Act / Assert
        rejected(
            call(&client, "BeginSelection", &123u32).await,
            "InvalidInput",
        );
        rejected(
            call(&client, "AppendFiles", &(&id, 0u32, vec![(1u32, "name")])).await,
            "InvalidInput",
        );
        for name in [
            "../secret",
            "/absolute",
            "a//b",
            "a\\b",
            "a:b",
            "bad\nname",
            "",
        ] {
            rejected(
                append(&client, &id, 0, &[(&good, "valid"), (&other, name)]).await,
                "InvalidInput",
            );
            assert_eq!(upload(&client, &id).await.0, 0);
        }
        for invalid in [source.as_file(), &device, &directory, &path_only] {
            rejected(
                append(&client, &id, 0, &[(&good, "valid"), (invalid, "secret")]).await,
                "InvalidInput",
            );
            assert_eq!(upload(&client, &id).await.0, 0);
        }
        append(&client, &id, 0, &[(&good, "valid")]).await.unwrap();
        assert_eq!(upload(&client, &id).await.0, 1);
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_repeated_fd_indices_or_excess_entries_when_decoded_then_no_owned_fd_amplification_occurs(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let client = private.client().await;
        let id = begin(&client, "request").await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        // Act / Assert: encode many references to the ONE attached descriptor.
        // Ordinary SDK serialization attaches a separate FD per entry instead.
        for count in [0, 2, 16, 17, 200] {
            let names: Vec<_> = (0..count).map(|n| format!("name-{n}")).collect();
            let entries: Vec<_> = names.iter().map(|name| (0u32, name.as_str())).collect();
            let bytes = zbus::zvariant::to_bytes(
                zbus::zvariant::serialized::Context::new_dbus(zbus::zvariant::Endian::Little, 0),
                &(&id, 0u32, entries),
            )
            .unwrap();
            let owned: std::os::fd::OwnedFd = file.try_clone().unwrap().into();
            // SAFETY: u and h have identical 32-bit wire representation/alignment;
            // every encoded index is zero and exactly one FD is attached. All
            // strings/arrays are SDK-encoded. This is valid D-Bus, invalid policy.
            let message = unsafe {
                Message::method_call(PATH, "AppendFiles")
                    .unwrap()
                    .destination(SERVICE)
                    .unwrap()
                    .interface(INTERFACE)
                    .unwrap()
                    .build_raw_body(&bytes, "sua(hs)", vec![owned.into()])
                    .unwrap()
            };
            let mut replies = MessageStream::from(&client);
            client.send(&message).await.unwrap();
            let reply = loop {
                let reply = replies.next().await.unwrap().unwrap();
                if reply.header().reply_serial() == Some(message.primary_header().serial_num()) {
                    break reply;
                }
            };
            drop(replies);
            let result = if reply.message_type() == MessageType::Error {
                Err(zbus::Error::from(reply))
            } else {
                Ok(reply)
            };
            rejected(result, "InvalidInput");
            assert_eq!(upload(&client, &id).await.0, 0);
        }
        append(&client, &id, 0, &[(&file, "valid")]).await.unwrap();
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_unknown_wire_routes_when_called_then_standard_errors_are_fixed() {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let client = private.client().await;
        // Act / Assert
        rejected(call(&client, "SendUnknown", &()).await, "UnknownMethod");
        rejected(
            client
                .call_method(
                    Some(SERVICE),
                    "/other",
                    Some(INTERFACE),
                    "BeginSelection",
                    &"request",
                )
                .await,
            "UnknownObject",
        );
        rejected(
            client
                .call_method(
                    Some(SERVICE),
                    PATH,
                    Some("io.other.Interface"),
                    "BeginSelection",
                    &"request",
                )
                .await,
            "UnknownInterface",
        );
        rejected(
            client
                .call_method(
                    Some(SERVICE),
                    PATH,
                    Some("org.freedesktop.DBus.Introspectable"),
                    "Introspect",
                    &"extra",
                )
                .await,
            "InvalidInput",
        );
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[test]
fn given_noncanonical_or_stale_token_when_parsed_then_authority_is_rejected() {
    // Arrange / Act / Assert
    assert_eq!(upload_id(&token(123, 1), 123), Ok(UploadId(123, 1)));
    for invalid in [
        String::new(),
        token(0, 1),
        token(124, 1),
        token(123, 0),
        token(123, 1).to_uppercase(),
        format!("{}x", token(123, 1)),
        "é".repeat(25),
        token(123, 1).replace('-', ":"),
    ] {
        assert_eq!(upload_id(&invalid, 123), Err(Error::InvalidInput));
    }
}

#[tokio::test]
async fn given_existing_owner_when_second_adapter_connects_then_it_fails_before_core_start() {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut first = SelectionBus::connect(private.socket().await).await.unwrap();
        // Act / Assert
        assert!(matches!(
            SelectionBus::connect(private.socket().await).await,
            Err(BusError::Transport)
        ));
        first.close_connection().await;
        first.close_connection().await;
        let mut replacement = SelectionBus::connect(private.socket().await).await.unwrap();
        replacement.close_connection().await;
    })
    .await;
}

#[tokio::test]
async fn given_stopped_runtime_host_when_run_then_no_transports_start_and_cleanup_is_joined() {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        let mut runtime =
            NativeRuntime::new(123, Limits::default(), crate::RQS::default()).unwrap();
        let stop = CancellationToken::new();
        stop.cancel();
        // Act / Assert
        assert_eq!(bus.run(&mut runtime, stop).await, Ok(()));
        assert_eq!(
            runtime.selection_coordinator().lifecycle,
            Lifecycle::Stopped
        );
        assert!(bus.closed.is_cancelled() && bus.pump.is_finished());
    })
    .await;
}

#[tokio::test]
async fn given_core_bind_failure_when_host_runs_then_runtime_pump_and_bus_are_closed() {
    scenario(async {
        // Arrange: port rejection occurs before Bluetooth/mDNS bootstrap.
        let private = PrivateBus::new();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        let occupied = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        let core = crate::RQS::new(
            crate::Visibility::Invisible,
            Some(occupied.local_addr().unwrap().port().into()),
            None,
        );
        let mut runtime = NativeRuntime::new(123, Limits::default(), core).unwrap();
        // Act / Assert
        assert_eq!(
            bus.run(&mut runtime, CancellationToken::new()).await,
            Err(BusError::Runtime(RuntimeError::CoreUnavailable))
        );
        assert_eq!(
            runtime.selection_coordinator().lifecycle,
            Lifecycle::Stopped
        );
        assert!(bus.closed.is_cancelled() && bus.pump.is_finished());
    })
    .await;
}

#[tokio::test]
async fn given_bus_loss_with_preparation_when_observed_then_owned_resources_are_released() {
    scenario(async {
        // Arrange
        let mut private = PrivateBus::new();
        let host = Harness::start(SelectionBus::connect(private.socket().await).await.unwrap());
        let client = private.client().await;
        let id = begin(&client, "request").await;
        let source = tempfile::NamedTempFile::new().unwrap();
        let file = File::open(source.path()).unwrap();
        append(&client, &id, 0, &[(&file, "a")]).await.unwrap();
        upload(&client, &id).await; // FIFO barrier: the append message has been dropped.
        let handles = || {
            std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(Result::ok)
                .filter_map(|entry| std::fs::read_link(entry.path()).ok())
                .filter(|path| path == source.path())
                .count()
        };
        assert_eq!(handles(), 3); // fixture writer, caller reader, admitted service reader.
                                  // Act
        private.child.kill().unwrap();
        private.child.wait().unwrap();
        let mut host = host;
        let (coordinator, outcome) = host.task.take().unwrap().await.unwrap();
        // Assert
        assert_eq!(outcome, Err(BusError::Transport));
        assert_eq!(coordinator.usage(), ResourceUsage::default());
        assert_eq!(handles(), 2); // no source FD remains in the bus, inbox or coordinator.
        assert!(source.path().is_file());
    })
    .await;
}

#[tokio::test]
async fn given_unconsumed_call_flood_when_inbox_fills_then_pump_fails_closed_without_admission() {
    scenario(async {
        // Arrange: intentionally no serial consumer, only the bounded pump.
        let private = PrivateBus::new();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        let client = private.client().await;
        // Act
        for n in 0..INBOX + 2 {
            let message = Message::method_call(PATH, "BeginSelection")
                .unwrap()
                .destination(SERVICE)
                .unwrap()
                .interface(INTERFACE)
                .unwrap()
                .build(&format!("request-{n}"))
                .unwrap();
            client.send(&message).await.unwrap();
        }
        bus.failed.cancelled().await;
        // Assert
        assert!(bus.inbox.len() <= INBOX);
        assert!(bus.clients.is_empty());
        bus.close_connection().await;
        assert!(bus.pump.is_finished());
    })
    .await;
}

#[tokio::test]
async fn given_full_client_or_preparation_budget_when_called_then_capacity_is_explicit_and_abort_releases_it(
) {
    scenario(async {
        // Arrange
        let private = PrivateBus::new();
        let mut host = Harness::with_limits(
            SelectionBus::connect(private.socket().await).await.unwrap(),
            Limits {
                clients: 1,
                ..Limits::default()
            },
        );
        let first = private.client().await;
        let second = private.client().await;
        let mut ids = Vec::new();
        for n in 0..4 {
            ids.push(begin(&first, &format!("request-{n}")).await);
        }
        // Act / Assert
        rejected(
            call(&first, "BeginSelection", &"overflow").await,
            "Capacity",
        );
        rejected(
            call(&second, "BeginSelection", &"request").await,
            "Capacity",
        );
        call(&first, "AbortSelection", &ids[0])
            .await
            .unwrap()
            .body()
            .deserialize::<()>()
            .unwrap();
        call(&first, "AbortSelection", &ids[0]).await.unwrap();
        assert_eq!(
            upload(&first, &ids[0]).await,
            (0, "aborted".into(), String::new())
        );
        assert_eq!(begin(&first, "request-0").await, ids[0]);
        rejected(
            call(&first, "CommitSelection", &ids[0]).await,
            "InvalidState",
        );
        assert_ne!(begin(&first, "overflow").await, ids[0]);
        host.clients.wait_for(|count| *count == 1).await.unwrap();
        first.close().await.unwrap();
        host.clients.wait_for(|count| *count == 0).await.unwrap();
        assert!(!begin(&second, "request").await.is_empty());
        assert_eq!(host.finish().await.1, Ok(()));
    })
    .await;
}

#[tokio::test]
async fn given_unrelated_session_connections_when_they_close_then_idle_selection_host_does_not_overflow(
) {
    scenario(async {
        // Arrange: leave the host inbox deliberately unconsumed during unrelated lifecycle churn.
        let private = PrivateBus::new();
        let mut bus = SelectionBus::connect(private.socket().await).await.unwrap();
        // Act
        for _ in 0..INBOX * 3 {
            private.client().await.close().await.unwrap();
        }
        // Broker round trip after churn is a barrier for preceding bus traffic.
        bus.broker_call("NameHasOwner", &SERVICE).await.unwrap();
        // Assert
        assert!(!bus.failed.is_cancelled());
        assert!(bus.inbox.len() < INBOX);
        bus.close_connection().await;
    })
    .await;
}

#[test]
fn given_internal_errors_when_mapped_then_wire_names_match_the_frozen_allowlist() {
    // Arrange
    let errors = [
        Error::InvalidInput,
        Error::Capacity,
        Error::UnknownClient,
        Error::UnknownUpload,
        Error::UnknownSnapshot,
        Error::InvalidState,
        Error::UnknownSelection,
        Error::UnknownTransfer,
        Error::UnknownEndpoint,
        Error::WrongOwner,
        Error::ReplayConflict,
        Error::RequestExpired,
        Error::StaleConsent,
        Error::StaleControl,
        Error::Unsynchronized,
        Error::NotRunning,
        Error::Busy,
        Error::Exhausted,
    ];
    // Act
    let names = errors.map(error_name).join("\n") + "\n";
    // Assert
    assert_eq!(
        names,
        include_str!("../../../dbus/native-selection-errors.txt")
    );
}
