//! Current-run completion transitions, never a replay of persisted history.
use super::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Default)]
pub(super) struct Results {
    epoch: Option<u128>,
    phases: BTreeMap<String, Phase>,
    visible: BTreeSet<String>,
    pending: VecDeque<String>,
}
impl Results {
    pub(super) fn pending_empty(&self) -> bool {
        self.pending.is_empty()
    }
    pub(super) fn observe(
        &mut self,
        epoch: u128,
        rows: impl Iterator<Item = (String, Direction, Phase)>,
    ) {
        let baseline = self.epoch != Some(epoch);
        if baseline {
            *self = Self {
                epoch: Some(epoch),
                ..Default::default()
            };
        }
        let rows = rows.collect::<Vec<_>>();
        let present = rows
            .iter()
            .map(|(id, _, _)| id.clone())
            .collect::<BTreeSet<_>>();
        self.phases.retain(|id, _| present.contains(id));
        self.visible.retain(|id| present.contains(id));
        self.pending.retain(|id| present.contains(id));
        for (id, direction, phase) in rows {
            let previous = self.phases.insert(id.clone(), phase);
            if !baseline
                && direction == Direction::Inbound
                && phase == Phase::Finished
                && previous.is_some_and(|old| !old.is_terminal())
            {
                self.visible.insert(id.clone());
                self.pending.push_back(id);
            }
        }
    }
    pub(super) fn has_visible(&self) -> bool {
        !self.visible.is_empty()
    }
    pub(super) fn visible(&self, id: &str) -> bool {
        self.visible.contains(id)
    }
    pub(super) fn hide_rows(&mut self) {
        self.visible.clear();
    }
    pub(super) fn dismiss(&mut self) {
        self.visible.clear();
        self.pending.clear();
    }
}
impl Panel {
    pub(super) fn schedule_results(self: &Rc<Self>) {
        if self.results.borrow().pending.is_empty() || self.result_idle.borrow().is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        let source = glib::idle_add_local_once(move || {
            if let Some(panel) = weak.upgrade() {
                panel.result_idle.borrow_mut().take();
                panel.present_next_result();
            }
        });
        self.result_idle.replace(Some(source));
    }
    fn present_next_result(self: &Rc<Self>) {
        let row = {
            let model = self.model.borrow();
            if model.preview
                || model.closing
                || model.state.disconnected
                || (model.safety_active && !model.safety.manual_allowed())
                || model.waiting
                || model.input_active
                || model.state.busy
                || model.state.pending
                || model.state.text_pending
                || model.state.settings_pending
                || model.dialog.is_some()
                || *self.ingress.gate.subscribe().borrow()
                || self.reveal.borrow().is_some()
                || self
                    .text_review
                    .borrow()
                    .as_ref()
                    .is_some_and(|r| r.visible())
                || self.composer.borrow().as_ref().is_some_and(|r| r.visible())
                || self.image.borrow().as_ref().is_some_and(|r| r.visible())
            {
                return;
            }
            let Some(view) = &model.state.view else {
                return;
            };
            if !view.synchronized || view.lifecycle != Lifecycle::Running {
                return;
            }
            let Some(id) = self.results.borrow_mut().pending.pop_front() else {
                return;
            };
            view.transfers
                .iter()
                .find(|t| {
                    t.transfer.id() == id
                        && t.direction == Direction::Inbound
                        && t.phase == Phase::Finished
                })
                .cloned()
        };
        if let Some(row) = row {
            match row.details.kind {
                rqs_lib::sessions::details::PayloadKind::Files => {
                    if let Some(revision) = row.reveal_revision {
                        self.reveal_result(row.transfer, revision);
                    } else {
                        self.notice("Completed. The saved result is unavailable for automatic folder selection; review Transfers.");
                    }
                }
                _ => {
                    if let (Some(revision), Some(handle)) = (row.content_revision, &self.handle) {
                        self.submit(|| handle.read_text(row.transfer, revision));
                    } else {
                        self.notice("Completed. Text is no longer available; review Transfers.");
                    }
                }
            }
        }
        self.schedule_results();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observe(results: &mut Results, epoch: u128, rows: &[(&str, Direction, Phase)]) {
        results.observe(
            epoch,
            rows.iter()
                .map(|(id, direction, phase)| ((*id).into(), *direction, *phase)),
        );
    }
    #[test]
    fn given_baseline_or_new_epoch_when_completed_records_are_read_then_old_results_never_reopen() {
        let mut results = Results::default();
        observe(
            &mut results,
            1,
            &[("old", Direction::Inbound, Phase::Finished)],
        );
        assert!(results.pending.is_empty());
        observe(
            &mut results,
            1,
            &[("old", Direction::Inbound, Phase::Finished)],
        );
        assert!(results.pending.is_empty());
        observe(
            &mut results,
            1,
            &[("new", Direction::Inbound, Phase::Receiving)],
        );
        observe(
            &mut results,
            2,
            &[("new", Direction::Inbound, Phase::Finished)],
        );
        assert!(results.pending.is_empty());
        assert!(!results.visible("new"));
    }
    #[test]
    fn given_live_transitions_when_finished_then_inbound_success_is_queued_once_in_observed_order_and_hide_preserves_active_work(
    ) {
        let mut results = Results::default();
        observe(&mut results, 1, &[]);
        for (id, direction, end) in [
            ("a", Direction::Inbound, Phase::Finished),
            ("b", Direction::Inbound, Phase::Failed),
            ("c", Direction::Outbound, Phase::Finished),
            ("d", Direction::Inbound, Phase::Cancelled),
            ("e", Direction::Inbound, Phase::Rejected),
            ("f", Direction::Inbound, Phase::Disconnected),
        ] {
            observe(&mut results, 1, &[(id, direction, Phase::Receiving)]);
            observe(&mut results, 1, &[(id, direction, end)]);
            assert_eq!(results.pending.len(), usize::from(id == "a"));
            observe(&mut results, 1, &[(id, direction, end)]);
            assert_eq!(results.pending.len(), usize::from(id == "a"));
        }
        observe(
            &mut results,
            1,
            &[
                ("x", Direction::Inbound, Phase::Receiving),
                ("y", Direction::Inbound, Phase::AwaitingConsent),
            ],
        );
        results.dismiss();
        observe(
            &mut results,
            1,
            &[
                ("x", Direction::Inbound, Phase::Finished),
                ("y", Direction::Inbound, Phase::AwaitingConsent),
            ],
        );
        assert_eq!(
            results
                .pending
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["x"]
        );
        results.hide_rows();
        assert!(!results.visible("x"));
        observe(
            &mut results,
            1,
            &[
                ("x", Direction::Inbound, Phase::Finished),
                ("y", Direction::Inbound, Phase::Finished),
            ],
        );
        assert_eq!(
            results
                .pending
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["x", "y"]
        );
        observe(&mut results, 1, &[]);
        assert!(results.pending.is_empty());
        assert!(results.visible.is_empty());
    }
}
