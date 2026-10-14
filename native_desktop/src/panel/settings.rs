//! Secondary explicit editor. GTK retains intent; daemon owns persistence/apply.
use super::*;
use rqs_lib::service::client::{
    controller::ActionResult,
    settings::{SaveIntent, SettingsEdit, SettingsView},
    ClientError,
};
use rqs_lib::service::native_settings::StoreError;
use std::{fs::File, rc::Weak};

pub(super) struct Editor {
    panel: Weak<Panel>,
    base: RefCell<Option<SettingsView>>,
    visibility: gtk::DropDown,
    port: gtk::Entry,
    appearance: gtk::DropDown,
    directory: gtk::Label,
    directory_intent: Cell<u8>,
    file: RefCell<Option<File>>,
    recover: gtk::CheckButton,
    status: gtk::Label,
    save: gtk::Button,
    reload: gtk::Button,
    retry: gtk::Button,
    restart: gtk::Button,
    choose: gtk::Button,
    default: gtk::Button,
    dirty: Cell<bool>,
    loading: Cell<bool>,
    busy: Cell<bool>,
    closed: Cell<bool>,
    task: RefCell<Option<glib::JoinHandle<()>>>,
    dialog: RefCell<Option<gio::Cancellable>>,
    login: gtk::CheckButton,
    login_apply: gtk::Button,
    login_reload: gtk::Button,
    login_status: gtk::Label,
    login_dirty: Cell<bool>,
    login_loading: Cell<bool>,
    login_busy: Cell<bool>,
    login_task: RefCell<Option<glib::JoinHandle<()>>>,
}
#[derive(Clone, Copy)]
enum Operation {
    Save,
    Reload,
    Retry,
    Restart,
}

pub(super) fn parse_port(value: &str) -> Result<i32, ()> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(-1);
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    value.parse::<u16>().map(i32::from).map_err(|_| ())
}
fn selected_visibility(selected: u32) -> Result<u8, ()> {
    match selected {
        1..=3 => Ok((selected - 1) as u8),
        _ => Err(()),
    }
}
fn status(view: &SettingsView) -> &'static str {
    if !view.available {
        return "Service settings are unavailable. Appearance changes are temporary.";
    }
    match view.error {
        1 => "Cannot read settings. Restore access, then reload saved values.",
        2 => "Saved settings are damaged or unavailable. Review values and confirm replacement.",
        3 => "Save outcome is uncertain. Retry the same save before making another edit.",
        4 => "Settings could not be applied. Review the receive folder and port, then restart the saved service.",
        _ if view.visibility_recovered => "Saved visibility is invalid. Receiving is hidden; choose a visibility and save.",
        _ if view.source == 0 => "Defaults loaded. Save to keep your settings.",
        _ if view.source == 1 => "Existing settings loaded. Save to import them; the original file is preserved.",
        _ if !view.running => "Saved. Service is stopped; start the saved service when ready.",
        _ if view.applied_live_revision != view.revision => "Saved values differ from the live policy. Save to apply them.",
        _ if view.restart_required => "Saved. Visibility and receive folder applied; restart required for the port.",
        _ => "Saved and applied.",
    }
}
fn failure(error: ControllerError) -> &'static str {
    match error {
        ControllerError::Client(ClientError::Settings(StoreError::Conflict | StoreError::ReplayConflict)) => "Settings changed. Reload saved values and review your edit before saving.",
        ControllerError::Client(ClientError::Settings(StoreError::Indeterminate)) => "Save outcome is uncertain. Retry the same save.",
        ControllerError::Client(ClientError::Settings(StoreError::Directory)) => "The selected receive folder is unavailable. Choose it again or use the default.",
        ControllerError::Client(ClientError::Settings(StoreError::RecoveryRequired)) => "Confirm backup and replacement of damaged settings before saving.",
        ControllerError::Client(ClientError::Settings(StoreError::BackupCapacity)) => "The settings backup limit was reached. Existing backups have been preserved.",
        ControllerError::Client(ClientError::Settings(_)) => "Settings were not acknowledged. Check storage access and reload; no live change is claimed.",
        ControllerError::Client(ClientError::PendingOperation | ClientError::AcknowledgementPending) => "Finish or discard the pending operation before changing service settings.",
        ControllerError::Client(ClientError::Rejected(rqs_lib::service::Error::Busy)) | ControllerError::Busy => "Service is busy. Finish transfers or discard prepared files before restarting.",
        ControllerError::Client(ClientError::DaemonChanged | ClientError::Transport | ClientError::Closed) | ControllerError::Closed => "Service connection lost. Reconnect to read its current settings.",
        _ => "The settings action was not accepted. Review its state before retrying.",
    }
}
fn login_failure(error: crate::login::Error) -> &'static str {
    use crate::login::Error;
    match error {
        Error::Unsupported => {
            "Login start is unavailable for a missing, masked or customized desktop service."
        }
        Error::Denied => "Login change was denied. Reload to read the current state.",
        Error::Conflict => "Login state changed. Reload and review before applying again.",
        Error::Uncertain => {
            "Login change is unconfirmed. Reload the actual state; no write will be repeated."
        }
        Error::Busy => "Another login settings request is in progress.",
        Error::Closed | Error::Unavailable => {
            "Login settings are unavailable. Check installation or reconnect the app."
        }
        Error::Protocol => "Login settings could not be verified. Reload before changing them.",
    }
}
impl Editor {
    pub fn open(panel: &Rc<Panel>) {
        let window = gtk::Window::builder()
            .title("rQuickShare Settings")
            .transient_for(&panel.window)
            .application(&panel.window.application().unwrap())
            .default_width(420)
            .resizable(false)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
        content.set_margin_top(20);
        content.set_margin_bottom(20);
        content.set_margin_start(20);
        content.set_margin_end(20);
        let editor = Rc::new(Self {
            panel: Rc::downgrade(panel),
            base: RefCell::new(None),
            visibility: gtk::DropDown::from_strings(&[
                "Select visibility…",
                "Visible",
                "Hidden",
                "Visible for one minute",
            ]),
            port: gtk::Entry::builder()
                .placeholder_text("Automatic")
                .input_purpose(gtk::InputPurpose::Digits)
                .build(),
            appearance: gtk::DropDown::from_strings(&["Compact", "Full"]),
            directory: label("Connect to read the receive folder"),
            directory_intent: Cell::new(0),
            file: RefCell::new(None),
            recover: gtk::CheckButton::with_label("Back up and replace damaged settings"),
            status: label("Service settings are unavailable. Appearance changes are temporary."),
            save: gtk::Button::with_label("Save settings"),
            reload: gtk::Button::with_label("Reload saved values"),
            retry: gtk::Button::with_label("Retry pending save"),
            restart: gtk::Button::with_label("Start / restart saved service"),
            choose: gtk::Button::with_label("Choose folder…"),
            default: gtk::Button::with_label("Use default"),
            dirty: Cell::new(false),
            loading: Cell::new(false),
            busy: Cell::new(false),
            closed: Cell::new(false),
            task: RefCell::new(None),
            dialog: RefCell::new(None),
            login: gtk::CheckButton::with_label("Start at sign-in"),
            login_apply: gtk::Button::with_label("Apply login start"),
            login_reload: gtk::Button::with_label("Reload login state"),
            login_status: label("Login settings are unavailable."),
            login_dirty: Cell::new(false),
            login_loading: Cell::new(false),
            login_busy: Cell::new(false),
            login_task: RefCell::new(None),
        });
        editor
            .appearance
            .set_selected(u32::from(panel.mode() == Mode::Full));
        editor.visibility.set_selected(0);
        for (title, widget) in [
            (
                "Visibility",
                editor.visibility.clone().upcast::<gtk::Widget>(),
            ),
            (
                "Port (empty or 0 = automatic)",
                editor.port.clone().upcast(),
            ),
            ("Receive folder", editor.directory.clone().upcast()),
        ] {
            let title = label(title);
            title.set_selectable(false);
            title.set_mnemonic_widget(Some(&widget));
            content.append(&title);
            content.append(&widget);
        }
        let folder = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        folder.append(&editor.choose);
        folder.append(&editor.default);
        content.append(&folder);
        let appearance_title = label("Card display mode");
        appearance_title.set_selectable(false);
        appearance_title.set_mnemonic_widget(Some(&editor.appearance));
        content.append(&appearance_title);
        content.append(&editor.appearance);
        content.append(&label(
            "Mode changes preview immediately. Save to keep them after closing the app.",
        ));
        content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        content.append(&editor.login);
        content.append(&label("Starts the daemon and tray with the card hidden. Applies to future sign-ins; active transfers keep running."));
        let login_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        login_actions.append(&editor.login_apply);
        login_actions.append(&editor.login_reload);
        content.append(&login_actions);
        content.append(&editor.login_status);
        content.append(&editor.recover);
        content.append(&editor.status);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.append(&editor.save);
        actions.append(&editor.reload);
        content.append(&actions);
        content.append(&editor.retry);
        content.append(&editor.restart);
        content.append(&label(
            "Finish active transfers or discard prepared files before restarting.",
        ));
        let reconnect = gtk::Button::with_label("Reconnect service");
        content.append(&reconnect);
        editor.directory.set_wrap(false);
        editor
            .directory
            .set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        editor.directory.set_max_width_chars(40);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(620)
            .child(&content)
            .build();
        window.set_child(Some(&scroll));
        panel.appearance.replace(Some(editor.appearance.clone()));
        panel.settings.replace(Some(window.clone()));
        panel.settings_owner.replace(Some(editor.clone()));
        if let Some(view) = panel.model.borrow().state.settings.clone() {
            editor.load(view, false);
        }
        let weak = Rc::downgrade(&editor);
        editor.login.connect_toggled(move |_| {
            if let Some(editor) = weak
                .upgrade()
                .filter(|editor| !editor.login_loading.get() && !editor.closed.get())
            {
                editor.login_dirty.set(true);
                editor.refresh_login();
            }
        });
        for (button, apply) in [(&editor.login_apply, true), (&editor.login_reload, false)] {
            let weak = Rc::downgrade(&editor);
            button.connect_clicked(move |_| {
                if let Some(editor) = weak.upgrade() {
                    editor.login_action(apply);
                }
            });
        }
        let weak = Rc::downgrade(&editor);
        editor
            .appearance
            .connect_selected_notify(move |appearance| {
                if let Some(editor) = weak.upgrade() {
                    if editor.loading.get() || editor.closed.get() {
                        return;
                    }
                    if let Some(panel) = editor.panel.upgrade() {
                        panel.set_mode(if appearance.selected() == 1 {
                            Mode::Full
                        } else {
                            Mode::Compact
                        });
                    }
                    editor.changed();
                }
            });
        let weak = Rc::downgrade(&editor);
        editor.visibility.connect_selected_notify(move |_| {
            if let Some(editor) = weak.upgrade() {
                editor.changed();
            }
        });
        let weak = Rc::downgrade(&editor);
        editor.port.connect_changed(move |_| {
            if let Some(editor) = weak.upgrade() {
                editor.changed();
            }
        });
        let weak = Rc::downgrade(&editor);
        editor.recover.connect_toggled(move |_| {
            if let Some(editor) = weak.upgrade() {
                editor.changed();
            }
        });
        for (button, operation) in [
            (&editor.save, Operation::Save),
            (&editor.reload, Operation::Reload),
            (&editor.retry, Operation::Retry),
            (&editor.restart, Operation::Restart),
        ] {
            let weak = Rc::downgrade(&editor);
            button.connect_clicked(move |_| {
                if let Some(editor) = weak.upgrade() {
                    editor.act(operation);
                }
            });
        }
        let weak = Rc::downgrade(&editor);
        editor.choose.connect_clicked(move |_| {
            if let Some(editor) = weak.upgrade() {
                editor.choose_folder();
            }
        });
        let weak = Rc::downgrade(&editor);
        editor.default.connect_clicked(move |_| {
            if let Some(editor) = weak.upgrade() {
                editor.file.borrow_mut().take();
                editor.directory_intent.set(1);
                editor.directory.set_text("Default Downloads folder");
                editor.changed();
            }
        });
        let weak = Rc::downgrade(panel);
        reconnect.connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.actions.activate_action("reconnect", None);
            }
        });
        let weak = Rc::downgrade(panel);
        window.connect_close_request(move |window| {
            window.set_visible(false);
            if let Some(panel) = weak.upgrade() {
                panel.apply_presentation();
            }
            glib::Propagation::Stop
        });
        editor.refresh();
        window.present();
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty.get()
    }
    pub fn sync_mode(&self, mode: Mode, manual: bool) {
        let loading = self.loading.replace(true);
        self.appearance.set_selected(u32::from(mode == Mode::Full));
        self.loading.set(loading);
        if manual {
            self.changed();
        }
    }
    fn changed(&self) {
        if self.loading.get() {
            return;
        }
        self.dirty.set(true);
        self.status
            .set_text("Unsaved changes. Mode preview is temporary until Save.");
        self.refresh();
    }
    fn load(&self, view: SettingsView, appearance: bool) {
        self.loading.set(true);
        self.visibility.set_selected(u32::from(view.visibility) + 1);
        self.port.set_text(&if view.port < 0 {
            String::new()
        } else {
            view.port.to_string()
        });
        self.directory
            .set_text(if view.directory_display.is_empty() {
                "Default Downloads folder"
            } else {
                &view.directory_display
            });
        self.directory_intent
            .set(if view.configured { 0 } else { 1 });
        self.file.borrow_mut().take();
        self.recover.set_active(false);
        if appearance {
            self.appearance.set_selected(view.card_mode.into());
        }
        self.dirty
            .set(!appearance && self.appearance.selected() != u32::from(view.card_mode));
        self.status.set_text(if self.dirty.get() {
            "Unsaved appearance change. Save to keep this mode."
        } else {
            status(&view)
        });
        self.base.replace(Some(view));
        self.loading.set(false);
        if appearance {
            if let Some(panel) = self.panel.upgrade() {
                panel.apply_mode(
                    if self.appearance.selected() == 1 {
                        Mode::Full
                    } else {
                        Mode::Compact
                    },
                    false,
                );
            }
        }
        self.refresh();
    }
    pub fn refresh(&self) {
        self.refresh_login();
        let Some(panel) = self.panel.upgrade() else {
            return;
        };
        if self.closed.get() {
            return;
        }
        let model = panel.model.borrow();
        let latest = model.state.settings.as_ref();
        if self.base.borrow().is_none() {
            if let Some(view) = latest {
                let view = view.clone();
                drop(model);
                self.load(view, false);
                return;
            }
        }
        let safe = !model.closing && (!model.safety_active || model.safety.manual_allowed());
        let available = panel.handle.is_some()
            && !model.state.disconnected
            && latest.is_some_and(|view| view.available);
        let ready = safe
            && available
            && !self.busy.get()
            && !model.waiting
            && !model.state.busy
            && !model.state.pending
            && !model.state.text_pending
            && !model.ingress.busy
            && !*panel.ingress.gate.subscribe().borrow();
        let base = self.base.borrow();
        let writable = base
            .as_ref()
            .is_some_and(|view| view.available && view.token.len() == 64 && view.error != 1);
        let damaged = base
            .as_ref()
            .is_some_and(|view| view.available && !view.configured && view.error != 1);
        self.recover.set_visible(damaged);
        self.recover.set_sensitive(ready);
        self.save.set_sensitive(
            ready
                && writable
                && selected_visibility(self.visibility.selected()).is_ok()
                && !model.state.settings_pending
                && (!damaged || self.recover.is_active()),
        );
        self.reload.set_sensitive(ready);
        self.retry.set_visible(model.state.settings_pending);
        self.retry
            .set_sensitive(ready && model.state.settings_pending);
        self.restart.set_sensitive(
            ready
                && !self.dirty.get()
                && !model.state.settings_pending
                && latest.is_some_and(|view| view.configured && matches!(view.error, 0 | 4)),
        );
        for widget in [
            self.visibility.clone().upcast::<gtk::Widget>(),
            self.port.clone().upcast(),
            self.choose.clone().upcast(),
            self.default.clone().upcast(),
        ] {
            widget.set_sensitive(ready && writable && !model.state.settings_pending);
        }
        self.appearance
            .set_sensitive(safe && !self.busy.get() && !model.state.settings_pending);
        if !available {
            self.status.set_text("Service settings are unavailable. Appearance changes are temporary. Reconnect to save.");
        }
    }
    pub fn refresh_login(&self) {
        let Some(panel) = self.panel.upgrade() else {
            return;
        };
        if self.closed.get() {
            return;
        }
        let model = panel.model.borrow();
        let safe = !model.closing && (!model.safety_active || model.safety.manual_allowed());
        let handle = panel.login.borrow();
        let view = handle
            .as_ref()
            .map(|handle| *handle.subscribe().borrow())
            .unwrap_or_default();
        self.login_loading.set(true);
        self.login.set_inconsistent(view.enabled.is_none());
        if !self.login_dirty.get() {
            if let Some(enabled) = view.enabled {
                self.login.set_active(enabled);
            }
        }
        if view.enabled == Some(self.login.is_active()) {
            self.login_dirty.set(false);
        }
        self.login_loading.set(false);
        let ready = safe && handle.is_some() && !view.busy && !self.login_busy.get();
        self.login.set_sensitive(ready && view.enabled.is_some());
        self.login_apply
            .set_sensitive(ready && view.enabled.is_some() && self.login_dirty.get());
        self.login_reload.set_sensitive(ready);
        self.login_status
            .set_text(if self.login_busy.get() || view.busy {
                "Waiting for login settings…"
            } else if let Some(error) = view.error {
                login_failure(error)
            } else if self.login_dirty.get() {
                "Unsaved login choice. Apply it separately from service settings."
            } else if view.enabled == Some(true) {
                "Daemon and tray will start at sign-in; the card stays hidden."
            } else if view.enabled == Some(false) {
                "Login start is off."
            } else {
                "Login settings are unavailable."
            });
    }
    fn login_action(self: &Rc<Self>, apply: bool) {
        self.refresh_login();
        if self.closed.get()
            || !(if apply {
                self.login_apply.is_sensitive()
            } else {
                self.login_reload.is_sensitive()
            })
        {
            return;
        }
        let Some(panel) = self.panel.upgrade() else {
            return;
        };
        let Some(handle) = panel.login.borrow().clone() else {
            return;
        };
        let view = *handle.subscribe().borrow();
        let completion = if apply {
            let Some(expected) = view.enabled else { return };
            handle.set(expected, self.login.is_active())
        } else {
            handle.read()
        };
        let completion = match completion {
            Ok(completion) => completion,
            Err(error) => {
                self.login_status.set_text(login_failure(error));
                return;
            }
        };
        self.login_busy.set(true);
        self.refresh_login();
        let weak = Rc::downgrade(self);
        self.login_task
            .replace(Some(glib::spawn_future_local(async move {
                let result = completion.wait().await;
                if let Some(editor) = weak.upgrade().filter(|editor| !editor.closed.get()) {
                    editor.login_busy.set(false);
                    if result.is_ok() {
                        editor.login_dirty.set(false);
                    }
                    editor.refresh_login();
                    if let Err(error) = result {
                        editor.login_status.set_text(login_failure(error));
                    }
                }
            })));
    }
    fn act(self: &Rc<Self>, operation: Operation) {
        self.refresh();
        let enabled = match operation {
            Operation::Save => self.save.is_sensitive(),
            Operation::Reload => self.reload.is_sensitive(),
            Operation::Retry => self.retry.is_sensitive(),
            Operation::Restart => self.restart.is_sensitive(),
        };
        if !enabled {
            return;
        }
        let Some(panel) = self.panel.upgrade() else {
            return;
        };
        let Some(handle) = &panel.handle else { return };
        let completion = match operation {
            Operation::Reload => handle.reload_settings(),
            Operation::Retry => handle.retry_settings(),
            Operation::Restart => handle.restart_service(),
            Operation::Save => {
                let Ok(visibility) = selected_visibility(self.visibility.selected()) else {
                    self.status
                        .set_text("Choose a visibility setting before saving.");
                    return;
                };
                let Ok(port) = parse_port(self.port.text().as_str()) else {
                    self.status
                        .set_text("Port must be empty or an integer from 0 to 65535.");
                    return;
                };
                let Some(base) = self.base.borrow().clone() else {
                    return;
                };
                let Ok(request) = rqs_lib::service::process::fresh_epoch() else {
                    self.status
                        .set_text("Cannot create a save identity. Try again.");
                    return;
                };
                let directory = match self.file.borrow().as_ref().map(File::try_clone).transpose() {
                    Ok(file) => file,
                    Err(_) => {
                        self.status
                            .set_text("Cannot retain the selected folder. Choose it again.");
                        return;
                    }
                };
                handle.save_settings(SettingsEdit {
                    intent: SaveIntent {
                        epoch: base.epoch,
                        revision: base.revision,
                        token: base.token,
                        request: format!("{request:032x}"),
                        visibility,
                        port,
                        card_mode: self.appearance.selected() as u8,
                        directory: self.directory_intent.get(),
                        recover: self.recover.is_active(),
                    },
                    directory,
                })
            }
        };
        let completion = match completion {
            Ok(completion) => completion,
            Err(error) => {
                self.status.set_text(failure(error));
                return;
            }
        };
        self.busy.set(true);
        self.status.set_text("Waiting for the service…");
        self.refresh();
        let weak = Rc::downgrade(self);
        self.task.replace(Some(glib::spawn_future_local(async move {
            let result = completion.wait().await;
            if let Some(editor) = weak.upgrade().filter(|editor| !editor.closed.get()) {
                editor.busy.set(false);
                match result {
                    Ok(ActionResult::Settings(view)) => editor.load(view, true),
                    Ok(_) if matches!(operation, Operation::Restart) => editor.status.set_text("Restart requested. Reconnect to read the new service state."),
                    Ok(_) => editor.status.set_text("Settings acknowledgement was incomplete. Reload before making another edit."),
                    Err(error) => editor.status.set_text(failure(error)),
                }
                editor.refresh();
            }
        })));
    }
    fn choose_folder(self: &Rc<Self>) {
        if !self.choose.is_sensitive() {
            return;
        }
        let Some(panel) = self.panel.upgrade() else {
            return;
        };
        let Some(permit) = panel.ingress.gate.try_acquire() else {
            return;
        };
        self.busy.set(true);
        self.refresh();
        let cancellable = gio::Cancellable::new();
        self.dialog.replace(Some(cancellable.clone()));
        let dialog = gtk::FileDialog::new();
        let weak = Rc::downgrade(self);
        dialog.select_folder(panel.settings.borrow().as_ref(), Some(&cancellable), move |result| {
            let Some(editor) = weak.upgrade().filter(|editor| !editor.closed.get()) else { return };
            editor.dialog.borrow_mut().take();
            let Some(panel) = editor.panel.upgrade() else { return };
            let path = match result {
                Ok(file) => match file.path() { Some(path) => path, None => { editor.busy.set(false); editor.status.set_text("Choose an accessible local folder."); editor.refresh(); return } },
                Err(error) => {
                    editor.busy.set(false);
                    if !error.matches(gtk::DialogError::Dismissed) && !error.matches(gio::IOErrorEnum::Cancelled) { editor.status.set_text("Cannot open the folder chooser."); }
                    editor.refresh(); return;
                }
            };
            let receive = panel.ingress.start_directory(&panel.runtime, path, permit);
            let weak = Rc::downgrade(&editor);
            editor.task.replace(Some(glib::spawn_future_local(async move {
                let result = receive.await;
                if let Some(editor) = weak.upgrade().filter(|editor| !editor.closed.get()) {
                    editor.busy.set(false);
                    match result {
                        Ok(Ok((file, display))) => { editor.file.replace(Some(file)); editor.directory_intent.set(2); editor.directory.set_text(&display); editor.changed(); },
                        _ => editor.status.set_text("Cannot acquire the selected folder. Check access and choose again."),
                    }
                    editor.refresh();
                }
            })));
        });
    }
    pub fn close(&self) {
        self.closed.set(true);
        if let Some(task) = self.login_task.borrow_mut().take() {
            task.abort();
        }
        if let Some(dialog) = self.dialog.borrow_mut().take() {
            dialog.cancel();
        }
        if let Some(task) = self.task.borrow_mut().take() {
            task.abort();
        }
        self.file.borrow_mut().take();
    }
}

#[cfg(test)]
pub(super) fn verify_editor_smoke(editor: &Editor) {
    // Run on the existing single GTK test thread; no real settings owner is connected.
    assert!(!editor.save.is_sensitive());
    assert!(editor.login.is_inconsistent());
    assert!(!editor.login.is_sensitive());
    assert!(!editor.login_apply.is_sensitive());
    assert!(!editor.login_reload.is_sensitive());
    assert_eq!(editor.visibility.selected(), 0);
    assert!(!editor.choose.is_sensitive());
    assert!(!editor.recover.is_visible());
    assert!(editor.appearance.is_sensitive());
    if let Some(output) = std::env::var_os("RQS_SETTINGS_PREVIEW_OUTPUT") {
        use gtk::gdk::prelude::PaintableExt;
        let output = std::path::PathBuf::from(output);
        assert!(output.is_absolute());
        let panel = editor.panel.upgrade().unwrap();
        let window = panel.settings.borrow().as_ref().unwrap().clone();
        let captured = Rc::new(Cell::new(false));
        let finished = captured.clone();
        let success = Rc::new(Cell::new(false));
        let saved = success.clone();
        window.add_tick_callback(move |window, _| {
            if window.width() <= 0 || window.height() <= 0 {
                return glib::ControlFlow::Continue;
            }
            let paintable = gtk::WidgetPaintable::new(Some(window));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, window.width().into(), window.height().into());
            let Some(node) = snapshot.to_node() else {
                return glib::ControlFlow::Continue;
            };
            if let Some(renderer) = window.renderer() {
                saved.set(
                    renderer
                        .render_texture(&node, None)
                        .save_to_png(&output)
                        .is_ok(),
                );
            }
            finished.set(true);
            glib::ControlFlow::Break
        });
        while !captured.get() {
            glib::MainContext::default().iteration(true);
        }
        assert!(success.get());
    }
    editor.port.set_text("655350");
    assert_eq!(editor.port.text(), "655350");
    assert_eq!(parse_port(editor.port.text().as_str()), Err(()));
    let mut view = SettingsView::unavailable(1);
    view.available = true;
    view.token = "a".repeat(64);
    view.error = 2;
    editor.load(view.clone(), true);
    assert!(editor.recover.is_visible());
    assert!(!editor.recover.is_active());
    editor.recover.set_active(true);
    assert!(!editor.save.is_sensitive()); // Preview cannot turn confirmation into write authority.
    view.configured = true;
    view.source = 2;
    view.error = 0;
    view.revision = 1;
    view.running = true;
    view.applied_live_revision = 1;
    view.card_mode = 1;
    let panel = editor.panel.upgrade().unwrap();
    panel.model.borrow_mut().automatic = true;
    editor.load(view, true);
    assert!(panel.model.borrow().automatic); // Loading preferences is not manual presentation intent.
    assert_eq!(panel.mode(), Mode::Full);
    assert!(!editor.is_dirty());
    assert!(!editor.recover.is_visible());
    assert!(!editor.save.is_sensitive());
    panel.model.borrow_mut().automatic = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_unavailable_corrupt_unapplied_or_uncertain_settings_when_presented_then_no_saved_and_applied_claim_is_fabricated(
    ) {
        // Arrange
        let mut applied = SettingsView::unavailable(1);
        applied.available = true;
        applied.configured = true;
        applied.source = 2;
        applied.running = true;
        applied.revision = 2;
        applied.applied_live_revision = 2;
        // Act / Assert
        assert_eq!(status(&applied), "Saved and applied.");
        for case in 0..9 {
            let mut view = applied.clone();
            match case {
                0 => view.available = false,
                1..=4 => view.error = case as u8,
                5 => view.running = false,
                6 => view.applied_live_revision = 1,
                7 => view.restart_required = true,
                8 => view.visibility_recovered = true,
                _ => unreachable!(),
            }
            assert_ne!(status(&view), "Saved and applied.", "{case}");
        }
        for error in [
            StoreError::Conflict,
            StoreError::ReplayConflict,
            StoreError::Indeterminate,
            StoreError::Directory,
            StoreError::RecoveryRequired,
            StoreError::BackupCapacity,
            StoreError::Write,
            StoreError::Read,
            StoreError::UnsafeStorage,
        ] {
            let message = failure(ControllerError::Client(ClientError::Settings(error)));
            assert!(!message.contains("Saved and applied"));
            assert!(!message.contains("/proc"));
        }
    }
    #[test]
    fn given_port_input_when_parsed_then_only_automatic_or_u16_values_are_accepted() {
        // Arrange / Act / Assert
        for (input, expected) in [
            ("", -1),
            (" ", -1),
            ("0", 0),
            ("00000", 0),
            ("000001", 1),
            ("0065535", 65535),
            ("1", 1),
            ("65535", 65535),
            (" 42 ", 42),
        ] {
            assert_eq!(parse_port(input), Ok(expected), "{input}");
        }
        for input in [
            "-1", "+1", "65536", "655350", "1.0", "NaN", "inf", "²", "1 2",
        ] {
            assert_eq!(parse_port(input), Err(()), "{input}");
        }
        for (selected, expected) in [
            (0, Err(())),
            (1, Ok(0)),
            (2, Ok(1)),
            (3, Ok(2)),
            (4, Err(())),
            (u32::MAX, Err(())),
        ] {
            assert_eq!(selected_visibility(selected), expected);
        }
    }
}
