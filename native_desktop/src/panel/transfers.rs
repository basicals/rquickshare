//! A separate metadata/history surface; current capabilities stay with the live controller.
use super::*;

/// Shared measured columns across live capabilities and persisted metadata rows.
#[derive(Clone)]
pub(super) struct Columns([gtk::SizeGroup; 5]);
impl Default for Columns {
    fn default() -> Self {
        Self(std::array::from_fn(|_| {
            gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal)
        }))
    }
}
impl Columns {
    pub(super) fn row(&self, cells: [&gtk::Widget; 6]) -> gtk::Box {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        row.set_height_request(36);
        row.set_widget_name("history-row");
        row.add_css_class("history-row");
        for (index, child) in cells.into_iter().enumerate() {
            let cell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            cell.set_widget_name(&format!("history-column-{index}"));
            cell.add_css_class("history-cell");
            cell.set_valign(gtk::Align::Fill);
            cell.set_hexpand(index == 0);
            child.set_valign(gtk::Align::Center);
            if index == 5 {
                // Preserve an action slot even when every current row is metadata-only.
                cell.set_width_request(112);
            }
            cell.append(child);
            if index > 0 {
                self.0[index - 1].add_widget(&cell);
            }
            row.append(&cell);
        }
        row
    }
}
fn date(timestamp: u64) -> String {
    glib::DateTime::from_unix_utc(timestamp.min(i64::MAX as u64) as i64)
        .ok()
        .and_then(|d| d.to_local().ok())
        .and_then(|d| d.format("%x %X").ok())
        .map(|d| d.to_string())
        .unwrap_or_default()
}
fn field(text: &str) -> gtk::Label {
    let label = label(text);
    label.set_wrap(false);
    label
}
pub(super) fn timestamp(timestamp: Option<u64>) -> gtk::Label {
    field(&timestamp.map(date).unwrap_or_default())
}

pub(super) struct Window {
    window: gtk::Window,
    history: gtk::Box,
    live: gtk::Box,
    notice: gtk::Label,
    clear: gtk::Button,
    rendered: RefCell<Option<(String, Vec<String>)>>,
    columns: Columns,
}
impl Window {
    pub(super) fn new(panel: &Rc<Panel>) -> Rc<Self> {
        let window = gtk::Window::builder()
            .title("History")
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
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        footer.set_widget_name("history-footer");
        footer.set_halign(gtk::Align::End);
        let refresh = gtk::Button::with_label("Refresh");
        refresh.set_action_name(Some("panel.journal-refresh"));
        footer.append(&refresh);
        let clear = gtk::Button::with_label("Clear completed");
        clear.set_action_name(Some("panel.journal-clear"));
        footer.append(&clear);
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
        let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let live = panel.box_widget("transfers");
        live.set_spacing(0);
        body.append(&live);
        let history = gtk::Box::new(gtk::Orientation::Vertical, 0);
        body.append(&history);
        scroll.set_child(Some(&body));
        root.append(&scroll);
        root.append(&footer);
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
            columns: panel.history_columns.clone(),
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
            0=>"",
            1=>"History could not be saved. Sharing is still available. Refresh to retry, or reset history while preserving a recovery copy.",
            2=>"Loading history…",3=>"Saved history is unavailable with this service version.",_=>"History could not be read safely. Refresh to retry; sharing remains available.",
        }});
        self.notice
            .set_visible(model.state.disconnected || error != 0);
        self.clear.set_tooltip_text(Some("Completed metadata is kept for up to 30 days and 1000 operations. File and message contents are not stored."));
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
                let peer = label(if record.peer.is_empty() {
                    "Unnamed device"
                } else {
                    &record.peer
                });
                peer.set_wrap(false);
                peer.set_ellipsize(gtk::pango::EllipsizeMode::End);
                peer.set_max_width_chars(24);
                peer.set_hexpand(true);
                let peer_cell = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let icon = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                icon.set_width_request(32);
                peer_cell.append(&icon);
                peer_cell.append(&peer);
                let direction = field(if record.inbound { "Receive" } else { "Send" });
                let content = field(&format!("{} · {} bytes", record.kind, record.total));
                let status = label(match record.status.as_str() {
                    "finished" => "Completed",
                    "rejected" => "Declined",
                    "cancelled" => "Cancelled",
                    "failed" => "Failed",
                    "disconnected" => "Disconnected",
                    "interrupted" => "Interrupted",
                    _ => "In progress",
                });
                status.set_wrap(false);
                let date = timestamp(Some(record.updated));
                let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let row = self.columns.row([
                    peer_cell.upcast_ref(),
                    direction.upcast_ref(),
                    content.upcast_ref(),
                    status.upcast_ref(),
                    date.upcast_ref(),
                    actions.upcast_ref(),
                ]);
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

#[cfg(test)]
pub(super) fn verify_columns() {
    // Arrange: one live-style row with an action, one metadata row without it.
    let columns = Columns::default();
    let first = [
        "Mercury",
        "Receive",
        "JPG · 75991 bytes",
        "Completed",
        "17/10/26 14:14:33",
    ]
    .map(field);
    let second = [
        "A longer peer name",
        "Send",
        "Text · 1 bytes",
        "Failed",
        "1/1/26 0:00:00",
    ]
    .map(field);
    let action = gtk::Button::with_label("Show in folder");
    let empty = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let one = columns.row([
        first[0].upcast_ref(),
        first[1].upcast_ref(),
        first[2].upcast_ref(),
        first[3].upcast_ref(),
        first[4].upcast_ref(),
        action.upcast_ref(),
    ]);
    let two = columns.row([
        second[0].upcast_ref(),
        second[1].upcast_ref(),
        second[2].upcast_ref(),
        second[3].upcast_ref(),
        second[4].upcast_ref(),
        empty.upcast_ref(),
    ]);
    // Act: allocate the actual GTK rows at the same table width.
    assert!(one.has_css_class("history-row") && two.has_css_class("history-row"));
    one.allocate(860, 40, -1, None);
    two.allocate(860, 40, -1, None);
    // Assert: every column starts at the same coordinate with the same width.
    let mut a = one.first_child();
    let mut b = two.first_child();
    let mut count = 0;
    while let (Some(left), Some(right)) = (a, b) {
        assert!(left.has_css_class("history-cell") && right.has_css_class("history-cell"));
        assert_eq!(
            left.compute_bounds(&one).unwrap().x(),
            right.compute_bounds(&two).unwrap().x(),
            "column {count}"
        );
        assert_eq!(left.width(), right.width(), "column {count}");
        assert!(
            left.width() > 0,
            "reserved empty action slot must have width"
        );
        a = left.next_sibling();
        b = right.next_sibling();
        count += 1;
    }
    assert_eq!(count, 6);
}
