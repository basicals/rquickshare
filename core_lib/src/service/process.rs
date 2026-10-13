//! Linux process adapters: local bus sockets and retained single-instance ownership.

use std::ffi::OsString;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use rand::TryRngCore;
use tokio::net::UnixStream;
use tokio_util::sync::CancellationToken;

use super::bus::{BusError, SelectionBus};
use super::runtime::NativeRuntime;
use super::Limits;
use crate::settings::{SettingsError, StartupSettings, LEGACY_IDENTIFIER};
use crate::RQS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessError {
    Arguments,
    Paths,
    Address,
    Owner,
    Socket,
    Random,
    Signal,
    Settings(SettingsError),
    NativeSettings(super::native_settings::StoreError),
    Bus(BusError),
}
impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Native startup error: {self:?}")
    }
}
impl std::error::Error for ProcessError {}

#[derive(Default, PartialEq, Eq)]
pub struct Arguments {
    pub help: bool,
    pub check: bool,
    pub settings: Option<PathBuf>,
}
impl Arguments {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, ProcessError> {
        let mut args = args.into_iter();
        let mut result = Self::default();
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--help") if !result.help => result.help = true,
                Some("--check") if !result.check => result.check = true,
                Some("--settings") if result.settings.is_none() => {
                    let path = args
                        .next()
                        .filter(|path| !path.is_empty())
                        .ok_or(ProcessError::Arguments)?;
                    if path.to_string_lossy().starts_with("--") {
                        return Err(ProcessError::Arguments);
                    }
                    result.settings = Some(path.into());
                }
                _ => return Err(ProcessError::Arguments),
            }
        }
        Ok(result)
    }
}

pub fn settings_path() -> Result<PathBuf, ProcessError> {
    let dirs = directories::BaseDirs::new().ok_or(ProcessError::Paths)?;
    Ok(dirs
        .data_dir()
        .join(LEGACY_IDENTIFIER)
        .join(".settings.json"))
}
pub fn native_settings_paths() -> Result<(PathBuf, PathBuf), ProcessError> {
    let dirs = directories::BaseDirs::new().ok_or(ProcessError::Paths)?;
    Ok((
        dirs.config_dir()
            .join("io.github.morozhenko_apps.RQuickShare"),
        settings_path()?,
    ))
}

pub fn check_native_settings(store: &super::native_settings::Store) -> Result<(), ProcessError> {
    let snapshot = store.load().map_err(ProcessError::NativeSettings)?;
    let values = snapshot.values.ok_or(ProcessError::NativeSettings(
        super::native_settings::StoreError::RecoveryRequired,
    ))?;
    if let Some(directory) = values.directory {
        directory.open().map_err(ProcessError::NativeSettings)?;
    }
    Ok(())
}

/// The caller retains LaunchGuard across every joined generation.
pub async fn serve_native_settings(
    directory: PathBuf,
    legacy: PathBuf,
    address: BusAddress,
    stop: CancellationToken,
) -> Result<(), ProcessError> {
    use super::native_settings::{Store, StoreError};
    loop {
        if stop.is_cancelled() {
            return Ok(());
        }
        let epoch = fresh_epoch()?;
        let stream = tokio::select! { biased; _ = stop.cancelled() => return Ok(()), result = address.connect() => result? };
        let Some(mut bus) = SelectionBus::connect_stoppable(stream, stop.clone())
            .await
            .map_err(ProcessError::Bus)?
        else {
            return Ok(());
        };
        let store = Store::new(directory.clone(), legacy.clone());
        // Join this owned operation even when stop arrives during disk IO.
        let loaded = tokio::task::spawn_blocking(move || {
            let snapshot = store.load();
            let prepared = snapshot
                .as_ref()
                .map_err(|error| *error)
                .and_then(|snapshot| {
                    let values = snapshot
                        .values
                        .as_ref()
                        .ok_or(StoreError::RecoveryRequired)?;
                    let settings = values.startup()?;
                    let root = values
                        .directory
                        .as_ref()
                        .map(|record| record.open().and_then(crate::receive::Directory::adopt))
                        .transpose()?;
                    Ok((settings, root))
                });
            (store, snapshot, prepared)
        })
        .await;
        let (store, initial, prepared) = match loaded {
            Ok(loaded) => loaded,
            Err(_) => {
                bus.close_unstarted().await;
                return Err(ProcessError::NativeSettings(StoreError::Read));
            }
        };
        if stop.is_cancelled() {
            bus.close_unstarted().await;
            return Ok(());
        }
        let recovered = initial
            .as_ref()
            .is_ok_and(|snapshot| snapshot.visibility_recovered);
        let port = prepared
            .as_ref()
            .map(|(settings, _)| settings.port.map_or(-1, |port| port as i32))
            .unwrap_or(-1);
        let runtime = match prepared {
            Ok((settings, root)) => {
                let core = RQS::new(settings.visibility, settings.port, settings.download_path);
                core.receive_policy.set_directory(root);
                core.set_foreground(false);
                NativeRuntime::new(epoch, Limits::default(), core)
                    .map_err(|rejected| ProcessError::Bus(BusError::Runtime(rejected.error)))
            }
            Err(_) => NativeRuntime::configuration_only(epoch, Limits::default())
                .map_err(|error| ProcessError::Bus(BusError::Runtime(error))),
        };
        let mut runtime = match runtime {
            Ok(runtime) => runtime,
            Err(error) => {
                bus.close_unstarted().await;
                return Err(error);
            }
        };
        if !runtime.is_configuration_only() {
            let started = tokio::select! { biased; _ = stop.cancelled() => None, result = runtime.start() => Some(result) };
            match started {
                Some(Ok(())) => (),
                failed => {
                    let _ = runtime.shutdown().await;
                    if failed.is_none() {
                        bus.close_unstarted().await;
                        return Ok(());
                    }
                    runtime = match NativeRuntime::configuration_only(epoch, Limits::default()) {
                        Ok(runtime) => runtime,
                        Err(error) => {
                            bus.close_unstarted().await;
                            return Err(ProcessError::Bus(BusError::Runtime(error)));
                        }
                    };
                }
            }
        }
        let running = !runtime.is_configuration_only();
        bus.set_visibility_recovered(recovered);
        bus.set_settings(store, initial, epoch, running, port)
            .map_err(ProcessError::Bus)?;
        if !running {
            bus.mark_settings_unapplied();
        }
        if running {
            let wakeup = tokio::select! { biased; _ = stop.cancelled() => None, result = async {
                let stream = address.connect().await?;
                super::wakeup::Wakeup::connect(stream).await.map_err(|_| ProcessError::Bus(BusError::Transport))
            } => Some(result) };
            match wakeup {
                Some(Ok(wakeup)) => bus.set_wakeup(wakeup),
                failed => {
                    let _ = runtime.shutdown().await;
                    bus.close_unstarted().await;
                    return match failed {
                        Some(Err(error)) => Err(error),
                        _ => Ok(()),
                    };
                }
            }
        }
        let outcome = bus
            .run_prepared(&mut runtime, stop.clone())
            .await
            .map_err(ProcessError::Bus);
        let restart = bus.restart_requested();
        outcome?;
        if !restart || stop.is_cancelled() {
            return Ok(());
        }
    }
}

pub fn runtime_directory() -> Result<PathBuf, ProcessError> {
    let directory = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(format!("/run/user/{}", rustix::process::geteuid().as_raw()))
        });
    if !directory.is_absolute() {
        return Err(ProcessError::Paths);
    }
    Ok(directory)
}

/// Keep this guard through joined shutdown. Never unlink the shared lock inode.
pub struct LaunchGuard {
    _file: File,
}
impl LaunchGuard {
    pub fn acquire(directory: &Path) -> Result<Self, ProcessError> {
        let uid = rustix::process::geteuid().as_raw();
        let metadata = directory
            .symlink_metadata()
            .map_err(|_| ProcessError::Owner)?;
        if !directory.is_absolute()
            || !metadata.is_dir()
            || metadata.uid() != uid
            || metadata.mode() & 0o077 != 0
        {
            return Err(ProcessError::Owner);
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(
                (rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::NONBLOCK
                    | rustix::fs::OFlags::CLOEXEC)
                    .bits() as i32,
            )
            .open(directory.join("rquickshare-owner.lock"))
            .map_err(|_| ProcessError::Owner)?;
        let metadata = file.metadata().map_err(|_| ProcessError::Owner)?;
        if !metadata.is_file()
            || metadata.uid() != uid
            || metadata.mode() & 0o077 != 0
            || metadata.nlink() != 1
        {
            return Err(ProcessError::Owner);
        }
        rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive)
            .map_err(|_| ProcessError::Owner)?;
        Ok(Self { _file: file })
    }
}

#[derive(PartialEq, Eq)]
pub enum BusAddress {
    File(PathBuf),
    Abstract(Vec<u8>),
}
impl BusAddress {
    pub fn session(directory: &Path) -> Result<Self, ProcessError> {
        match std::env::var_os("DBUS_SESSION_BUS_ADDRESS") {
            Some(value) => Self::parse(value.to_str().ok_or(ProcessError::Address)?),
            None => {
                if !directory.is_absolute() {
                    return Err(ProcessError::Address);
                }
                let path = directory.join("bus");
                use std::os::unix::ffi::OsStrExt;
                validate_socket_name(path.as_os_str().as_bytes())?;
                Ok(Self::File(path))
            }
        }
    }
    pub fn parse(value: &str) -> Result<Self, ProcessError> {
        if value.len() > 4096 || value.contains(';') {
            return Err(ProcessError::Address);
        }
        let value = value.strip_prefix("unix:").ok_or(ProcessError::Address)?;
        let mut socket = None;
        let mut guid = false;
        for entry in value.split(',') {
            let (key, value) = entry.split_once('=').ok_or(ProcessError::Address)?;
            match key {
                "path" | "abstract" if socket.is_none() => {
                    let decoded = percent_decode(value)?;
                    validate_socket_name(&decoded)?;
                    socket = Some(if key == "path" {
                        if decoded.first() != Some(&b'/') {
                            return Err(ProcessError::Address);
                        }
                        Self::File(PathBuf::from(OsString::from_vec(decoded)))
                    } else {
                        Self::Abstract(decoded)
                    });
                }
                "guid" if !guid => {
                    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                        return Err(ProcessError::Address);
                    }
                    guid = true;
                }
                _ => return Err(ProcessError::Address),
            }
        }
        socket.ok_or(ProcessError::Address)
    }
    pub async fn connect(&self) -> Result<UnixStream, ProcessError> {
        let address = match self {
            Self::File(path) => rustix::net::SocketAddrUnix::new(path),
            Self::Abstract(name) => rustix::net::SocketAddrUnix::new_abstract_name(name),
        }
        .map_err(|_| ProcessError::Address)?;
        let socket = rustix::net::socket_with(
            rustix::net::AddressFamily::UNIX,
            rustix::net::SocketType::STREAM,
            rustix::net::SocketFlags::NONBLOCK | rustix::net::SocketFlags::CLOEXEC,
            None,
        )
        .map_err(|_| ProcessError::Socket)?;
        match rustix::net::connect(&socket, &address) {
            Ok(()) => (),
            Err(rustix::io::Errno::INPROGRESS) => (),
            Err(_) => return Err(ProcessError::Socket),
        }
        let stream = UnixStream::from_std(std::os::unix::net::UnixStream::from(socket))
            .map_err(|_| ProcessError::Socket)?;
        stream.writable().await.map_err(|_| ProcessError::Socket)?;
        if stream
            .take_error()
            .map_err(|_| ProcessError::Socket)?
            .is_some()
        {
            return Err(ProcessError::Socket);
        }
        Ok(stream)
    }
}

fn validate_socket_name(value: &[u8]) -> Result<(), ProcessError> {
    if value.is_empty() || value.len() > 107 || value.contains(&0) {
        return Err(ProcessError::Address);
    }
    Ok(())
}
fn percent_decode(value: &str) -> Result<Vec<u8>, ProcessError> {
    let mut result = Vec::new();
    let mut bytes = value.bytes();
    while let Some(byte) = bytes.next() {
        result.push(if byte == b'%' {
            let high = bytes
                .next()
                .and_then(|byte| char::from(byte).to_digit(16))
                .ok_or(ProcessError::Address)?;
            let low = bytes
                .next()
                .and_then(|byte| char::from(byte).to_digit(16))
                .ok_or(ProcessError::Address)?;
            (high * 16 + low) as u8
        } else {
            byte
        });
    }
    Ok(result)
}

pub fn fresh_epoch() -> Result<u128, ProcessError> {
    let mut bytes = [0; 16];
    rand::rngs::OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| ProcessError::Random)?;
    let epoch = u128::from_le_bytes(bytes);
    if epoch == 0 {
        return Err(ProcessError::Random);
    }
    Ok(epoch)
}

pub async fn serve(
    settings: StartupSettings,
    address: BusAddress,
    stop: CancellationToken,
) -> Result<(), ProcessError> {
    let epoch = fresh_epoch()?;
    let stream = tokio::select! {
        biased;
        _ = stop.cancelled() => return Ok(()),
        result = address.connect() => result?,
    };
    let Some(mut bus) = SelectionBus::connect_stoppable(stream, stop.clone())
        .await
        .map_err(ProcessError::Bus)?
    else {
        return Ok(());
    };
    bus.set_visibility_recovered(settings.visibility_recovered);
    if stop.is_cancelled() {
        bus.close_unstarted().await;
        return Ok(());
    }
    let wakeup = tokio::select! {
        biased;
        _ = stop.cancelled() => None,
        result = async {
            let stream = address.connect().await?;
            super::wakeup::Wakeup::connect(stream).await.map_err(|_| ProcessError::Bus(BusError::Transport))
        } => Some(result),
    };
    match wakeup {
        Some(Ok(wakeup)) => bus.set_wakeup(wakeup),
        Some(Err(error)) => {
            bus.close_unstarted().await;
            return Err(error);
        }
        None => {
            bus.close_unstarted().await;
            return Ok(());
        }
    }
    let core = RQS::new(settings.visibility, settings.port, settings.download_path);
    core.set_foreground(false);
    let mut runtime = match NativeRuntime::new(epoch, Limits::default(), core) {
        Ok(runtime) => runtime,
        Err(_) => {
            bus.close_unstarted().await;
            return Err(ProcessError::Bus(BusError::Transport));
        }
    };
    bus.run(&mut runtime, stop).await.map_err(ProcessError::Bus)
}

pub struct ShutdownSignals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}
impl ShutdownSignals {
    pub fn new() -> Result<Self, ProcessError> {
        Ok(Self {
            interrupt: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                .map_err(|_| ProcessError::Signal)?,
            terminate: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .map_err(|_| ProcessError::Signal)?,
        })
    }
    pub async fn wait(&mut self) -> Result<(), ProcessError> {
        tokio::select! {
            signal = self.interrupt.recv() => signal.ok_or(ProcessError::Signal),
            signal = self.terminate.recv() => signal.ok_or(ProcessError::Signal),
        }
    }
}

pub fn read_settings(path: &Path) -> Result<StartupSettings, ProcessError> {
    StartupSettings::read(path).map_err(ProcessError::Settings)
}

#[cfg(test)]
mod tests;
