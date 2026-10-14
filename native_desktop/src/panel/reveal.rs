//! Explicit per-file reveal; stale GUI completions cannot launch external desktop work.
use super::*;
use rqs_lib::{
    service::client::{controller::ActionResult, Transfer},
    sessions::details::PayloadKind,
};
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub(super) struct Key {
    epoch: u128,
    transfer: Transfer,
    revision: u64,
    index: u32,
}
struct Observation<'a> {
    epoch: u128,
    id: &'a str,
    revision: Option<u64>,
    direction: Direction,
    phase: Phase,
    details: &'a TransferDetails,
}
fn matches(epoch: u128, id: &str, revision: u64, index: u32, observed: &Observation<'_>) -> bool {
    observed.epoch == epoch
        && observed.id == id
        && observed.revision == Some(revision)
        && revision != 0
        && observed.direction == Direction::Inbound
        && observed.phase == Phase::Finished
        && observed.details.kind == PayloadKind::Files
        && observed.details.status == DetailsStatus::Ready
        && observed.details.transferred_bytes == observed.details.total_bytes
        && (index as usize) < observed.details.files.len()
}
fn current(key: &Key, state: &PanelState) -> bool {
    !state.disconnected
        && !state.pending
        && !state.text_pending
        && !state.settings_pending
        && state.view.as_ref().is_some_and(|view| {
            view.synchronized
                && view.lifecycle == Lifecycle::Running
                && view.transfers.iter().any(|row| {
                    row.transfer == key.transfer
                        && matches(
                            key.epoch,
                            key.transfer.id(),
                            key.revision,
                            key.index,
                            &Observation {
                                epoch: view.epoch,
                                id: row.transfer.id(),
                                revision: row.reveal_revision,
                                direction: row.direction,
                                phase: row.phase,
                                details: &row.details,
                            },
                        )
                })
        })
}
fn file_button(index: u32, ready: bool, action: impl Fn(u32) + 'static) -> gtk::Button {
    let button = gtk::Button::with_label("Show in folder");
    button.set_widget_name(&format!("reveal-{index}"));
    button.set_sensitive(ready);
    button.connect_clicked(move |_| action(index));
    button
}
pub(super) struct Pending {
    key: Key,
    cancel: CancellationToken,
    task: RefCell<Option<glib::JoinHandle<()>>>,
}
impl Pending {
    fn close(&self) {
        self.cancel.cancel();
        if let Some(task) = self.task.borrow_mut().take() {
            task.abort();
        }
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.close();
    }
}
impl Panel {
    fn reveal_allowed(&self, key: &Key, owned: bool) -> bool {
        let model = self.model.borrow();
        let state = self
            .handle
            .as_ref()
            .map(|handle| handle.subscribe().borrow().clone())
            .unwrap_or_else(|| model.state.clone());
        !model.closing
            && !model.preview
            && model.safety.manual_allowed()
            && !model.waiting
            && model.dialog.is_none()
            && current(key, &state)
            && (owned
                || (!state.busy && !model.ingress.busy && !*self.ingress.gate.subscribe().borrow()))
    }
    pub(super) fn close_reveal(&self) {
        if let Some(pending) = self.reveal.borrow_mut().take() {
            pending.close();
        }
    }
    pub(super) fn refresh_reveal(&self) {
        let pending = self.reveal.borrow().clone();
        if let Some(pending) = pending {
            if !self.reveal_allowed(&pending.key, true) {
                self.close_reveal();
            }
        }
    }
    pub(super) fn revealing(&self, target: &Transfer) -> bool {
        self.reveal
            .borrow()
            .as_ref()
            .is_some_and(|pending| pending.key.transfer == *target)
    }
    pub(super) fn file_action(
        self: &Rc<Self>,
        transfer: &TransferView,
        full: bool,
    ) -> Option<Rc<FileAction>> {
        let revision = transfer.reveal_revision?;
        if !full
            || transfer.direction != Direction::Inbound
            || transfer.phase != Phase::Finished
            || transfer.details.kind != PayloadKind::Files
        {
            return None;
        }
        let weak = Rc::downgrade(self);
        let target = transfer.transfer.clone();
        Some(Rc::new(move |row, index| {
            let weak = weak.clone();
            let target = target.clone();
            let ready = weak.upgrade().is_some_and(|panel| {
                let epoch = panel
                    .model
                    .borrow()
                    .state
                    .view
                    .as_ref()
                    .map_or(0, |view| view.epoch);
                panel.handle.is_some()
                    && panel.reveal_allowed(
                        &Key {
                            epoch,
                            transfer: target.clone(),
                            revision,
                            index,
                        },
                        false,
                    )
            });
            let button = file_button(index, ready, move |index| {
                if let Some(panel) = weak.upgrade() {
                    panel.reveal_file(target.clone(), revision, index);
                }
            });
            row.append(&button);
        }))
    }
    pub(super) fn reveal_file(self: &Rc<Self>, target: Transfer, revision: u64, index: u32) {
        let epoch = self
            .model
            .borrow()
            .state
            .view
            .as_ref()
            .map_or(0, |view| view.epoch);
        let key = Key {
            epoch,
            transfer: target,
            revision,
            index,
        };
        if !self.reveal_allowed(&key, false) {
            return;
        }
        let Some(handle) = &self.handle else {
            return;
        };
        let Some(permit) = self.ingress.gate.try_acquire() else {
            return;
        };
        let completion = match handle.acquire_file(key.transfer.clone(), revision, index) {
            Ok(completion) => completion,
            Err(_) => {
                self.notice(
                    "Cannot show this file right now. Review the current transfer and try again.",
                );
                return;
            }
        };
        self.close_reveal();
        let pending = Rc::new(Pending {
            key: key.clone(),
            cancel: CancellationToken::new(),
            task: RefCell::new(None),
        });
        let weak = Rc::downgrade(self);
        let operation_owner = Rc::downgrade(&pending);
        let task = glib::spawn_future_local(async move {
            let result = completion.wait().await;
            let Some(panel) = weak.upgrade() else {
                return;
            };
            let Some(operation) = operation_owner.upgrade() else {
                return;
            };
            if operation.cancel.is_cancelled() || !panel.reveal_allowed(&key, true) {
                panel.close_reveal();
                return;
            }
            let read = match result {
                Ok(ActionResult::File(read))
                    if read.epoch == key.epoch
                        && read.transfer == key.transfer
                        && read.revision == key.revision
                        && read.index == key.index =>
                {
                    read
                }
                _ => {
                    panel.close_reveal();
                    panel.notice(
                        "Cannot show this file. It may have moved, changed, or become unavailable.",
                    );
                    panel.render();
                    return;
                }
            };
            let file = match read.file().try_clone() {
                Ok(file) => Arc::new(file),
                Err(_) => {
                    panel.close_reveal();
                    panel.notice("Cannot access this received file right now.");
                    panel.render();
                    return;
                }
            };
            // Final GUI guard immediately precedes handing the FD to the owned portal worker.
            if !panel.reveal_allowed(&key, true) {
                panel.close_reveal();
                return;
            }
            let receive =
                panel
                    .ingress
                    .start_reveal(&panel.runtime, file, operation.cancel.clone(), permit);
            drop(operation);
            drop(panel);
            let result = receive.await;
            if let Some(panel) = weak.upgrade() {
                if let Some(operation) = operation_owner.upgrade() {
                    if operation.cancel.is_cancelled() || !panel.reveal_allowed(&key, true) {
                        panel.close_reveal();
                        return;
                    }
                    panel.close_reveal();
                    panel.notice(match result {
                        Ok(Ok(())) => "Containing folder shown.",
                        Ok(Err(crate::portal::Error::Cancelled)) => "Folder request cancelled.",
                        _ => "Your desktop could not show this folder. Try again.",
                    });
                    panel.render();
                }
            }
        });
        pending.task.replace(Some(task));
        self.reveal.replace(Some(pending));
        self.notice("Opening containing folder…");
        self.mark_manual();
        self.render();
    }
}
#[cfg(test)]
pub(super) mod tests;
