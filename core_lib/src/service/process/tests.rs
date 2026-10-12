use std::os::unix::fs::{symlink, PermissionsExt};

use tokio::net::UnixListener;

fn private_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

use super::*;

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
