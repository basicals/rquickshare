//! Headless local acquisition and fixed GUI activation. Paths never cross IPC.
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
    path::PathBuf,
};

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

pub async fn ensure_gui(address: &BusAddress) -> Result<(), &'static str> {
    rqs_lib::service::activation::activate_desktop(
        address
            .connect()
            .await
            .map_err(|_| "Session bus unavailable")?,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .map_err(|_| "Native panel could not start. Check installation and desktop session support.")
}
pub async fn open(address: &BusAddress) -> Result<(), &'static str> {
    ensure_gui(address).await?;
    let mut client = DesktopClient::connect(
        address
            .connect()
            .await
            .map_err(|_| "Session bus unavailable")?,
    )
    .await
    .map_err(message)?;
    let result = client.show().await.map_err(message);
    let closed = client.close().await.map_err(message);
    result?;
    closed
}

#[derive(Debug)]
pub struct Failure {
    pub code: desktop::ErrorCode,
    pub message: &'static str,
}
impl Failure {
    pub fn new(code: desktop::ErrorCode, message: &'static str) -> Self {
        Self { code, message }
    }
}
fn desktop_failure(error: DesktopError) -> Failure {
    use desktop::ErrorCode;
    let code = match error {
        DesktopError::Busy => ErrorCode::Busy,
        DesktopError::Recovery => ErrorCode::Recovery,
        DesktopError::Unavailable => ErrorCode::BackendUnavailable,
        DesktopError::InvalidInput => ErrorCode::InvalidSelection,
        _ => ErrorCode::PanelUnavailable,
    };
    Failure::new(code, message(error))
}
pub async fn show_error(
    code: desktop::ErrorCode,
    address: &BusAddress,
) -> Result<(), &'static str> {
    ensure_gui(address).await?;
    let mut client = DesktopClient::connect(
        address
            .connect()
            .await
            .map_err(|_| "Session bus unavailable")?,
    )
    .await
    .map_err(message)?;
    let result = client.show_error(code).await.map_err(message);
    let closed = client.close().await.map_err(message);
    result?;
    closed
}

pub async fn send(paths: Vec<PathBuf>, address: &BusAddress) -> Result<(), Failure> {
    let source = tokio::task::spawn_blocking(move || {
        OpenedSelection::acquire(&paths, SelectionLimits::default())
    })
    .await
    .map_err(|_| {
        Failure::new(
            desktop::ErrorCode::InvalidSelection,
            "File acquisition failed",
        )
    })?
    .map_err(|_| {
        Failure::new(
            desktop::ErrorCode::InvalidSelection,
            "Cannot read the complete selection. Check local access and file limits.",
        )
    })?;
    if !desktop::valid_selection(&source) {
        return Err(Failure::new(
            desktop::ErrorCode::InvalidSelection,
            "Selection exceeds the native manifest limits",
        ));
    }
    ensure_gui(address)
        .await
        .map_err(|message| Failure::new(desktop::ErrorCode::PanelUnavailable, message))?;
    let mut client = DesktopClient::connect(address.connect().await.map_err(|_| {
        Failure::new(
            desktop::ErrorCode::PanelUnavailable,
            "Session bus unavailable",
        )
    })?)
    .await
    .map_err(desktop_failure)?;
    let result = client.import(source).await.map_err(desktop_failure);
    let closed = client.close().await.map_err(desktop_failure);
    result?;
    closed
}
fn message(error: DesktopError) -> &'static str {
    match error {
    DesktopError::Unavailable=>"Native daemon unavailable. Check installation, then reconnect the panel.",
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
    #[tokio::test]
    async fn given_missing_or_current_gui_when_activated_then_no_child_is_spawned_and_manual_open_reuses_the_owner(
    ) {
        let broker = crate::test_bus::Broker::new();
        assert_eq!(
            ensure_gui(&broker.address).await,
            Err("Native panel could not start. Check installation and desktop session support.")
        );
        let server = desktop::DesktopServer::connect_optional(
            broker.address.connect().await.unwrap(),
            None,
            desktop::PreparationGate::new(),
        )
        .await
        .unwrap();
        let updates = server.handle().subscribe();
        ensure_gui(&broker.address).await.unwrap();
        assert_eq!(updates.borrow().show_revision, 0);
        open(&broker.address).await.unwrap();
        assert_eq!(updates.borrow().show_revision, 1);
        server.close().await.unwrap();
    }
    #[tokio::test]
    async fn given_unavailable_backend_or_unreadable_source_when_cli_fails_then_fixed_error_is_visible_without_retrying_import(
    ) {
        // Arrange
        let private = crate::test_bus::Broker::new();
        let gate = desktop::PreparationGate::new();
        let server = desktop::DesktopServer::connect_optional(
            private.address.connect().await.unwrap(),
            None,
            gate.clone(),
        )
        .await
        .unwrap();
        let handle = server.handle();
        let mut updates = handle.subscribe();
        let source = tempfile::NamedTempFile::new().unwrap();
        source.as_file().set_len(3).unwrap();
        // Act / Assert
        let failure = send(vec![source.path().to_owned()], &private.address)
            .await
            .unwrap_err();
        assert_eq!(failure.code, desktop::ErrorCode::BackendUnavailable);
        assert!(!failure.message.contains(source.path().to_str().unwrap()));
        show_error(failure.code, &private.address).await.unwrap();
        updates
            .wait_for(|state| state.error_revision == 1)
            .await
            .unwrap();
        assert_eq!(
            updates.borrow().error,
            Some(desktop::ErrorCode::BackendUnavailable)
        );
        assert!(!updates.borrow().staging && !*gate.subscribe().borrow());
        assert!(source.path().exists());
        let missing = source.path().join("missing-source");
        let failure = send(vec![missing], &private.address).await.unwrap_err();
        assert_eq!(failure.code, desktop::ErrorCode::InvalidSelection);
        assert_eq!(updates.borrow().error_revision, 1);
        for (error, expected) in [
            (DesktopError::Busy, desktop::ErrorCode::Busy),
            (DesktopError::Recovery, desktop::ErrorCode::Recovery),
            (
                DesktopError::InvalidInput,
                desktop::ErrorCode::InvalidSelection,
            ),
            (
                DesktopError::GuiChanged,
                desktop::ErrorCode::PanelUnavailable,
            ),
        ] {
            assert_eq!(desktop_failure(error).code, expected);
        }
        server.close().await.unwrap();
    }
}
