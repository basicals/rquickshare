//! One exact retained command. No client-owned expiry or automatic mode replay.
use super::*;
use crate::service::native_settings::StoreError;
use crate::service::visibility_service::INTERFACE;
pub use crate::service::visibility_service::{VisibilityIntent, VisibilityView};

impl NativeClient {
    pub async fn read_visibility(&mut self) -> Result<VisibilityView> {
        let reply = self.wire(INTERFACE, "Read", &()).await?;
        let (view,): (VisibilityView,) = reply
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)?;
        if !view.valid(self.epoch) {
            return Err(ClientError::Protocol);
        }
        Ok(view)
    }
    pub fn has_pending_visibility(&self) -> bool {
        self.pending_visibility.is_some()
    }
    pub async fn set_visibility(
        &mut self,
        intent: VisibilityIntent,
    ) -> Result<settings::SettingsView> {
        if self.pending_visibility.is_some() || self.pending_settings.is_some() {
            return Err(ClientError::PendingOperation);
        }
        if !intent.valid(self.epoch) {
            return Err(ClientError::InvalidInput);
        }
        self.pending_visibility = Some(intent);
        self.retry_visibility().await
    }
    pub async fn retry_visibility(&mut self) -> Result<settings::SettingsView> {
        let intent = self
            .pending_visibility
            .as_ref()
            .ok_or(ClientError::InvalidInput)?
            .clone();
        let result = self
            .wire(INTERFACE, "Set", &intent)
            .await
            .and_then(|reply| self.checked_settings(reply));
        match &result {
            Ok(view) if view.last_request != intent.request => return Err(ClientError::Protocol),
            Ok(_) => self.pending_visibility = None,
            Err(
                ClientError::Transport
                | ClientError::DaemonChanged
                | ClientError::Protocol
                | ClientError::Settings(StoreError::Indeterminate),
            ) => (),
            Err(_) => self.pending_visibility = None,
        }
        result
    }
}
