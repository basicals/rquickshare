//! Keyed rows keep focus and expanded manifests across progress/readiness updates.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

struct Entry {
    root: gtk::Box,
    transfer: Option<TransferView>,
    endpoint: Option<(String, bool)>,
}
#[derive(Default)]
pub(super) struct Rows {
    entries: BTreeMap<(String, String), Entry>,
}
impl Rows {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }
}
fn named(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
    if root.widget_name() == name {
        return Some(root.clone());
    }
    let mut next = root.first_child();
    while let Some(child) = next {
        if let Some(found) = named(&child, name) {
            return Some(found);
        }
        next = child.next_sibling();
    }
    None
}
fn expansion_for_replacement(rows: &Rows, key: &(String, String), default: bool) -> bool {
    rows.entries
        .get(key)
        .and_then(|entry| {
            named(entry.root.upcast_ref(), "transfer-details").and_downcast::<gtk::Expander>()
        })
        .map(|expander| expander.is_expanded())
        .unwrap_or(default)
}
#[cfg(test)]
fn section_title(text: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let title = label(text);
    title.add_css_class("dim-label");
    row.append(&title);
    row
}
fn bound_current_endpoint<'a>(
    direction: Direction,
    associated: Option<&'a str>,
    discovered: &BTreeSet<&str>,
) -> Option<&'a str> {
    if direction != Direction::Outbound {
        return None;
    }
    associated.filter(|id| discovered.contains(id))
}
fn card_transfer_visible(
    direction: Direction,
    phase: Phase,
    associated: Option<&str>,
    discovered: &BTreeSet<&str>,
) -> bool {
    !phase.is_terminal() && bound_current_endpoint(direction, associated, discovered).is_none()
}
fn payload_label(details: &TransferDetails) -> String {
    use rqs_lib::sessions::details::PayloadKind;
    match details.kind {
        PayloadKind::Files if details.files.len() == 1 => details.files[0]
            .name
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .filter(|ext| {
                !ext.is_empty() && ext.len() <= 12 && ext.bytes().all(|b| b.is_ascii_alphanumeric())
            })
            .map(|ext| ext.to_ascii_uppercase())
            .unwrap_or_else(|| "File".into()),
        PayloadKind::Files => format!("{} files", details.files.len()),
        PayloadKind::Text => "Text".into(),
        PayloadKind::Url => "Link".into(),
    }
}
fn transferring(phase: Phase) -> bool {
    matches!(phase, Phase::Sending | Phase::Receiving)
}
fn collapse_disclosure(page: &str, previous: Option<Phase>, next: Phase) -> bool {
    page == "nearby"
        && (next.is_terminal()
            || (previous == Some(Phase::AwaitingConsent) && next != Phase::AwaitingConsent))
}
fn progress_fraction(done: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        done.min(total) as f64 / total as f64
    }
}
fn same_description(a: &TransferView, b: &TransferView) -> bool {
    a.transfer == b.transfer
        && a.completed_files == b.completed_files
        && a.recipient == b.recipient
        && a.associated_endpoint == b.associated_endpoint
        && a.direction == b.direction
        && a.phase == b.phase
        && a.consent_revision == b.consent_revision
        && a.content_revision == b.content_revision
        && a.reveal_revision == b.reveal_revision
        && a.details.kind == b.details.kind
        && a.details.status == b.details.status
        && a.details.sender == b.details.sender
        && a.details.device == b.details.device
        && a.details.pin == b.details.pin
        && a.details.total_bytes == b.details.total_bytes
        && a.details.files == b.details.files
}
fn arrange(container: &gtk::Box, desired: &[gtk::Box]) {
    let mut next = container.first_child();
    while let Some(child) = next {
        next = child.next_sibling();
        if !desired
            .iter()
            .any(|row| row.upcast_ref::<gtk::Widget>() == &child)
        {
            container.remove(&child);
        }
    }
    let mut previous: Option<gtk::Widget> = None;
    for row in desired {
        if row.parent().as_ref() != Some(container.upcast_ref()) {
            if let Some(parent) = row.parent().and_downcast::<gtk::Box>() {
                parent.remove(row);
            }
            container.append(row);
        }
        if row.prev_sibling() != previous {
            container.reorder_child_after(row, previous.as_ref());
        }
        previous = Some(row.clone().upcast());
    }
}
#[derive(Clone, Copy)]
enum Badge {
    Available,
    Unavailable,
    Transfer(Phase),
}
fn badge_style(badge: Badge) -> (&'static str, &'static str) {
    match badge {
        Badge::Available => ("●", "success"),
        Badge::Unavailable | Badge::Transfer(Phase::Cancelled | Phase::Rejected) => {
            ("−", "dim-label")
        }
        Badge::Transfer(Phase::Failed | Phase::Disconnected) => ("!", "error"),
        Badge::Transfer(Phase::Finished) => ("✓", "success"),
        Badge::Transfer(Phase::AwaitingConsent) => ("?", "warning"),
        Badge::Transfer(Phase::Queued | Phase::CancelRequested) => ("…", "warning"),
        Badge::Transfer(Phase::Sending) => ("↑", "accent"),
        Badge::Transfer(Phase::Receiving) => ("↓", "accent"),
        Badge::Transfer(Phase::Active) => ("●", "accent"),
    }
}
fn device_icon(
    kind: rqs_lib::service::client::state::DeviceKind,
    status: &str,
    badge: Badge,
) -> gtk::Overlay {
    use rqs_lib::service::client::state::DeviceKind;
    let overlay = gtk::Overlay::new();
    let image = gtk::Image::from_icon_name(match kind {
        DeviceKind::Phone => "phone-symbolic",
        DeviceKind::Tablet => "tablet-symbolic",
        _ => "computer-symbolic",
    });
    image.set_pixel_size(28);
    overlay.set_child(Some(&image));
    let (symbol, class) = badge_style(badge);
    let marker = gtk::Label::new(Some(symbol));
    marker.add_css_class("device-status");
    marker.set_widget_name("device-status");
    marker.add_css_class(class);
    marker.set_halign(gtk::Align::End);
    marker.set_valign(gtk::Align::End);
    overlay.add_overlay(&marker);
    overlay.set_tooltip_text(Some(status));
    overlay.update_property(&[gtk::accessible::Property::Label(status)]);
    overlay
}
pub(super) fn transfer_row(
    sender: &str,
    kind: rqs_lib::service::client::state::DeviceKind,
    phase: Phase,
    details: &gtk::Box,
    expanded: bool,
) -> gtk::Box {
    let incoming = phase == Phase::AwaitingConsent;
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    heading.set_widget_name("transfer-heading");
    heading.append(&device_icon(
        kind,
        phase_text(phase),
        Badge::Transfer(phase),
    ));
    let title = label(if sender.is_empty() {
        if incoming {
            "Incoming request"
        } else {
            "Transfer"
        }
    } else {
        sender
    });
    title.set_hexpand(true);
    title.set_wrap(false);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_max_width_chars(20);
    title.set_width_request(0);
    heading.append(&title);
    let summary = label("");
    summary.set_widget_name("row-summary");
    summary.set_selectable(false);
    summary.set_wrap(false);
    heading.append(&summary);
    let progress = gtk::ProgressBar::new();
    progress.set_widget_name("row-progress");
    progress.set_width_request(72);
    progress.set_valign(gtk::Align::Center);
    progress.set_visible(matches!(phase, Phase::Sending | Phase::Receiving));
    heading.append(&progress);
    let status = label(phase_text(phase));
    status.set_widget_name("row-status");
    status.set_visible(phase.is_terminal());
    heading.append(&status);
    let expander = gtk::Expander::new(None);
    expander.set_resize_toplevel(true);
    expander.set_widget_name("transfer-details");
    expander.set_child(Some(details));
    expander.set_expanded(expanded);
    let review = gtk::Button::with_label("Review");
    review.set_widget_name("row-action");
    review.set_hexpand(false);
    review.set_halign(gtk::Align::End);
    let expanded = expander.clone();
    review.connect_clicked(move |_| {
        expanded.set_expanded(!expanded.is_expanded());
    });
    // The history expander already provides the only disclosure control.
    if incoming {
        heading.append(&review);
    }
    expander.set_label_widget(Some(&heading));
    root.append(&expander);
    root
}
fn flatten_history(root: &gtk::Box, details: &gtk::Box) -> gtk::Box {
    let expander = named(root.upcast_ref(), "transfer-details")
        .and_downcast::<gtk::Expander>()
        .expect("transfer row owns its disclosure");
    let heading = expander
        .label_widget()
        .and_downcast::<gtk::Box>()
        .expect("transfer heading");
    expander.set_label_widget(None::<&gtk::Widget>);
    expander.set_child(None::<&gtk::Widget>);
    root.remove(&expander);
    root.append(&heading);
    for name in ["review-text", "cancel", "cancel-reveal"] {
        if let Some(action) = named(details.upcast_ref(), name) {
            let parent = action.parent().and_downcast::<gtk::Box>().unwrap();
            parent.remove(&action);
            heading.append(&action);
        }
    }
    heading
}
impl Panel {
    fn flatten_history_transfer(
        self: &Rc<Self>,
        root: &gtk::Box,
        details: &gtk::Box,
        transfer: &TransferView,
        ready: bool,
        incoming: bool,
    ) {
        let heading = flatten_history(root, details);
        if incoming {
            let review = gtk::Button::with_label("Review");
            review.set_widget_name("history-review");
            review.set_sensitive(ready);
            let weak = Rc::downgrade(self);
            let id = transfer.transfer.id().to_owned();
            review.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    panel.set_drawer(Drawer::Open);
                    panel.pages.set_visible_child_name("nearby");
                    panel.render();
                    panel.expand_transfer(&id);
                    panel.present_surface();
                }
            });
            heading.append(&review);
        } else if transfer.direction == Direction::Inbound {
            if let Some(revision) = transfer.reveal_revision {
                if transfer.phase == Phase::Finished {
                    let reveal = gtk::Button::with_label("Show in folder");
                    reveal.set_widget_name("result-reveal");
                    reveal.set_sensitive(ready);
                    let weak = Rc::downgrade(self);
                    let target = transfer.transfer.clone();
                    reveal.connect_clicked(move |_| {
                        if let Some(panel) = weak.upgrade() {
                            panel.reveal_result(target.clone(), revision);
                        }
                    });
                    heading.append(&reveal);
                } else if !transfer.completed_files.is_empty() {
                    let saved = gtk::Box::new(gtk::Orientation::Vertical, 8);
                    for (index, file) in transfer.completed_files.iter().enumerate() {
                        let name = label(&file.name);
                        name.set_max_width_chars(40);
                        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
                        saved.append(&name);
                        if let Some(action) =
                            named(details.upcast_ref(), &format!("reveal-{index}"))
                        {
                            let parent = action.parent().and_downcast::<gtk::Box>().unwrap();
                            parent.remove(&action);
                            saved.append(&action);
                        }
                    }
                    let popover = gtk::Popover::new();
                    popover.set_child(Some(&saved));
                    let menu = gtk::MenuButton::builder()
                        .label("Saved files")
                        .popover(&popover)
                        .build();
                    heading.append(&menu);
                }
            }
        }
    }
    pub(super) fn expand_transfer(&self, id: &str) {
        if let Some(entry) = self
            .rows
            .borrow()
            .entries
            .get(&("nearby".to_owned(), id.to_owned()))
        {
            if let Some(expander) =
                named(entry.root.upcast_ref(), "transfer-details").and_downcast::<gtk::Expander>()
            {
                expander.set_expanded(true);
                expander.grab_focus();
            }
        }
    }
    fn cached_transfer(
        self: &Rc<Self>,
        rows: &mut Rows,
        page: &str,
        transfer: &TransferView,
        ready: bool,
        incoming: bool,
    ) -> gtk::Box {
        let key = (page.to_owned(), transfer.transfer.id().to_owned());
        let reusable = rows.entries.get(&key).is_some_and(|entry| {
            entry
                .transfer
                .as_ref()
                .is_some_and(|old| same_description(old, transfer))
        });
        if !reusable {
            // Live phase changes must not discard the user's expansion choice.
            let expanded = expansion_for_replacement(rows, &key, page == "incoming");
            let previous = rows
                .entries
                .get(&key)
                .and_then(|entry| entry.transfer.as_ref())
                .map(|transfer| transfer.phase);
            let expanded = expanded && !collapse_disclosure(page, previous, transfer.phase);
            let details = if incoming {
                self.incoming_card(transfer, ready, true)
            } else {
                self.transfer_card(transfer, ready, true)
            };
            let root = transfer_row(
                transfer
                    .recipient
                    .as_deref()
                    .unwrap_or(&transfer.details.sender),
                transfer.details.device,
                transfer.phase,
                &details,
                expanded,
            );
            // The expander changes layout independently of daemon state updates.
            // Re-measure after GTK applies the new child allocation, keeping the
            // current lower edge pinned before the height changes.
            if let Some(expander) =
                named(root.upcast_ref(), "transfer-details").and_downcast::<gtk::Expander>()
            {
                let weak = Rc::downgrade(self);
                let card_page = page == "nearby";
                expander.connect_expanded_notify(move |_| {
                    if !card_page {
                        return;
                    }
                    if let Some(panel) = weak.upgrade() {
                        panel.pin_surface_bottom();
                        let weak = Rc::downgrade(&panel);
                        glib::idle_add_local_once(move || {
                            if let Some(panel) = weak.upgrade() {
                                panel.resize_to_content();
                            }
                        });
                    }
                });
            }
            if page == "nearby" && incoming {
                root.add_css_class("incoming-request");
            }
            if page != "nearby" {
                if let Some(disclosure) = named(root.upcast_ref(), "row-action") {
                    disclosure.set_visible(false);
                }
            }
            for name in [
                "detail-peer",
                "detail-summary",
                "detail-code",
                "phase",
                "progress",
            ] {
                if let Some(duplicate) = named(details.upcast_ref(), name) {
                    duplicate.set_visible(false);
                }
            }
            // One disclosure exposes the manifest directly; no nested Review-all toggle.
            if let Some(manifest) =
                named(details.upcast_ref(), "detail-manifest").and_downcast::<gtk::Expander>()
            {
                manifest.set_expanded(true);
                if let (Some(list), Some(parent)) = (
                    manifest.child(),
                    manifest.parent().and_downcast::<gtk::Box>(),
                ) {
                    manifest.set_child(None::<&gtk::Widget>);
                    parent.insert_child_after(&list, manifest.prev_sibling().as_ref());
                    parent.remove(&manifest);
                }
            }
            if page == "transfers" {
                self.flatten_history_transfer(&root, &details, transfer, ready, incoming);
            }
            rows.entries.insert(
                key.clone(),
                Entry {
                    root,
                    transfer: Some(transfer.clone()),
                    endpoint: None,
                },
            );
        }
        let root = rows.entries[&key].root.clone();
        for (name, enabled) in [
            ("history-review", ready),
            ("result-reveal", ready && transfer.reveal_revision.is_some()),
            ("review-text", ready && transfer.content_revision.is_some()),
            (
                "accept",
                ready && transfer.pending_control.is_none() && transfer.can_accept(),
            ),
            (
                "decline",
                ready && transfer.pending_control.is_none() && transfer.consent_revision.is_some(),
            ),
            (
                "cancel",
                ready
                    && transfer.pending_control.is_none()
                    && transfer.phase != Phase::CancelRequested,
            ),
        ] {
            if let Some(button) = named(root.upcast_ref(), name) {
                button.set_sensitive(enabled);
            }
        }
        for index in 0..transfer.completed_files.len() {
            if let Some(button) = named(root.upcast_ref(), &format!("reveal-{index}")) {
                button.set_sensitive(ready && transfer.reveal_revision.is_some());
            }
        }
        if let Some(cancel) = named(root.upcast_ref(), "cancel-reveal") {
            cancel.set_visible(self.revealing(&transfer.transfer));
        }
        if let Some(progress) = named(root.upcast_ref(), "progress").and_downcast::<gtk::Label>() {
            progress.set_text(&format!(
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
        }
        if let Some(summary) = named(root.upcast_ref(), "row-summary").and_downcast::<gtk::Label>()
        {
            let text = if transfer.phase.is_terminal() {
                format!(
                    "{} · {} bytes",
                    payload_label(&transfer.details),
                    transfer.details.total_bytes
                )
            } else if !transfer.details.pin.is_empty() {
                format!(
                    "{} · Code: {}",
                    payload_label(&transfer.details),
                    transfer.details.pin
                )
            } else {
                payload_label(&transfer.details)
            };
            summary.set_text(&text);
            summary.set_visible(!transferring(transfer.phase));
        }
        if let Some(progress) =
            named(root.upcast_ref(), "row-progress").and_downcast::<gtk::ProgressBar>()
        {
            progress.set_visible(transferring(transfer.phase));
            progress.set_fraction(progress_fraction(
                transfer.details.transferred_bytes,
                transfer.details.total_bytes,
            ));
            progress.set_tooltip_text(Some(phase_text(transfer.phase)));
        }
        root
    }
    pub(super) fn render_rows(
        self: &Rc<Self>,
        view: &rqs_lib::service::client::state::StateView,
        selection: Option<rqs_lib::service::client::Selection>,
        ready: bool,
        _current: Option<&TransferView>,
    ) {
        let mut rows = self.rows.borrow_mut();
        let mut used = BTreeSet::new();
        let mut pages: BTreeMap<&str, Vec<gtk::Box>> = ["nearby", "incoming", "transfers"]
            .into_iter()
            .map(|page| (page, vec![]))
            .collect();
        let endpoints = view
            .endpoints
            .iter()
            .map(|peer| (peer.id.clone(), (peer.name.clone(), true)))
            .collect::<BTreeMap<_, _>>();
        for (id, projection) in endpoints {
            let key = ("nearby".to_owned(), id.clone());
            if rows
                .entries
                .get(&key)
                .is_none_or(|entry| entry.endpoint.as_ref() != Some(&projection))
            {
                let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
                let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                controls.add_css_class("device-row");
                let state = if projection.1 {
                    Badge::Available
                } else {
                    Badge::Unavailable
                };
                controls.append(&device_icon(
                    rqs_lib::service::client::state::DeviceKind::Unknown,
                    if projection.1 {
                        "Nearby advertised device"
                    } else {
                        "Unavailable"
                    },
                    state,
                ));
                let title = label(&projection.0);
                title.set_selectable(false);
                title.set_hexpand(true);
                title.set_wrap(false);
                title.set_ellipsize(gtk::pango::EllipsizeMode::End);
                title.set_max_width_chars(20);
                controls.append(&title);
                let progress = gtk::ProgressBar::new();
                progress.set_widget_name("device-progress");
                progress.set_width_request(72);
                progress.set_valign(gtk::Align::Center);
                controls.append(&progress);
                let strip = gtk::Revealer::new();
                strip.set_widget_name("send-strip");
                strip.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
                strip.set_transition_duration(160);
                let inputs = gtk::Box::new(gtk::Orientation::Horizontal, 3);
                for (kind, icon, title) in [
                    (outbound::Kind::Files, "text-x-generic-symbolic", "File"),
                    (outbound::Kind::Folder, "folder-symbolic", "Folder"),
                    (outbound::Kind::Text, "insert-text-symbolic", "Text"),
                    (
                        outbound::Kind::Clipboard,
                        "edit-paste-symbolic",
                        "Clipboard",
                    ),
                ] {
                    let button = gtk::Button::from_icon_name(icon);
                    button.set_tooltip_text(Some(title));
                    button.update_property(&[gtk::accessible::Property::Label(title)]);
                    let weak = Rc::downgrade(self);
                    let target = outbound::Target {
                        epoch: view.epoch,
                        endpoint: id.clone(),
                    };
                    button.connect_clicked(move |_| {
                        if let Some(panel) = weak.upgrade() {
                            panel.input_for(kind, target.clone());
                        }
                    });
                    inputs.append(&button);
                }
                strip.set_child(Some(&inputs));
                controls.append(&strip);
                let send = gtk::Button::with_label("Send");
                send.set_widget_name("send-device");
                let weak = Rc::downgrade(self);
                let endpoint_for_send = id.clone();
                send.connect_clicked(move |_| {
                    if let Some(panel) = weak.upgrade() {
                        panel.send_to(&endpoint_for_send);
                    }
                });
                controls.append(&send);
                root.append(&controls);
                let code = label("");
                code.set_widget_name("device-code");
                root.append(&code);
                rows.entries.insert(
                    key.clone(),
                    Entry {
                        root,
                        transfer: None,
                        endpoint: Some(projection.clone()),
                    },
                );
            }
            let root = rows.entries[&key].root.clone();
            if let Some(strip) =
                named(root.upcast_ref(), "send-strip").and_downcast::<gtk::Revealer>()
            {
                strip.set_reveal_child(
                    selection.is_none()
                        && self.model.borrow().strip_target.as_deref() == Some(id.as_str()),
                );
                strip.set_sensitive(ready);
            }
            let active = view.transfers.iter().find(|transfer| {
                transfer.direction == Direction::Outbound
                    && !transfer.phase.is_terminal()
                    && transfer.associated_endpoint.as_deref() == Some(id.as_str())
            });
            if let Some(marker) =
                named(root.upcast_ref(), "device-status").and_downcast::<gtk::Label>()
            {
                let (symbol, class) = badge_style(
                    active.map_or(Badge::Available, |transfer| Badge::Transfer(transfer.phase)),
                );
                marker.set_text(symbol);
                for old in ["success", "warning", "error", "accent", "dim-label"] {
                    marker.remove_css_class(old);
                }
                marker.add_css_class(class);
            }
            if let Some(progress) =
                named(root.upcast_ref(), "device-progress").and_downcast::<gtk::ProgressBar>()
            {
                progress.set_visible(active.is_some());
                if let Some(transfer) = active {
                    progress.set_fraction(progress_fraction(
                        transfer.details.transferred_bytes,
                        transfer.details.total_bytes,
                    ));
                    progress.set_tooltip_text(Some(phase_text(transfer.phase)));
                }
            }
            if let Some(code) = named(root.upcast_ref(), "device-code").and_downcast::<gtk::Label>()
            {
                let pin = active
                    .map(|transfer| transfer.details.pin.as_str())
                    .unwrap_or_default();
                code.set_visible(!pin.is_empty());
                code.set_text(&format!("Code: {pin}"));
            }
            if let Some(send) = named(root.upcast_ref(), "send-device") {
                send.set_sensitive(ready && projection.1 && active.is_none());
            }
            pages.get_mut("nearby").unwrap().push(root);
            used.insert(key);
        }
        let discovered: BTreeSet<&str> =
            view.endpoints.iter().map(|peer| peer.id.as_str()).collect();
        for transfer in &view.transfers {
            for page in ["nearby", "incoming", "transfers"] {
                let incoming = transfer.direction == Direction::Inbound
                    && transfer.phase == Phase::AwaitingConsent;
                if page == "incoming" && !incoming {
                    continue;
                }
                if page == "nearby"
                    && !self.results.borrow().visible(transfer.transfer.id())
                    && !card_transfer_visible(
                        transfer.direction,
                        transfer.phase,
                        transfer.associated_endpoint.as_deref(),
                        &discovered,
                    )
                {
                    continue;
                }
                let root = self.cached_transfer(&mut rows, page, transfer, ready, incoming);
                if page == "nearby" {
                    if let Some(status) = named(root.upcast_ref(), "row-status") {
                        status.set_visible(transfer.phase.is_terminal());
                    }
                    if let Some(button) =
                        named(root.upcast_ref(), "row-action").and_downcast::<gtk::Button>()
                    {
                        button.set_label(if transfer.direction == Direction::Inbound {
                            "Receive"
                        } else {
                            "Send"
                        });
                        button.set_sensitive(transfer.direction == Direction::Inbound);
                    }
                }
                pages.get_mut(page).unwrap().push(root);
                used.insert((page.to_owned(), transfer.transfer.id().to_owned()));
            }
        }
        for (page, title) in [
            (
                "nearby",
                "No nearby devices. Make the receiving device visible.",
            ),
            ("incoming", "No incoming requests"),
            ("transfers", "No transfers"),
        ] {
            if pages[page].is_empty() {
                let key = (
                    page.to_owned(),
                    format!("empty:{}:{title}", selection.is_some()),
                );
                let entry = rows.entries.entry(key.clone()).or_insert_with(|| {
                    let root = card();
                    if !title.is_empty() {
                        root.append(&label(title));
                    }
                    Entry {
                        root,
                        transfer: None,
                        endpoint: None,
                    }
                });
                pages.get_mut(page).unwrap().push(entry.root.clone());
                used.insert(key);
            }
        }
        for (page, desired) in pages {
            arrange(&self.box_widget(page), &desired);
        }
        rows.entries.retain(|key, _| used.contains(key));
    }
}

#[cfg(test)]
pub(super) fn verify_keyed_layout() {
    // Arrange
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let first = card();
    let expanded = gtk::Expander::new(Some("Files"));
    expanded.set_child(Some(&label("Untrusted filename")));
    expanded.set_expanded(true);
    first.append(&expanded);
    let second = card();
    // Act / Assert: identical projection does not detach or replace either row.
    arrange(&container, &[first.clone(), second.clone()]);
    let child = expanded.child().unwrap();
    arrange(&container, &[first.clone(), second.clone()]);
    assert_eq!(container.first_child(), Some(first.clone().upcast()));
    assert_eq!(expanded.child(), Some(child.clone()));
    assert!(expanded.is_expanded());
    arrange(&container, &[second.clone(), first.clone()]);
    assert_eq!(container.first_child(), Some(second.clone().upcast()));
    assert_eq!(container.last_child(), Some(first.clone().upcast()));
    assert!(expanded.is_expanded());
    assert_eq!(expanded.child(), Some(child));
    arrange(&container, std::slice::from_ref(&first));
    assert!(second.parent().is_none());
    assert_eq!(container.first_child(), Some(first.clone().upcast()));

    // Reparenting is required when a completed operation moves between the
    // unlinked activity drawer and an authoritative same-endpoint group.
    let activity = gtk::Box::new(gtk::Orientation::Vertical, 0);
    arrange(&activity, std::slice::from_ref(&first));
    assert!(container.first_child().is_none());
    assert_eq!(activity.first_child(), Some(first.clone().upcast()));
    arrange(&container, std::slice::from_ref(&first));
    assert!(activity.first_child().is_none());
    assert_eq!(container.first_child(), Some(first.upcast()));

    // Arrange: a synthetic transfer with an untrusted sender name.
    let mut cache = Rows::default();
    let key = ("nearby".to_owned(), "transfer-one".to_owned());
    assert!(!expansion_for_replacement(&cache, &key, false));
    assert!(expansion_for_replacement(&cache, &key, true));
    let details = card();
    details.append(&label("Text · 2507 bytes"));
    let row = transfer_row(
        "<Mercury>",
        rqs_lib::service::client::state::DeviceKind::Phone,
        Phase::Finished,
        &details,
        false,
    );
    let expander = named(row.upcast_ref(), "transfer-details")
        .and_downcast::<gtk::Expander>()
        .unwrap();
    assert!(!expander.is_expanded());
    cache.entries.insert(
        key.clone(),
        Entry {
            root: row,
            transfer: None,
            endpoint: None,
        },
    );
    // Act / Assert: collapsed state is retained even for an incoming fallback.
    assert!(!expansion_for_replacement(&cache, &key, true));
    expander.set_expanded(true);
    assert!(expansion_for_replacement(&cache, &key, false));
    assert_eq!(expander.child(), Some(details.upcast()));
    assert!(named(cache.entries[&key].root.upcast_ref(), "transfer-details").is_some());
    expander.set_expanded(false);
    assert!(!expansion_for_replacement(&cache, &key, false));

    // Arrange: history content actions exist below the old disclosure.
    let details = card();
    let clicks = Rc::new(Cell::new(0));
    for name in ["review-text", "cancel", "cancel-reveal"] {
        let action = gtk::Button::with_label(name);
        action.set_widget_name(name);
        let clicks = clicks.clone();
        action.connect_clicked(move |_| clicks.set(clicks.get() + 1));
        details.append(&action);
    }
    details.append(&label("Duplicate detail metadata"));
    let history = transfer_row(
        "<Mercury>",
        rqs_lib::service::client::state::DeviceKind::Phone,
        Phase::Finished,
        &details,
        true,
    );
    // Act: reuse the original controls, removing the disclosure and detail body.
    let heading = flatten_history(&history, &details);
    // Assert: controls are directly clickable and no hidden duplicate body remains.
    assert!(named(history.upcast_ref(), "transfer-details").is_none());
    assert_eq!(history.observe_children().n_items(), 1);
    assert_eq!(history.first_child(), Some(heading.clone().upcast()));
    assert!(details.parent().is_none());
    for name in ["review-text", "cancel", "cancel-reveal"] {
        let action = named(history.upcast_ref(), name)
            .and_downcast::<gtk::Button>()
            .unwrap();
        assert_eq!(action.parent(), Some(heading.clone().upcast()));
        action.emit_clicked();
    }
    assert_eq!(clicks.get(), 3);
    assert_eq!(details.observe_children().n_items(), 1);

    let live = ["ep-1", "ep-2"].into_iter().collect::<BTreeSet<_>>();
    assert_eq!(
        bound_current_endpoint(Direction::Outbound, Some("ep-1"), &live),
        Some("ep-1")
    );
    assert_eq!(
        bound_current_endpoint(Direction::Outbound, Some("ep-3"), &live),
        None
    );
    assert_eq!(
        bound_current_endpoint(Direction::Outbound, Some("Mercury"), &live),
        None
    );
    assert_eq!(
        bound_current_endpoint(Direction::Inbound, Some("ep-1"), &live),
        None
    );
    assert_eq!(
        bound_current_endpoint(Direction::Outbound, None, &live),
        None
    );
    let untrusted_header = section_title("<Mercury> is not a verified identity");
    assert_eq!(
        untrusted_header
            .first_child()
            .unwrap()
            .downcast::<gtk::Label>()
            .unwrap()
            .text(),
        "<Mercury> is not a verified identity"
    );
}

#[cfg(test)]
mod badge_tests {
    use super::*;
    #[test]
    fn given_each_transfer_and_discovery_state_when_badged_then_shape_and_color_distinguish_outcomes(
    ) {
        for (phase, expected) in [
            (Phase::Queued, ("…", "warning")),
            (Phase::Active, ("●", "accent")),
            (Phase::AwaitingConsent, ("?", "warning")),
            (Phase::Receiving, ("↓", "accent")),
            (Phase::Sending, ("↑", "accent")),
            (Phase::CancelRequested, ("…", "warning")),
            (Phase::Finished, ("✓", "success")),
            (Phase::Rejected, ("−", "dim-label")),
            (Phase::Cancelled, ("−", "dim-label")),
            (Phase::Failed, ("!", "error")),
            (Phase::Disconnected, ("!", "error")),
        ] {
            assert_eq!(badge_style(Badge::Transfer(phase)), expected);
        }
        assert_eq!(badge_style(Badge::Available), ("●", "success"));
        assert_eq!(badge_style(Badge::Unavailable), ("−", "dim-label"));
    }
}

#[cfg(test)]
mod device_contract_tests {
    use super::*;
    #[test]
    fn given_transfer_direction_phase_and_association_when_projected_then_card_contains_only_live_peer_rows(
    ) {
        // Arrange / Act / Assert
        let peers = ["current"].into_iter().collect();
        for phase in [
            Phase::Finished,
            Phase::Rejected,
            Phase::Cancelled,
            Phase::Failed,
            Phase::Disconnected,
        ] {
            for direction in [Direction::Inbound, Direction::Outbound] {
                assert!(!card_transfer_visible(direction, phase, None, &peers));
            }
        }
        for phase in [
            Phase::Queued,
            Phase::Active,
            Phase::AwaitingConsent,
            Phase::Sending,
            Phase::Receiving,
            Phase::CancelRequested,
        ] {
            assert!(!card_transfer_visible(
                Direction::Outbound,
                phase,
                Some("current"),
                &peers
            ));
            assert!(card_transfer_visible(
                Direction::Outbound,
                phase,
                Some("vanished"),
                &peers
            ));
            assert!(card_transfer_visible(
                Direction::Outbound,
                phase,
                None,
                &peers
            ));
            assert!(card_transfer_visible(
                Direction::Inbound,
                phase,
                Some("current"),
                &peers
            ));
        }
    }
    #[test]
    fn given_consent_or_completion_when_phase_changes_then_only_card_disclosure_resets_and_history_choice_survives(
    ) {
        for page in ["nearby", "transfers", "incoming"] {
            for (previous, next, expected) in [
                (None, Phase::AwaitingConsent, false),
                (Some(Phase::AwaitingConsent), Phase::AwaitingConsent, false),
                (Some(Phase::AwaitingConsent), Phase::Active, true),
                (Some(Phase::AwaitingConsent), Phase::Receiving, true),
                (Some(Phase::Active), Phase::Receiving, false),
                (Some(Phase::Receiving), Phase::Receiving, false),
                (Some(Phase::Receiving), Phase::Finished, true),
                (Some(Phase::Receiving), Phase::Failed, true),
            ] {
                assert_eq!(
                    collapse_disclosure(page, previous, next),
                    page == "nearby" && expected
                );
            }
        }
    }
    #[test]
    fn given_byte_progress_when_unknown_or_overrun_then_bar_stays_finite_and_bounded() {
        for (done, total, expected) in [
            (0, 0, 0.0),
            (1, 0, 0.0),
            (0, 1, 0.0),
            (1, 2, 0.5),
            (2, 2, 1.0),
            (3, 2, 1.0),
            (u64::MAX, u64::MAX, 1.0),
        ] {
            assert_eq!(progress_fraction(done, total), expected);
        }
    }
}
