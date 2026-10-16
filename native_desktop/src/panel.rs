//! GTK widgets live exclusively on the GLib thread. No service IO in callbacks.

use crate::{
    card_view::{compact_index, Mode},
    ingress::{Ingress, IngressState},
    placement::{self, Edge, Rect},
};
use gtk::{gio, glib, prelude::*};
use gtk4_layer_shell::{Edge as LayerEdge, LayerShell};
use rqs_lib::service::client::{
    controller::{Completion, ControllerError, PanelHandle, PanelState},
    desktop::{DesktopHandle, DesktopState, PreparationPermit},
    state::{DetailsStatus, TransferDetails, TransferView},
};
use rqs_lib::service::{Direction, Lifecycle, Phase};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};
use tokio::runtime::Handle;

mod compose;
mod image;
mod incoming_preview;
mod menu;
mod presentation;
mod recipients;
mod reveal;
mod rows;
mod settings;
mod text;
mod tray;

#[derive(Default)]
struct Model {
    state: PanelState,
    ingress: IngressState,
    dialog: Option<gio::Cancellable>,
    dialog_permit: Option<PreparationPermit>,
    desktop: DesktopState,
    last_hint: Option<(i32, i32)>,
    waiting: bool,
    closing: bool,
    notice: Option<&'static str>,
    mode: Mode,
    preview_page: Option<String>,
    safety: crate::desktop_guard::SafetyState,
    safety_active: bool,
    automatic: bool,
    automatic_focus: Option<String>,
    manual_requested: bool,
    preview: bool,
    saved_mode: Option<u8>,
    recipients: BTreeMap<String, String>,
}
pub struct Panel {
    pub window: gtk::Window,
    pub pages: gtk::Stack,
    builder: gtk::Builder,
    model: RefCell<Model>,
    handle: Option<PanelHandle>,
    layer: bool,
    runtime: Handle,
    ingress: Arc<Ingress>,
    completion: RefCell<Option<glib::JoinHandle<()>>>,
    desktop: RefCell<Option<DesktopHandle>>,
    policy: RefCell<crate::card_policy::CardPolicy>,
    actions: gio::SimpleActionGroup,
    context: gtk::PopoverMenu,
    settings: RefCell<Option<gtk::Window>>,
    appearance: RefCell<Option<gtk::DropDown>>,
    settings_owner: RefCell<Option<Rc<settings::Editor>>>,
    text_review: RefCell<Option<Rc<text::Review>>>,
    composer: RefCell<Option<Rc<compose::Composer>>>,
    image: RefCell<Option<Rc<image::Image>>>,
    preview_image: incoming_preview::Fixture,
    reveal: RefCell<Option<Rc<reveal::Pending>>>,
    text_generation: RefCell<Rc<()>>,
    rows: RefCell<rows::Rows>,
    dismissal: RefCell<Option<glib::JoinHandle<()>>>,
    reconnect: Cell<bool>,
    reconnect_button: gtk::Button,
    style: gtk::CssProvider,
    tray_menu: RefCell<Option<crate::tray::menu::Handle>>,
    login: RefCell<Option<crate::login::Handle>>,
}

fn label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_selectable(true);
    label
}
fn card() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
    row.add_css_class("device-row");
    row
}
fn clear(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

/// Display metadata is plain text and keeps every relative name available.
fn details_card(details: &TransferDetails) -> gtk::Box {
    details_card_mode(details, true)
}
type FileAction = dyn Fn(&gtk::Box, u32);
fn details_card_mode(details: &TransferDetails, full: bool) -> gtk::Box {
    details_card_mode_with_files(details, full, None)
}
fn details_card_mode_with_files(
    details: &TransferDetails,
    full: bool,
    action: Option<Rc<FileAction>>,
) -> gtk::Box {
    let row = card();
    match details.status {
        DetailsStatus::Unavailable => row.append(&label("Waiting for sender and payload details")),
        DetailsStatus::Unsupported => row.append(&label(
            "This payload cannot be reviewed safely. Decline or cancel it.",
        )),
        DetailsStatus::Ready => {
            let sender = label(&format!("{} · advertised device", details.sender));
            if !full {
                sender.set_wrap(false);
                sender.set_ellipsize(gtk::pango::EllipsizeMode::End);
                sender.set_max_width_chars(38);
            }
            row.append(&sender);
            if !details.pin.is_empty() {
                let pin = label(&format!("Code: {}", details.pin));
                pin.add_css_class("verification-code");
                row.append(&pin);
                row.append(&label(
                    "Check this code on the sending device before accepting.",
                ));
            }
            if details.kind != rqs_lib::sessions::details::PayloadKind::Files {
                row.append(&label(&format!(
                    "{} · {} bytes",
                    if details.kind == rqs_lib::sessions::details::PayloadKind::Url {
                        "Link"
                    } else {
                        "Text"
                    },
                    details.total_bytes
                )));
                return row;
            }
            row.append(&label(&format!(
                "{} {} · {} bytes",
                details.files.len(),
                if details.files.len() == 1 {
                    "file"
                } else {
                    "files"
                },
                details.total_bytes
            )));
            if !full {
                return row;
            }
            let expander = gtk::Expander::new(Some("Review all files"));
            let files = details.files.clone();
            // File widgets are materialized only while expanded, bounded by the manifest limit.
            expander.connect_expanded_notify(move |expander| {
                if expander.is_expanded() {
                    let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
                    for (index, file) in files.iter().enumerate() {
                        let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
                        row.append(&label(&format!("{} · {} bytes", file.name, file.size)));
                        if let Some(action) = &action {
                            action(&row, index as u32);
                        }
                        list.append(&row);
                    }
                    expander.set_child(Some(&list));
                } else {
                    expander.set_child(gtk::Widget::NONE);
                }
            });
            row.append(&expander);
        }
    }
    row
}

fn source_summary(
    kind: rqs_lib::sessions::details::PayloadKind,
    file_count: u32,
    byte_count: u64,
) -> String {
    use rqs_lib::sessions::details::PayloadKind;
    match kind {
        PayloadKind::Files => format!("{} files prepared · choose recipients", file_count),
        PayloadKind::Text | PayloadKind::Url => format!(
            "{} prepared · {} bytes · choose recipients",
            if kind == PayloadKind::Url {
                "Link"
            } else {
                "Text"
            },
            byte_count
        ),
    }
}

pub fn controls_ready(state: &PanelState, local_busy: bool) -> bool {
    !local_busy
        && !state.busy
        && !state.pending
        && !state.text_pending
        && !state.disconnected
        && state
            .view
            .as_ref()
            .is_some_and(|view| view.synchronized && view.lifecycle == Lifecycle::Running)
}
fn error_message(error: Option<rqs_lib::service::client::ClientError>) -> &'static str {
    use rqs_lib::service::client::ClientError::*;
    match error {
        Some(Settings(rqs_lib::service::native_settings::StoreError::Indeterminate)) => {
            "Settings save outcome is uncertain. Open Settings and retry the pending save."
        }
        Some(Settings(
            rqs_lib::service::native_settings::StoreError::Conflict
            | rqs_lib::service::native_settings::StoreError::ReplayConflict,
        )) => "Settings changed. Open Settings and reload before saving.",
        Some(Settings(_)) => "Settings need attention. Open Settings to review the result.",
        Some(DaemonChanged | Transport | Closed) => {
            "Service connection lost. Reconnect starts a fresh client; choose unsent files again."
        }
        Some(Incompatible | Protocol | Exhausted) => {
            "Service is incompatible or returned invalid state. Update it, then reconnect."
        }
        Some(PendingOperation | AcknowledgementPending) => {
            "Operation needs recovery. Do not repeat the send."
        }
        Some(Source) => "Cannot read the selected files. Check access and select them again.",
        Some(Rejected(rqs_lib::service::Error::UnknownEndpoint)) => {
            "A recipient is no longer available. Review the selected devices; nothing was queued."
        }
        Some(Rejected(rqs_lib::service::Error::Capacity)) => {
            "Sharing resources are busy. Nothing was queued. Try fewer recipients or wait."
        }
        Some(_) => "The operation was not accepted. Review the current state and try again.",
        None => "Choose files, a folder or text, then choose a nearby recipient.",
    }
}

impl Panel {
    pub fn bind_login(&self, handle: crate::login::Handle) {
        self.login.replace(Some(handle));
        self.refresh_login();
    }
    pub fn refresh_login(&self) {
        if let Some(editor) = self.settings_owner.borrow().as_ref() {
            editor.refresh_login();
        }
    }

    pub fn new(
        application: &gtk::Application,
        handle: Option<PanelHandle>,
        runtime: Handle,
        ingress: Arc<Ingress>,
        layer: bool,
    ) -> Rc<Self> {
        let builder = gtk::Builder::from_string(include_str!("../ui/panel.ui"));
        let window: gtk::Window = builder.object("panel").expect("panel widget");
        window.set_application(Some(application));
        if layer {
            window.init_layer_shell();
            window.set_namespace(Some("rquickshare"));
            window.set_layer(gtk4_layer_shell::Layer::Top);
            window.set_exclusive_zone(0);
            window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
        }
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("../../docs/prototypes/native-panel.css"));
        gtk::style_context_add_provider_for_display(
            &gtk::prelude::WidgetExt::display(&window),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let reconnect_button = gtk::Button::with_label("Reconnect service");
        reconnect_button.set_action_name(Some("panel.reconnect"));
        builder
            .object::<gtk::Box>("recovery")
            .unwrap()
            .append(&reconnect_button);
        let panel = Rc::new(Self {
            pages: builder.object("pages").expect("pages widget"),
            window,
            builder,
            model: RefCell::new(Model {
                safety_active: layer,
                preview: handle.is_none() && !layer,
                state: PanelState {
                    disconnected: handle.is_none() && layer,
                    ..Default::default()
                },
                ..Default::default()
            }),
            handle,
            layer,
            runtime,
            ingress,
            completion: RefCell::new(None),
            desktop: RefCell::new(None),
            policy: RefCell::new(crate::card_policy::CardPolicy::default()),
            actions: gio::SimpleActionGroup::new(),
            context: gtk::PopoverMenu::from_model(None::<&gio::MenuModel>),
            settings: RefCell::new(None),
            appearance: RefCell::new(None),
            settings_owner: RefCell::new(None),
            text_review: RefCell::new(None),
            composer: RefCell::new(None),
            image: RefCell::new(None),
            preview_image: incoming_preview::Fixture::default(),
            reveal: RefCell::new(None),
            text_generation: RefCell::new(Rc::new(())),
            rows: RefCell::new(rows::Rows::default()),
            dismissal: RefCell::new(None),
            reconnect: Cell::new(false),
            reconnect_button,
            style: provider,
            tray_menu: RefCell::new(None),
            login: RefCell::new(None),
        });
        let weak = Rc::downgrade(&panel);
        panel.window.connect_close_request(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.hide();
            }
            glib::Propagation::Stop
        });
        let keyboard = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(&panel);
        keyboard.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if let Some(panel) = weak.upgrade() {
                    panel.hide();
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        panel.window.add_controller(keyboard);
        for (id, folder) in [("files", false), ("folder", true)] {
            let weak = Rc::downgrade(&panel);
            panel.button(id).connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    panel.choose(folder);
                }
            });
        }
        for id in ["discard", "recover", "abort"] {
            let weak = Rc::downgrade(&panel);
            panel.button(id).connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    if id == "abort" && panel.model.borrow().desktop.staging {
                        if let Some(desktop) = panel.desktop.borrow().as_ref() {
                            desktop.abort_staging();
                        }
                        return;
                    }
                    if let Some(handle) = &panel.handle {
                        let action = || match id {
                            "recover" if panel.model.borrow().state.visibility_pending => {
                                handle.retry_visibility()
                            }
                            "recover" => handle.recover(),
                            "abort" => handle.abort_preparation(),
                            _ => {
                                let selection = panel
                                    .model
                                    .borrow()
                                    .state
                                    .view
                                    .as_ref()
                                    .and_then(|view| view.selections.first().cloned());
                                match selection {
                                    Some(selection) => handle.discard(selection),
                                    None => Err(ControllerError::Closed),
                                }
                            }
                        };
                        panel.submit(action);
                    }
                }
            });
        }
        panel.configure_menu(application);
        let weak = Rc::downgrade(&panel);
        panel.pages.connect_visible_child_name_notify(move |pages| {
            if let Some(panel) = weak.upgrade() {
                panel.box_widget("source").set_visible(
                    panel.model.borrow().mode == Mode::Full
                        && pages.visible_child_name().as_deref() == Some("nearby"),
                );
            }
        });
        panel.render();
        panel
    }
    pub fn reconnect_requested(&self) -> bool {
        self.reconnect.get()
    }
    pub fn mode(&self) -> Mode {
        self.model.borrow().mode
    }
    fn button(&self, id: &str) -> gtk::Button {
        self.builder.object(id).expect("button widget")
    }
    fn box_widget(&self, id: &str) -> gtk::Box {
        self.builder.object(id).expect("box widget")
    }
    fn text(&self, id: &str, text: &str) {
        self.builder
            .object::<gtk::Label>(id)
            .expect("label widget")
            .set_text(text);
    }
    pub fn update(self: &Rc<Self>, state: PanelState) {
        let previous_epoch = self
            .model
            .borrow()
            .state
            .view
            .as_ref()
            .map(|view| view.epoch);
        let current_epoch = state.view.as_ref().map(|view| view.epoch);
        if state.disconnected || previous_epoch != current_epoch {
            self.close_text_review();
            self.close_composer();
            self.close_image();
            self.close_reveal();
        }
        let saved = state
            .settings
            .as_ref()
            .filter(|view| view.configured)
            .map(|view| view.card_mode);
        let changed = {
            let mut model = self.model.borrow_mut();
            if state.disconnected || previous_epoch != current_epoch {
                model.recipients.clear();
            }
            if let Some(view) = &state.view {
                for endpoint in &view.endpoints {
                    if let Some(name) = model.recipients.get_mut(&endpoint.id) {
                        *name = endpoint.name.clone();
                    }
                }
            }
            let changed = saved.is_some() && saved != model.saved_mode;
            model.saved_mode = saved;
            model.state = state;
            changed
        };
        if changed
            && !self
                .settings_owner
                .borrow()
                .as_ref()
                .is_some_and(|editor| editor.is_dirty())
        {
            self.apply_mode(
                if saved == Some(1) {
                    Mode::Full
                } else {
                    Mode::Compact
                },
                false,
            );
        }
        if let Some(editor) = self.settings_owner.borrow().as_ref() {
            editor.refresh();
        }
        self.refresh_text_review();
        self.refresh_composer();
        self.refresh_image();
        self.refresh_reveal();
        self.render();
        self.apply_presentation();
    }
    pub fn bind_desktop(&self, desktop: DesktopHandle) {
        self.desktop.replace(Some(desktop));
    }
    pub fn desktop_update(self: &Rc<Self>, state: DesktopState) -> bool {
        let (present, error) = {
            let model = self.model.borrow();
            (
                state.show_revision != model.desktop.show_revision,
                (state.error_revision != model.desktop.error_revision)
                    .then_some(state.error)
                    .flatten(),
            )
        };
        self.model.borrow_mut().desktop = state;
        if let Some(error) = error {
            self.mark_manual();
            let mut model = self.model.borrow_mut();
            model.notice = Some(error.message());
            model.manual_requested = true;
        }
        self.render();
        if error.is_some() {
            self.present_surface()
        } else {
            !present || self.present_recipient()
        }
    }
    fn present_recipient(self: &Rc<Self>) -> bool {
        self.set_mode(Mode::Full);
        self.pages.set_visible_child_name("nearby");
        self.model.borrow_mut().manual_requested = true;
        self.present_surface()
    }
    fn present_surface(&self) -> bool {
        let hint = self.model.borrow().last_hint;
        if let Some(hint) = hint {
            if self.place_at(hint) {
                return true;
            }
        }
        let monitors = gtk::prelude::WidgetExt::display(&self.window).monitors();
        let Some(monitor) = monitors.item(0).and_downcast::<gtk::gdk::Monitor>() else {
            return false;
        };
        let geometry = monitor.geometry();
        let Some(x) = geometry
            .x()
            .checked_add(geometry.width())
            .and_then(|x| x.checked_sub(1))
        else {
            return false;
        };
        let Some(y) = geometry
            .y()
            .checked_add(geometry.height())
            .and_then(|y| y.checked_sub(1))
        else {
            return false;
        };
        self.place_at((x, y))
    }
    pub fn preparation_update(self: &Rc<Self>) {
        self.render();
        self.apply_presentation();
    }
    pub fn ingress_update(self: &Rc<Self>, state: IngressState) {
        self.model.borrow_mut().ingress = state;
        self.render();
        self.apply_presentation();
    }
    pub fn notice(&self, text: &'static str) {
        self.model.borrow_mut().notice = Some(text);
        self.text("status", text);
    }
    pub fn close(&self) {
        if self.model.borrow().closing {
            return;
        }
        self.preview_image.cancel();
        self.close_text_review();
        self.close_composer();
        self.close_image();
        self.close_reveal();
        if let Some(task) = self.completion.borrow_mut().take() {
            task.abort();
        }
        if let Some(task) = self.dismissal.borrow_mut().take() {
            task.abort();
        }
        if let Some(editor) = self.settings_owner.borrow_mut().take() {
            editor.close();
        }
        if let Some(settings) = self.settings.borrow_mut().take() {
            settings.destroy();
        }
        self.appearance.borrow_mut().take();
        self.context.unparent();
        let mut model = self.model.borrow_mut();
        model.closing = true;
        model.recipients.clear();
        model.dialog_permit.take();
        if let Some(dialog) = model.dialog.take() {
            dialog.cancel();
        }
        gtk::style_context_remove_provider_for_display(
            &gtk::prelude::WidgetExt::display(&self.window),
            &self.style,
        );
        self.window.destroy();
    }
    fn submit(self: &Rc<Self>, command: impl FnOnce() -> Result<Completion, ControllerError>) {
        {
            let model = self.model.borrow();
            if self.handle.is_none()
                || model.waiting
                || model.closing
                || model.state.busy
                || model.state.disconnected
                || (model.safety_active && !model.safety.manual_allowed())
                || model.dialog.is_some()
                || model.ingress.busy
                || *self.ingress.gate.subscribe().borrow()
            {
                return;
            }
        }
        let completion = match command() {
            Ok(completion) => completion,
            Err(_) => {
                self.notice("Panel is busy or disconnected. Review the current state.");
                return;
            }
        };
        self.model.borrow_mut().waiting = true;
        self.render();
        let weak = Rc::downgrade(self);
        let generation = self.text_generation.borrow().clone();
        let task = glib::spawn_future_local(async move {
            let result = completion.wait().await;
            if let Some(panel) = weak.upgrade() {
                {
                    let mut model = panel.model.borrow_mut();
                    model.waiting = false;
                    if recipients::confirmed(&result) {
                        model.recipients.clear();
                    }
                    model.notice = match &result {
                        Ok(rqs_lib::service::client::controller::ActionResult::Settings(view)) if view.error != 0 =>
                            Some("Visibility was saved but could not be fully applied. Review discovery status and Settings."),
                        Err(ControllerError::Client(error)) => Some(error_message(Some(*error))),
                        Err(_) => Some("Operation did not complete. Review its state before retrying."),
                        _ => None,
                    };
                }
                panel.render();
                panel.apply_presentation();
                if let Ok(rqs_lib::service::client::controller::ActionResult::Text(read)) = result {
                    if Rc::ptr_eq(&generation, &panel.text_generation.borrow()) {
                        panel.show_text_review(read);
                    }
                }
            }
        });
        self.completion.replace(Some(task));
    }
    fn choose(self: &Rc<Self>, folder: bool) {
        {
            let mut model = self.model.borrow_mut();
            if !controls_ready(
                &model.state,
                model.waiting
                    || (model.safety_active && !model.safety.manual_allowed())
                    || model.ingress.busy
                    || model.dialog.is_some()
                    || model.closing
                    || *self.ingress.gate.subscribe().borrow(),
            ) || model
                .state
                .view
                .as_ref()
                .is_some_and(|view| !view.selections.is_empty())
            {
                return;
            }
            let Some(permit) = self.ingress.gate.try_acquire() else {
                return;
            };
            model.dialog_permit = Some(permit);
            model.dialog = Some(gio::Cancellable::new());
            model.notice = None;
        }
        self.render();
        let dialog = gtk::FileDialog::new();
        let cancellable = self.model.borrow().dialog.clone();
        let weak = Rc::downgrade(self);
        let selected = move |files: Result<Vec<gio::File>, glib::Error>| {
            if let Some(panel) = weak.upgrade() {
                panel.model.borrow_mut().dialog = None;
                let permit = panel.model.borrow_mut().dialog_permit.take();
                if panel.model.borrow().closing {
                    return;
                }
                if let Ok(files) = files {
                    let paths: Option<Vec<_>> = files.iter().map(gio::File::path).collect();
                    match (paths, panel.handle.clone(), permit) {
                        (Some(paths), Some(handle), Some(permit)) if !paths.is_empty() => {
                            panel.ingress.start(&panel.runtime, paths, handle, permit);
                        }
                        _ => panel.notice("Only local files and folders can be selected."),
                    }
                } else if let Err(error) = files {
                    if !error.matches(gtk::DialogError::Dismissed)
                        && !error.matches(gio::IOErrorEnum::Cancelled)
                    {
                        panel.notice(
                            "Cannot select these files. Choose up to 256 accessible local roots.",
                        );
                    }
                }
                panel.render();
            }
        };
        if folder {
            dialog.select_folder(Some(&self.window), cancellable.as_ref(), move |result| {
                selected(result.map(|file| vec![file]))
            });
        } else {
            dialog.open_multiple(Some(&self.window), cancellable.as_ref(), move |result| {
                selected(result.and_then(|files| {
                    if files.n_items() as usize
                        > rqs_lib::file_source::SelectionLimits::default().max_files
                    {
                        return Err(glib::Error::new(
                            gio::IOErrorEnum::InvalidArgument,
                            "Selection exceeds the file limit",
                        ));
                    }
                    files
                        .iter::<gio::File>()
                        .map(|file| {
                            file.map_err(|_| {
                                glib::Error::new(
                                    gio::IOErrorEnum::InvalidArgument,
                                    "Invalid chooser entry",
                                )
                            })
                        })
                        .collect()
                }))
            });
        }
    }
    pub fn render(self: &Rc<Self>) {
        // Publish after readiness/action updates using an immutable model projection.
        let model = self.model.borrow();
        let preview = model.preview;
        let full = model.mode == Mode::Full;
        self.box_widget("source")
            .set_visible(full && self.pages.visible_child_name().as_deref() == Some("nearby"));
        self.builder
            .object::<gtk::StackSwitcher>("tabs")
            .unwrap()
            .set_visible(full);
        self.pages.set_visible(full);
        self.box_widget("compact").set_visible(!full);
        self.button("mode")
            .set_label(if full { "Compact" } else { "Full" });
        self.button("mode").set_tooltip_text(Some(if full {
            "Use compact card"
        } else {
            "Show full view"
        }));
        self.window
            .set_default_size(420, if full { 497 } else { 1 });
        let ready = !preview
            && (!model.safety_active || model.safety.manual_allowed())
            && controls_ready(
                &model.state,
                model.waiting
                    || (model.safety_active && !model.safety.manual_allowed())
                    || model.ingress.busy
                    || model.dialog.is_some()
                    || model.closing
                    || *self.ingress.gate.subscribe().borrow(),
            );
        let visibility = model
            .state
            .visibility
            .as_ref()
            .filter(|view| view.available && view.running && !model.state.disconnected);
        let mode = visibility.map(|view| view.mode);
        self.text(
            "visibility",
            match mode {
                Some(0) => "Discovery: visible",
                Some(1) => "Discovery: hidden",
                Some(2) => "Discovery: visible for up to one minute",
                _ => "Discovery: unavailable",
            },
        );
        let visibility_action = self
            .actions
            .lookup_action("visibility")
            .and_downcast::<gio::SimpleAction>()
            .unwrap();
        visibility_action.set_state(
            &match mode {
                Some(0) => "visible",
                Some(1) => "hidden",
                Some(2) => "temporary",
                _ => "unavailable",
            }
            .to_variant(),
        );
        visibility_action.set_enabled(
            ready
                && visibility.is_some()
                && !model.state.settings_pending
                && !model.state.visibility_pending
                && model.state.settings.as_ref().is_some_and(|view| {
                    view.available && view.configured && matches!(view.error, 0 | 4)
                }),
        );
        self.actions
            .lookup_action("visibility-retry")
            .and_downcast::<gio::SimpleAction>()
            .unwrap()
            .set_enabled(ready && model.state.visibility_pending);
        let selection = model
            .state
            .view
            .as_ref()
            .and_then(|view| view.selections.first().cloned());
        let selected = selection.is_some();
        self.render_recipients(&model, selected);
        self.button("files").set_sensitive(ready && !selected);
        self.button("folder").set_sensitive(ready && !selected);
        self.button("discard").set_sensitive(ready && selected);
        for (name, enabled) in [
            ("files", ready && !selected),
            ("folder", ready && !selected),
            ("send-text", ready && !selected),
            ("send-image", ready && !selected),
            ("discard", ready && selected),
            ("nearby", !preview),
            ("incoming", !preview),
            ("transfers", !preview),
        ] {
            self.actions
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
                .unwrap()
                .set_enabled(enabled);
        }
        self.actions
            .lookup_action("reconnect")
            .and_downcast::<gio::SimpleAction>()
            .unwrap()
            .set_enabled(
                !preview
                    && (self.handle.is_none() || model.state.disconnected)
                    && !model.ingress.busy
                    && model.dialog.is_none()
                    && !model.desktop.staging
                    && !*self.ingress.gate.subscribe().borrow(),
            );
        let unavailable = !preview && (self.handle.is_none() || model.state.disconnected);
        self.reconnect_button.set_visible(unavailable);
        self.box_widget("recovery").set_visible(
            unavailable
                || ((model.state.pending
                    || model.state.text_pending
                    || model.state.visibility_pending
                    || model.desktop.staging)
                    && !model.state.disconnected),
        );
        for id in ["recover", "abort"] {
            self.button(id).set_visible(!unavailable);
        }
        for id in ["recover", "abort"] {
            self.button(id).set_sensitive(
                !preview
                    && !model.waiting
                    && !model.state.busy
                    && (model.state.pending
                        || (id == "recover"
                            && (model.state.text_pending || model.state.visibility_pending))
                        || (id == "abort" && model.desktop.staging))
                    && !model.state.disconnected,
            );
        }
        for id in ["recover", "abort"] {
            self.actions
                .lookup_action(id)
                .and_downcast::<gio::SimpleAction>()
                .unwrap()
                .set_enabled(self.button(id).is_sensitive());
        }
        let summary = model
            .state
            .view
            .as_ref()
            .and_then(|view| view.sources.first())
            .map(|source| source_summary(source.kind, source.file_count, source.byte_count));
        self.text(
            "selection",
            summary
                .as_deref()
                .unwrap_or("Choose files, a folder or text"),
        );
        self.text(
            "badge",
            if preview {
                "Preview"
            } else if model.state.disconnected {
                "Disconnected"
            } else if ready {
                "Ready"
            } else {
                "Waiting"
            },
        );
        let current = model.state.view.as_ref().and_then(|view| {
            model
                .automatic_focus
                .as_deref()
                .and_then(|id| {
                    view.transfers
                        .iter()
                        .find(|transfer| transfer.transfer.id() == id)
                })
                .or_else(|| {
                    compact_index(
                        view.transfers
                            .iter()
                            .map(|transfer| (transfer.direction, transfer.phase)),
                    )
                    .map(|index| &view.transfers[index])
                })
        });
        let status = if preview {
            "Preview only · service actions disabled"
        } else if let Some(notice) = model.notice {
            notice
        } else if self.handle.is_none() {
            "Transfer service unavailable. Check installation, then reconnect."
        } else if model.desktop.staging {
            "Receiving files from the local sender. Abort to release this preparation."
        } else if model.ingress.busy || *self.ingress.gate.subscribe().borrow() {
            "Preparing selection…"
        } else if model.ingress.failed {
            "Preparation failed. Review service state or select accessible files."
        } else if model.state.visibility_pending {
            "Visibility change outcome is uncertain. Use Retry visibility change."
        } else if model.state.error.is_some() {
            error_message(model.state.error)
        } else if model
            .state
            .view
            .as_ref()
            .is_some_and(|view| !view.warnings.is_empty())
        {
            "Visibility was recovered as hidden. Review service settings before enabling discovery."
        } else if model
            .state
            .view
            .as_ref()
            .is_some_and(|view| view.lifecycle == Lifecycle::Stopped)
        {
            "Transfer service is stopped. Open Settings to repair or start it."
        } else if let Some(transfer) = current {
            match transfer.pending_control.map(|(action, _)| action) {
                Some(rqs_lib::service::ControlAction::Accept) => "Accepting request…",
                Some(rqs_lib::service::ControlAction::Reject) => "Declining request…",
                Some(rqs_lib::service::ControlAction::Cancel) => "Cancelling transfer…",
                None => phase_text(transfer.phase),
            }
        } else {
            error_message(None)
        };
        self.text("status", status);
        self.sync_tray_menu();

        let Some(view) = &model.state.view else {
            self.rows.borrow_mut().clear();
            for id in ["nearby", "incoming", "transfers", "compact"] {
                clear(&self.box_widget(id));
                self.box_widget(id).append(&label(if preview {
                    "Preview only"
                } else {
                    "Waiting for complete service state"
                }));
            }
            if !preview {
                let compact = self.box_widget("compact");
                clear(&compact);
                compact.append(&label("Transfer service unavailable"));
                compact.append(&label("Reconnect starts a fresh client. Unsent preparation is released; choose files again."));
            }
            if let Some(page) = model.preview_page.as_deref() {
                self.render_preview(page);
            }
            return;
        };
        self.render_rows(view, selection, ready, current, &model.recipients);
    }
    fn incoming_card(
        self: &Rc<Self>,
        transfer: &TransferView,
        ready: bool,
        full: bool,
    ) -> gtk::Box {
        let row = details_card_mode(&transfer.details, full);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        for (title, accept) in [("Decline", false), ("Accept", true)] {
            let button = gtk::Button::with_label(title);
            button.set_widget_name(if accept { "accept" } else { "decline" });
            button.set_sensitive(
                ready
                    && transfer.pending_control.is_none()
                    && transfer.consent_revision.is_some()
                    && (!accept || transfer.can_accept()),
            );
            let weak = Rc::downgrade(self);
            let transfer = transfer.clone();
            button.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    if let (Some(handle), Some(round)) = (&panel.handle, transfer.consent_revision)
                    {
                        panel.submit(|| handle.respond(transfer.transfer.clone(), round, accept));
                    }
                }
            });
            buttons.append(&button);
        }
        row.append(&buttons);
        row
    }
    fn transfer_card(
        self: &Rc<Self>,
        transfer: &TransferView,
        ready: bool,
        full: bool,
    ) -> gtk::Box {
        let row =
            details_card_mode_with_files(&transfer.details, full, self.file_action(transfer, full));
        if let Some(recipient) = &transfer.recipient {
            row.prepend(&label(&format!("Recipient: {recipient}")));
        }
        let phase = label(phase_text(transfer.phase));
        phase.set_widget_name("phase");
        row.append(&phase);
        if transfer.details.status == DetailsStatus::Ready {
            let progress = label(&format!(
                "{} / {} bytes locally {}",
                transfer.details.transferred_bytes,
                transfer.details.total_bytes,
                if transfer.direction == Direction::Inbound {
                    if transfer.details.kind == rqs_lib::sessions::details::PayloadKind::Files {
                        "written"
                    } else {
                        "received"
                    }
                } else {
                    "sent"
                }
            ));
            progress.set_widget_name("progress");
            row.append(&progress);
        }
        if full && transfer.direction == Direction::Inbound && transfer.phase == Phase::Finished {
            if let Some(revision) = transfer.content_revision {
                let review = gtk::Button::with_label("Review text");
                review.set_widget_name("review-text");
                review.set_sensitive(ready);
                let weak = Rc::downgrade(self);
                let target = transfer.transfer.clone();
                review.connect_clicked(move |_| {
                    if let Some(panel) = weak.upgrade() {
                        if let Some(handle) = &panel.handle {
                            panel.submit(|| handle.read_text(target.clone(), revision));
                        }
                    }
                });
                row.append(&review);
            }
        }
        if full && transfer.reveal_revision.is_some() {
            let cancel = gtk::Button::with_label("Cancel folder request");
            cancel.set_widget_name("cancel-reveal");
            cancel.set_visible(self.revealing(&transfer.transfer));
            let weak = Rc::downgrade(self);
            cancel.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    panel.close_reveal();
                    panel.notice("Folder request cancelled.");
                    panel.render();
                }
            });
            row.append(&cancel);
        }
        if !transfer.phase.is_terminal() {
            let cancel = gtk::Button::with_label("Cancel");
            cancel.set_widget_name("cancel");
            cancel.set_sensitive(
                ready
                    && transfer.pending_control.is_none()
                    && transfer.phase != Phase::CancelRequested,
            );
            let weak = Rc::downgrade(self);
            let transfer = transfer.transfer.clone();
            cancel.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    if let Some(handle) = &panel.handle {
                        panel.submit(|| handle.cancel(transfer.clone()));
                    }
                }
            });
            row.append(&cancel);
        }
        row
    }
    pub fn show_at(&self, point: (i32, i32)) -> bool {
        self.mark_manual();
        self.model.borrow_mut().manual_requested = true;
        self.place_at(point)
    }
    fn place_at(&self, point: (i32, i32)) -> bool {
        let height = self.window.measure(gtk::Orientation::Vertical, 420).0.max(
            if self.model.borrow().mode == Mode::Full {
                497
            } else {
                1
            },
        );
        let monitors = gtk::prelude::WidgetExt::display(&self.window).monitors();
        for monitor in monitors.iter::<gtk::gdk::Monitor>().filter_map(Result::ok) {
            let geometry = monitor.geometry();
            let Some(placement) = placement::place(
                Rect {
                    x: geometry.x(),
                    y: geometry.y(),
                    width: geometry.width(),
                    height: geometry.height(),
                },
                point,
                (420, height),
            ) else {
                continue;
            };
            if self.layer {
                if self.window.monitor().as_ref() != Some(&monitor) {
                    self.window.set_visible(false);
                    self.window.set_monitor(Some(&monitor));
                }
                for edge in [
                    LayerEdge::Left,
                    LayerEdge::Right,
                    LayerEdge::Top,
                    LayerEdge::Bottom,
                ] {
                    self.window.set_anchor(edge, false);
                    self.window.set_margin(edge, 0);
                }
                for (edge, margin) in [
                    (placement.horizontal, placement.horizontal_margin),
                    (placement.vertical, placement.vertical_margin),
                ] {
                    let edge = match edge {
                        Edge::Left => LayerEdge::Left,
                        Edge::Right => LayerEdge::Right,
                        Edge::Top => LayerEdge::Top,
                        Edge::Bottom => LayerEdge::Bottom,
                    };
                    self.window.set_anchor(edge, true);
                    self.window.set_margin(edge, margin);
                }
            }
            self.model.borrow_mut().last_hint = Some(point);
            let model = self.model.borrow();
            if !model.safety_active
                || (model.safety.manual_allowed()
                    && (!model.automatic || model.safety.automatic_allowed()))
            {
                if !model.automatic || !self.window.is_visible() {
                    self.window.present();
                }
            } else {
                self.window.set_visible(false);
            }
            return true;
        }
        self.notice("Tray position is outside available monitors. Activate the tray icon again.");
        false
    }
    pub fn preview(self: &Rc<Self>, page: &str) {
        self.model.borrow_mut().preview_page = Some(page.into());
        self.render_preview(page);
        if page == "incoming" {
            incoming_preview::Fixture::start(self);
        }
    }
    pub fn preview_ready(&self) -> bool {
        self.model.borrow().preview_page.as_deref() != Some("incoming")
            || self.preview_image.finished()
    }
    fn render_preview(&self, page: &str) {
        self.pages.set_visible_child_name(page);
        let nearby = self.box_widget("nearby");
        clear(&nearby);
        for name in ["Example phone", "Example laptop"] {
            let row = card();
            let choice = gtk::CheckButton::with_label(name);
            choice.set_active(true);
            choice.set_sensitive(false);
            row.append(&choice);
            row.append(&label("Nearby advertised device · preview"));
            nearby.append(&row);
        }
        self.text("recipient-summary", "2 selected · preview");
        self.button("send-many").set_label("Send to 2");
        let details = TransferDetails {
            status: DetailsStatus::Ready,
            sender: "Example phone".into(),
            pin: "4821".into(),
            total_bytes: 12345,
            files: vec![rqs_lib::service::client::state::FileDetail {
                name: "Example.pdf".into(),
                size: 12345,
            }]
            .into(),
            ..Default::default()
        };
        let incoming = self.box_widget("incoming");
        clear(&incoming);
        let row = details_card(&details);
        if let Some(picture) = self.preview_image.picture() {
            row.append(&picture);
        }
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        for title in ["Decline", "Accept"] {
            let button = gtk::Button::with_label(title);
            button.set_sensitive(false);
            actions.append(&button);
        }
        row.append(&actions);
        incoming.append(&row);
        let transfers = self.box_widget("transfers");
        clear(&transfers);
        for (recipient, status) in [
            ("Example phone", "2048 / 12345 bytes locally sent"),
            ("Example laptop", "Queued"),
        ] {
            let row = card();
            row.append(&label(&format!("Recipient: {recipient}")));
            row.append(&label(status));
            let cancel = gtk::Button::with_label("Cancel");
            cancel.set_sensitive(false);
            row.append(&cancel);
            transfers.append(&row);
        }
        let compact = self.box_widget("compact");
        clear(&compact);
        match page {
            "incoming" => {
                let row = details_card_mode(&details, false);
                let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                for title in ["Decline", "Accept"] {
                    let button = gtk::Button::with_label(title);
                    button.set_sensitive(false);
                    buttons.append(&button);
                }
                row.append(&buttons);
                let all = gtk::Button::with_label("Details and all requests");
                all.set_action_name(Some("panel.mode"));
                all.set_action_target_value(Some(&"full".to_variant()));
                row.append(&all);
                compact.append(&row);
            }
            "transfers" => {
                let row = card();
                row.append(&label("Recipient: Example phone"));
                row.append(&label("2048 / 12345 bytes locally sent"));
                let cancel = gtk::Button::with_label("Cancel");
                cancel.set_sensitive(false);
                row.append(&cancel);
                compact.append(&row);
            }
            _ => {
                compact.append(&label("Ready to share"));
                let send = gtk::Button::with_label("Send files…");
                send.set_sensitive(false);
                compact.append(&send);
            }
        }
    }
}

fn phase_text(phase: Phase) -> &'static str {
    match phase {
        Phase::Queued => "Queued",
        Phase::Active => "Connecting",
        Phase::AwaitingConsent => "Waiting for consent",
        Phase::Receiving => "Receiving",
        Phase::Sending => "Sending",
        Phase::CancelRequested => "Cancellation requested",
        Phase::Finished => "Completed",
        Phase::Rejected => "Declined",
        Phase::Cancelled => "Cancelled",
        Phase::Failed => "Failed",
        Phase::Disconnected => "Disconnected",
    }
}

#[cfg(test)]
mod tests;
