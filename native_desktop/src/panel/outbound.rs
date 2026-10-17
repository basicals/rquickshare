//! Explicit input intents bind one endpoint; rendering can never start a send.
use super::*;
use rqs_lib::service::client::{controller::ActionResult, Selection};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

const URI_BYTES: usize = 256 * 1024;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Target {
    pub epoch: u128,
    pub endpoint: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    Files,
    Folder,
    Text,
    Clipboard,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    Files,
    Image,
    Text,
}
fn format(files: bool, image: bool, text: bool) -> Option<Format> {
    if files {
        Some(Format::Files)
    } else if image {
        Some(Format::Image)
    } else if text {
        Some(Format::Text)
    } else {
        None
    }
}
fn paths(body: &[u8], gnome: bool) -> Result<Vec<PathBuf>, ()> {
    if body.len() > URI_BYTES || body.contains(&0) {
        return Err(());
    }
    let text = std::str::from_utf8(body).map_err(|_| ())?;
    let mut lines = text.lines();
    if gnome && !matches!(lines.next(), Some("copy" | "cut")) {
        return Err(());
    }
    let mut result = Vec::new();
    for line in lines
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        if result.len() >= 256 || !line.starts_with("file://") {
            return Err(());
        }
        let authority = line
            .strip_prefix("file://")
            .and_then(|rest| rest.split_once('/'))
            .map(|(host, _)| host)
            .ok_or(())?;
        if !authority.is_empty() && !authority.eq_ignore_ascii_case("localhost") {
            return Err(());
        }
        let mut encoded = line.bytes();
        while let Some(byte) = encoded.next() {
            if byte == b'%'
                && (!encoded.next().is_some_and(|b| b.is_ascii_hexdigit())
                    || !encoded.next().is_some_and(|b| b.is_ascii_hexdigit()))
            {
                return Err(());
            }
        }
        let path = gio::File::for_uri(line).path().ok_or(())?;
        if !path.is_absolute() {
            return Err(());
        }
        result.push(path);
    }
    if result.is_empty() {
        Err(())
    } else {
        Ok(result)
    }
}
async fn clipboard_paths(clipboard: &gtk::gdk::Clipboard) -> Result<Vec<PathBuf>, ()> {
    let (stream, mime) = clipboard
        .read_future(
            &["x-special/gnome-copied-files", "text/uri-list"],
            glib::Priority::DEFAULT,
        )
        .await
        .map_err(|_| ())?;
    let read = async {
        let mut bytes = Vec::new();
        loop {
            let chunk = stream
                .read_bytes_future(
                    (URI_BYTES + 1 - bytes.len()).min(8192),
                    glib::Priority::DEFAULT,
                )
                .await
                .map_err(|_| ())?;
            if chunk.is_empty() {
                break;
            }
            bytes.extend_from_slice(&chunk);
            if bytes.len() > URI_BYTES {
                return Err(());
            }
        }
        paths(&bytes, mime == "x-special/gnome-copied-files")
    }
    .await;
    stream
        .close_future(glib::Priority::DEFAULT)
        .await
        .map_err(|_| ())?;
    read
}

impl Panel {
    pub(super) fn close_input(&self) {
        self.input_generation.replace(Rc::new(()));
        if let Some(cancel) = self.input_cancel.borrow_mut().take() {
            cancel.cancel();
        }
        if let Some(task) = self.input_task.borrow_mut().take() {
            task.abort();
        }
        let mut model = self.model.borrow_mut();
        model.input_active = false;
        model.strip_target = None;
    }
    fn input_allowed(
        &self,
        epoch: u128,
        target: Option<&Target>,
        selection: Option<&Selection>,
    ) -> bool {
        let model = self.model.borrow();
        let state = self
            .handle
            .as_ref()
            .map(|h| h.subscribe().borrow().clone())
            .unwrap_or_else(|| model.state.clone());
        !model.closing
            && !model.preview
            && (!model.safety_active || model.safety.manual_allowed())
            && !state.disconnected
            && !state.busy
            && !state.pending
            && !state.text_pending
            && !state.settings_pending
            && state.view.as_ref().is_some_and(|view| {
                view.epoch == epoch
                    && view.synchronized
                    && view.lifecycle == Lifecycle::Running
                    && target.is_none_or(|t| {
                        t.epoch == epoch && view.endpoints.iter().any(|peer| peer.id == t.endpoint)
                    })
                    && selection.map_or_else(
                        || view.selections.is_empty(),
                        |s| view.selections.contains(s),
                    )
            })
    }
    pub(super) fn input_for(self: &Rc<Self>, kind: Kind, target: Target) {
        if !self.input_allowed(target.epoch, Some(&target), None)
            || *self.ingress.gate.subscribe().borrow()
        {
            return;
        }
        self.model.borrow_mut().strip_target = None;
        self.render();
        match kind {
            Kind::Files | Kind::Folder => self.choose_for(kind == Kind::Folder, Some(target)),
            Kind::Text => self.show_composer_for(Some(target)),
            Kind::Clipboard => self.capture_clipboard(Some(target)),
        }
    }
    pub(super) fn finish_prepared(
        self: &Rc<Self>,
        epoch: u128,
        target: Option<Target>,
        generation: Rc<()>,
        selection: Option<Selection>,
    ) {
        if !Rc::ptr_eq(&generation, &self.input_generation.borrow()) {
            return;
        }
        let Some(selection) = selection else {
            self.notice("Content preparation did not complete. Review Recover or discard the pending preparation.");
            self.render();
            return;
        };
        if let Some(handle) = &self.handle {
            let mut model = self.model.borrow_mut();
            model.state = handle.subscribe().borrow().clone();
            // The reply follows joined acquisition; cached UI watch delivery may lag it.
            model.ingress = *self.ingress.subscribe().borrow();
        }
        if !self.input_allowed(epoch, target.as_ref(), Some(&selection)) {
            self.notice("The target or connection changed. Content was kept; choose a device explicitly to send it.");
            self.render();
            return;
        }
        self.model.borrow_mut().strip_target = None;
        if let Some(target) = target {
            if let Some(handle) = &self.handle {
                self.submit(|| handle.send(selection, target.endpoint));
            }
        } else {
            self.set_drawer(Drawer::Open);
            self.pages.set_visible_child_name("nearby");
            self.render();
            self.apply_presentation();
        }
    }
    pub(super) fn wait_selection(
        self: &Rc<Self>,
        receive: tokio::sync::oneshot::Receiver<Option<Selection>>,
        epoch: u128,
        target: Option<Target>,
        generation: Rc<()>,
    ) {
        self.model.borrow_mut().input_active = true;
        let weak = Rc::downgrade(self);
        let task = glib::spawn_future_local(async move {
            let selection = receive.await.unwrap_or(None);
            if let Some(panel) = weak.upgrade() {
                panel.model.borrow_mut().input_active = false;
                panel.finish_prepared(epoch, target, generation, selection);
            }
        });
        if let Some(previous) = self.input_task.replace(Some(task)) {
            previous.abort();
        }
        self.render();
    }
    pub(super) fn capture_clipboard(self: &Rc<Self>, target: Option<Target>) {
        self.sending.popdown();
        let Some(epoch) = self.model.borrow().state.view.as_ref().map(|v| v.epoch) else {
            return;
        };
        if !self.input_allowed(epoch, target.as_ref(), None) {
            return;
        }
        let Some(permit) = self.ingress.gate.try_acquire() else {
            return;
        };
        let Some(handle) = self.handle.clone() else {
            return;
        };
        let generation = self.input_generation.borrow().clone();
        let cancel = CancellationToken::new();
        self.input_cancel.replace(Some(cancel.clone()));
        self.model.borrow_mut().input_active = true;
        let clipboard = gtk::prelude::WidgetExt::display(&self.window).clipboard();
        let formats = clipboard.formats();
        let selected = format(
            formats.contain_mime_type("text/uri-list")
                || formats.contain_mime_type("x-special/gnome-copied-files")
                || formats.contains_type(gtk::gdk::FileList::static_type()),
            formats
                .mime_types()
                .iter()
                .any(|mime| mime.starts_with("image/"))
                || formats.contains_type(gtk::gdk::Texture::static_type()),
            formats.contain_mime_type("text/plain")
                || formats.contain_mime_type("text/plain;charset=utf-8")
                || formats.contains_type(String::static_type()),
        );
        let weak = Rc::downgrade(self);
        let ingress = self.ingress.clone();
        let runtime = self.runtime.clone();
        let task = glib::spawn_future_local(async move {
            let safe = || {
                weak.upgrade().is_some_and(|p| {
                    Rc::ptr_eq(&generation, &p.input_generation.borrow())
                        && p.input_allowed(epoch, target.as_ref(), None)
                })
            };
            let prepared = async {
                let completion = match selected {
                    Some(Format::Files) => {
                        let paths = clipboard_paths(&clipboard).await?;
                        if !safe() {
                            return Err(());
                        }
                        return ingress
                            .start(&runtime, paths, handle.clone(), permit, None)
                            .await
                            .map_err(|_| ())?
                            .ok_or(());
                    }
                    Some(Format::Image) => {
                        let bytes = image::clipboard_png(&clipboard).await?;
                        if !safe() {
                            return Err(());
                        }
                        let (captured, permit) = ingress
                            .start_image(&runtime, bytes, cancel, permit)
                            .await
                            .map_err(|_| ())?;
                        let captured = captured.map_err(|_| ())?;
                        if !safe() {
                            return Err(());
                        }
                        let completion = handle.choose(captured.source).map_err(|_| ())?;
                        let result = completion.wait().await.map_err(|_| ());
                        drop(permit);
                        return match result? {
                            ActionResult::Selection(selection) => Ok(selection),
                            _ => Err(()),
                        };
                    }
                    Some(Format::Text) => {
                        let text = compose::clipboard_text(&clipboard).await?;
                        if !safe() {
                            return Err(());
                        }
                        handle.choose_text(text).map_err(|_| ())?
                    }
                    None => return Err(()),
                };
                let result = completion.wait().await.map_err(|_| ());
                drop(permit);
                match result? {
                    ActionResult::Selection(selection) => Ok(selection),
                    _ => Err(()),
                }
            }
            .await;
            if let Some(panel) = weak.upgrade() {
                panel.model.borrow_mut().input_active = false;
                if prepared.is_err() && Rc::ptr_eq(&generation, &panel.input_generation.borrow()) {
                    panel.notice("Clipboard content is unavailable, unsupported or exceeds limits. Nothing was sent; review any pending preparation before retrying.");
                    panel.render();
                } else {
                    panel.finish_prepared(epoch, target, generation, prepared.ok());
                }
            }
        });
        if let Some(previous) = self.input_task.replace(Some(task)) {
            previous.abort();
        }
        self.render();
    }
}

#[cfg(test)]
pub(super) fn verify(panel: &Rc<Panel>) {
    // Arrange: actual checked panel guard, no transport or domain substitute.
    let saved = panel.model.borrow().state.clone();
    let saved_safety = panel.model.borrow().safety_active;
    panel.model.borrow_mut().safety_active = false;
    let target = Target {
        epoch: 123,
        endpoint: "device".into(),
    };
    assert!(panel.input_allowed(123, Some(&target), None));
    assert!(!panel.input_allowed(124, Some(&target), None));
    assert!(!panel.input_allowed(
        123,
        Some(&Target {
            epoch: 124,
            ..target.clone()
        }),
        None
    ));
    assert!(!panel.input_allowed(
        123,
        Some(&Target {
            endpoint: "missing".into(),
            ..target.clone()
        }),
        None
    ));
    panel.model.borrow_mut().state.disconnected = true;
    assert!(!panel.input_allowed(123, Some(&target), None));
    panel.model.borrow_mut().state = saved.clone();
    panel.model.borrow_mut().state.pending = true;
    assert!(!panel.input_allowed(123, Some(&target), None));
    panel.model.borrow_mut().state = saved;
    panel.set_drawer(Drawer::Open);
    panel.send_to("device");
    fn revealer(root: &gtk::Widget) -> Option<gtk::Revealer> {
        if root.widget_name() == "send-strip" {
            return root.clone().downcast().ok();
        }
        let mut next = root.first_child();
        while let Some(child) = next {
            next = child.next_sibling();
            if let Some(found) = revealer(&child) {
                return Some(found);
            }
        }
        None
    }
    let strip = revealer(panel.box_widget("nearby").upcast_ref()).unwrap();
    assert!(strip.reveals_child());
    assert!(!panel.sending.is_visible());
    panel.send_to("device");
    assert!(!strip.reveals_child());
    assert!(!panel.sending.is_visible());
    panel.model.borrow_mut().safety_active = saved_safety;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_overlapping_formats_when_selected_then_files_win_before_image_and_text_without_fallback(
    ) {
        for files in [false, true] {
            for image in [false, true] {
                for text in [false, true] {
                    let expected = match (files, image, text) {
                        (true, _, _) => Some(Format::Files),
                        (false, true, _) => Some(Format::Image),
                        (false, false, true) => Some(Format::Text),
                        _ => None,
                    };
                    assert_eq!(format(files, image, text), expected);
                }
            }
        }
    }
    #[test]
    fn given_copied_file_lists_when_parsed_then_local_roots_are_bounded_and_remote_or_malformed_entries_fail(
    ) {
        for (body, gnome, expected) in [
            ("file:///tmp/a%20b\r\nfile:///tmp/c\r\n", false, true),
            ("copy\nfile:///tmp/a", true, true),
            ("cut\nfile:///tmp/a", true, true),
            ("# comment\nfile:///tmp/a", false, true),
            ("", false, false),
            ("copy\n", true, false),
            ("move\nfile:///tmp/a", true, false),
            ("https://example.com/file", false, false),
            ("/tmp/a", false, false),
            ("file:///tmp/a\nhttps://example.com/file", false, false),
            ("file:///tmp/a\0", false, false),
            ("file:///tmp/%zz", false, false),
            ("file://remote-server/tmp/a", false, false),
            ("file://localhost/tmp/a", false, true),
            ("file://user@localhost/tmp/a", false, false),
            ("file://localhost:1/tmp/a", false, false),
        ] {
            assert_eq!(paths(body.as_bytes(), gnome).is_ok(), expected, "{body:?}");
        }
        for count in [1, 255, 256, 257] {
            let body = "file:///tmp/a\n".repeat(count);
            assert_eq!(paths(body.as_bytes(), false).is_ok(), count <= 256);
        }
        assert!(paths(&vec![b'x'; URI_BYTES + 1], false).is_err());
        assert!(paths(&[0xff], false).is_err());
        assert_eq!(
            paths(b"file:///tmp/a%20b", false).unwrap(),
            vec![PathBuf::from("/tmp/a b")]
        );
    }
}
