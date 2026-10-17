//! Recipient choices never own source authority or start work until explicit Send.
use super::*;
use rqs_lib::service::client::{controller::ActionResult, OperationValue, Outcome};

fn choice_ready(model: &Model, gate_busy: bool) -> bool {
    !model.preview
        && controls_ready(
            &model.state,
            gate_busy
                || model.waiting
                || model.closing
                || model.dialog.is_some()
                || model.ingress.busy
                || model.desktop.staging
                || (model.safety_active && !model.safety.manual_allowed()),
        )
}
fn send_ready(model: &Model, gate_busy: bool, has_source: bool) -> bool {
    choice_ready(model, gate_busy)
        && has_source
        && !model.recipients.is_empty()
        && model.recipients.len() <= rqs_lib::service::batch::RECIPIENTS
        && model.state.view.as_ref().is_some_and(|view| {
            model
                .recipients
                .keys()
                .all(|id| view.endpoints.iter().any(|peer| &peer.id == id))
        })
}
pub(super) fn confirmed(result: &Result<ActionResult, ControllerError>) -> bool {
    matches!(result, Ok(ActionResult::Transfers(_)))
        || matches!(result, Ok(ActionResult::Recovered(Some(Outcome::Operation(receipt))))
            if matches!(receipt.result, OperationValue::Transfers(_)))
}
impl Panel {
    pub(super) fn choose_recipient(self: &Rc<Self>, id: &str, active: bool) {
        {
            let model = self.model.borrow();
            // Keyed-row synchronization can emit toggled during render under a model borrow.
            if model.recipients.contains_key(id) == active {
                return;
            }
        }
        {
            let mut model = self.model.borrow_mut();
            if choice_ready(&model, *self.ingress.gate.subscribe().borrow()) {
                if !active {
                    model.recipients.remove(id);
                } else if model.recipients.len() < rqs_lib::service::batch::RECIPIENTS {
                    let name = model.state.view.as_ref().and_then(|view| {
                        view.endpoints
                            .iter()
                            .find(|peer| peer.id == id)
                            .map(|peer| peer.name.clone())
                    });
                    if let Some(name) = name {
                        model.recipients.insert(id.to_owned(), name);
                    }
                }
            }
        }
        // Also restores a stale/programmatic toggle rejected by a safety/capacity check.
        self.render();
    }
    pub(super) fn render_recipients(&self, model: &Model, has_source: bool) {
        let unavailable = model
            .recipients
            .keys()
            .filter(|id| {
                !model
                    .state
                    .view
                    .as_ref()
                    .is_some_and(|view| view.endpoints.iter().any(|peer| &peer.id == *id))
            })
            .count();
        self.box_widget("recipients").set_visible(
            model.drawer == Drawer::Open
                && (has_source || !model.recipients.is_empty())
                && self.pages.visible_child_name().as_deref() == Some("nearby"),
        );
        self.text(
            "recipient-summary",
            &if model.recipients.is_empty() {
                "Choose up to 10 recipients".into()
            } else if unavailable != 0 {
                format!(
                    "{} selected · {unavailable} unavailable",
                    model.recipients.len()
                )
            } else {
                format!("{} selected", model.recipients.len())
            },
        );
        self.button("send-many")
            .set_label(&if model.recipients.is_empty() {
                "Send".into()
            } else {
                format!("Send to {}", model.recipients.len())
            });
        self.actions
            .lookup_action("send-many")
            .and_downcast::<gio::SimpleAction>()
            .unwrap()
            .set_enabled(send_ready(
                model,
                *self.ingress.gate.subscribe().borrow(),
                has_source,
            ));
    }
    pub(super) fn send_recipients(self: &Rc<Self>) {
        let (selection, endpoints) = {
            let model = self.model.borrow();
            let selection = model
                .state
                .view
                .as_ref()
                .and_then(|view| view.selections.first())
                .cloned();
            if !send_ready(
                &model,
                *self.ingress.gate.subscribe().borrow(),
                selection.is_some(),
            ) {
                return;
            }
            (
                selection.unwrap(),
                model.recipients.keys().cloned().collect(),
            )
        };
        if let Some(handle) = &self.handle {
            self.submit(|| handle.send_many(selection, endpoints));
        }
    }
}

#[cfg(test)]
pub(super) mod tests;
