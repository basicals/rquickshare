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
        for id in [1, 2, 4, 5, 8, 11, 18, 19, 20, 22, 23] {
            let action = Action::from_id(id).expect("fixed tray action");
            let cleanup = action == Action::Quit
                || (action == Action::Open && !model.preview && self.window.is_visible());
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
            card_visible: self.window.is_visible(),
            incoming: model
                .state
                .view
                .as_ref()
                .map(|view| {
                    view.transfers
                        .iter()
                        .filter(|transfer| {
                            transfer.direction == Direction::Inbound
                                && transfer.phase == Phase::AwaitingConsent
                        })
                        .count() as u16
                })
                .unwrap_or(0),
            active: model
                .state
                .view
                .as_ref()
                .map(|view| {
                    view.transfers
                        .iter()
                        .filter(|transfer| {
                            !transfer.phase.is_terminal()
                                && transfer.phase != Phase::AwaitingConsent
                        })
                        .count() as u16
                })
                .unwrap_or(0),
            enabled,
            full: model.drawer == Drawer::Open,
            visibility: model
                .state
                .visibility
                .as_ref()
                .filter(|view| view.available && view.running && !model.state.disconnected)
                .map(|view| view.mode),
        }
    }
    pub(super) fn sync_tray_menu(&self) {
        if let Some(handle) = self.tray_menu.borrow().as_ref() {
            handle.publish(self.tray_projection());
        }
    }
    pub fn activate_at(self: &Rc<Self>, point: (i32, i32)) -> bool {
        if self.model.borrow().closing {
            return false;
        }
        if self.window.is_visible() {
            self.hide();
            true
        } else {
            self.show_at(point)
        }
    }
    pub fn tray_action(self: &Rc<Self>, action: Action) -> bool {
        if self.tray_projection().enabled & action.bit() == 0 {
            return false;
        }
        match action {
            Action::Open if self.window.is_visible() => self.hide(),
            Action::Open => {
                self.mark_manual();
                self.pages.set_visible_child_name("nearby");
                self.model.borrow_mut().manual_requested = true;
                self.present_surface();
            }
            Action::Compact | Action::Full => {
                self.set_drawer(if action == Action::Full {
                    Drawer::Open
                } else {
                    Drawer::Closed
                });
                self.model.borrow_mut().manual_requested = true;
                self.present_surface();
            }
            Action::Visible | Action::Hidden | Action::Temporary => {
                let mode = match action {
                    Action::Visible => "visible",
                    Action::Hidden => "hidden",
                    _ => "temporary",
                };
                self.actions
                    .activate_action("visibility", Some(&mode.to_variant()));
            }
            Action::Receive
            | Action::Files
            | Action::Folder
            | Action::Text
            | Action::Image
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
    panel.preview_image.cancel(); // End the old asynchronous incoming-preview fixture before live projection checks.
    panel.model.borrow_mut().preview = false;
    let previous_preview_page = panel.model.borrow_mut().preview_page.take();
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
    assert_eq!(full.enabled & Action::Text.bit(), 0);
    assert_eq!(full.enabled & Action::Image.bit(), 0);
    assert_ne!(full.enabled & Action::Files.bit(), 0); // Presentation projection; no service handle exists.
    let settings = rqs_lib::service::client::settings::SettingsView {
        epoch: format!("{:032x}", 1),
        available: true,
        source: 2,
        configured: true,
        revision: 1,
        token: "a".repeat(64),
        visibility: 2,
        port: -1,
        card_mode: 1,
        directory_display: String::new(),
        visibility_recovered: false,
        applied_live_revision: 1,
        running: true,
        restart_required: false,
        error: 0,
        last_request: String::new(),
        device_name: String::new(),
        resolved_name: String::new(),
        name_status: 0,
    };
    panel.model.borrow_mut().state.settings = Some(settings);
    panel.model.borrow_mut().state.visibility_sampled_at = Some(std::time::Instant::now());
    for (mode, target) in [(0, "visible"), (1, "hidden"), (2, "temporary")] {
        panel.model.borrow_mut().state.visibility =
            Some(rqs_lib::service::client::visibility::VisibilityView {
                epoch: format!("{:032x}", 1),
                available: true,
                revision: 1,
                running: true,
                mode,
                remaining_ms: if mode == 2 { 60_000 } else { 0 },
            });
        panel.render();
        assert_eq!(panel.tray_projection().visibility, Some(mode));
        assert_ne!(panel.tray_projection().enabled & Action::Temporary.bit(), 0);
        let state = panel
            .actions
            .lookup_action("visibility")
            .unwrap()
            .state()
            .unwrap();
        assert_eq!(state.str(), Some(target));
        assert!(panel.builder.object::<gtk::Label>("visibility").is_none());
        assert_eq!(
            panel
                .button("quick-receive")
                .has_css_class("receive-active"),
            mode == 2
        );
    }
    // Confirmed visibility drives feedback, not click intent or unrelated busy controls.
    let confirmed = panel.model.borrow().state.clone();
    for case in 0..11 {
        {
            let mut model = panel.model.borrow_mut();
            model.state = confirmed.clone();
            match case {
                0 => model.state.busy = true,
                1 => model.state.visibility_pending = true,
                2 => model.state.settings_pending = true,
                3 => model.state.disconnected = true,
                4 => model.state.visibility.as_mut().unwrap().available = false,
                5 => model.state.visibility.as_mut().unwrap().running = false,
                6 => Arc::make_mut(model.state.view.as_mut().unwrap()).synchronized = false,
                7 => {
                    Arc::make_mut(model.state.view.as_mut().unwrap()).lifecycle = Lifecycle::Stopped
                }
                8 => model.state.view = None,
                9 => model.preview = true,
                10 => model.state.visibility = None,
                _ => unreachable!(),
            }
        }
        panel.render();
        assert_eq!(
            panel
                .button("quick-receive")
                .has_css_class("receive-active"),
            case < 3,
            "case {case}"
        );
    }
    {
        let mut model = panel.model.borrow_mut();
        model.state = confirmed;
        model.preview = false;
    }
    super::countdown::verify(panel);
    panel.set_drawer(Drawer::Open);
    panel.render();
    assert!(!panel.box_widget("source").get_visible());
    let scroll = panel
        .builder
        .object::<gtk::ScrolledWindow>("nearby-scroll")
        .unwrap();
    let (minimum, natural, _, _) = scroll.measure(gtk::Orientation::Vertical, 390);
    let content = panel
        .box_widget("nearby")
        .measure(gtk::Orientation::Vertical, 390)
        .1;
    assert_eq!(natural, content.max(minimum)); // GTK's intrinsic frame/scrollbar minimum still applies.
    assert!(
        natural < 120,
        "an empty list must not reserve the old120px floor"
    );
    super::tests::capture_card(panel, "empty-expanded");
    panel.set_drawer(Drawer::Closed);
    assert!(panel
        .button("quick-receive")
        .tooltip_text()
        .unwrap()
        .contains("press again to stop"));
    super::tests::capture_card(panel, "temporary-collapsed");
    for case in 0..4 {
        {
            let mut model = panel.model.borrow_mut();
            model.state.visibility_pending = case == 0;
            model.state.settings_pending = case == 1;
            model.state.disconnected = case == 2;
            model.state.visibility.as_mut().unwrap().running = case != 3;
        }
        panel.render();
        assert_eq!(panel.tray_projection().enabled & Action::Temporary.bit(), 0);
    }
    {
        let mut model = panel.model.borrow_mut();
        model.state.disconnected = false;
        model.state.settings_pending = false;
        model.state.visibility_pending = false;
        model.state.visibility.as_mut().unwrap().running = true;
    }
    panel.render();
    // Arrange: observe the real visibility action without a connected service writer.
    let visibility = panel
        .actions
        .lookup_action("visibility")
        .and_downcast::<gio::SimpleAction>()
        .unwrap();
    let targets = Rc::new(RefCell::new(Vec::new()));
    let observed = targets.clone();
    let observer = visibility.connect_activate(move |_, value| {
        observed
            .borrow_mut()
            .push(value.unwrap().str().unwrap().to_owned());
    });
    panel.actions.activate_action("receive", None);
    assert_eq!(&*targets.borrow(), &["hidden"]);
    assert!(panel
        .button("quick-receive")
        .has_css_class("receive-active")); // No optimistic success before acknowledgement.
    panel
        .model
        .borrow_mut()
        .state
        .visibility
        .as_mut()
        .unwrap()
        .mode = 1;
    panel.render();
    assert!(!panel
        .button("quick-receive")
        .has_css_class("receive-active"));
    for mode in [Drawer::Closed, Drawer::Open] {
        for shown in [false, true] {
            panel.set_drawer(mode);
            panel.window.set_visible(shown);
            let page = panel.pages.visible_child_name();
            targets.borrow_mut().clear();
            // Act: the same action bound to Compact Receive, repeated without a pending request.
            for _ in 0..2 {
                panel.actions.activate_action("receive", None);
                // Assert: request temporary visibility, preserving the chosen view and page.
                assert_eq!(panel.drawer(), mode);
                assert_eq!(panel.pages.visible_child_name(), page);
                assert!(panel.window.is_visible());
            }
            assert_eq!(&*targets.borrow(), &["temporary", "temporary"]);
        }
    }
    visibility.disconnect(observer);
    panel.set_drawer(Drawer::Closed);
    let current = panel.model.borrow().state.clone(); // The legacy saved card_mode is1.
    panel.update(current);
    assert_eq!(panel.drawer(), Drawer::Closed); // An acknowledgement cannot resurrect Full or reopen the drawer.
    panel.actions.activate_action("send", None);
    assert_eq!(panel.drawer(), Drawer::Open);
    assert_eq!(
        panel.sending.parent(),
        Some(panel.button("quick-send").upcast())
    );
    panel.sending.popdown();
    panel.set_drawer(Drawer::Open);
    assert!(!panel.tray_action(Action::Compact));
    assert_eq!(panel.drawer(), Drawer::Open);
    panel.tray_action(Action::Settings);
    assert!(panel.settings.borrow().as_ref().unwrap().is_visible());
    assert_ne!(panel.tray_projection().enabled & Action::About.bit(), 0);
    panel.tray_action(Action::About);
    assert!(panel.about.borrow().as_ref().unwrap().is_visible());
    panel.safety_update(crate::desktop_guard::SafetyState {
        locked: Some(true),
        inhibited: Some(false),
        closed: false,
    });
    let locked = panel.tray_projection();
    assert_eq!(locked.enabled, Action::Quit.bit());
    panel.tray_action(Action::Compact);
    assert_eq!(panel.drawer(), Drawer::Open);
    assert!(!panel.tray_context((1, 1)));
    assert!(!panel.settings.borrow().as_ref().unwrap().is_visible());
    assert!(!panel.about.borrow().as_ref().unwrap().is_visible());
    panel.tray_action(Action::About);
    assert!(!panel.about.borrow().as_ref().unwrap().is_visible());
    panel.safety_update(crate::desktop_guard::SafetyState {
        locked: Some(false),
        inhibited: Some(false),
        closed: false,
    });
    assert_eq!(panel.tray_projection().enabled & Action::Text.bit(), 0);
    assert_ne!(panel.tray_projection().enabled & Action::Files.bit(), 0);
    assert!(panel.tray_context((1, 1)));
    panel.context.popdown();
    {
        let mut model = panel.model.borrow_mut();
        model.preview = true;
        model.preview_page = previous_preview_page;
        model.safety_active = false;
        model.state = previous_state;
    }
    panel.render();
}
