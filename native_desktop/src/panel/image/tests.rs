use super::*;
const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 224, 18, 145, 211, 0, 0, 0,
    205, 0, 101, 106, 153, 132, 66, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
fn finish(image: &Image) {
    let task = image.task.borrow_mut().take().unwrap();
    glib::MainContext::default().block_on(task).unwrap();
}
pub(in crate::panel) fn verify_image_smoke(panel: &Rc<Panel>, application: &gtk::Application) {
    // Arrange: isolated clipboard/display and real joined PNG worker.
    let clipboard = gtk::prelude::WidgetExt::display(&panel.window).clipboard();
    let provider =
        gtk::gdk::ContentProvider::for_bytes("image/png", &glib::Bytes::from_static(PNG));
    clipboard.set_content(Some(&provider)).unwrap();
    let guard = Rc::new(Cell::new(true));
    let attempts = Rc::new(Cell::new(0));
    let check = guard.clone();
    let count = attempts.clone();
    let image = Image::new(
        Some(application),
        1,
        panel.ingress.gate.try_acquire().unwrap(),
        move || check.get(),
        move |source| {
            assert_eq!(source.files().len(), 1);
            assert_eq!(source.files()[0].relative_name(), "Clipboard.png");
            assert!(source.files()[0].memory_bytes() > 0);
            count.set(count.get() + 1);
            Err(ControllerError::Busy)
        },
        |_| panic!("Rejected admission cannot complete preparation"),
        Worker {
            ingress: panel.ingress.clone(),
            runtime: panel.runtime.clone(),
        },
    );
    // Act / Assert: opening never captures, prepares or sends.
    image.window.present();
    assert!(image.source.borrow().is_none() && image.picture.paintable().is_none());
    assert!(!image.prepare.is_sensitive());
    assert_eq!(attempts.get(), 0);
    image.capture.emit_clicked();
    finish(&image);
    assert!(image.prepare.is_sensitive());
    assert_eq!(attempts.get(), 0);
    let size = image.source.borrow().as_ref().unwrap().total_bytes();
    let paintable = image.picture.paintable().unwrap();
    assert_eq!(
        (paintable.intrinsic_width(), paintable.intrinsic_height()),
        (1, 1)
    );
    assert!(*panel.ingress.gate.subscribe().borrow());
    // A stale guard cannot prepare; rejected admission keeps the immutable local source.
    guard.set(false);
    image.prepare.emit_clicked();
    assert_eq!(attempts.get(), 0);
    guard.set(true);
    image.prepare.emit_clicked();
    assert_eq!(attempts.get(), 1);
    assert_eq!(image.source.borrow().as_ref().unwrap().total_bytes(), size);
    // Unsupported MIME and invalid PNG preserve the previous snapshot and return the permit.
    clipboard.set_text("unsupported text");
    image.capture.emit_clicked();
    finish(&image);
    assert_eq!(image.source.borrow().as_ref().unwrap().total_bytes(), size);
    let invalid = gtk::gdk::ContentProvider::for_bytes(
        "image/png",
        &glib::Bytes::from_static(b"invalid png"),
    );
    clipboard.set_content(Some(&invalid)).unwrap();
    image.capture.emit_clicked();
    finish(&image);
    assert_eq!(image.source.borrow().as_ref().unwrap().total_bytes(), size);
    assert!(image.prepare.is_sensitive());
    assert_eq!(attempts.get(), 1);
    // EOF and exact byte bounds, independently of clipboard conversion support.
    for (size, accepted) in [
        (0, false),
        (1, true),
        (MAX_MEMORY_FILE_BYTES, true),
        (MAX_MEMORY_FILE_BYTES + 1, false),
    ] {
        let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from_owned(vec![7; size]));
        let result = glib::MainContext::default().block_on(read_bounded(stream.upcast_ref()));
        assert_eq!(result.is_ok(), accepted);
        if let Ok(bytes) = result {
            assert_eq!(bytes.len(), size);
        }
        glib::MainContext::default()
            .block_on(stream.close_future(glib::Priority::DEFAULT))
            .unwrap();
    }
    panel.image.replace(Some(image.clone()));
    let original = panel.model.borrow().state.clone();
    let mut changed = original.clone();
    if let Some(view) = changed.view.as_mut() {
        Arc::make_mut(view).epoch += 1;
    }
    panel.update(changed);
    assert!(
        image.closed.get()
            && image.source.borrow().is_none()
            && image.picture.paintable().is_none()
    );
    assert!(!*panel.ingress.gate.subscribe().borrow());
    panel.update(original);
    let image = Image::new(
        Some(application),
        1,
        panel.ingress.gate.try_acquire().unwrap(),
        || true,
        |_| panic!("Lock must not prepare an image"),
        |_| panic!("Lock must not send"),
        Worker {
            ingress: panel.ingress.clone(),
            runtime: panel.runtime.clone(),
        },
    );
    clipboard.set_content(Some(&provider)).unwrap();
    image.capture.emit_clicked();
    panel.image.replace(Some(image.clone()));
    panel.safety_update(crate::desktop_guard::SafetyState {
        locked: Some(true),
        inhibited: Some(false),
        closed: false,
    });
    assert!(image.closed.get() && image.task.borrow().is_none() && image.source.borrow().is_none());
    assert!(!*panel.ingress.gate.subscribe().borrow());
    assert_eq!(attempts.get(), 1);
}
