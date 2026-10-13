//! Bounded toolkit-independent owner for a native panel's dedicated SDK connection.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::state::{StateView, TransferView};
use super::{ClientError, NativeClient, OperationValue, Outcome, Selection, Transfer, Upload};
use crate::file_source::OpenedSelection;
use crate::service::{ControlAction, Direction, Phase};

const QUEUE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerError {
    Busy,
    Closed,
    Client(ClientError),
}
impl From<ClientError> for ControllerError {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}
type Result<T> = std::result::Result<T, ControllerError>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PanelState {
    pub view: Option<Arc<StateView>>,
    pub busy: bool,
    pub pending: bool,
    pub error: Option<ClientError>,
    pub disconnected: bool,
    pub settings: Option<super::settings::SettingsView>,
    pub settings_pending: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionResult {
    Settings(super::settings::SettingsView),
    Selection(Selection),
    Transfer(Transfer),
    Completed,
    Recovered(Option<Outcome>),
}

pub struct Completion(oneshot::Receiver<Result<ActionResult>>);
impl Completion {
    /// Dropping this wait never cancels the worker's already admitted action.
    pub async fn wait(self) -> Result<ActionResult> {
        self.0.await.map_err(|_| ControllerError::Closed)?
    }
}

enum Action {
    SettingsRestart,
    SettingsSave(super::settings::SettingsEdit),
    SettingsReload,
    SettingsRetry,
    Dismiss(Vec<super::notices::Notice>),
    Choose(OpenedSelection),
    Send(Selection, String),
    Discard(Selection),
    Respond(Transfer, u64, bool),
    Cancel(Transfer),
    Recover,
    AbortPreparation,
}
struct Request {
    action: Action,
    reply: oneshot::Sender<Result<ActionResult>>,
}

#[derive(Clone)]
pub struct PanelHandle {
    commands: mpsc::Sender<Request>,
    state: watch::Receiver<PanelState>,
}
impl PanelHandle {
    pub fn restart_service(&self) -> Result<Completion> {
        self.submit(Action::SettingsRestart)
    }
    pub fn save_settings(&self, edit: super::settings::SettingsEdit) -> Result<Completion> {
        self.submit(Action::SettingsSave(edit))
    }
    pub fn reload_settings(&self) -> Result<Completion> {
        self.submit(Action::SettingsReload)
    }
    pub fn retry_settings(&self) -> Result<Completion> {
        self.submit(Action::SettingsRetry)
    }
    pub fn subscribe(&self) -> watch::Receiver<PanelState> {
        self.state.clone()
    }
    fn submit(&self, action: Action) -> Result<Completion> {
        let (reply, receiver) = oneshot::channel();
        self.commands
            .try_send(Request { action, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => ControllerError::Busy,
                mpsc::error::TrySendError::Closed(_) => ControllerError::Closed,
            })?;
        Ok(Completion(receiver))
    }
    pub fn dismiss_notices(&self, notices: Vec<super::notices::Notice>) -> Result<Completion> {
        if notices.is_empty() || notices.len() > 256 {
            return Err(ClientError::InvalidInput.into());
        }
        self.submit(Action::Dismiss(notices))
    }
    pub fn choose(&self, selection: OpenedSelection) -> Result<Completion> {
        if !super::desktop::valid_selection(&selection) {
            return Err(ClientError::InvalidInput.into());
        }
        self.submit(Action::Choose(selection))
    }
    pub fn send(&self, selection: Selection, endpoint: String) -> Result<Completion> {
        self.submit(Action::Send(selection, endpoint))
    }
    pub fn discard(&self, selection: Selection) -> Result<Completion> {
        self.submit(Action::Discard(selection))
    }
    pub fn respond(&self, transfer: Transfer, round: u64, accept: bool) -> Result<Completion> {
        self.submit(Action::Respond(transfer, round, accept))
    }
    pub fn cancel(&self, transfer: Transfer) -> Result<Completion> {
        self.submit(Action::Cancel(transfer))
    }
    pub fn recover(&self) -> Result<Completion> {
        self.submit(Action::Recover)
    }
    pub fn abort_preparation(&self) -> Result<Completion> {
        self.submit(Action::AbortPreparation)
    }
}

/// Keep this owner for the application's lifetime; hiding a window does not drop it.
pub struct PanelController {
    handle: PanelHandle,
    stop: CancellationToken,
    task: Option<JoinHandle<std::result::Result<(), ClientError>>>,
}
impl PanelController {
    pub fn spawn(client: NativeClient) -> Self {
        let (commands, receiver) = mpsc::channel(QUEUE);
        let (state, updates) = watch::channel(PanelState::default());
        let stop = CancellationToken::new();
        let task = tokio::spawn(
            Worker {
                client,
                preparation: None,
                state,
            }
            .run(receiver, stop.clone()),
        );
        Self {
            handle: PanelHandle {
                commands,
                state: updates,
            },
            stop,
            task: Some(task),
        }
    }
    pub fn handle(&self) -> PanelHandle {
        self.handle.clone()
    }
    pub async fn close(mut self) -> Result<()> {
        self.stop.cancel();
        self.task
            .take()
            .ok_or(ControllerError::Closed)?
            .await
            .map_err(|_| ControllerError::Closed)?
            .map_err(ControllerError::Client)
    }
}
impl Drop for PanelController {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

struct Preparation {
    source: OpenedSelection,
    upload: Option<Upload>,
    next: u32,
}
struct Worker {
    client: NativeClient,
    preparation: Option<Preparation>,
    state: watch::Sender<PanelState>,
}
fn fatal(error: ClientError) -> bool {
    matches!(
        error,
        ClientError::Transport
            | ClientError::DaemonChanged
            | ClientError::Protocol
            | ClientError::Incompatible
            | ClientError::Closed
            | ClientError::Exhausted
    )
}
fn consent(view: &TransferView, round: u64, accept: bool) -> std::result::Result<(), ClientError> {
    if view.direction != Direction::Inbound
        || view.phase != Phase::AwaitingConsent
        || view.consent_revision != Some(round)
    {
        return Err(ClientError::Rejected(crate::service::Error::StaleConsent));
    }
    if accept && !view.details.valid(true)
        || accept && view.details.status != super::state::DetailsStatus::Ready
    {
        return Err(ClientError::Rejected(crate::service::Error::InvalidState));
    }
    if let Some((action, _)) = view.pending_control {
        let expected = if accept {
            ControlAction::Accept
        } else {
            ControlAction::Reject
        };
        if action != expected {
            return Err(ClientError::Rejected(crate::service::Error::StaleConsent));
        }
    }
    Ok(())
}
impl Worker {
    fn update(&self, error: Option<ClientError>, busy: bool) {
        self.state.send_modify(|state| {
            state.error = error;
            state.busy = busy;
            state.pending = self.client.has_pending() || self.preparation.is_some();
            state.settings_pending = self.client.has_pending_settings();
            state.disconnected |= error.is_some_and(fatal);
        });
    }
    async fn refresh(&mut self) -> std::result::Result<(), ClientError> {
        let view = self.client.read_state().await?;
        let settings = self.client.read_settings().await?;
        self.state.send_modify(|state| {
            state.view = Some(Arc::new(view));
            state.settings = Some(settings);
        });
        Ok(())
    }
    fn recovered_preparation(
        &mut self,
        outcome: Outcome,
    ) -> std::result::Result<Option<Selection>, ClientError> {
        let preparing = self.preparation.as_mut().ok_or(ClientError::Protocol)?;
        match outcome {
            Outcome::Operation(receipt) => match receipt.result {
                OperationValue::Upload(upload) if preparing.upload.is_none() => {
                    preparing.upload = Some(upload)
                }
                OperationValue::Selection(selection) => {
                    self.preparation = None;
                    return Ok(Some(selection));
                }
                OperationValue::Empty => {
                    self.preparation = None;
                }
                _ => return Err(ClientError::Protocol),
            },
            Outcome::Appended {
                upload,
                next_sequence,
            } if preparing.upload.as_ref() == Some(&upload)
                && next_sequence
                    == preparing
                        .next
                        .checked_add(1)
                        .ok_or(ClientError::Exhausted)? =>
            {
                preparing.next = next_sequence
            }
            _ => return Err(ClientError::Protocol),
        }
        Ok(None)
    }
    async fn prepare(&mut self) -> std::result::Result<ActionResult, ClientError> {
        if let Some(outcome) = self.client.recover_pending().await? {
            if let Some(selection) = self.recovered_preparation(outcome)? {
                return Ok(ActionResult::Selection(selection));
            }
            if self.preparation.is_none() {
                return Ok(ActionResult::Completed);
            }
        }
        loop {
            let preparation = self.preparation.as_mut().ok_or(ClientError::Protocol)?;
            let Some(upload) = preparation.upload.as_ref() else {
                preparation.upload = Some(self.client.begin_selection().await?);
                continue;
            };
            let start = preparation.next as usize * super::BATCH_FILES;
            if start < preparation.source.files().len() {
                let end = (start + super::BATCH_FILES).min(preparation.source.files().len());
                preparation.next = self
                    .client
                    .append_batch(
                        upload,
                        preparation.next,
                        &preparation.source.files()[start..end],
                    )
                    .await?;
            } else {
                let selection = self.client.commit_selection(upload).await?;
                self.preparation = None;
                return Ok(ActionResult::Selection(selection));
            }
        }
    }
    async fn act(&mut self, action: Action) -> std::result::Result<ActionResult, ClientError> {
        if self.client.has_pending() || self.preparation.is_some() {
            match action {
                Action::Recover | Action::AbortPreparation => (),
                _ => return Err(ClientError::PendingOperation),
            }
        }
        self.refresh().await?;
        match action {
            Action::SettingsRestart => self
                .client
                .restart_service()
                .await
                .map(|()| ActionResult::Completed),
            Action::SettingsSave(edit) => self
                .client
                .save_settings(edit)
                .await
                .map(ActionResult::Settings),
            Action::SettingsReload => self
                .client
                .reload_settings()
                .await
                .map(ActionResult::Settings),
            Action::SettingsRetry => self
                .client
                .retry_settings()
                .await
                .map(ActionResult::Settings),
            Action::Dismiss(notices) => {
                for batch in notices.chunks(16) {
                    self.client.dismiss_notices(batch).await?;
                }
                Ok(ActionResult::Completed)
            }
            Action::Choose(source) => {
                self.preparation = Some(Preparation {
                    source,
                    upload: None,
                    next: 0,
                });
                self.prepare().await
            }
            Action::Recover if self.preparation.is_some() => self.prepare().await,
            Action::Recover => self
                .client
                .recover_pending()
                .await
                .map(ActionResult::Recovered),
            Action::AbortPreparation => {
                if let Some(outcome) = self.client.recover_pending().await? {
                    if self.preparation.is_none() {
                        return Ok(ActionResult::Recovered(Some(outcome)));
                    }
                    if let Some(selection) = self.recovered_preparation(outcome)? {
                        self.client.discard_selection(&selection).await?;
                        return Ok(ActionResult::Completed);
                    }
                }
                if let Some(preparation) = &self.preparation {
                    if let Some(upload) = &preparation.upload {
                        self.client.abort_selection(upload).await?;
                    }
                }
                self.preparation = None;
                Ok(ActionResult::Completed)
            }
            Action::Send(selection, endpoint) => self
                .client
                .send_selection(&selection, &endpoint)
                .await
                .map(ActionResult::Transfer),
            Action::Discard(selection) => {
                self.client.discard_selection(&selection).await?;
                Ok(ActionResult::Completed)
            }
            Action::Respond(transfer, round, accept) => {
                let view = self
                    .client
                    .cached_state()
                    .ok_or(ClientError::Protocol)?
                    .transfers
                    .iter()
                    .find(|view| view.transfer.id() == transfer.id())
                    .ok_or(ClientError::Rejected(
                        crate::service::Error::UnknownTransfer,
                    ))?;
                consent(view, round, accept)?;
                self.client.respond(&transfer, round, accept).await?;
                Ok(ActionResult::Completed)
            }
            Action::Cancel(transfer) => {
                self.client.cancel(&transfer).await?;
                Ok(ActionResult::Completed)
            }
        }
    }
    async fn run(
        mut self,
        mut commands: mpsc::Receiver<Request>,
        stop: CancellationToken,
    ) -> std::result::Result<(), ClientError> {
        let initialized = tokio::select! { biased; _ = stop.cancelled() => None, result = self.refresh() => Some(result) };
        if let Some(result) = initialized {
            self.update(result.err(), false);
        }
        loop {
            let healthy = !self.state.borrow().disconnected;
            let request = tokio::select! {
                biased;
                _ = stop.cancelled() => break,
                request = commands.recv() => match request { Some(request) => request, None => break },
                change = self.client.next_change(), if healthy => {
                    let result = match change { Ok(_) => tokio::select! { biased; _ = stop.cancelled() => break, result = self.refresh() => result }, Err(error) => Err(error) };
                    self.update(result.err(), false);
                    continue;
                }
            };
            if !healthy {
                let _ = request.reply.send(Err(ControllerError::Client(
                    self.state.borrow().error.unwrap_or(ClientError::Transport),
                )));
                continue;
            }
            self.update(None, true);
            let result = tokio::select! { biased; _ = stop.cancelled() => break, result = self.act(request.action) => result };
            let refreshed = tokio::select! { biased; _ = stop.cancelled() => break, result = self.refresh() => result };
            let error = result.as_ref().err().copied().or(refreshed.err());
            self.update(error, false);
            // Preserve a successful mutation outcome even if its following view read failed.
            let _ = request.reply.send(result.map_err(ControllerError::Client));
        }
        commands.close();
        while let Ok(request) = commands.try_recv() {
            let _ = request.reply.send(Err(ControllerError::Closed));
        }
        self.preparation = None;
        let result = self.client.close().await;
        self.state.send_modify(|state| {
            state.disconnected = true;
            state.busy = false;
            state.pending = false;
        });
        result
    }
}

#[cfg(test)]
mod tests;
