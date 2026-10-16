//! Offline card fixture. Peer image routing/IPC is deliberately not inferred.
use super::*;
use rquickshare_desktop::preview::{Decoder, Pixels};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub(super) struct Fixture {
    pixels: RefCell<Option<Pixels>>,
    done: Cell<bool>,
    cancel: CancellationToken,
    task: RefCell<Option<glib::JoinHandle<()>>>,
}
impl Fixture {
    pub(super) fn finished(&self) -> bool {
        self.done.get()
    }
    pub(super) fn cancel(&self) {
        self.cancel.cancel();
        if let Some(task) = self.task.borrow_mut().take() {
            task.abort();
        }
        self.pixels.borrow_mut().take();
    }
    pub(super) fn start(panel: &Rc<Panel>) {
        if panel.preview_image.done.get() || panel.preview_image.task.borrow().is_some() {
            return;
        }
        let decoder = match Decoder::installed() {
            Ok(decoder) => decoder,
            Err(_) => {
                panel.preview_image.done.set(true);
                return;
            }
        };
        let cancel = panel.preview_image.cancel.clone();
        let worker = panel.runtime.spawn(async move {
            decoder
                .decode(
                    include_bytes!("../../assets/preview-fixture.png").to_vec(),
                    cancel,
                )
                .await
        });
        let weak = Rc::downgrade(panel);
        let task = glib::spawn_future_local(async move {
            let result = worker.await;
            let Some(panel) = weak.upgrade() else {
                return;
            };
            if panel.model.borrow().closing || panel.preview_image.cancel.is_cancelled() {
                return;
            }
            panel.preview_image.done.set(true);
            if let Ok(Ok(pixels)) = result {
                panel.preview_image.pixels.replace(Some(pixels));
            }
            let page = panel.model.borrow().preview_page.clone();
            if let Some(page) = page {
                panel.render_preview(&page);
            }
        });
        panel.preview_image.task.replace(Some(task));
    }
    pub(super) fn picture(&self) -> Option<gtk::Picture> {
        let pixels = self.pixels.borrow();
        let pixels = pixels.as_ref()?;
        let texture = gtk::gdk::MemoryTexture::new(
            pixels.width as i32,
            pixels.height as i32,
            gtk::gdk::MemoryFormat::R8g8b8a8,
            &glib::Bytes::from_owned(pixels.rgba.clone()),
            pixels.width as usize * 4,
        );
        let picture = gtk::Picture::for_paintable(&texture);
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_size_request(128, 64);
        Some(picture)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(task) = self.task.get_mut().take() {
            task.abort();
        }
    }
}
