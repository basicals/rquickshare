#![cfg(target_os = "linux")]

use std::process::Stdio;

use rqs_lib::service::bus::SERVICE;
use rqs_lib::service::process::LaunchGuard;

fn private_directory() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    directory
}
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::{TcpListener, UnixListener};
use tokio::process::{Child, Command};

fn daemon(directory: &std::path::Path, settings: &std::path::Path, address: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rquickshare-daemon"));
    command
        .args(["--settings", settings.to_str().unwrap()])
        .env("XDG_RUNTIME_DIR", directory)
        .env("DBUS_SESSION_BUS_ADDRESS", address)
        .env("RUST_LOG", "trace")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}
async fn finished(child: &mut Child) -> std::process::ExitStatus {
    tokio::time::timeout(std::time::Duration::from_secs(10), child.wait())
        .await
        .expect("Owned daemon must finish")
        .unwrap()
}

#[tokio::test]
async fn given_settings_check_when_native_binary_runs_then_recovery_is_reported_without_bus_lock_or_file_changes(
) {
    // Arrange
    let directory = private_directory();
    let path = directory.path().join("settings");
    for bytes in [
        br#"{"visibility":42,"unknown":"private-value"}"#.as_slice(),
        br#"{"visibility":1}"#,
        b"broken-private-value",
    ] {
        std::fs::write(&path, bytes).unwrap();
        let mut command = daemon(directory.path(), &path, "invalid-bus-address");
        command.arg("--check");
        // Act
        let output = command.output().await.unwrap();
        // Assert
        assert_eq!(output.status.success(), bytes[0] == b'{');
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!directory.path().join("rquickshare-owner.lock").exists());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("private-value") && !stderr.contains(path.to_str().unwrap()));
        if bytes.starts_with(br#"{"visibility":42"#) {
            assert!(stderr.contains("starts hidden"));
        }
    }
}

#[tokio::test]
async fn given_existing_desktop_owner_when_daemon_launches_then_it_fails_before_bus_or_transports()
{
    // Arrange
    let directory = private_directory();
    let path = directory.path().join("settings");
    std::fs::write(&path, b"{}").unwrap();
    let _guard = LaunchGuard::acquire(directory.path()).unwrap();
    // Act
    let output = daemon(directory.path(), &path, "invalid-address")
        .output()
        .await
        .unwrap();
    // Assert
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim(),
        "Native startup error: Owner"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"{}");
}

#[tokio::test]
async fn given_waiting_authentication_when_os_shutdown_signal_arrives_then_owned_binary_closes_socket_and_releases_guard(
) {
    for signal in [rustix::process::Signal::INT, rustix::process::Signal::TERM] {
        // Arrange
        let directory = private_directory();
        let settings = directory.path().join("settings");
        std::fs::write(&settings, br#"{"visibility":1}"#).unwrap();
        let socket = directory.path().join("bus");
        let listener = UnixListener::bind(&socket).unwrap();
        let mut child = daemon(
            directory.path(),
            &settings,
            &format!("unix:path={}", socket.display()),
        )
        .spawn()
        .unwrap();
        let (mut peer, _) =
            tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept())
                .await
                .unwrap()
                .unwrap();
        let mut first = [0; 64];
        assert!(peer.read(&mut first).await.unwrap() > 0); // signal handlers precede SASL handshake
        assert!(LaunchGuard::acquire(directory.path()).is_err());
        // Act
        rustix::process::kill_process(
            rustix::process::Pid::from_raw(child.id().unwrap() as i32).unwrap(),
            signal,
        )
        .unwrap();
        let status = finished(&mut child).await;
        // Assert
        assert!(status.success());
        let mut remaining = Vec::new();
        peer.read_to_end(&mut remaining).await.unwrap();
        assert!(remaining.len() < 64 * 1024);
        let _new = LaunchGuard::acquire(directory.path()).unwrap();
        assert_eq!(std::fs::read(&settings).unwrap(), br#"{"visibility":1}"#);
    }
}

#[tokio::test]
async fn given_occupied_core_port_when_real_daemon_starts_then_failed_startup_joins_and_releases_bus_name_and_guard(
) {
    // Arrange
    let directory = private_directory();
    let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let settings = directory.path().join("settings");
    std::fs::write(
        &settings,
        format!(
            "{{\"visibility\":1,\"port\":{}}}",
            listener.local_addr().unwrap().port()
        ),
    )
    .unwrap();
    let socket = directory.path().join("bus");
    let mut broker = Command::new("dbus-daemon")
        .args([
            "--session",
            "--nofork",
            "--print-address=1",
            "--address",
            &format!("unix:path={}", socket.display()),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut address = String::new();
    BufReader::new(broker.stdout.take().unwrap())
        .read_line(&mut address)
        .await
        .unwrap();
    // Act
    let output = daemon(directory.path(), &settings, address.trim())
        .output()
        .await
        .unwrap();
    // Assert
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim(),
        "Native startup error: Bus(Runtime(CoreUnavailable))"
    );
    let probe = zbus::connection::Builder::address(address.trim())
        .unwrap()
        .build()
        .await
        .unwrap();
    let reply = probe
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NameHasOwner",
            &SERVICE,
        )
        .await
        .unwrap();
    assert!(!reply.body().deserialize::<bool>().unwrap());
    let _new = LaunchGuard::acquire(directory.path()).unwrap();
    probe.close().await.unwrap();
    broker.kill().await.unwrap();
    let _ = finished(&mut broker).await;
}
