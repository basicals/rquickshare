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
fn section_title(text: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let title = label(text);
    title.add_css_class("dim-label");
    row.append(&title);
    row
}
fn same_description(a: &TransferView, b: &TransferView) -> bool {
    a.transfer == b.transfer
        && a.completed_files == b.completed_files
        && a.recipient == b.recipient
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
        if row.parent().is_none() {
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
    let expander = gtk::Expander::new(None);
    expander.set_widget_name("transfer-details");
    expander.set_child(Some(details));
    expander.set_expanded(expanded);
    let review = gtk::Button::with_label(if incoming { "Review" } else { "Details" });
    review.set_hexpand(false);
    review.set_halign(gtk::Align::End);
    let expanded = expander.clone();
    review.connect_clicked(move |_| {
        expanded.set_expanded(!expanded.is_expanded());
    });
    heading.append(&review);
    expander.set_label_widget(Some(&heading));
    root.append(&expander);
    root
}
impl Panel {
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
            let expanded = rows
                .entries
                .get(&key)
                .and_then(|entry| {
                    named(entry.root.upcast_ref(), "transfer-details")
                        .and_downcast::<gtk::Expander>()
                })
                .map(|expander| expander.is_expanded())
                .unwrap_or(page == "incoming");
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
                expander.connect_expanded_notify(move |_| {
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
        root
    }
    pub(super) fn render_rows(
        self: &Rc<Self>,
        view: &rqs_lib::service::client::state::StateView,
        selection: Option<rqs_lib::service::client::Selection>,
        ready: bool,
        _current: Option<&TransferView>,
        recipients: &BTreeMap<String, String>,
    ) {
        let mut rows = self.rows.borrow_mut();
        let mut used = BTreeSet::new();
        let mut pages: BTreeMap<&str, Vec<gtk::Box>> = ["nearby", "incoming", "transfers"]
            .into_iter()
            .map(|page| (page, vec![]))
            .collect();
        let mut endpoints = view
            .endpoints
            .iter()
            .map(|peer| (peer.id.clone(), (peer.name.clone(), true)))
            .collect::<BTreeMap<_, _>>();
        for (id, name) in recipients {
            endpoints
                .entry(id.clone())
                .or_insert_with(|| (name.clone(), false));
        }
        for (id, projection) in endpoints {
            let key = ("nearby".to_owned(), id.clone());
            let selected = recipients.contains_key(&id);
            if rows
                .entries
                .get(&key)
                .is_none_or(|entry| entry.endpoint.as_ref() != Some(&projection))
            {
                let root = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                root.add_css_class("device-row");
                let state = if projection.1 {
                    Badge::Available
                } else {
                    Badge::Unavailable
                };
                root.append(&device_icon(
                    rqs_lib::service::client::state::DeviceKind::Unknown,
                    if projection.1 {
                        "Nearby advertised device"
                    } else {
                        "Unavailable"
                    },
                    state,
                ));
                let choice = gtk::CheckButton::with_label(&projection.0);
                choice.set_hexpand(true);
                choice.set_widget_name("select-recipient");
                if let Some(title) = choice.child().and_downcast::<gtk::Label>() {
                    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    title.set_max_width_chars(24);
                }
                choice.set_active(selected);
                let endpoint = id.clone();
                let weak = Rc::downgrade(self);
                choice.connect_toggled(move |choice| {
                    if let Some(panel) = weak.upgrade() {
                        panel.choose_recipient(&endpoint, choice.is_active());
                    }
                });
                root.append(&choice);
                let send = gtk::Button::with_label("Send");
                send.set_widget_name("send-device");
                let weak = Rc::downgrade(self);
                send.connect_clicked(move |_| {
                    if let Some(panel) = weak.upgrade() {
                        panel.send_to(&id);
                    }
                });
                root.append(&send);
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
            let choice = named(root.upcast_ref(), "select-recipient")
                .and_downcast::<gtk::CheckButton>()
                .unwrap();
            choice.set_active(selected);
            choice.set_sensitive(
                ready
                    && (selected
                        || (projection.1
                            && recipients.len() < rqs_lib::service::batch::RECIPIENTS)),
            );
            if let Some(send) = named(root.upcast_ref(), "send-device") {
                send.set_sensitive(ready && projection.1);
            }
            pages.get_mut("nearby").unwrap().push(root);
            used.insert(key);
        }
        let mut active_rows = Vec::new();
        let mut terminal_rows = Vec::new();
        let recent: BTreeSet<_> = view
            .transfers
            .iter()
            .rev()
            .filter(|transfer| transfer.phase.is_terminal())
            .take(5)
            .map(|transfer| transfer.transfer.id())
            .collect();
        for transfer in &view.transfers {
            if transfer.phase.is_terminal()
                && !recent.contains(transfer.transfer.id())
                && transfer.completed_files.is_empty()
                && transfer.content_revision.is_none()
            {
                continue;
            }
            for page in ["nearby", "incoming", "transfers"] {
                let incoming = transfer.direction == Direction::Inbound
                    && transfer.phase == Phase::AwaitingConsent;
                if (page == "incoming" && !incoming)
                    || (page == "transfers" && transfer.phase.is_terminal())
                {
                    continue;
                }
                let root = self.cached_transfer(&mut rows, page, transfer, ready, incoming);
                if page == "nearby" {
                    if transfer.phase.is_terminal() {
                        terminal_rows.push(root);
                    } else {
                        active_rows.push(root);
                    }
                } else {
                    pages.get_mut(page).unwrap().push(root);
                }
                used.insert((page.to_owned(), transfer.transfer.id().to_owned()));
            }
        }
        // A transfer's advertised name is not an endpoint identity. Keep
        // unlinked finished operations accessible, but out of the discovery list.
        let devices = std::mem::take(pages.get_mut("nearby").unwrap());
        let nearby = pages.get_mut("nearby").unwrap();
        if !active_rows.is_empty() {
            nearby.push(section_title("Current transfers"));
            nearby.extend(active_rows.into_iter().rev());
        }
        nearby.push(section_title("Discovered devices"));
        if devices.is_empty() {
            let empty = card();
            empty.append(&label("No nearby devices. Make the receiving device visible."));
            nearby.push(empty);
        } else {
            nearby.extend(devices);
        }
        if !terminal_rows.is_empty() {
            let key = ("nearby".to_owned(), "activity:unlinked".to_owned());
            let entry = rows.entries.entry(key.clone()).or_insert_with(|| {
                let root = card();
                let expander = gtk::Expander::new(Some("Recent activity · device not verified"));
                expander.set_widget_name("unlinked-activity");
                expander.set_expanded(false);
                let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
                expander.set_child(Some(&body));
                let weak = Rc::downgrade(self);
                expander.connect_expanded_notify(move |_| {
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
                root.append(&expander);
                Entry {
                    root,
                    transfer: None,
                    endpoint: None,
                }
            });
            let expander = named(entry.root.upcast_ref(), "unlinked-activity")
                .and_downcast::<gtk::Expander>()
                .expect("session activity expander");
            let body = expander.child().and_downcast::<gtk::Box>()
                .expect("session activity container");
            arrange(&body, &terminal_rows);
            nearby.push(entry.root.clone());
            used.insert(key);
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
    assert_eq!(container.first_child(), Some(first.upcast()));
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
