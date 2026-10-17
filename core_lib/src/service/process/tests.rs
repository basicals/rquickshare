use std::os::unix::fs::{symlink, PermissionsExt};

use tokio::net::UnixListener;

fn private_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

use super::*;

#[test]
fn given_native_settings_check_when_missing_corrupt_or_directory_identity_changes_then_check_is_read_only_and_never_defaults_corruption(
) {
    use crate::service::native_settings::{CardMode, DirectoryIntent, Edit, Store, StoreError};
    // Arrange
    let root = private_directory();
    let directory = root.path().join("native");
    let legacy = root.path().join("legacy");
    let store = Store::new(directory.clone(), legacy);
    // Act / Assert
    check_native_settings(&store).unwrap();
    assert!(!directory.exists());
    let selected = root.path().join("selected");
    std::fs::create_dir(&selected).unwrap();
    let initial = store.load().unwrap();
    store
        .save(Edit {
            base: initial.base,
            request: "00000000000000000000000000000001".into(),
            visibility: 1,
            port: Some(0),
            card_mode: CardMode::Compact,
            device_name: None,
            directory: DirectoryIntent::Replace(File::open(&selected).unwrap()),
            recover: false,
        })
        .unwrap();
    check_native_settings(&store).unwrap();
    let bytes = std::fs::read(directory.join("native-settings.json")).unwrap();
    std::fs::rename(&selected, root.path().join("moved")).unwrap();
    std::fs::create_dir(&selected).unwrap();
    assert_eq!(
        check_native_settings(&store),
        Err(ProcessError::NativeSettings(StoreError::Directory))
    );
    assert_eq!(
        std::fs::read(directory.join("native-settings.json")).unwrap(),
        bytes
    );
    std::fs::write(directory.join("native-settings.json"), b"{").unwrap();
    assert_eq!(
        check_native_settings(&store),
        Err(ProcessError::NativeSettings(StoreError::RecoveryRequired))
    );
    assert_eq!(
        std::fs::read(directory.join("native-settings.json")).unwrap(),
        b"{"
    );
}

#[tokio::test]
async fn given_corrupt_native_config_when_repaired_and_explicitly_restarted_then_fresh_owner_serves_truthful_recovery_without_radio_start(
) {
    use futures::StreamExt;

    use crate::service::bus::{tests::PrivateBus, SERVICE};
    use crate::service::client::{
        settings::{SaveIntent, SettingsEdit},
        ClientError, NativeClient,
    };
    use crate::service::native_settings::Store;
    use crate::service::{Error, Lifecycle};
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // Arrange: reserve the configured TCP port to fail before any radio tasks.
        let occupied = std::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        let private = PrivateBus::new();
        let root = private_directory();
        let directory = root.path().join("native");
        let legacy = root.path().join("legacy");
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(directory.join("native-settings.json"), b"{").unwrap();
        std::fs::set_permissions(
            directory.join("native-settings.json"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let observer = private.client().await;
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender("org.freedesktop.DBus")
            .unwrap()
            .path("/org/freedesktop/DBus")
            .unwrap()
            .interface("org.freedesktop.DBus")
            .unwrap()
            .member("NameOwnerChanged")
            .unwrap()
            .arg(0, SERVICE)
            .unwrap()
            .build();
        let mut owners = zbus::MessageStream::for_match_rule(rule, &observer, Some(8))
            .await
            .unwrap();
        let stop = CancellationToken::new();
        let end = stop.clone();
        let address = BusAddress::parse(&private.address_for_test()).unwrap();
        let task = tokio::spawn(serve_native_settings(
            directory.clone(),
            legacy.clone(),
            address,
            end,
        ));
        let first = owners.next().await.unwrap().unwrap();
        let (_, _, old): (String, String, String) = first.body().deserialize().unwrap();
        assert!(!old.is_empty());
        let mut client = NativeClient::connect(private.socket().await).await.unwrap();
        let before = client.read_settings().await.unwrap();
        assert!(!before.configured);
        assert_eq!(before.error, 2);
        assert!(!before.running);
        assert_eq!(
            client.read_state().await.unwrap().lifecycle,
            Lifecycle::Stopped
        );
        assert!(matches!(
            client.begin_selection().await,
            Err(ClientError::Rejected(Error::NotRunning))
        ));
        // Act: explicit recovery does not implicitly start transports.
        let intent = SaveIntent {
            epoch: before.epoch.clone(),
            revision: before.revision,
            token: before.token.clone(),
            request: "00000000000000000000000000000001".into(),
            visibility: 1,
            port: occupied.local_addr().unwrap().port().into(),
            card_mode: 0,
            device_name: String::new(),
            directory: 1,
            recover: true,
        };
        let saved = client
            .save_settings(SettingsEdit {
                intent,
                directory: None,
            })
            .await
            .unwrap();
        assert!(saved.configured);
        assert!(!saved.running);
        assert!(saved.restart_required);
        assert_eq!(saved.applied_live_revision, 0);
        client.restart_service().await.unwrap();
        let new = loop {
            let message = owners.next().await.unwrap().unwrap();
            let (_, _, new): (String, String, String) = message.body().deserialize().unwrap();
            if !new.is_empty() && new != old {
                break new;
            }
        };
        // Assert: the new generation reports bind failure, with settings still editable.
        assert_ne!(new, old);
        let mut next = NativeClient::connect(private.socket().await).await.unwrap();
        let after = next.read_settings().await.unwrap();
        assert_ne!(after.epoch, before.epoch);
        assert_eq!(after.revision, 1);
        assert!(after.configured);
        assert!(!after.running);
        assert_eq!(after.error, 4);
        assert_eq!(
            next.read_state().await.unwrap().lifecycle,
            Lifecycle::Stopped
        );
        assert!(matches!(
            client.read_settings().await,
            Err(ClientError::DaemonChanged)
        ));
        next.close().await.unwrap();
        client.close().await.unwrap();
        stop.cancel();
        task.await.unwrap().unwrap();
        observer.close().await.unwrap();
        assert_eq!(
            Store::new(directory, legacy).load().unwrap().base.revision,
            1
        );
    })
    .await
    .expect("recovery owner must join");
}

#[test]
fn given_daemon_arguments_when_parsed_then_only_explicit_check_help_and_settings_are_accepted() {
    // Arrange / Act / Assert
    let args =
        Arguments::parse(["--check", "--settings", "/tmp/settings"].map(OsString::from)).unwrap();
    assert!(args.check && !args.help);
    assert_eq!(args.settings, Some("/tmp/settings".into()));
    assert!(Arguments::parse([OsString::from("--help")]).unwrap().help);
    for args in [
        vec!["--settings"],
        vec!["--settings", ""],
        vec!["--settings", "--check"],
        vec!["--check", "--check"],
        vec!["--unknown"],
        vec!["--settings", "a", "--settings", "b"],
    ] {
        assert!(matches!(
            Arguments::parse(args.into_iter().map(OsString::from)),
            Err(ProcessError::Arguments)
        ));
    }
}

#[test]
fn given_local_bus_address_when_parsed_then_percent_bytes_and_guid_are_preserved_without_transport_fallback(
) {
    // Arrange / Act / Assert
    assert!(
        matches!(BusAddress::parse("unix:path=/tmp/a%20b%2C%3B%25,guid=0123456789abcdef0123456789abcdef").unwrap(), BusAddress::File(path) if path == Path::new("/tmp/a b,;%"))
    );
    assert!(
        matches!(BusAddress::parse("unix:abstract=session%20name").unwrap(), BusAddress::Abstract(name) if name == b"session name")
    );
    for invalid in ["", "unix:", "tcp:host=127.0.0.1,port=1234", "unixexec:path=/bin/sh", "unix:dir=/tmp", "unix:path=relative", "unix:path=/tmp/a;unix:path=/tmp/b", "unix:path=/tmp/a,path=/tmp/b", "unix:path=/tmp/a,abstract=b", "unix:path=/tmp/a,foo=bar", "unix:abstract=", "unix:path=/tmp/%", "unix:path=/tmp/%gg", "unix:path=/tmp/%00", "unix:path=/tmp/a,guid=x", "unix:path=/tmp/a,guid=0123456789abcdef0123456789abcdef,guid=0123456789abcdef0123456789abcdef"] {
        assert!(matches!(BusAddress::parse(invalid), Err(ProcessError::Address)));
    }
    assert!(BusAddress::parse(&format!("unix:abstract={}", "x".repeat(107))).is_ok());
    assert!(matches!(
        BusAddress::parse(&format!("unix:abstract={}", "x".repeat(108))),
        Err(ProcessError::Address)
    ));
    assert!(matches!(
        BusAddress::parse(&"x".repeat(4097)),
        Err(ProcessError::Address)
    ));
}

#[test]
fn given_shared_owner_guard_when_another_launcher_contends_then_only_release_returns_the_slot() {
    // Arrange
    let directory = private_directory();
    let guard = LaunchGuard::acquire(directory.path()).unwrap();
    let path = directory.path().join("rquickshare-owner.lock");
    let inode = path.metadata().unwrap().ino();
    // Act / Assert
    assert!(matches!(
        LaunchGuard::acquire(directory.path()),
        Err(ProcessError::Owner)
    ));
    assert_eq!(
        rustix::io::fcntl_getfd(&guard._file).unwrap(),
        rustix::io::FdFlags::CLOEXEC
    );
    drop(guard);
    let _next = LaunchGuard::acquire(directory.path()).unwrap();
    assert_eq!(path.metadata().unwrap().ino(), inode);
}

#[test]
fn given_unsafe_runtime_or_lock_object_when_acquired_then_no_alternative_owner_is_created() {
    // Arrange / Act / Assert
    for kind in [
        "symlink",
        "directory",
        "public-lock",
        "public-runtime",
        "hardlink",
        "fifo",
    ] {
        let directory = private_directory();
        let path = directory.path().join("rquickshare-owner.lock");
        let other = directory.path().join("other");
        match kind {
            "symlink" => {
                std::fs::write(&other, b"keep").unwrap();
                symlink(&other, &path).unwrap();
            }
            "directory" => std::fs::create_dir(&path).unwrap(),
            "public-lock" => {
                std::fs::write(&path, b"keep").unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            }
            "public-runtime" => {
                std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755))
                    .unwrap()
            }
            "hardlink" => {
                std::fs::write(&other, b"keep").unwrap();
                std::fs::set_permissions(&other, std::fs::Permissions::from_mode(0o600)).unwrap();
                std::fs::hard_link(&other, &path).unwrap();
            }
            _ => rustix::fs::mkfifoat(
                rustix::fs::CWD,
                &path,
                rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
            )
            .unwrap(),
        }
        assert!(matches!(
            LaunchGuard::acquire(directory.path()),
            Err(ProcessError::Owner)
        ));
        if other.is_file() {
            assert_eq!(std::fs::read(&other).unwrap(), b"keep");
        }
    }
    let directory = private_directory();
    let alias = directory.path().join("alias");
    symlink(directory.path(), &alias).unwrap();
    assert!(matches!(
        LaunchGuard::acquire(&alias),
        Err(ProcessError::Owner)
    ));
    assert!(matches!(
        LaunchGuard::acquire(Path::new("relative")),
        Err(ProcessError::Owner)
    ));
}

#[tokio::test]
async fn given_owned_unix_listener_when_connected_then_the_adapter_uses_real_nonblocking_cloexec_socket(
) {
    // Arrange
    let directory = private_directory();
    let path = directory.path().join("bus");
    let listener = UnixListener::bind(&path).unwrap();
    // Act
    let stream = BusAddress::File(path).connect().await.unwrap();
    let (peer, _) = listener.accept().await.unwrap();
    // Assert
    assert_eq!(
        rustix::io::fcntl_getfd(&stream).unwrap(),
        rustix::io::FdFlags::CLOEXEC
    );
    assert!(rustix::fs::fcntl_getfl(&stream)
        .unwrap()
        .contains(rustix::fs::OFlags::NONBLOCK));
    drop(peer);
    drop(stream);
    assert!(matches!(
        BusAddress::File(directory.path().join("missing"))
            .connect()
            .await,
        Err(ProcessError::Socket)
    ));
}

#[test]
fn given_os_randomness_when_epochs_are_created_then_no_zero_or_reused_identity_is_observed() {
    // Arrange / Act
    let first = fresh_epoch().unwrap();
    let second = fresh_epoch().unwrap();
    // Assert (observation, not a mathematical uniqueness guarantee)
    assert_ne!(first, 0);
    assert_ne!(second, 0);
    assert_ne!(first, second);
}

#[test]
fn given_private_runtime_when_daemon_and_desktop_guards_overlap_then_each_role_has_one_independent_retained_owner(
) {
    let directory = private_directory();
    let daemon = LaunchGuard::acquire(directory.path()).unwrap();
    let desktop = LaunchGuard::acquire_desktop(directory.path()).unwrap();
    assert!(matches!(
        LaunchGuard::acquire(directory.path()),
        Err(ProcessError::Owner)
    ));
    assert!(matches!(
        LaunchGuard::acquire_desktop(directory.path()),
        Err(ProcessError::Owner)
    ));
    drop(desktop);
    let desktop = LaunchGuard::acquire_desktop(directory.path()).unwrap();
    assert!(matches!(
        LaunchGuard::acquire(directory.path()),
        Err(ProcessError::Owner)
    ));
    drop(daemon);
    let daemon = LaunchGuard::acquire(directory.path()).unwrap();
    drop((daemon, desktop));
}
