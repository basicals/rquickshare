//! Shared descriptor-pinned private file IO; callers own schema and transactions.
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::Path;

use rand::TryRngCore;
use rustix::fs::{Mode, OFlags};

use super::native_settings::{PersistenceIo, StoreError};
type Result<T> = std::result::Result<T, StoreError>;
pub(super) fn directory(directory: &Path, create: bool) -> Result<Option<File>> {
    if !directory.is_absolute() {
        return Err(StoreError::UnsafeStorage);
    }
    if create {
        if let Some(parent) = directory.parent() {
            std::fs::create_dir_all(parent).map_err(|_| StoreError::Write)?;
        }
        match std::fs::DirBuilder::new().mode(0o700).create(&directory) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err(StoreError::Write),
        }
    }
    let fd = match rustix::fs::open(
        directory,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
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
pub(super) fn replace(
    directory: &File,
    destination: &str,
    bytes: &[u8],
    io: &impl PersistenceIo,
) -> Result<()> {
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
        io.write(&mut file, bytes).map_err(|_| StoreError::Write)?;
        io.sync_file(&file).map_err(|_| StoreError::Write)?;
        io.rename(directory, &name, destination)
            .map_err(|_| StoreError::Write)?;
        io.sync_directory(directory)
            .map_err(|_| StoreError::Indeterminate)
    })();
    let _ = rustix::fs::unlinkat(directory, name.as_str(), rustix::fs::AtFlags::empty());
    result
}
pub(super) fn read(directory: &File, name: &str, limit: usize) -> Result<Option<Vec<u8>>> {
    let fd = match rustix::fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(_) => return Err(StoreError::UnsafeStorage),
    };
    let file = File::from(fd);
    let metadata = file.metadata().map_err(|_| StoreError::Read)?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
    {
        return Err(StoreError::UnsafeStorage);
    }
    if metadata.len() > limit as u64 {
        return Err(StoreError::Capacity);
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StoreError::Read)?;
    if bytes.len() > limit {
        return Err(StoreError::Capacity);
    }
    Ok(Some(bytes))
}
