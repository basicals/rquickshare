//! Accessible card menus and the secondary appearance surface.
use super::*;

impl Panel {
    pub(super) fn configure_menu(self: &Rc<Self>, application: &gtk::Application) {
        self.window
            .insert_action_group("panel", Some(&self.actions));
        let mode = gio::SimpleAction::new_stateful(
            "mode",
            Some(glib::VariantTy::STRING),
            &"compact".to_variant(),
        );
        let weak = Rc::downgrade(self);
        mode.connect_activate(move |_, value| {
            if let (Some(panel), Some(mode)) = (
                weak.upgrade(),
                value.and_then(glib::Variant::str).and_then(Mode::parse),
            ) {
                panel.set_mode(mode);
            }
        });
        self.actions.add_action(&mode);
        let visibility = gio::SimpleAction::new_stateful(
            "visibility",
            Some(glib::VariantTy::STRING),
            &"unavailable".to_variant(),
        );
        let weak = Rc::downgrade(self);
        visibility.connect_activate(move |_, value| {
            let mode = match value.and_then(glib::Variant::str) {
                Some("visible") => 0,
                Some("hidden") => 1,
                Some("temporary") => 2,
                _ => return,
            };
            if let Some(panel) = weak.upgrade() {
                if let Some(handle) = panel.handle.as_ref() {
                    panel.submit(|| handle.set_visibility(mode));
                }
            }
        });
        self.actions.add_action(&visibility);
        for name in [
            "receive",
            "send",
            "nearby",
            "incoming",
            "transfers",
            "files",
            "folder",
            "send-text",
            "send-image",
            "send-many",
            "discard",
            "recover",
            "abort",
            "settings",
            "about",
            "visibility-retry",
            "reconnect",
            "hide",
            "quit",
        ] {
            let action = gio::SimpleAction::new(name, None);
            let weak = Rc::downgrade(self);
            let application = application.clone();
            action.connect_activate(move |_, _| {
                let Some(panel) = weak.upgrade() else { return };
                if panel.model.borrow().closing {
                    return;
                }
                match name {
                    "nearby" | "incoming" | "transfers" => {
                        panel.set_mode(Mode::Full);
                        panel.pages.set_visible_child_name("nearby");
                        panel.render();
                        let ids = panel
                            .model
                            .borrow()
                            .state
                            .view
                            .as_ref()
                            .map(|view| {
                                view.transfers
                                    .iter()
                                    .filter(|transfer| {
                                        if name == "incoming" {
                                            transfer.direction == Direction::Inbound
                                                && transfer.phase == Phase::AwaitingConsent
                                        } else {
                                            !transfer.phase.is_terminal()
                                        }
                                    })
                                    .map(|transfer| transfer.transfer.id().to_owned())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        if name != "nearby" {
                            for id in ids {
                                panel.expand_transfer(&id);
                            }
                        }
                    }
                    "receive" => panel.receive(),
                    "send" => panel.show_sending(),
                    "send-text" => panel.show_composer(),
                    "send-image" => panel.show_image(),
                    "send-many" => panel.send_recipients(),
                    "files" | "folder" => {
                        panel.set_mode(Mode::Full);
                        panel.pages.set_visible_child_name("nearby");
                        panel.choose(name == "folder");
                    }
                    "discard" | "recover" | "abort" => panel.button(name).emit_clicked(),
                    "settings" => panel.show_settings(),
                    "about" => panel.show_about(),
                    "visibility-retry" => {
                        if let Some(handle) = panel.handle.as_ref() {
                            panel.submit(|| handle.retry_visibility());
                        }
                    }
                    "reconnect" => {
                        panel.reconnect.set(true);
                        application.quit();
                    }
                    "hide" => panel.hide(),
                    "quit" => {
                        panel.close();
                        application.quit();
                    }
                    _ => unreachable!(),
                }
            });
            self.actions.add_action(&action);
        }
        let weak = Rc::downgrade(self);
        self.window.connect_visible_notify(move |_| {
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(panel) = weak.upgrade() {
                    panel.sync_tray_menu();
                }
            });
        });
        let menu = gio::Menu::new();
        let visibility = gio::Menu::new();
        for (title, target) in [
            ("Visible", "visible"),
            ("Hidden", "hidden"),
            ("Visible for one minute", "temporary"),
        ] {
            visibility.append(Some(title), Some(&format!("panel.visibility::{target}")));
        }
        let navigation = gio::Menu::new();
        for (name, title) in [
            ("receive", "Receive files"),
            ("send", "Send files…"),
            ("incoming", "Incoming requests"),
            ("transfers", "Active transfers"),
        ] {
            navigation.append(Some(title), Some(&format!("panel.{name}")));
        }
        menu.append_section(None, &navigation);
        menu.append_section(Some("Discovery visibility"), &visibility);
        let secondary = gio::Menu::new();
        for (name, title) in [
            ("settings", "Settings…"),
            ("about", "About rQuickShare"),
            ("hide", "Hide card"),
            ("quit", "Quit panel"),
        ] {
            secondary.append(Some(title), Some(&format!("panel.{name}")));
        }
        menu.append_section(None, &secondary);
        let sending = gio::Menu::new();
        for (name, title) in [
            ("files", "Files…"),
            ("folder", "Folder…"),
            ("send-text", "Text or link…"),
            ("send-image", "Clipboard image…"),
        ] {
            sending.append(Some(title), Some(&format!("panel.{name}")));
        }
        self.sending.set_menu_model(Some(&sending));
        self.sending.set_parent(&self.box_widget("root"));
        self.configure_movement();
        self.context.set_menu_model(Some(&menu));
        self.context.set_parent(&self.box_widget("root"));
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        let weak = Rc::downgrade(self);
        gesture.connect_pressed(move |gesture, _, x, y| {
            if let Some(panel) = weak.upgrade() {
                if panel.model.borrow().closing {
                    return;
                }
                panel
                    .context
                    .set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                panel.context.popup();
                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        });
        self.box_widget("root").add_controller(gesture);
        let weak = Rc::downgrade(self);
        self.button("mode").connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                let mode = panel.model.borrow().mode;
                panel.set_mode(if mode == Mode::Compact {
                    Mode::Full
                } else {
                    Mode::Compact
                });
            }
        });
    }
    pub(super) fn receive(self: &Rc<Self>) {
        let pending = self
            .model
            .borrow()
            .state
            .view
            .as_ref()
            .and_then(|view| {
                view.transfers.iter().find(|transfer| {
                    transfer.direction == Direction::Inbound
                        && transfer.phase == Phase::AwaitingConsent
                })
            })
            .map(|transfer| transfer.transfer.id().to_owned());
        self.set_mode(Mode::Full);
        self.pages.set_visible_child_name("nearby");
        if pending.is_none() {
            self.actions
                .activate_action("visibility", Some(&"temporary".to_variant()));
        }
        self.render();
        if let Some(id) = pending {
            self.expand_transfer(&id);
        }
        self.present_surface();
    }
    pub(super) fn show_sending(self: &Rc<Self>) {
        self.set_mode(Mode::Full);
        self.pages.set_visible_child_name("nearby");
        self.render();
        self.present_surface();
        if self
            .model
            .borrow()
            .state
            .view
            .as_ref()
            .is_none_or(|view| view.selections.is_empty())
        {
            self.sending
                .set_pointing_to(Some(&gtk::gdk::Rectangle::new(14, 44, 1, 1)));
            self.sending.popup();
        }
    }
    pub(super) fn send_to(self: &Rc<Self>, endpoint: &str) {
        let selection = {
            let model = self.model.borrow();
            if !controls_ready(
                &model.state,
                model.waiting
                    || model.closing
                    || model.ingress.busy
                    || model.dialog.is_some()
                    || *self.ingress.gate.subscribe().borrow()
                    || (model.safety_active && !model.safety.manual_allowed()),
            ) {
                return;
            }
            let Some(view) = &model.state.view else {
                return;
            };
            if !view.endpoints.iter().any(|peer| peer.id == endpoint) {
                return;
            }
            view.selections.first().cloned()
        };
        if let Some(selection) = selection {
            if let Some(handle) = &self.handle {
                self.submit(|| handle.send_many(selection, vec![endpoint.to_owned()]));
            }
        } else {
            self.model.borrow_mut().recipients.clear();
            self.choose_recipient(endpoint, true);
            self.show_sending();
        }
    }
    pub fn set_mode(self: &Rc<Self>, mode: Mode) {
        self.apply_mode(mode, true);
    }
    pub(super) fn apply_mode(self: &Rc<Self>, mode: Mode, manual: bool) {
        if manual {
            self.mark_manual();
        }
        {
            let model = self.model.borrow();
            if model.closing || model.mode == mode {
                return;
            }
        }
        self.pin_surface_bottom();
        self.model.borrow_mut().mode = mode;
        self.actions
            .lookup_action("mode")
            .and_downcast::<gio::SimpleAction>()
            .unwrap()
            .set_state(&mode.name().to_variant());
        if let Some(editor) = self.settings_owner.borrow().as_ref() {
            editor.sync_mode(mode, manual);
        } else if let Some(appearance) = self.appearance.borrow().as_ref() {
            appearance.set_selected(u32::from(mode == Mode::Full));
        }
        self.render();
        if self.layer && self.window.is_visible() && !self.model.borrow().position_pinned {
            let hint = self.model.borrow().last_hint;
            if let Some(hint) = hint {
                self.show_at(hint);
            }
        }
    }
    fn show_settings(self: &Rc<Self>) {
        {
            let model = self.model.borrow();
            if model.safety_active && !model.safety.manual_allowed() {
                return;
            }
        }
        self.mark_manual();
        if let Some(window) = self.settings.borrow().as_ref() {
            if let Some(editor) = self.settings_owner.borrow().as_ref() {
                editor.reopen();
            }
            window.present();
            return;
        }
        super::settings::Editor::open(self);
    }
    pub(super) fn show_about(self: &Rc<Self>) {
        {
            let model = self.model.borrow();
            if model.closing || (model.safety_active && !model.safety.manual_allowed()) {
                return;
            }
        }
        self.mark_manual();
        if let Some(about) = self.about.borrow().as_ref() {
            about.present();
            return;
        }
        let about = gtk::AboutDialog::builder()
            .transient_for(&self.window)
            .application(&self.window.application().unwrap())
            .program_name("rQuickShare")
            .version(env!("CARGO_PKG_VERSION"))
            .comments("Send and receive nearby files and text")
            .logo_icon_name(crate::tray::ICON_NAME)
            .license_type(gtk::License::Gpl30Only)
            .website("https://github.com/morozhenko-apps/rquickshare")
            .website_label("Project website")
            .build();
        about.set_hide_on_close(true);
        self.about.replace(Some(about.clone()));
        about.present();
    }
}
