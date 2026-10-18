//! Accessible card menus and the secondary appearance surface.
use super::*;

impl Panel {
    pub(super) fn configure_menu(self: &Rc<Self>, application: &gtk::Application) {
        self.window
            .insert_action_group("panel", Some(&self.actions));
        let devices = gio::SimpleAction::new_stateful("devices", None, &false.to_variant());
        let weak = Rc::downgrade(self);
        devices.connect_activate(move |_, _| {
            if let Some(panel) = weak.upgrade() {
                let drawer = panel.drawer();
                panel.set_drawer(if drawer == Drawer::Closed {
                    Drawer::Open
                } else {
                    Drawer::Closed
                });
            }
        });
        self.actions.add_action(&devices);
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
            "journal-refresh",
            "journal-clear",
            "files",
            "folder",
            "send-text",
            "send-image",
            "send-clipboard",
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
                    "transfers" => panel.show_transfers(),
                    "journal-refresh" => {
                        if let Some(handle) = &panel.handle {
                            panel.submit(|| handle.refresh_journal());
                        }
                    }
                    "journal-clear" => {
                        let token = panel
                            .model
                            .borrow()
                            .state
                            .journal
                            .as_ref()
                            .map(|v| v.token.clone());
                        if let (Some(token), Some(handle)) = (token, &panel.handle) {
                            panel.submit(|| handle.clear_journal(token));
                        }
                    }
                    "nearby" | "incoming" => {
                        panel.set_drawer(Drawer::Open);
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
                                panel.focus_transfer(&id);
                            }
                        }
                    }
                    "receive" => panel.receive(),
                    "send" => panel.show_sending(),
                    "send-text" => panel.show_composer(),
                    "send-image" => panel.show_image(),
                    "send-clipboard" => panel.capture_clipboard(None),
                    "files" | "folder" => {
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
                    if !panel.model.borrow().closing {
                        panel.render();
                    }
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
            ("transfers", "History"),
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
            ("send-clipboard", "Clipboard"),
        ] {
            sending.append(Some(title), Some(&format!("panel.{name}")));
        }
        self.sending.set_menu_model(Some(&sending));
        self.sending.set_parent(&self.button("quick-send"));
        self.sending.set_position(gtk::PositionType::Top);
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
        if temporary_receiving(&self.model.borrow()) {
            self.actions
                .activate_action("visibility", Some(&"hidden".to_variant()));
            self.render();
        } else if let Some(id) = pending {
            self.set_drawer(Drawer::Open);
            self.pages.set_visible_child_name("nearby");
            self.render();
            self.focus_transfer(&id);
        } else {
            self.actions
                .activate_action("visibility", Some(&"temporary".to_variant()));
            self.render();
        }
        if !self.window.is_visible() {
            self.present_surface();
        }
    }
    pub(super) fn show_sending(self: &Rc<Self>) {
        // Choose content first; a later device-row Send is the sole admission action.
        if self.sending.is_visible() {
            self.sending.popdown();
            return;
        }
        self.render();
        if !self.window.is_visible() {
            self.present_surface();
        }
        if self
            .model
            .borrow()
            .state
            .view
            .as_ref()
            .is_some_and(|view| view.selections.is_empty())
        {
            self.sending.popup();
        }
    }
    pub(super) fn send_to(self: &Rc<Self>, endpoint: &str) {
        let selection = {
            let model = self.model.borrow();
            if !controls_ready(
                &model.state,
                model.waiting
                    || model.input_active
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
                self.submit(|| handle.queue_recipient(selection, endpoint.to_owned()));
            }
        } else {
            let mut model = self.model.borrow_mut();
            model.strip_target = if model.strip_target.as_deref() == Some(endpoint) {
                None
            } else {
                Some(endpoint.to_owned())
            };
            drop(model);
            self.render();
        }
    }
    pub fn set_drawer(self: &Rc<Self>, drawer: Drawer) {
        self.mark_manual();
        if self.model.borrow().closing || self.drawer() == drawer {
            return;
        }
        self.pin_surface_bottom();
        self.model.borrow_mut().strip_target = None;
        self.model.borrow_mut().drawer = drawer;
        if drawer == Drawer::Closed {
            self.results.borrow_mut().hide_rows();
        }
        self.actions
            .lookup_action("devices")
            .and_downcast::<gio::SimpleAction>()
            .unwrap()
            .set_state(&(drawer == Drawer::Open).to_variant());
        self.render();
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
            .comments("Independent fork of Martichou/rquickshare. Send and receive nearby files and text")
            .logo_icon_name(crate::tray::ICON_NAME)
            .license_type(gtk::License::Gpl30Only)
            .website("https://github.com/morozhenko-apps/rquickshare")
            .website_label("Project website")
            .build();
        about.set_hide_on_close(true);
        self.watch_idle_widget(&about);
        self.about.replace(Some(about.clone()));
        about.present();
    }
}
