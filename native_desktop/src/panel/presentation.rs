//! Presentation effects have no authority to answer consent or start a transfer.
use super::*;
use crate::card_policy::{Effect, Row, Surface};

impl Panel {
    pub(super) fn mark_manual(&self) {
        self.policy.borrow_mut().manual();
        let mut model = self.model.borrow_mut();
        model.automatic = false;
        model.automatic_focus = None;
    }
    pub(super) fn hide(&self) {
        self.policy.borrow_mut().dismiss();
        let mut model = self.model.borrow_mut();
        model.automatic = false;
        model.automatic_focus = None;
        model.manual_requested = false;
        self.context.popdown();
        self.window.set_visible(false);
    }
    pub fn safety_update(self: &Rc<Self>, safety: crate::desktop_guard::SafetyState) {
        {
            let mut model = self.model.borrow_mut();
            model.safety = safety;
            model.safety_active = true;
        }
        self.apply_presentation();
    }
    pub(super) fn apply_presentation(self: &Rc<Self>) {
        let (state, surface, requested, protected) = {
            let model = self.model.borrow();
            if model.closing || (self.handle.is_none() && !model.safety_active) {
                return;
            }
            (
                model.state.clone(),
                Surface {
                    visible: self.window.is_visible()
                        || self
                            .settings
                            .borrow()
                            .as_ref()
                            .is_some_and(|settings| settings.is_visible()),
                    automatic: model.automatic,
                    busy: model.dialog.is_some()
                        || model.waiting
                        || model.ingress.busy
                        || model.state.pending
                        || *self.ingress.gate.subscribe().borrow(),
                    healthy: !model.state.disconnected
                        && model.state.view.as_ref().is_some_and(|view| {
                            view.synchronized && view.lifecycle == Lifecycle::Running
                        }),
                    safety: model.safety,
                },
                model.manual_requested,
                model.safety_active,
            )
        };
        if protected && !surface.safety.manual_allowed() {
            self.context.popdown();
            self.window.set_visible(false);
            if let Some(settings) = self.settings.borrow().as_ref() {
                settings.set_visible(false);
            }
        } else if requested && !surface.visible {
            self.present_surface();
            return;
        }
        let Some(view) = state.view else { return };
        let rows = view
            .transfers
            .iter()
            .map(|transfer| Row {
                id: transfer.transfer.id(),
                direction: transfer.direction,
                phase: transfer.phase,
                round: transfer.consent_revision,
            })
            .collect::<Vec<_>>();
        let effect = self
            .policy
            .borrow_mut()
            .reconcile(view.epoch, &rows, surface);
        match effect {
            Effect::None => (),
            Effect::Hide => {
                self.context.popdown();
                self.window.set_visible(false);
                let mut model = self.model.borrow_mut();
                model.automatic = false;
                model.automatic_focus = None;
            }
            Effect::Show(notice) => {
                let full = {
                    let mut model = self.model.borrow_mut();
                    model.automatic = true;
                    model.automatic_focus = Some(notice.id);
                    model.mode == Mode::Full
                };
                if full {
                    self.pages
                        .set_visible_child_name(if notice.phase == Phase::AwaitingConsent {
                            "incoming"
                        } else {
                            "transfers"
                        });
                }
                self.render();
                if !self.present_surface() {
                    self.notice("Cannot place the card on an available monitor. Open the tray to review requests.");
                }
            }
        }
    }
}
