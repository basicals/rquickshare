//! Shared read-only startup configuration. Recovery never rewrites the user's file.

use std::fmt;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::Visibility;

pub const MAX_SETTINGS_BYTES: usize = 1024 * 1024;
pub const LEGACY_IDENTIFIER: &str = "dev.mandre.rquickshare";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsError {
    Read,
    Format,
    Capacity,
    Port,
    DownloadPath,
}
impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Settings error: {self:?}")
    }
}
impl std::error::Error for SettingsError {}

#[derive(Clone, Debug, PartialEq)]
pub struct StartupSettings {
    pub visibility: Visibility,
    pub visibility_recovered: bool,
    pub port: Option<u32>,
    pub download_path: Option<PathBuf>,
}

impl Default for StartupSettings {
    fn default() -> Self {
        Self {
            visibility: Visibility::Visible,
            visibility_recovered: false,
            port: None,
            download_path: None,
        }
    }
}

pub fn visibility_setting(value: Option<&Value>) -> (Visibility, bool) {
    match value {
        None => (Visibility::Visible, false),
        Some(value) => match value.as_u64().and_then(Visibility::try_from_raw_value) {
            Some(visibility) => (visibility, false),
            None => (Visibility::Invisible, true),
        },
    }
}

impl StartupSettings {
    pub fn parse(bytes: &[u8]) -> Result<Self, SettingsError> {
        if bytes.len() > MAX_SETTINGS_BYTES {
            return Err(SettingsError::Capacity);
        }
        let value: Value = serde_json::from_slice(bytes).map_err(|_| SettingsError::Format)?;
        let object = value.as_object().ok_or(SettingsError::Format)?;
        let (visibility, visibility_recovered) = visibility_setting(object.get("visibility"));
        let port = match object.get("port") {
            None | Some(Value::Null) => None,
            Some(value) => Some(u32::from(
                value
                    .as_u64()
                    .and_then(|port| u16::try_from(port).ok())
                    .ok_or(SettingsError::Port)?,
            )),
        };
        let download_path = match object.get("download_path") {
            None | Some(Value::Null) => None,
            Some(Value::String(path)) if !path.is_empty() && !path.contains('\0') => {
                Some(PathBuf::from(path))
            }
            Some(_) => return Err(SettingsError::DownloadPath),
        };
        Ok(Self {
            visibility,
            visibility_recovered,
            port,
            download_path,
        })
    }

    pub fn read(path: &Path) -> Result<Self, SettingsError> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
        }
        let file = match options.open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return match path.symlink_metadata() {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        Ok(Self::default())
                    }
                    _ => Err(SettingsError::Read),
                };
            }
            Err(_) => return Err(SettingsError::Read),
        };
        let metadata = file.metadata().map_err(|_| SettingsError::Read)?;
        if !metadata.is_file() {
            return Err(SettingsError::Read);
        }
        if metadata.len() > MAX_SETTINGS_BYTES as u64 {
            return Err(SettingsError::Capacity);
        }
        let mut bytes = Vec::new();
        file.take(MAX_SETTINGS_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| SettingsError::Read)?;
        Self::parse(&bytes)
    }
}

#[cfg(test)]
mod tests;
