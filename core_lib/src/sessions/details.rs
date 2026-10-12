//! Allowlisted display metadata; never contains payload contents or absolute paths.

use std::fmt;
use std::sync::Arc;

use crate::hdl::InnerState;
use crate::utils::DeviceType;
use crate::State;

pub const FILE_LIMIT: usize = 256;
pub const NAME_LIMIT: usize = 1024;
pub const MANIFEST_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DetailsStatus {
    #[default]
    Unavailable,
    Ready,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DeviceKind {
    #[default]
    Unknown,
    Phone,
    Tablet,
    Laptop,
}
#[derive(Clone, Eq, PartialEq)]
pub struct FileDetail {
    pub name: String,
    pub size: u64,
}
impl fmt::Debug for FileDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileDetail")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Default, Eq, PartialEq)]
pub struct TransferDetails {
    pub status: DetailsStatus,
    pub sender: String,
    pub device: DeviceKind,
    pub pin: String,
    pub total_bytes: u64,
    /// Locally written inbound / locally sent outbound bytes, not peer durability.
    pub transferred_bytes: u64,
    pub files: Arc<[FileDetail]>,
}
impl fmt::Debug for TransferDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransferDetails")
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

pub(crate) fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= NAME_LIMIT
        && !name.chars().any(char::is_control)
        && !name.contains('\\')
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
impl TransferDetails {
    pub(crate) fn unsupported() -> Self {
        Self {
            status: DetailsStatus::Unsupported,
            ..Self::default()
        }
    }
    pub(crate) fn valid(&self, waiting: bool) -> bool {
        if self.status != DetailsStatus::Ready {
            return self.sender.is_empty()
                && self.device == DeviceKind::Unknown
                && self.pin.is_empty()
                && self.total_bytes == 0
                && self.transferred_bytes == 0
                && self.files.is_empty();
        }
        if self.sender.is_empty()
            || self.sender.len() > 256
            || self.sender.chars().any(char::is_control)
            || (waiting && (self.pin.len() != 4 || !self.pin.bytes().all(|b| b.is_ascii_digit())))
            || (!waiting && !self.pin.is_empty())
            || self.files.is_empty()
            || self.files.len() > FILE_LIMIT
            || self.transferred_bytes > self.total_bytes
        {
            return false;
        }
        let mut bytes = 0usize;
        let mut total = 0u64;
        for file in self.files.iter() {
            if !safe_name(&file.name) {
                return false;
            }
            let Some(next) = bytes.checked_add(file.name.len()) else {
                return false;
            };
            bytes = next;
            let Some(next) = total.checked_add(file.size) else {
                return false;
            };
            total = next;
        }
        bytes <= MANIFEST_BYTES && total == self.total_bytes
    }
    pub(crate) fn same_description(&self, other: &Self) -> bool {
        self.sender == other.sender
            && self.device == other.device
            && self.pin == other.pin
            && self.files == other.files
            && self.total_bytes == other.total_bytes
    }
    pub(crate) fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.sender.capacity()
            + self.pin.capacity()
            + self.files.len() * std::mem::size_of::<FileDetail>()
            + self
                .files
                .iter()
                .map(|file| file.name.capacity())
                .sum::<usize>()
    }
    pub(crate) fn from_state(state: &InnerState) -> Self {
        let Some(metadata) = &state.transfer_metadata else {
            return Self::default();
        };
        if state.transferred_files.is_empty() {
            return if metadata.files.is_some()
                && state.text_payload.is_none()
                && metadata.text_type.is_none()
            {
                Self::default()
            } else {
                Self::unsupported()
            };
        }
        if state.transferred_files.len() > FILE_LIMIT
            || state.text_payload.is_some()
            || metadata.text_type.is_some()
        {
            return Self::unsupported();
        }
        let Some(sender) = &state.remote_device_info else {
            return Self::unsupported();
        };
        if sender.name.is_empty()
            || sender.name.len() > 256
            || sender.name.chars().any(char::is_control)
        {
            return Self::unsupported();
        }
        let waiting = state.state == State::WaitingForUserConsent;
        let pin = if waiting {
            state.pin_code.as_deref().unwrap_or("")
        } else {
            ""
        };
        if waiting && (pin.len() != 4 || !pin.bytes().all(|b| b.is_ascii_digit())) {
            return Self::unsupported();
        }
        let mut files = Vec::with_capacity(state.transferred_files.len());
        let mut bytes = 0usize;
        for file in state.transferred_files.values() {
            let Some(name) = file.file_url.file_name().and_then(|name| name.to_str()) else {
                return Self::unsupported();
            };
            let parent = file.parent_folder.as_deref().unwrap_or("");
            let length = name.len().saturating_add(if parent.is_empty() {
                0
            } else {
                parent.len().saturating_add(1)
            });
            bytes = bytes.saturating_add(length);
            if length > NAME_LIMIT || bytes > MANIFEST_BYTES || file.total_size < 0 {
                return Self::unsupported();
            }
            let name = if parent.is_empty() {
                name.to_owned()
            } else {
                format!("{parent}/{name}")
            };
            if !safe_name(&name) {
                return Self::unsupported();
            }
            files.push(FileDetail {
                name,
                size: file.total_size as u64,
            });
        }
        files.sort_unstable_by(|a, b| (&a.name, a.size).cmp(&(&b.name, b.size)));
        let result = Self {
            status: DetailsStatus::Ready,
            sender: sender.name.clone(),
            device: match sender.device_type {
                DeviceType::Unknown => DeviceKind::Unknown,
                DeviceType::Phone => DeviceKind::Phone,
                DeviceType::Tablet => DeviceKind::Tablet,
                DeviceType::Laptop => DeviceKind::Laptop,
            },
            pin: pin.to_owned(),
            total_bytes: metadata.total_bytes,
            transferred_bytes: metadata.ack_bytes,
            files: files.into(),
        };
        if result.valid(waiting) {
            result
        } else {
            Self::unsupported()
        }
    }
}

#[cfg(test)]
mod tests;
