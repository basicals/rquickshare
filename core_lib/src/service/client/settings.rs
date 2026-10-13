//! Typed settings mutations have their own exact retry identity and retained FD.
use std::fs::File;

use super::*;
use crate::service::native_settings::StoreError;
use crate::service::settings_service::INTERFACE as SETTINGS;
pub use crate::service::settings_service::{SaveIntent, SettingsView};

pub struct SettingsEdit {
    pub intent: SaveIntent,
    pub directory: Option<File>,
}
impl NativeClient {
    pub async fn restart_service(&mut self) -> Result<()> {
        let epoch = format!("{:032x}", self.epoch);
        self.wire(SETTINGS, "Restart", &epoch)
            .await?
            .body()
            .deserialize::<()>()
            .map_err(|_| ClientError::Protocol)
    }
    fn checked_settings(&self, reply: Message) -> Result<SettingsView> {
        if !reply.data().fds().is_empty() {
            return Err(ClientError::Protocol);
        }
        let (view,): (SettingsView,) = reply
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)?;
        if !view.valid(self.epoch) {
            return Err(ClientError::Protocol);
        }
        Ok(view)
    }
    pub async fn read_settings(&mut self) -> Result<SettingsView> {
        let reply = self.wire(SETTINGS, "Read", &()).await?;
        self.checked_settings(reply)
    }
    pub async fn reload_settings(&mut self) -> Result<SettingsView> {
        let reply = self.wire(SETTINGS, "Reload", &()).await?;
        self.checked_settings(reply)
    }
    pub fn has_pending_settings(&self) -> bool {
        self.pending_settings.is_some()
    }
    pub async fn save_settings(&mut self, edit: SettingsEdit) -> Result<SettingsView> {
        if self.pending_settings.is_some() {
            return Err(ClientError::PendingOperation);
        }
        if !edit.intent.valid(self.epoch)
            || edit.directory.is_some() != (edit.intent.directory == 2)
        {
            return Err(ClientError::InvalidInput);
        }
        self.pending_settings = Some(edit);
        self.retry_settings().await
    }
    pub async fn retry_settings(&mut self) -> Result<SettingsView> {
        let edit = self
            .pending_settings
            .as_ref()
            .ok_or(ClientError::InvalidInput)?;
        let intent = edit.intent.clone();
        let directory = edit
            .directory
            .as_ref()
            .map(File::try_clone)
            .transpose()
            .map_err(|_| ClientError::Source)?;
        let files: Vec<Fd<'_>> = directory
            .as_ref()
            .map(|file| Fd::from(file.as_fd()))
            .into_iter()
            .collect();
        let result = self
            .wire(SETTINGS, "Save", &(intent.clone(), files))
            .await
            .and_then(|reply| self.checked_settings(reply));
        match &result {
            Ok(view) if view.last_request != intent.request => return Err(ClientError::Protocol),
            Ok(_) => self.pending_settings = None,
            Err(
                ClientError::Transport
                | ClientError::DaemonChanged
                | ClientError::Protocol
                | ClientError::Settings(StoreError::Indeterminate),
            ) => (),
            Err(_) => self.pending_settings = None,
        }
        result
    }
}
