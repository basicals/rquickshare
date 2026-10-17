//! Owner-local receive policy and per-request directory authority.
use std::{
    fmt,
    fs::File,
    io,
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

#[derive(Clone)]
pub(crate) struct ReceivePolicy(Arc<RwLock<Option<Directory>>>);
#[derive(Clone)]
pub(crate) struct Directory {
    path: PathBuf,
    #[cfg(target_os = "linux")]
    authority: Option<Arc<File>>,
}
#[derive(Clone)]
pub(crate) struct Root {
    display: PathBuf,
    #[cfg(target_os = "linux")]
    authority: Arc<File>,
}
impl fmt::Debug for ReceivePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceivePolicy").finish_non_exhaustive()
    }
}
impl fmt::Debug for Directory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceiveDirectory").finish_non_exhaustive()
    }
}
impl fmt::Debug for Root {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceiveRoot").finish_non_exhaustive()
    }
}
impl ReceivePolicy {
    pub(crate) fn set_directory(&self, directory: Option<Directory>) {
        *self
            .0
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = directory;
    }
    pub(crate) fn new(path: Option<PathBuf>) -> Self {
        Self(Arc::new(RwLock::new(path.map(Directory::configured))))
    }
    pub(crate) fn set(&self, path: Option<PathBuf>) {
        *self
            .0
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = path.map(Directory::configured);
    }
    pub(crate) fn snapshot(&self) -> Directory {
        self.0
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .unwrap_or_else(|| Directory::configured(crate::utils::get_download_dir()))
    }
}
impl Directory {
    #[cfg(target_os = "linux")]
    pub(crate) fn adopt(file: File) -> Result<Self, crate::service::native_settings::StoreError> {
        let record = crate::service::native_settings::DirectoryRecord::acquire(&file)?;
        Ok(Self {
            path: record.path(),
            authority: Some(Arc::new(file)),
        })
    }
    fn configured(path: PathBuf) -> Self {
        Self {
            path,
            #[cfg(target_os = "linux")]
            authority: None,
        }
    }
    pub(crate) fn pin(&self) -> io::Result<Root> {
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            let authority = match &self.authority {
                Some(file) => file.clone(),
                None => Arc::new(File::from(rustix::fs::open(
                    &self.path,
                    rustix::fs::OFlags::PATH
                        | rustix::fs::OFlags::DIRECTORY
                        | rustix::fs::OFlags::CLOEXEC,
                    rustix::fs::Mode::empty(),
                )?)),
            };
            let display = std::fs::read_link(format!("/proc/self/fd/{}", authority.as_raw_fd()))?;
            Ok(Root { display, authority })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let display = self.path.canonicalize()?;
            if !display.is_dir() {
                return Err(io::ErrorKind::NotADirectory.into());
            }
            Ok(Root { display })
        }
    }
}
impl Root {
    pub(crate) fn display(&self) -> &Path {
        &self.display
    }
    pub(crate) fn planning_path(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            PathBuf::from(format!("/proc/self/fd/{}", self.authority.as_raw_fd()))
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.display.clone()
        }
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn open_received(&self, relative: &str) -> io::Result<File> {
        use rustix::fs::{openat2, Mode, OFlags, ResolveFlags};
        Ok(File::from(openat2(
            &self.authority,
            relative,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )?))
    }
    #[cfg(any(test, not(target_os = "linux")))]
    pub(crate) fn create_file(&self, destination: &Path) -> io::Result<File> {
        #[cfg(target_os = "linux")]
        {
            let (parent, leaf) = self.destination_parent(destination)?;
            Ok(File::from(rustix::fs::openat(
                parent,
                leaf,
                rustix::fs::OFlags::WRONLY
                    | rustix::fs::OFlags::CREATE
                    | rustix::fs::OFlags::EXCL
                    | rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::from_raw_mode(0o666),
            )?))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let destination =
                crate::hdl::inbound::materialize_safe_destination_at(&self.display, destination)
                    .map_err(|_| io::ErrorKind::PermissionDenied)?;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
        }
    }
    #[cfg(target_os = "linux")]
    fn destination_parent(&self, destination: &Path) -> io::Result<(File, std::ffi::OsString)> {
        let relative = destination
            .strip_prefix(&self.display)
            .map_err(|_| io::ErrorKind::InvalidInput)?;
        let components = relative
            .components()
            .map(|component| match component {
                Component::Normal(name) => Ok(name),
                _ => Err(io::Error::from(io::ErrorKind::InvalidInput)),
            })
            .collect::<io::Result<Vec<_>>>()?;
        let (leaf, parents) = components.split_last().ok_or(io::ErrorKind::InvalidInput)?;
        {
            use rustix::fs::{mkdirat, openat, Mode, OFlags};
            let mut parent: Option<File> = None;
            for name in parents {
                let directory = parent.as_ref().unwrap_or(&self.authority);
                let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
                let opened = match openat(directory, *name, flags, Mode::empty()) {
                    Ok(fd) => fd,
                    Err(rustix::io::Errno::NOENT) => {
                        match mkdirat(directory, *name, Mode::from_raw_mode(0o777)) {
                            Ok(()) | Err(rustix::io::Errno::EXIST) => (),
                            Err(error) => return Err(error.into()),
                        }
                        openat(directory, *name, flags, Mode::empty())?
                    }
                    Err(error) => return Err(error.into()),
                };
                parent = Some(File::from(opened));
            }
            let directory = match parent {
                Some(parent) => parent,
                None => File::from(openat(
                    &self.authority,
                    ".",
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                    Mode::empty(),
                )?),
            };
            Ok((directory, (*leaf).to_owned()))
        }
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn stage_file(&self, destination: &Path) -> io::Result<(File, Publication)> {
        use rustix::fs::{openat, Mode, OFlags};
        let (parent, leaf) = self.destination_parent(destination)?;
        // No pathname exists to clean up, even after an abrupt process exit.
        let file = File::from(openat(
            &parent,
            ".",
            OFlags::TMPFILE | OFlags::RDWR | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )?);
        Ok((file, Publication { parent, leaf }))
    }
}

#[cfg(target_os = "linux")]
pub(crate) struct Publication {
    parent: File,
    leaf: std::ffi::OsString,
}
#[cfg(target_os = "linux")]
impl fmt::Debug for Publication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceivePublication").finish_non_exhaustive()
    }
}
#[cfg(target_os = "linux")]
impl Publication {
    pub(crate) fn publish(&self, file: &File, size: u64) -> io::Result<()> {
        use std::os::fd::AsRawFd;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() != size {
            return Err(io::ErrorKind::InvalidData.into());
        }
        file.sync_data()?;
        rustix::fs::linkat(
            rustix::fs::CWD,
            format!("/proc/self/fd/{}", file.as_raw_fd()),
            &self.parent,
            &self.leaf,
            rustix::fs::AtFlags::SYMLINK_FOLLOW,
        )?;
        Ok(())
    }
    pub(crate) fn sync(&self) -> io::Result<()> {
        self.parent.sync_all()
    }
}

#[cfg(test)]
mod tests;
