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
        for name in [
            "nearby",
            "incoming",
            "transfers",
            "files",
            "folder",
            "discard",
            "recover",
            "abort",
            "settings",
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
                        panel.pages.set_visible_child_name(name);
                    }
                    "files" | "folder" => {
                        panel.set_mode(Mode::Full);
                        panel.pages.set_visible_child_name("nearby");
                        panel.choose(name == "folder");
                    }
                    "discard" | "recover" | "abort" => panel.button(name).emit_clicked(),
                    "settings" => panel.show_settings(),
                    "hide" => panel.window.set_visible(false),
                    "quit" => {
                        panel.close();
                        application.quit();
                    }
                    _ => unreachable!(),
                }
            });
            self.actions.add_action(&action);
        }
        let menu = gio::Menu::new();
        let modes = gio::Menu::new();
        modes.append(Some("Compact view"), Some("panel.mode::compact"));
        modes.append(Some("Full view"), Some("panel.mode::full"));
        menu.append_section(None, &modes);
        let navigation = gio::Menu::new();
        for (name, title) in [
            ("files", "Send files…"),
            ("folder", "Send a folder…"),
            ("incoming", "Incoming requests"),
            ("transfers", "Transfers"),
        ] {
            navigation.append(Some(title), Some(&format!("panel.{name}")));
        }
        menu.append_section(None, &navigation);
        let preparation = gio::Menu::new();
        for (name, title) in [
            ("discard", "Discard prepared files"),
            ("recover", "Recover operation"),
            ("abort", "Discard preparation"),
        ] {
            preparation.append(Some(title), Some(&format!("panel.{name}")));
        }
        menu.append_section(None, &preparation);
        let secondary = gio::Menu::new();
        for (name, title) in [
            ("settings", "Settings…"),
            ("hide", "Hide card"),
            ("quit", "Quit panel"),
        ] {
            secondary.append(Some(title), Some(&format!("panel.{name}")));
        }
        menu.append_section(None, &secondary);
        self.builder
            .object::<gtk::MenuButton>("menu")
            .unwrap()
            .set_menu_model(Some(&menu));
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
    pub fn set_mode(self: &Rc<Self>, mode: Mode) {
        {
            let mut model = self.model.borrow_mut();
            if model.closing || model.mode == mode {
                return;
            }
            model.mode = mode;
        }
        self.actions
            .lookup_action("mode")
            .and_downcast::<gio::SimpleAction>()
            .unwrap()
            .set_state(&mode.name().to_variant());
        let appearance = self.appearance.borrow().clone();
        if let Some(appearance) = appearance {
            appearance.set_selected(u32::from(mode == Mode::Full));
        }
        self.render();
        if self.layer && self.window.is_visible() {
            let hint = self.model.borrow().last_hint;
            if let Some(hint) = hint {
                self.show_at(hint);
            }
        }
    }
    fn show_settings(self: &Rc<Self>) {
        if let Some(window) = self.settings.borrow().as_ref() {
            window.present();
            return;
        }
        let window = gtk::Window::builder()
            .title("rQuickShare Settings")
            .transient_for(&self.window)
            .application(&self.window.application().unwrap())
            .default_width(360)
            .resizable(false)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        for side in [
            gtk::PositionType::Top,
            gtk::PositionType::Bottom,
            gtk::PositionType::Left,
            gtk::PositionType::Right,
        ] {
            match side {
                gtk::PositionType::Top => content.set_margin_top(20),
                gtk::PositionType::Bottom => content.set_margin_bottom(20),
                gtk::PositionType::Left => content.set_margin_start(20),
                gtk::PositionType::Right => content.set_margin_end(20),
                _ => unreachable!(),
            }
        }
        content.append(&label("Appearance"));
        content.append(&label("Card display mode"));
        let appearance = gtk::DropDown::from_strings(&["Compact", "Full"]);
        appearance.set_selected(u32::from(self.model.borrow().mode == Mode::Full));
        let weak = Rc::downgrade(self);
        appearance.connect_selected_notify(move |appearance| {
            if let Some(panel) = weak.upgrade() {
                panel.set_mode(if appearance.selected() == 1 {
                    Mode::Full
                } else {
                    Mode::Compact
                });
            }
        });
        content.append(&appearance);
        content.append(&label("Display mode is kept while rQuickShare is running."));
        window.set_child(Some(&content));
        window.connect_close_request(|window| {
            window.set_visible(false);
            glib::Propagation::Stop
        });
        self.appearance.replace(Some(appearance));
        self.settings.replace(Some(window.clone()));
        window.present();
    }
}
