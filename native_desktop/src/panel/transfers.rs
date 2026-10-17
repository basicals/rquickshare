//! A separate metadata/history surface; current capabilities stay with the live controller.
use super::*;

pub(super) struct Window {
    window: gtk::Window,
    history: gtk::Box,
    live: gtk::Box,
    notice: gtk::Label,
    clear: gtk::Button,
    rendered: RefCell<Option<(String, Vec<String>)>>,
}
impl Window {
    pub(super) fn new(panel: &Rc<Panel>) -> Rc<Self> {
        let window = gtk::Window::builder()
            .title("Transfers")
            .default_width(860)
            .default_height(560)
            .application(&panel.window.application().expect("application owner"))
            .build();
        window.set_icon_name(Some(crate::tray::ICON_NAME));
        window.insert_action_group("panel", Some(&panel.actions));
        let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = label("Current and completed transfers");
        title.set_selectable(false);
        title.set_hexpand(true);
        header.append(&title);
        let refresh = gtk::Button::with_label("Refresh");
        refresh.set_action_name(Some("panel.journal-refresh"));
        header.append(&refresh);
        let clear = gtk::Button::with_label("Clear completed");
        clear.set_action_name(Some("panel.journal-clear"));
        header.append(&clear);
        root.append(&header);
        let notice = label("");
        root.append(&notice);
        let scroll = panel
            .builder
            .object::<gtk::ScrolledWindow>("transfers-scroll")
            .expect("transfer scroll");
        panel.pages.remove(&scroll);
        scroll.set_child(None::<&gtk::Widget>);
        scroll.set_vexpand(true);
        scroll.set_propagate_natural_height(false);
        scroll.set_min_content_height(0);
        scroll.set_max_content_height(-1);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let live = panel.box_widget("transfers");
        body.append(&live);
        let history = gtk::Box::new(gtk::Orientation::Vertical, 8);
        body.append(&history);
        scroll.set_child(Some(&body));
        root.append(&scroll);
        window.set_child(Some(&root));
        window.connect_close_request(|window| {
            window.set_visible(false);
            glib::Propagation::Stop
        });
        Rc::new(Self {
            window,
            history,
            live,
            notice,
            clear,
            rendered: RefCell::new(None),
        })
    }
    pub(super) fn surface(&self) -> gtk::Window {
        self.window.clone()
    }
    pub(super) fn present(&self) {
        self.window.present();
    }
    pub(super) fn close(&self) {
        self.window.destroy();
    }
    pub(super) fn render(&self, model: &Model) {
        if !self.window.is_visible() {
            return;
        }
        let journal = model.state.journal.as_ref();
        let error = journal.map_or(2, |view| view.error);
        self.live.set_visible(
            model
                .state
                .view
                .as_ref()
                .is_some_and(|v| !v.transfers.is_empty())
                || journal.is_none_or(|v| v.records.is_empty()),
        );
        self.notice.set_text(if model.state.disconnected {"Service disconnected. Live states may be stale; reconnect before controlling a transfer."}else{match error {
            0=>"Completed metadata is kept for up to 30 days and 1000 operations. File and message contents are not stored.",
            1=>"History could not be saved. Sharing is still available. Refresh to retry, or reset history while preserving a recovery copy.",
            2=>"Loading history…",3=>"Saved history is unavailable with this service version.",_=>"History could not be read safely. Refresh to retry; sharing remains available.",
        }});
        self.clear.set_label(if error == 1 {
            "Reset history"
        } else {
            "Clear completed"
        });
        self.clear.set_sensitive(
            !model.waiting
                && !model.state.disconnected
                && journal.is_some_and(|view| view.token.len() == 64),
        );
        let ids = model
            .state
            .view
            .as_ref()
            .map(|v| {
                v.transfers
                    .iter()
                    .map(|t| t.transfer.id().to_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let key = (
            journal.map_or(String::new(), |v| v.token.clone()),
            ids.clone(),
        );
        if self.rendered.borrow().as_ref() == Some(&key) {
            return;
        }
        clear(&self.history);
        if let Some(journal) = journal {
            for record in journal
                .records
                .iter()
                .rev()
                .filter(|r| !ids.contains(&r.id))
            {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                row.set_margin_top(8);
                row.set_margin_bottom(8);
                let peer = label(if record.peer.is_empty() {
                    "Unnamed device"
                } else {
                    &record.peer
                });
                peer.set_wrap(false);
                peer.set_ellipsize(gtk::pango::EllipsizeMode::End);
                peer.set_max_width_chars(24);
                peer.set_hexpand(true);
                row.append(&peer);
                row.append(&label(if record.inbound { "Receive" } else { "Send" }));
                row.append(&label(&format!("{} · {} bytes", record.kind, record.total)));
                let status = label(match record.status.as_str() {
                    "finished" => "Completed",
                    "rejected" => "Declined",
                    "cancelled" => "Cancelled",
                    "failed" => "Failed",
                    "disconnected" => "Disconnected",
                    "interrupted" => "Interrupted",
                    _ => "In progress",
                });
                status.set_width_chars(12);
                row.append(&status);
                let date =
                    glib::DateTime::from_unix_utc(record.updated.min(i64::MAX as u64) as i64)
                        .ok()
                        .and_then(|d| d.to_local().ok())
                        .and_then(|d| d.format("%x %X").ok())
                        .map(|d| d.to_string())
                        .unwrap_or_default();
                row.append(&label(&date));
                row.set_tooltip_text(Some("Metadata only. Content review and file actions require the original live transfer capability."));
                self.history.append(&row);
            }
        }
        *self.rendered.borrow_mut() = Some(key);
    }
}
impl Panel {
    pub(super) fn show_transfers(self: &Rc<Self>) {
        if self.transfers.borrow().is_none() {
            let window = Window::new(self);
            *self.transfers.borrow_mut() = Some(window);
        }
        if let Some(window) = self.transfers.borrow().as_ref() {
            window.present();
            window.render(&self.model.borrow());
        }
    }
}
