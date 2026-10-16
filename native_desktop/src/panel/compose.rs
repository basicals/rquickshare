//! One private text editor; clipboard capture and preparation require explicit actions.
use super::*;
use rqs_lib::{prepared_source::NativeText, sessions::text::MAX_TEXT_BYTES};

fn insertion_allowed(current: usize, inserted: &str) -> bool {
    !inserted.contains('\0')
        && current
            .checked_add(inserted.len())
            .is_some_and(|bytes| bytes <= MAX_TEXT_BYTES)
}
async fn read_bounded(stream: &gio::InputStream) -> Result<NativeText, ()> {
    let mut body = Vec::new();
    loop {
        let count = (16 * 1024).min(MAX_TEXT_BYTES + 1 - body.len());
        let chunk = stream
            .read_bytes_future(count, glib::Priority::DEFAULT)
            .await
            .map_err(|_| ())?;
        if chunk.is_empty() {
            break;
        }
        body.extend_from_slice(&chunk);
        if body.len() > MAX_TEXT_BYTES {
            return Err(());
        }
    }
    NativeText::new(String::from_utf8(body).map_err(|_| ())?).ok_or(())
}
async fn clipboard_text(clipboard: &gtk::gdk::Clipboard) -> Result<NativeText, ()> {
    let (stream, mime) = clipboard
        .read_future(
            &["text/plain;charset=utf-8", "text/plain"],
            glib::Priority::DEFAULT,
        )
        .await
        .map_err(|_| ())?;
    let result = if mime == "text/plain;charset=utf-8" || mime == "text/plain" {
        read_bounded(&stream).await
    } else {
        Err(())
    };
    let closed = stream.close_future(glib::Priority::DEFAULT).await;
    if closed.is_err() {
        return Err(());
    }
    result
}
type Prepare = dyn Fn(NativeText) -> Result<Completion, ControllerError>;
pub(super) struct Composer {
    window: gtk::Window,
    buffer: gtk::TextBuffer,
    editor: gtk::TextView,
    paste: gtk::Button,
    prepare: gtk::Button,
    status: gtk::Label,
    permit: RefCell<Option<PreparationPermit>>,
    task: RefCell<Option<glib::JoinHandle<()>>>,
    busy: Cell<bool>,
    closed: Cell<bool>,
    guard: Box<dyn Fn() -> bool>,
    submit: Box<Prepare>,
    prepared: Box<dyn Fn(bool)>,
    epoch: u128,
}
impl Composer {
    fn new(
        application: Option<&gtk::Application>,
        epoch: u128,
        permit: PreparationPermit,
        guard: impl Fn() -> bool + 'static,
        submit: impl Fn(NativeText) -> Result<Completion, ControllerError> + 'static,
        prepared: impl Fn(bool) + 'static,
    ) -> Rc<Self> {
        let window = gtk::Window::builder()
            .title("Send text")
            .default_width(480)
            .default_height(360)
            .build();
        if let Some(application) = application {
            window.set_application(Some(application));
        }
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);
        let heading = label("Write or paste text, then prepare it and choose recipients.");
        heading.set_selectable(false);
        root.append(&heading);
        let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        buffer.set_enable_undo(false);
        let editor = gtk::TextView::with_buffer(&buffer);
        editor.set_wrap_mode(gtk::WrapMode::WordChar);
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .min_content_height(200)
            .child(&editor)
            .build();
        root.append(&scroll);
        let status = label("Clipboard is read only when you paste. Nothing is sent yet.");
        status.set_selectable(false);
        root.append(&status);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let paste = gtk::Button::with_label("Paste clipboard");
        let prepare = gtk::Button::with_label("Prepare text");
        let cancel = gtk::Button::with_label("Cancel");
        buttons.append(&paste);
        buttons.append(&prepare);
        buttons.append(&cancel);
        root.append(&buttons);
        window.set_child(Some(&root));
        let composer = Rc::new(Self {
            window,
            buffer,
            editor,
            paste,
            prepare,
            status,
            permit: RefCell::new(Some(permit)),
            task: RefCell::new(None),
            busy: Cell::new(false),
            closed: Cell::new(false),
            guard: Box::new(guard),
            submit: Box::new(submit),
            prepared: Box::new(prepared),
            epoch,
        });
        let weak = Rc::downgrade(&composer);
        composer.buffer.connect_insert_text(move |buffer, _, text| {
            let current = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .len();
            if !insertion_allowed(current, text) {
                buffer.stop_signal_emission_by_name("insert-text");
                if let Some(composer) = weak.upgrade() {
                    composer
                        .status
                        .set_text("Text must be UTF-8 without NUL and at most 128 KiB.");
                }
            }
        });
        let weak = Rc::downgrade(&composer);
        composer.buffer.connect_changed(move |_| {
            if let Some(composer) = weak.upgrade() {
                composer.refresh();
            }
        });
        let weak = Rc::downgrade(&composer);
        composer.editor.connect_paste_clipboard(move |editor| {
            editor.stop_signal_emission_by_name("paste-clipboard");
            if let Some(composer) = weak.upgrade() {
                composer.paste_explicitly();
            }
        });
        // Disable the independent PRIMARY middle-click paste path.
        let middle = gtk::GestureClick::new();
        middle.set_button(2);
        middle.set_propagation_phase(gtk::PropagationPhase::Capture);
        middle.connect_pressed(|gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
        });
        composer.editor.add_controller(middle);
        let weak = Rc::downgrade(&composer);
        composer.paste.connect_clicked(move |_| {
            if let Some(composer) = weak.upgrade() {
                composer.paste_explicitly();
            }
        });
        let weak = Rc::downgrade(&composer);
        composer.prepare.connect_clicked(move |_| {
            if let Some(composer) = weak.upgrade() {
                composer.prepare_explicitly();
            }
        });
        let weak = Rc::downgrade(&composer);
        cancel.connect_clicked(move |_| {
            if let Some(composer) = weak.upgrade() {
                composer.close();
            }
        });
        let weak = Rc::downgrade(&composer);
        composer.window.connect_close_request(move |_| {
            if let Some(composer) = weak.upgrade() {
                composer.close();
            }
            glib::Propagation::Stop
        });
        composer.refresh();
        composer
    }
    fn ready(&self) -> bool {
        !self.closed.get() && !self.busy.get() && self.permit.borrow().is_some() && (self.guard)()
    }
    fn refresh(&self) {
        let ready = self.ready();
        self.editor.set_editable(ready);
        self.paste.set_sensitive(ready);
        self.prepare
            .set_sensitive(ready && self.buffer.char_count() != 0);
    }
    fn body(&self) -> Option<NativeText> {
        NativeText::new(
            self.buffer
                .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false)
                .to_string(),
        )
    }
    fn paste_explicitly(self: &Rc<Self>) {
        if !self.ready() {
            return;
        }
        self.busy.set(true);
        self.refresh();
        let clipboard = gtk::prelude::WidgetExt::display(&self.window).clipboard();
        let weak = Rc::downgrade(self);
        let task = glib::spawn_future_local(async move {
            let result = clipboard_text(&clipboard).await;
            if let Some(composer) = weak.upgrade() {
                composer.busy.set(false);
                if composer.closed.get() || !(composer.guard)() {
                    composer.close();
                    return;
                }
                match result {
                    Ok(text) => {
                        composer.buffer.set_text(text.text());
                        composer.status.set_text(
                            "Text captured. Review it, then prepare and choose recipients.",
                        );
                    }
                    Err(()) => composer.status.set_text(
                        "No supported text was captured. Use 1–128 KiB of UTF-8 text without NUL.",
                    ),
                }
                composer.refresh();
            }
        });
        if let Some(previous) = self.task.replace(Some(task)) {
            previous.abort();
        }
    }
    fn prepare_explicitly(self: &Rc<Self>) {
        if !self.ready() {
            return;
        }
        let Some(text) = self.body() else {
            self.status.set_text("Enter 1–128 KiB of text without NUL.");
            return;
        };
        let completion = match (self.submit)(text) {
            Ok(completion) => completion,
            Err(_) => {
                self.status.set_text(
                    "Preparation was not queued. Review the current source and connection.",
                );
                return;
            }
        };
        self.busy.set(true);
        self.refresh();
        let weak = Rc::downgrade(self);
        let task = glib::spawn_future_local(async move {
            let result = completion.wait().await;
            if let Some(composer) = weak.upgrade() {
                if composer.closed.get() {
                    return;
                }
                let successful = matches!(
                    result,
                    Ok(rqs_lib::service::client::controller::ActionResult::Selection(_))
                );
                composer.close();
                (composer.prepared)(successful);
            }
        });
        if let Some(previous) = self.task.replace(Some(task)) {
            previous.abort();
        }
    }
    pub(super) fn visible(&self) -> bool {
        !self.closed.get() && self.window.is_visible()
    }
    pub(super) fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        if let Some(task) = self.task.borrow_mut().take() {
            task.abort();
        }
        self.buffer.set_text("");
        self.permit.borrow_mut().take();
        self.window.destroy();
    }
}
impl Drop for Composer {
    fn drop(&mut self) {
        self.close();
    }
}
impl Panel {
    pub(super) fn compose_allowed(&self, epoch: u128) -> bool {
        let model = self.model.borrow();
        !model.closing
            && !model.preview
            && model.safety.manual_allowed()
            && !model.state.disconnected
            && !model.state.pending
            && !model.state.text_pending
            && !model.state.settings_pending
            && !model.state.busy
            && !model.waiting
            && model.dialog.is_none()
            // This editor already owns the shared permit. Its own worker may publish
            // idle asynchronously; the cached ingress projection is not authority.
            && model.state.view.as_ref().is_some_and(|view| {
                view.epoch == epoch
                    && view.synchronized
                    && view.lifecycle == Lifecycle::Running
                    && view.selections.is_empty()
            })
    }
    pub(super) fn close_composer(&self) {
        if let Some(composer) = self.composer.borrow_mut().take() {
            composer.close();
        }
    }
    pub(super) fn refresh_composer(&self) {
        let current = self.composer.borrow().clone();
        if let Some(composer) = current {
            let model = self.model.borrow();
            let safe = !model.closing
                && !model.state.disconnected
                && model.safety.manual_allowed()
                && model
                    .state
                    .view
                    .as_ref()
                    .is_some_and(|view| view.epoch == composer.epoch);
            drop(model);
            if !safe || composer.closed.get() {
                self.close_composer();
            } else {
                composer.refresh();
            }
        }
    }
    pub(super) fn show_composer(self: &Rc<Self>) {
        if let Some(composer) = self
            .composer
            .borrow()
            .as_ref()
            .filter(|composer| composer.visible())
        {
            composer.window.present();
            return;
        }
        let epoch = self
            .model
            .borrow()
            .state
            .view
            .as_ref()
            .map(|view| view.epoch);
        let Some(epoch) = epoch.filter(|epoch| self.compose_allowed(*epoch)) else {
            return;
        };
        if self.handle.is_none() || *self.ingress.gate.subscribe().borrow() {
            return;
        }
        let Some(permit) = self.ingress.gate.try_acquire() else {
            return;
        };
        self.close_text_review();
        let weak = Rc::downgrade(self);
        let submit = Rc::downgrade(self);
        let done = Rc::downgrade(self);
        let composer = Composer::new(
            self.window.application().as_ref(),
            epoch,
            permit,
            move || {
                weak.upgrade()
                    .is_some_and(|panel| panel.compose_allowed(epoch))
            },
            move |text| {
                submit
                    .upgrade()
                    .and_then(|panel| panel.handle.as_ref().map(|handle| handle.choose_text(text)))
                    .unwrap_or(Err(ControllerError::Closed))
            },
            move |successful| {
                if let Some(panel) = done.upgrade() {
                    let safe = {
                        let model = panel.model.borrow();
                        !model.closing
                            && model.safety.manual_allowed()
                            && model
                                .state
                                .view
                                .as_ref()
                                .is_some_and(|view| view.epoch == epoch)
                    };
                    if !safe {
                        return;
                    }
                    if !successful {
                        panel.notice("Text preparation did not complete. Review Recover or discard the pending preparation.");
                    }
                    panel.set_drawer(Drawer::Open);
                    panel.pages.set_visible_child_name("nearby");
                    panel.render();
                    panel.apply_presentation();
                }
            },
        );
        composer.window.set_transient_for(Some(&self.window));
        self.mark_manual();
        composer.window.present();
        self.composer.replace(Some(composer));
        self.render();
    }
}

#[cfg(test)]
pub(super) mod tests;
