//! Headless local acquisition and event-driven GUI startup. Paths never cross IPC.
use futures::StreamExt;
use rqs_lib::{
    file_source::{OpenedSelection, SelectionLimits},
    service::{
        client::desktop::{self, DesktopClient, DesktopError},
        process::BusAddress,
    },
};
use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};
use zbus::fdo;

const ARGUMENT_BYTES: usize = 64 * 1024;
const USAGE: &str = "Usage: rquickshare send -- <local-path-or-file-URI>...";

pub fn paths(args: impl IntoIterator<Item = OsString>) -> Result<Vec<PathBuf>, &'static str> {
    let mut args = args.into_iter();
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--")) {
        return Err(USAGE);
    }
    let mut paths = Vec::new();
    let mut bytes = 0usize;
    for arg in args {
        bytes = bytes.checked_add(arg.as_bytes().len()).ok_or(USAGE)?;
        if paths.len() >= 256
            || bytes > ARGUMENT_BYTES
            || arg.is_empty()
            || arg.as_bytes().contains(&0)
        {
            return Err(USAGE);
        }
        let raw = arg.as_bytes();
        let path = if let Some(uri) = raw.strip_prefix(b"file:") {
            if uri.contains(&b'?') || uri.contains(&b'#') {
                return Err("File URI must not contain query or fragment");
            }
            let path = if let Some(authority) = uri.strip_prefix(b"//") {
                let split = authority
                    .iter()
                    .position(|byte| *byte == b'/')
                    .ok_or("File URI must be absolute")?;
                if ![b"".as_slice(), b"localhost".as_slice()].contains(&&authority[..split]) {
                    return Err("Only local file URIs are supported");
                }
                &authority[split..]
            } else {
                uri
            };
            if !path.starts_with(b"/") {
                return Err("File URI must be absolute");
            }
            let mut decoded = Vec::with_capacity(path.len());
            let mut i = 0;
            while i < path.len() {
                if path[i] == b'%' {
                    let high = path
                        .get(i + 1)
                        .copied()
                        .and_then(hex)
                        .ok_or("Invalid file URI escape")?;
                    let low = path
                        .get(i + 2)
                        .copied()
                        .and_then(hex)
                        .ok_or("Invalid file URI escape")?;
                    decoded.push(high * 16 + low);
                    i += 3;
                } else {
                    decoded.push(path[i]);
                    i += 1;
                }
            }
            if decoded.contains(&0) {
                return Err("Invalid file URI");
            }
            PathBuf::from(OsString::from_vec(decoded))
        } else {
            if raw
                .iter()
                .position(|byte| *byte == b':')
                .is_some_and(|colon| {
                    colon > 0
                        && raw[0].is_ascii_alphabetic()
                        && raw[..colon]
                            .iter()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"+-.".contains(byte))
                })
            {
                return Err("Only local paths and file URIs are supported");
            }
            PathBuf::from(arg)
        };
        paths.push(path);
    }
    if paths.is_empty() {
        return Err(USAGE);
    }
    Ok(paths)
}
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub async fn ensure_gui(address: &BusAddress, executable: &Path) -> Result<(), &'static str> {
    let connection = rqs_lib::service::bus::connection_builder(
        address
            .connect()
            .await
            .map_err(|_| "Session bus unavailable")?,
    )
    .build()
    .await
    .map_err(|_| "Session bus unavailable")?;
    let outcome=async {
        let broker=fdo::DBusProxy::new(&connection).await.map_err(|_|"Session bus unavailable")?;
        let mut changes=broker.receive_name_owner_changed_with_args(&[(0,desktop::SERVICE)]).await.map_err(|_|"Cannot observe panel startup")?;
        let name=desktop::SERVICE.try_into().map_err(|_|"Invalid panel service")?;
        match broker.get_name_owner(name).await {
            Ok(_)=>return Ok(()),
            Err(fdo::Error::NameHasNoOwner(_))=>(),
            Err(_)=>return Err("Cannot resolve panel owner"),
        }
        let mut child=tokio::process::Command::new(executable).stdin(std::process::Stdio::null()).spawn().map_err(|_|"Cannot start the native panel")?;
        loop { tokio::select! {
            change=changes.next()=>{
                let args=change.as_ref().and_then(|change|change.args().ok()).ok_or("Panel startup connection closed")?;
                if args.new_owner().is_some() {return Ok(())}
            },
            _=child.wait()=>{
                return if broker.get_name_owner(desktop::SERVICE.try_into().map_err(|_|"Invalid panel service")?).await.is_ok() {Ok(())} else {Err("Native panel failed to start. Check the daemon and Layer Shell support.")};
            }
        }}
    }.await;
    let closed = connection.close().await;
    outcome?;
    closed.map_err(|_| "Cannot close startup connection")
}

pub async fn send(
    paths: Vec<PathBuf>,
    address: &BusAddress,
    executable: &Path,
) -> Result<(), &'static str> {
    let source = tokio::task::spawn_blocking(move || {
        OpenedSelection::acquire(&paths, SelectionLimits::default())
    })
    .await
    .map_err(|_| "File acquisition failed")?
    .map_err(|_| "Cannot read the complete selection. Check local access and file limits.")?;
    if !desktop::valid_selection(&source) {
        return Err("Selection exceeds the native manifest limits");
    }
    ensure_gui(address, executable).await?;
    let mut client = DesktopClient::connect(
        address
            .connect()
            .await
            .map_err(|_| "Session bus unavailable")?,
    )
    .await
    .map_err(message)?;
    let result = client.import(source).await.map_err(message);
    let closed = client.close().await.map_err(message);
    result?;
    closed
}
fn message(error: DesktopError) -> &'static str {
    match error {
    DesktopError::Busy=>"The panel already has files or an active preparation. Finish or discard it in the panel.",
    DesktopError::Recovery=>"Preparation needs recovery in the panel. Do not repeat the send.",
    DesktopError::GuiChanged|DesktopError::Transport|DesktopError::Protocol=>"Panel connection changed. Review the panel before retrying; no automatic send was requested.",
    DesktopError::InvalidInput=>"The complete selection is unsupported or unreadable.",
    DesktopError::Denied=>"Panel access was denied.",
    _=>"The panel could not prepare the files. Review its state before retrying.",
}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_local_arguments_when_parsed_then_bytes_are_preserved_and_remote_or_malformed_input_rejected(
    ) {
        // Arrange / Act / Assert
        for (input, expected) in [
            ("file:///tmp/a%20b", "/tmp/a b"),
            ("file://localhost/tmp/%D1%84", "/tmp/ф"),
            ("file:/tmp/%2520", "/tmp/%20"),
            ("./a:b", "./a:b"),
            ("./space %?#", "./space %?#"),
        ] {
            assert_eq!(
                paths(["--", input].map(Into::into)).unwrap(),
                [PathBuf::from(expected)]
            );
        }
        let raw = OsString::from_vec(vec![b'/', b't', b'/', 0xff]);
        assert_eq!(
            paths([OsString::from("--"), raw.clone()]).unwrap(),
            [PathBuf::from(raw)]
        );
        for input in [
            "",
            "https://host/a",
            "https:a",
            "file://host/tmp/a",
            "file://localhost",
            "file:relative",
            "file:///tmp/%",
            "file:///tmp/%0",
            "file:///tmp/%gg",
            "file:///tmp/%00",
            "file:///tmp/a?q",
            "file:///tmp/a#x",
        ] {
            assert!(paths(["--", input].map(Into::into)).is_err(), "{input}");
        }
        assert!(paths([OsString::from("--"), OsString::from_vec(vec![b'a', 0])]).is_err());
        assert!(paths([]).is_err());
        assert!(paths(["--".into()]).is_err());
        assert!(paths(["file:///tmp/a".into()]).is_err());
        for count in [255, 256, 257] {
            assert_eq!(
                paths(
                    std::iter::once(OsString::from("--"))
                        .chain((0..count).map(|_| OsString::from("/a")))
                )
                .is_ok(),
                count <= 256
            );
        }
        for length in [ARGUMENT_BYTES - 1, ARGUMENT_BYTES, ARGUMENT_BYTES + 1] {
            assert_eq!(
                paths(["--".into(), OsString::from("a".repeat(length))]).is_ok(),
                length <= ARGUMENT_BYTES
            );
        }
    }
    struct Broker(std::process::Child);
    impl Drop for Broker {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    #[tokio::test]
    async fn given_missing_gui_when_startup_child_exits_then_failure_is_event_driven_and_broker_remains_usable(
    ) {
        // Arrange: isolated broker with no activation directories, condition-based readiness and owned teardown.
        let temp = tempfile::TempDir::new().unwrap();
        let socket = temp.path().join("bus");
        let mut bus = Broker(
            std::process::Command::new("dbus-daemon")
                .arg("--config-file")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../docs/prototypes/preview-bus.conf"
                ))
                .args(["--nofork", "--nopidfile", "--print-address=1", "--address"])
                .arg(format!("unix:path={}", socket.display()))
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        use std::io::BufRead;
        let mut address = String::new();
        std::io::BufReader::new(bus.0.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        let address = BusAddress::parse(address.trim()).unwrap();
        // Act / Assert: child exit, not a sleep or retry loop.
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            ensure_gui(&address, Path::new("/bin/false")),
        )
        .await
        .unwrap();
        assert_eq!(
            result,
            Err("Native panel failed to start. Check the daemon and Layer Shell support.")
        );
        let owner = rqs_lib::service::bus::connection_builder(address.connect().await.unwrap())
            .build()
            .await
            .unwrap();
        owner.request_name(desktop::SERVICE).await.unwrap();
        assert_eq!(
            ensure_gui(&address, Path::new("/nonexistent/unlaunched-panel")).await,
            Ok(())
        );
        owner.close().await.unwrap();
        drop(bus);
    }
}
