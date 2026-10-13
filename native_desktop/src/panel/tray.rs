//! Exported menu is a projection; existing GTK actions remain the execution owner.
use super::*;
use crate::tray::menu::{Action, Handle, Projection};

impl Panel {
    pub fn bind_tray_menu(self: &Rc<Self>, handle: Handle) {
        self.tray_menu.replace(Some(handle));
        self.sync_tray_menu();
    }
    fn tray_projection(&self) -> Projection {
        let model = self.model.borrow();
        let safe = !model.closing
            && !model.preview
            && ((!self.layer && !model.safety_active) || model.safety.manual_allowed());
        let mut enabled = 0;
        for id in (1..=11).chain([16]) {
            let action = Action::from_id(id).expect("fixed tray action");
            let cleanup = matches!(action, Action::Hide | Action::Quit);
            let allowed = !model.closing
                && (cleanup || safe)
                && action.name().is_none_or(|name| {
                    self.actions
                        .lookup_action(name)
                        .is_some_and(|action| action.is_enabled())
                });
            if allowed {
                enabled |= action.bit();
            }
        }
        Projection {
            enabled,
            full: model.mode == Mode::Full,
        }
    }
    pub(super) fn sync_tray_menu(&self) {
        if let Some(handle) = self.tray_menu.borrow().as_ref() {
            handle.publish(self.tray_projection());
        }
    }
    pub fn tray_action(self: &Rc<Self>, action: Action) -> bool {
        if self.tray_projection().enabled & action.bit() == 0 {
            return false;
        }
        match action {
            Action::Open => {
                self.mark_manual();
                self.model.borrow_mut().manual_requested = true;
                self.present_surface();
            }
            Action::Compact | Action::Full => {
                self.set_mode(if action == Action::Full {
                    Mode::Full
                } else {
                    Mode::Compact
                });
                self.model.borrow_mut().manual_requested = true;
                self.present_surface();
            }
            Action::Files
            | Action::Folder
            | Action::Text
            | Action::Incoming
            | Action::Transfers => {
                self.mark_manual();
                self.model.borrow_mut().manual_requested = true;
                if self.present_surface() {
                    self.actions
                        .activate_action(action.name().expect("named tray action"), None);
                }
            }
            _ => self
                .actions
                .activate_action(action.name().expect("named tray action"), None),
        }
        self.sync_tray_menu();
        self.reconnect_requested() || self.model.borrow().closing
    }
    pub fn tray_context(self: &Rc<Self>, point: (i32, i32)) -> bool {
        if self.tray_projection().enabled & Action::Open.bit() == 0 {
            return false;
        }
        if !self.show_at(point) {
            return false;
        }
        self.context
            .set_pointing_to(Some(&gtk::gdk::Rectangle::new(0, 0, 1, 1)));
        self.context.popup();
        true
    }
}

#[cfg(test)]
pub(super) fn verify_tray_projection(panel: &Rc<Panel>) {
    // Existing GTK smoke thread; no broker/menu side effects are mocked.
    assert_eq!(panel.tray_projection().enabled & Action::Open.bit(), 0); // Preview inert.
    panel.model.borrow_mut().preview = false;
    let previous_state = panel.model.borrow().state.clone();
    panel.model.borrow_mut().state.view =
        Some(Arc::new(rqs_lib::service::client::state::StateView {
            sources: vec![],
            epoch: 1,
            revision: 1,
            lifecycle: Lifecycle::Running,
            synchronized: true,
            resources: rqs_lib::service::ResourceUsage::default(),
            endpoints: vec![],
            selections: vec![],
            transfers: vec![],
            warnings: vec![],
            notices: vec![],
        }));
    panel.render();
    let full = panel.tray_projection();
    assert_ne!(full.enabled & Action::Settings.bit(), 0);
    assert_ne!(full.enabled & Action::Text.bit(), 0);
    assert_ne!(full.enabled & Action::Files.bit(), 0); // Presentation projection; no service handle exists.
    panel.tray_action(Action::Compact);
    assert_eq!(panel.mode(), Mode::Compact);
    panel.tray_action(Action::Full);
    assert_eq!(panel.mode(), Mode::Full);
    panel.tray_action(Action::Settings);
    assert!(panel.settings.borrow().as_ref().unwrap().is_visible());
    panel.safety_update(crate::desktop_guard::SafetyState {
        locked: Some(true),
        inhibited: Some(false),
        closed: false,
    });
    let locked = panel.tray_projection();
    assert_eq!(locked.enabled, Action::Hide.bit() | Action::Quit.bit());
    panel.tray_action(Action::Compact);
    assert_eq!(panel.mode(), Mode::Full);
    assert!(!panel.tray_context((1, 1)));
    assert!(!panel.settings.borrow().as_ref().unwrap().is_visible());
    panel.safety_update(crate::desktop_guard::SafetyState {
        locked: Some(false),
        inhibited: Some(false),
        closed: false,
    });
    assert_ne!(panel.tray_projection().enabled & Action::Text.bit(), 0);
    assert_ne!(panel.tray_projection().enabled & Action::Files.bit(), 0);
    assert!(panel.tray_context((1, 1)));
    panel.context.popdown();
    {
        let mut model = panel.model.borrow_mut();
        model.preview = true;
        model.safety_active = false;
        model.state = previous_state;
    }
    panel.render();
}
