//! Private text surface. Only an explicit guarded button owns clipboard content.
use super::*;
use crate::desktop_guard::SafetyState;
use rqs_lib::{
    service::client::text::TextRead,
    sessions::{details::PayloadKind, text::ReceivedText},
};

#[derive(Clone)]
struct TextKey {
    epoch: u128,
    id: String,
    revision: u64,
    kind: PayloadKind,
    bytes: u64,
}
struct Observation<'a> {
    epoch: u128,
    id: &'a str,
    revision: Option<u64>,
    direction: Direction,
    phase: Phase,
    kind: PayloadKind,
    bytes: u64,
}
impl TextKey {
    fn matches(&self, observed: &Observation<'_>) -> bool {
        self.epoch == observed.epoch
            && self.id == observed.id
            && self.revision != 0
            && Some(self.revision) == observed.revision
            && observed.direction == Direction::Inbound
            && observed.phase == Phase::Finished
            && self.kind != PayloadKind::Files
            && self.kind == observed.kind
            && self.bytes == observed.bytes
    }
}
fn permitted(
    key: &TextKey,
    observed: Option<&Observation<'_>>,
    safety: SafetyState,
    ready: bool,
) -> bool {
    ready && safety.manual_allowed() && observed.is_some_and(|observed| key.matches(observed))
}
type Guard = dyn Fn(&TextKey) -> bool;
pub(super) struct Review {
    window: gtk::Window,
    text: gtk::Label,
    copy: gtk::Button,
    status: gtk::Label,
    body: RefCell<Option<Arc<ReceivedText>>>,
    key: TextKey,
    guard: Box<Guard>,
    closed: Cell<bool>,
}
impl Review {
    fn new(
        application: Option<&gtk::Application>,
        key: TextKey,
        body: Arc<ReceivedText>,
        guard: impl Fn(&TextKey) -> bool + 'static,
    ) -> Rc<Self> {
        let window = gtk::Window::builder()
            .title("Received text")
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
        let heading = label(if key.kind == PayloadKind::Url {
            "Received link · plain text"
        } else {
            "Received text"
        });
        heading.set_selectable(false);
        root.append(&heading);
        let text = label(body.text());
        text.set_selectable(false);
        text.set_use_markup(false);
        text.set_yalign(0.0);
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(200)
            .child(&text)
            .build();
        root.append(&scroll);
        let status = label("Nothing has been copied. Use Copy to replace this clipboard.");
        status.set_selectable(false);
        root.append(&status);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let copy = gtk::Button::with_label("Copy");
        copy.set_widget_name("copy-received-text");
        let close = gtk::Button::with_label("Close");
        buttons.append(&copy);
        buttons.append(&close);
        root.append(&buttons);
        window.set_child(Some(&root));
        let review = Rc::new(Self {
            window,
            text,
            copy,
            status,
            body: RefCell::new(Some(body)),
            key,
            guard: Box::new(guard),
            closed: Cell::new(false),
        });
        let weak = Rc::downgrade(&review);
        review.copy.connect_clicked(move |_| {
            if let Some(review) = weak.upgrade() {
                review.copy_explicitly();
            }
        });
        let weak = Rc::downgrade(&review);
        close.connect_clicked(move |_| {
            if let Some(review) = weak.upgrade() {
                review.close();
            }
        });
        let weak = Rc::downgrade(&review);
        review.window.connect_close_request(move |_| {
            if let Some(review) = weak.upgrade() {
                review.close();
            }
            glib::Propagation::Stop
        });
        review.refresh();
        review
    }
    pub(super) fn visible(&self) -> bool {
        !self.closed.get() && self.window.is_visible()
    }
    fn refresh(&self) {
        self.copy
            .set_sensitive(!self.closed.get() && (self.guard)(&self.key));
    }
    fn copy_explicitly(&self) {
        if self.closed.get() || !(self.guard)(&self.key) {
            self.status
                .set_text("Copy unavailable. Unlock and review the current transfer.");
            return;
        }
        let body = self.body.borrow();
        let Some(body) = body.as_ref() else { return };
        let display = gtk::prelude::WidgetExt::display(&self.window);
        let provider = gtk::gdk::ContentProvider::for_value(&body.text().to_value());
        if display.is_closed() || display.clipboard().set_content(Some(&provider)).is_err() {
            self.status
                .set_text("Copy failed. This clipboard did not accept the text.");
            return;
        }
        self.status.set_text("Copied to this clipboard");
    }
    pub(super) fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        self.body.borrow_mut().take();
        self.text.set_text("");
        self.copy.set_sensitive(false);
        self.window.destroy();
    }
}
impl Drop for Review {
    fn drop(&mut self) {
        self.close();
    }
}
impl Panel {
    fn resource_current(&self, key: &TextKey) -> bool {
        let model = self.model.borrow();
        let Some(view) = model.state.view.as_ref().filter(|view| view.synchronized) else {
            return false;
        };
        view.transfers
            .iter()
            .find(|row| row.transfer.id() == key.id)
            .is_some_and(|row| {
                key.matches(&Observation {
                    epoch: view.epoch,
                    id: row.transfer.id(),
                    revision: row.content_revision,
                    direction: row.direction,
                    phase: row.phase,
                    kind: row.details.kind,
                    bytes: row.details.total_bytes,
                })
            })
    }
    fn copy_permitted(&self, key: &TextKey) -> bool {
        let model = self.model.borrow();
        let observed = model
            .state
            .view
            .as_ref()
            .filter(|view| view.synchronized)
            .and_then(|view| {
                view.transfers
                    .iter()
                    .find(|row| row.transfer.id() == key.id)
                    .map(|row| Observation {
                        epoch: view.epoch,
                        id: row.transfer.id(),
                        revision: row.content_revision,
                        direction: row.direction,
                        phase: row.phase,
                        kind: row.details.kind,
                        bytes: row.details.total_bytes,
                    })
            });
        permitted(
            key,
            observed.as_ref(),
            model.safety,
            !model.closing
                && !model.preview
                && !model.waiting
                && !model.state.busy
                && !model.state.disconnected
                && model.dialog.is_none()
                && !model.ingress.busy
                && !*self.ingress.gate.subscribe().borrow(),
        )
    }
    pub(super) fn close_text_review(&self) {
        self.text_generation.replace(Rc::new(()));
        if let Some(review) = self.text_review.borrow_mut().take() {
            review.close();
        }
    }
    pub(super) fn refresh_text_review(&self) {
        let current = self.text_review.borrow().clone();
        if let Some(review) = current {
            let model = self.model.borrow();
            let safe = !model.closing && !model.state.disconnected && model.safety.manual_allowed();
            drop(model);
            if !safe || !self.resource_current(&review.key) || review.closed.get() {
                self.close_text_review();
            } else {
                review.refresh();
            }
        }
    }
    pub(super) fn show_text_review(self: &Rc<Self>, read: TextRead) {
        let key = TextKey {
            epoch: read.epoch,
            id: read.transfer.id().into(),
            revision: read.revision,
            kind: read.body.kind(),
            bytes: read.body.text().len() as u64,
        };
        let safe = {
            let model = self.model.borrow();
            !model.closing
                && !model.preview
                && !model.state.disconnected
                && model.safety.manual_allowed()
        };
        if !safe || !self.resource_current(&key) {
            self.notice(
                "Text is no longer available for review. Unlock and check the current transfer.",
            );
            return;
        }
        self.close_text_review();
        let weak = Rc::downgrade(self);
        let review = Review::new(
            self.window.application().as_ref(),
            key,
            read.body,
            move |key| {
                weak.upgrade()
                    .is_some_and(|panel| panel.copy_permitted(key))
            },
        );
        review.window.set_transient_for(Some(&self.window));
        self.mark_manual();
        review.window.present();
        self.text_review.replace(Some(review));
    }
}

#[cfg(test)]
pub(super) mod tests;
