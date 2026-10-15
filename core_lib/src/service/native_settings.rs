//! Single-writer native persistence. Call blocking methods from an owned IO worker.
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use rand::TryRngCore;
use rustix::fs::{Mode, OFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::settings::{StartupSettings, MAX_SETTINGS_BYTES};

const FILE: &str = "native-settings.json";
const BACKUP_LIMIT: usize = 8;
const MAX_PATH: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreError {
    Read,
    Capacity,
    UnsafeStorage,
    InvalidInput,
    Directory,
    Conflict,
    ReplayConflict,
    RecoveryRequired,
    BackupCapacity,
    Write,
    Indeterminate,
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Native settings: {self:?}")
    }
}
impl std::error::Error for StoreError {}
type Result<T> = std::result::Result<T, StoreError>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardMode {
    #[default]
    Compact,
    Full,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryRecord {
    path: Vec<u8>,
    device: u64,
    inode: u64,
}
impl fmt::Debug for DirectoryRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DirectoryRecord").finish_non_exhaustive()
    }
}
impl DirectoryRecord {
    pub fn same_object(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode
    }
    pub fn path(&self) -> PathBuf {
        std::ffi::OsString::from_vec(self.path.clone()).into()
    }
    fn validate(&self) -> Result<()> {
        if self.path.is_empty()
            || self.path.len() > MAX_PATH
            || self.path.contains(&0)
            || !self.path().is_absolute()
        {
            return Err(StoreError::InvalidInput);
        }
        Ok(())
    }
    pub fn acquire(file: &File) -> Result<Self> {
        let metadata = file.metadata().map_err(|_| StoreError::Directory)?;
        if !metadata.is_dir() {
            return Err(StoreError::Directory);
        }
        let path = std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))
            .map_err(|_| StoreError::Directory)?;
        let record = Self {
            path: path.as_os_str().as_bytes().to_vec(),
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        record.validate()?;
        // Reject deleted or renamed-to-a-different-object paths at acquisition.
        let opened = record.open()?;
        drop(opened);
        Ok(record)
    }
    pub fn open(&self) -> Result<File> {
        self.validate()?;
        let file = File::from(
            rustix::fs::open(
                self.path(),
                OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| StoreError::Directory)?,
        );
        let metadata = file.metadata().map_err(|_| StoreError::Directory)?;
        if metadata.dev() != self.device || metadata.ino() != self.inode {
            return Err(StoreError::Directory);
        }
        Ok(file)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Values {
    pub visibility: u8,
    pub port: Option<u16>,
    pub card_mode: CardMode,
    pub device_name: Option<crate::identity::DeviceName>,
    pub directory: Option<DirectoryRecord>,
}
impl Values {
    pub fn validate(&self) -> Result<()> {
        if crate::Visibility::try_from_raw_value(self.visibility.into()).is_none() {
            return Err(StoreError::InvalidInput);
        }
        if let Some(directory) = &self.directory {
            directory.validate()?;
        }
        Ok(())
    }
    pub fn startup(&self) -> Result<StartupSettings> {
        self.validate()?;
        Ok(StartupSettings {
            visibility: crate::Visibility::try_from_raw_value(self.visibility.into())
                .ok_or(StoreError::InvalidInput)?,
            visibility_recovered: false,
            port: self.port.map(u32::from),
            download_path: self.directory.as_ref().map(DirectoryRecord::path),
            device_name: self.device_name.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Base {
    pub revision: u64,
    pub token: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Source {
    Defaults,
    Legacy,
    Native,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    request: String,
    fingerprint: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    revision: u64,
    values: Values,
    receipt: Receipt,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyValues {
    visibility: u8,
    port: Option<u16>,
    card_mode: CardMode,
    directory: Option<DirectoryRecord>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyDocument {
    schema: u32,
    revision: u64,
    values: LegacyValues,
    receipt: Receipt,
}
fn decode_document(raw: &[u8]) -> Option<Document> {
    let version = serde_json::from_slice::<serde_json::Value>(raw)
        .ok()?
        .get("schema")?
        .as_u64()?;
    match version {
        1 => {
            let old: LegacyDocument = serde_json::from_slice(raw).ok()?;
            Some(Document {
                schema: old.schema,
                revision: old.revision,
                receipt: old.receipt,
                values: Values {
                    visibility: old.values.visibility,
                    port: old.values.port,
                    card_mode: old.values.card_mode,
                    directory: old.values.directory,
                    device_name: None,
                },
            })
        }
        2 => serde_json::from_slice(raw).ok(),
        _ => None,
    }
}

#[derive(Clone)]
pub struct Snapshot {
    pub base: Base,
    pub source: Source,
    pub values: Option<Values>,
    pub visibility_recovered: bool,
    raw: Option<Vec<u8>>,
    schema: u32,
    receipt: Option<Receipt>,
}
impl Snapshot {
    pub fn last_request(&self) -> Option<&str> {
        self.receipt
            .as_ref()
            .map(|receipt| receipt.request.as_str())
    }
}

pub enum DirectoryIntent {
    Retain,
    Default,
    Replace(File),
}
pub struct Edit {
    pub base: Base,
    pub request: String,
    pub visibility: u8,
    pub port: Option<u16>,
    pub card_mode: CardMode,
    pub directory: DirectoryIntent,
    pub device_name: Option<crate::identity::DeviceName>,
    pub recover: bool,
}

/// Only external side effects are replaceable; CAS and migration remain real.
pub trait PersistenceIo {
    fn write(&self, file: &mut File, bytes: &[u8]) -> std::io::Result<()>;
    fn sync_file(&self, file: &File) -> std::io::Result<()>;
    fn rename(&self, directory: &File, from: &str, to: &str) -> std::io::Result<()>;
    fn sync_directory(&self, directory: &File) -> std::io::Result<()>;
}
pub struct SystemIo;
impl PersistenceIo for SystemIo {
    fn write(&self, file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
        file.write_all(bytes)
    }
    fn sync_file(&self, file: &File) -> std::io::Result<()> {
        file.sync_all()
    }
    fn rename(&self, directory: &File, from: &str, to: &str) -> std::io::Result<()> {
        Ok(rustix::fs::renameat(directory, from, directory, to)?)
    }
    fn sync_directory(&self, directory: &File) -> std::io::Result<()> {
        directory.sync_all()
    }
}

pub struct Store<I = SystemIo> {
    directory: PathBuf,
    legacy: PathBuf,
    io: I,
}
impl Store<SystemIo> {
    pub fn new(directory: PathBuf, legacy: PathBuf) -> Self {
        Self::with_io(directory, legacy, SystemIo)
    }
}
impl<I: PersistenceIo> Store<I> {
    pub fn with_io(directory: PathBuf, legacy: PathBuf, io: I) -> Self {
        Self {
            directory,
            legacy,
            io,
        }
    }
    pub fn load(&self) -> Result<Snapshot> {
        let directory = self.open_directory(false)?;
        self.load_at(directory.as_ref())
    }
    fn load_at(&self, directory: Option<&File>) -> Result<Snapshot> {
        if let Some(directory) = directory {
            if let Some(raw) = read_at(directory, FILE)? {
                let parsed = decode_document(&raw).filter(|document| {
                    matches!(document.schema, 1 | 2)
                        && document.revision > 0
                        && document.values.validate().is_ok()
                        && valid_id(&document.receipt.request)
                        && valid_hash(&document.receipt.fingerprint)
                });
                let token = hash(&[b"native", &raw]);
                return Ok(match parsed {
                    Some(document) => Snapshot {
                        base: Base {
                            revision: document.revision,
                            token,
                        },
                        source: Source::Native,
                        values: Some(document.values),
                        visibility_recovered: false,
                        raw: Some(raw),
                        schema: document.schema,
                        receipt: Some(document.receipt),
                    },
                    None => Snapshot {
                        base: Base { revision: 0, token },
                        source: Source::Native,
                        values: None,
                        visibility_recovered: false,
                        raw: Some(raw),
                        schema: 0,
                        receipt: None,
                    },
                });
            }
        }
        let raw = read_legacy(&self.legacy)?;
        let source = if raw.is_some() {
            Source::Legacy
        } else {
            Source::Defaults
        };
        let token = hash(&[
            if raw.is_some() { b"legacy" } else { b"absent" },
            raw.as_deref().unwrap_or_default(),
        ]);
        let parsed = match &raw {
            Some(raw) => StartupSettings::parse(raw).ok(),
            None => Some(StartupSettings::default()),
        };
        let visibility_recovered = parsed
            .as_ref()
            .is_some_and(|settings| settings.visibility_recovered);
        let values = parsed.and_then(|settings| {
            // Legacy custom paths are not silently replaced when unavailable.
            let directory = match settings.download_path {
                Some(path) => {
                    let file = File::from(
                        rustix::fs::open(
                            path,
                            OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
                            Mode::empty(),
                        )
                        .ok()?,
                    );
                    Some(DirectoryRecord::acquire(&file).ok()?)
                }
                None => None,
            };
            Some(Values {
                visibility: settings.visibility as u8,
                port: settings.port.map(|port| port as u16),
                card_mode: CardMode::Compact,
                device_name: None,
                directory,
            })
        });
        Ok(Snapshot {
            base: Base { revision: 0, token },
            source,
            values,
            visibility_recovered,
            raw,
            schema: 0,
            receipt: None,
        })
    }
    pub fn save(&self, edit: Edit) -> Result<Snapshot> {
        if !valid_id(&edit.request) || !valid_hash(&edit.base.token) || edit.visibility > 2 {
            return Err(StoreError::InvalidInput);
        }
        let selected = match &edit.directory {
            DirectoryIntent::Replace(file) => Some(DirectoryRecord::acquire(file)?),
            _ => None,
        };
        let intent = match edit.directory {
            DirectoryIntent::Retain => 0u8,
            DirectoryIntent::Default => 1,
            DirectoryIntent::Replace(_) => 2,
        };
        let identity = selected
            .as_ref()
            .map(|record| (record.device, record.inode));
        let fingerprint = hash(&[&serde_json::to_vec(&(
            &edit.base,
            edit.visibility,
            edit.port,
            edit.card_mode,
            &edit.device_name,
            intent,
            identity,
            edit.recover,
        ))
        .map_err(|_| StoreError::InvalidInput)?]);
        let directory = self.open_directory(true)?.ok_or(StoreError::Write)?;
        let current = self.load_at(Some(&directory))?;
        if let Some(receipt) = &current.receipt {
            if receipt.request == edit.request {
                if receipt.fingerprint != fingerprint {
                    return Err(StoreError::ReplayConflict);
                }
                self.io
                    .sync_directory(&directory)
                    .map_err(|_| StoreError::Indeterminate)?;
                return Ok(current);
            }
        }
        if current.base != edit.base {
            return Err(StoreError::Conflict);
        }
        if current.values.is_none() && !edit.recover {
            return Err(StoreError::RecoveryRequired);
        }
        let values = Values {
            visibility: edit.visibility,
            port: edit.port,
            card_mode: edit.card_mode,
            device_name: edit.device_name,
            directory: if intent == 0 {
                current
                    .values
                    .as_ref()
                    .ok_or(StoreError::RecoveryRequired)?
                    .directory
                    .clone()
            } else {
                selected
            },
        };
        values.validate()?;
        if current.source != Source::Native || current.values.is_none() || current.schema == 1 {
            if let Some(raw) = &current.raw {
                self.backup(&directory, raw)?;
            }
        }
        let revision = current
            .base
            .revision
            .checked_add(1)
            .ok_or(StoreError::Capacity)?;
        let document = Document {
            schema: 2,
            revision,
            values,
            receipt: Receipt {
                request: edit.request,
                fingerprint,
            },
        };
        let bytes = serde_json::to_vec(&document).map_err(|_| StoreError::InvalidInput)?;
        if bytes.len() > MAX_SETTINGS_BYTES {
            return Err(StoreError::Capacity);
        }
        // Re-read after backup IO; never import/overwrite an externally edited source.
        if self.load_at(Some(&directory))?.base != current.base {
            return Err(StoreError::Conflict);
        }
        self.replace(&directory, FILE, &bytes)?;
        Ok(Snapshot {
            base: Base {
                revision,
                token: hash(&[b"native", &bytes]),
            },
            source: Source::Native,
            values: Some(document.values),
            visibility_recovered: false,
            raw: Some(bytes),
            schema: 2,
            receipt: Some(document.receipt),
        })
    }
    fn open_directory(&self, create: bool) -> Result<Option<File>> {
        if !self.directory.is_absolute() {
            return Err(StoreError::UnsafeStorage);
        }
        if create {
            if let Some(parent) = self.directory.parent() {
                std::fs::create_dir_all(parent).map_err(|_| StoreError::Write)?;
            }
            match std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&self.directory)
            {
                Ok(()) => (),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
                Err(_) => return Err(StoreError::Write),
            }
        }
        let fd = match rustix::fs::open(
            &self.directory,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::CLOEXEC
                | OFlags::NONBLOCK,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(rustix::io::Errno::NOENT) if !create => return Ok(None),
            Err(_) => return Err(StoreError::UnsafeStorage),
        };
        let file = File::from(fd);
        let metadata = file.metadata().map_err(|_| StoreError::Read)?;
        if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o077 != 0 {
            return Err(StoreError::UnsafeStorage);
        }
        Ok(Some(file))
    }
    fn backup(&self, directory: &File, bytes: &[u8]) -> Result<()> {
        let name = format!("backup-{}.json", hash(&[bytes]));
        if let Some(existing) = read_at(directory, &name)? {
            return if existing == bytes {
                Ok(())
            } else {
                Err(StoreError::UnsafeStorage)
            };
        }
        let entries = std::fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))
            .map_err(|_| StoreError::Read)?;
        let mut count = 0;
        for entry in entries {
            if entry
                .map_err(|_| StoreError::Read)?
                .file_name()
                .as_bytes()
                .starts_with(b"backup-")
            {
                count += 1;
            }
        }
        if count >= BACKUP_LIMIT {
            return Err(StoreError::BackupCapacity);
        }
        self.replace(directory, &name, bytes)
    }
    fn replace(&self, directory: &File, destination: &str, bytes: &[u8]) -> Result<()> {
        let mut nonce = [0; 16];
        rand::rngs::OsRng
            .try_fill_bytes(&mut nonce)
            .map_err(|_| StoreError::Write)?;
        let name = format!(".pending-{}", hex::encode(nonce));
        let mut file = File::from(
            rustix::fs::openat(
                directory,
                name.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|_| StoreError::Write)?,
        );
        let result = (|| {
            self.io
                .write(&mut file, bytes)
                .map_err(|_| StoreError::Write)?;
            self.io.sync_file(&file).map_err(|_| StoreError::Write)?;
            self.io
                .rename(directory, &name, destination)
                .map_err(|_| StoreError::Write)?;
            self.io
                .sync_directory(directory)
                .map_err(|_| StoreError::Indeterminate)
        })();
        let _ = rustix::fs::unlinkat(directory, name.as_str(), rustix::fs::AtFlags::empty());
        result
    }
}

fn hash(parts: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part);
    }
    hex::encode(digest.finalize())
}
fn valid_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn read_file(file: File, private: bool) -> Result<Vec<u8>> {
    let metadata = file.metadata().map_err(|_| StoreError::Read)?;
    if !metadata.is_file()
        || (private
            && (metadata.uid() != rustix::process::geteuid().as_raw()
                || metadata.mode() & 0o077 != 0
                || metadata.nlink() != 1))
    {
        return Err(StoreError::UnsafeStorage);
    }
    if metadata.len() > MAX_SETTINGS_BYTES as u64 {
        return Err(StoreError::Capacity);
    }
    let mut bytes = Vec::new();
    file.take(MAX_SETTINGS_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StoreError::Read)?;
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err(StoreError::Capacity);
    }
    Ok(bytes)
}
fn read_at(directory: &File, name: &str) -> Result<Option<Vec<u8>>> {
    match rustix::fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => read_file(File::from(fd), true).map(Some),
        Err(rustix::io::Errno::NOENT) => Ok(None),
        Err(_) => Err(StoreError::UnsafeStorage),
    }
}
fn read_legacy(path: &Path) -> Result<Option<Vec<u8>>> {
    match OpenOptions::new()
        .read(true)
        .custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC).bits() as i32)
        .open(path)
    {
        Ok(file) => read_file(file, false).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match path.symlink_metadata() {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(StoreError::Read),
            }
        }
        Err(_) => Err(StoreError::Read),
    }
}

#[cfg(test)]
mod tests;
