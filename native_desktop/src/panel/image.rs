//! Explicit bounded clipboard image capture and review; the controller alone prepares sources.
use super::*;
use rqs_lib::{
    clipboard_image::CapturedImage, file_source::OpenedSelection,
    memory_source::MAX_MEMORY_FILE_BYTES,
};
use tokio_util::sync::CancellationToken;

async fn read_bounded(stream: &gio::InputStream) -> Result<Vec<u8>, ()> {
    let mut body = Vec::new();
    loop {
        let count = (64 * 1024).min(MAX_MEMORY_FILE_BYTES + 1 - body.len());
        let bytes = stream
            .read_bytes_future(count, glib::Priority::DEFAULT)
            .await
            .map_err(|_| ())?;
        if bytes.is_empty() {
            break;
        }
        if body
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > MAX_MEMORY_FILE_BYTES)
        {
            return Err(());
        }
        body.try_reserve_exact(bytes.len()).map_err(|_| ())?;
        body.extend_from_slice(&bytes);
    }
    if body.is_empty() {
        Err(())
    } else {
        Ok(body)
    }
}
async fn clipboard_png(clipboard: &gtk::gdk::Clipboard) -> Result<Vec<u8>, ()> {
    let (stream, mime) = clipboard
        .read_future(&["image/png"], glib::Priority::DEFAULT)
        .await
        .map_err(|_| ())?;
    let result = if mime == "image/png" {
        read_bounded(&stream).await
    } else {
        Err(())
    };
    stream
        .close_future(glib::Priority::DEFAULT)
        .await
        .map_err(|_| ())?;
    result
}
struct Worker {
    ingress: Arc<Ingress>,
    runtime: Handle,
}
type Prepare = dyn Fn(OpenedSelection) -> Result<Completion, ControllerError>;
pub(super) struct Image {
    window: gtk::Window,
    picture: gtk::Picture,
    capture: gtk::Button,
    prepare: gtk::Button,
    status: gtk::Label,
    source: RefCell<Option<OpenedSelection>>,
    permit: RefCell<Option<PreparationPermit>>,
    task: RefCell<Option<glib::JoinHandle<()>>>,
    cancel: CancellationToken,
    busy: Cell<bool>,
    closed: Cell<bool>,
    guard: Box<dyn Fn() -> bool>,
    submit: Box<Prepare>,
    prepared: Box<dyn Fn(bool)>,
    worker: Worker,
    epoch: u128,
}
impl Image {
    fn new(
        application: Option<&gtk::Application>,
        epoch: u128,
        permit: PreparationPermit,
        guard: impl Fn() -> bool + 'static,
        submit: impl Fn(OpenedSelection) -> Result<Completion, ControllerError> + 'static,
        prepared: impl Fn(bool) + 'static,
        worker: Worker,
    ) -> Rc<Self> {
        let window = gtk::Window::builder()
            .title("Send image")
            .default_width(480)
            .default_height(420)
            .build();
        if let Some(application) = application {
            window.set_application(Some(application));
        }
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);
        let heading = label("Capture a clipboard image, review it, then choose recipients.");
        heading.set_selectable(false);
        root.append(&heading);
        let picture = gtk::Picture::new();
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_vexpand(true);
        root.append(&picture);
        let status = label("Clipboard is read only when you capture. Nothing is sent yet.");
        status.set_selectable(false);
        root.append(&status);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let capture = gtk::Button::with_label("Capture clipboard image");
        let prepare = gtk::Button::with_label("Prepare image");
        let cancel = gtk::Button::with_label("Cancel");
        buttons.append(&capture);
        buttons.append(&prepare);
        buttons.append(&cancel);
        root.append(&buttons);
        window.set_child(Some(&root));
        let image = Rc::new(Self {
            window,
            picture,
            capture,
            prepare,
            status,
            source: RefCell::new(None),
            permit: RefCell::new(Some(permit)),
            task: RefCell::new(None),
            cancel: CancellationToken::new(),
            busy: Cell::new(false),
            closed: Cell::new(false),
            guard: Box::new(guard),
            submit: Box::new(submit),
            prepared: Box::new(prepared),
            worker,
            epoch,
        });
        let weak = Rc::downgrade(&image);
        image.capture.connect_clicked(move |_| {
            if let Some(image) = weak.upgrade() {
                image.capture_explicitly();
            }
        });
        let weak = Rc::downgrade(&image);
        image.prepare.connect_clicked(move |_| {
            if let Some(image) = weak.upgrade() {
                image.prepare_explicitly();
            }
        });
        let weak = Rc::downgrade(&image);
        cancel.connect_clicked(move |_| {
            if let Some(image) = weak.upgrade() {
                image.close();
            }
        });
        let weak = Rc::downgrade(&image);
        image.window.connect_close_request(move |_| {
            if let Some(image) = weak.upgrade() {
                image.close();
            }
            glib::Propagation::Stop
        });
        image.refresh();
        image
    }
    fn ready(&self) -> bool {
        !self.closed.get() && !self.busy.get() && self.permit.borrow().is_some() && (self.guard)()
    }
    fn refresh(&self) {
        let ready = self.ready();
        self.capture.set_sensitive(ready);
        self.prepare
            .set_sensitive(ready && self.source.borrow().is_some());
    }
    fn show_capture(&self, image: CapturedImage) {
        let texture = gtk::gdk::MemoryTexture::new(
            image.preview_width as i32,
            image.preview_height as i32,
            gtk::gdk::MemoryFormat::R8g8b8a8,
            &glib::Bytes::from_owned(image.preview_rgba),
            image.preview_width as usize * 4,
        );
        self.picture.set_paintable(Some(&texture));
        self.status.set_text(&format!("{} × {} pixels · {} bytes · Clipboard.png\nReview the image, then prepare and choose recipients.", image.width, image.height, image.source.total_bytes()));
        self.source.replace(Some(image.source));
    }
    fn capture_explicitly(self: &Rc<Self>) {
        if !self.ready() {
            return;
        }
        self.busy.set(true);
        self.refresh();
        let clipboard = gtk::prelude::WidgetExt::display(&self.window).clipboard();
        let weak = Rc::downgrade(self);
        let task = glib::spawn_future_local(async move {
            let bytes = clipboard_png(&clipboard).await;
            let Some(image) = weak.upgrade() else {
                return;
            };
            if image.closed.get() || !(image.guard)() {
                image.close();
                return;
            }
            let bytes = match bytes {
                Ok(bytes) => bytes,
                Err(()) => {
                    image.busy.set(false);
                    image.status.set_text("No supported PNG was captured. Use a static image up to 32 MiB and 64 million pixels.");
                    image.refresh();
                    return;
                }
            };
            let Some(permit) = image.permit.borrow_mut().take() else {
                image.close();
                return;
            };
            let receive = image.worker.ingress.start_image(
                &image.worker.runtime,
                bytes,
                image.cancel.clone(),
                permit,
            );
            drop(image);
            let result = receive.await;
            if let Some(image) = weak.upgrade() {
                if image.closed.get() || !(image.guard)() {
                    image.close();
                    return;
                }
                match result {
                    Ok((result, permit)) => {
                        image.permit.replace(Some(permit));
                        match result {
                            Ok(captured) => image.show_capture(captured),
                            Err(_) => image.status.set_text("Image capture failed validation or exceeded resource limits. The previous image is kept."),
                        }
                        image.busy.set(false);
                        image.refresh();
                    }
                    Err(_) => {
                        image.close();
                    }
                }
            }
        });
        self.replace_task(task);
    }
    fn replace_task(&self, task: glib::JoinHandle<()>) {
        if let Some(previous) = self.task.replace(Some(task)) {
            previous.abort();
        }
    }
    fn prepare_explicitly(self: &Rc<Self>) {
        if !self.ready() {
            return;
        }
        let source = self
            .source
            .borrow()
            .as_ref()
            .and_then(|source| source.try_clone().ok());
        let Some(source) = source else {
            self.status
                .set_text("Image source could not be prepared. Capture it again.");
            return;
        };
        let completion = match (self.submit)(source) {
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
        self.replace_task(glib::spawn_future_local(async move {
            let result = completion.wait().await;
            if let Some(image) = weak.upgrade() {
                if image.closed.get() {
                    return;
                }
                let successful = matches!(
                    result,
                    Ok(rqs_lib::service::client::controller::ActionResult::Selection(_))
                );
                image.close();
                (image.prepared)(successful);
            }
        }));
    }
    pub(super) fn visible(&self) -> bool {
        !self.closed.get() && self.window.is_visible()
    }
    pub(super) fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        self.cancel.cancel();
        if let Some(task) = self.task.borrow_mut().take() {
            task.abort();
        }
        self.picture.set_paintable(None::<&gtk::gdk::Paintable>);
        self.source.borrow_mut().take();
        self.permit.borrow_mut().take();
        self.window.destroy();
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        self.close();
    }
}
impl Panel {
    pub(super) fn close_image(&self) {
        if let Some(image) = self.image.borrow_mut().take() {
            image.close();
        }
    }
    pub(super) fn refresh_image(&self) {
        if let Some(image) = self.image.borrow().clone() {
            let safe = {
                let model = self.model.borrow();
                !model.closing
                    && !model.state.disconnected
                    && model.safety.manual_allowed()
                    && model
                        .state
                        .view
                        .as_ref()
                        .is_some_and(|view| view.epoch == image.epoch)
            };
            if !safe || image.closed.get() {
                image.close();
            } else {
                image.refresh();
            }
        }
    }
    pub(super) fn show_image(self: &Rc<Self>) {
        if let Some(image) = self.image.borrow().as_ref().filter(|image| image.visible()) {
            image.window.present();
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
        if self.handle.is_none() {
            return;
        }
        let Some(permit) = self.ingress.gate.try_acquire() else {
            return;
        };
        self.close_text_review();
        let weak = Rc::downgrade(self);
        let submit = Rc::downgrade(self);
        let done = Rc::downgrade(self);
        let image = Image::new(
            self.window.application().as_ref(),
            epoch,
            permit,
            move || {
                weak.upgrade()
                    .is_some_and(|panel| panel.compose_allowed(epoch))
            },
            move |source| {
                submit
                    .upgrade()
                    .and_then(|panel| panel.handle.as_ref().map(|handle| handle.choose(source)))
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
                        panel.notice("Image preparation did not complete. Review Recover or discard the pending preparation.");
                    }
                    panel.set_mode(Mode::Full);
                    panel.pages.set_visible_child_name("nearby");
                    panel.render();
                    panel.apply_presentation();
                }
            },
            Worker {
                ingress: self.ingress.clone(),
                runtime: self.runtime.clone(),
            },
        );
        image.window.set_transient_for(Some(&self.window));
        self.mark_manual();
        image.window.present();
        self.image.replace(Some(image));
        self.render();
    }
}
#[cfg(test)]
pub(super) mod tests;
