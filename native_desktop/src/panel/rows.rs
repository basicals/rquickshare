//! Keyed rows keep focus and expanded manifests across progress/readiness updates.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

struct Entry {
    root: gtk::Box,
    transfer: Option<TransferView>,
    endpoint: Option<(String, Option<rqs_lib::service::client::Selection>)>,
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
fn same_description(a: &TransferView, b: &TransferView) -> bool {
    a.transfer == b.transfer
        && a.direction == b.direction
        && a.phase == b.phase
        && a.consent_revision == b.consent_revision
        && a.content_revision == b.content_revision
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
impl Panel {
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
            let full = page != "compact";
            let root = if incoming {
                self.incoming_card(transfer, ready, full)
            } else {
                self.transfer_card(transfer, ready, full)
            };
            if !full {
                let details = gtk::Button::with_label(if incoming {
                    "Details and all requests"
                } else {
                    "All transfers"
                });
                details.set_action_name(Some(if incoming {
                    "panel.incoming"
                } else {
                    "panel.transfers"
                }));
                root.append(&details);
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
        current: Option<&TransferView>,
    ) {
        let mut rows = self.rows.borrow_mut();
        let mut used = BTreeSet::new();
        let mut pages: BTreeMap<&str, Vec<gtk::Box>> =
            ["nearby", "incoming", "transfers", "compact"]
                .into_iter()
                .map(|page| (page, vec![]))
                .collect();
        for endpoint in &view.endpoints {
            let key = ("nearby".to_owned(), endpoint.id.clone());
            let projection = (endpoint.name.clone(), selection.clone());
            if rows
                .entries
                .get(&key)
                .is_none_or(|entry| entry.endpoint.as_ref() != Some(&projection))
            {
                let root = card();
                root.append(&label(&endpoint.name));
                root.append(&label("Nearby advertised device"));
                let send = gtk::Button::with_label("Send");
                send.set_widget_name("send");
                let weak = Rc::downgrade(self);
                let endpoint = endpoint.id.clone();
                let selection = selection.clone();
                send.connect_clicked(move |_| {
                    if let Some(panel) = weak.upgrade() {
                        if let (Some(handle), Some(selection)) = (&panel.handle, selection.clone())
                        {
                            panel.submit(|| handle.send(selection, endpoint.clone()));
                        }
                    }
                });
                root.append(&send);
                rows.entries.insert(
                    key.clone(),
                    Entry {
                        root,
                        transfer: None,
                        endpoint: Some(projection),
                    },
                );
            }
            let root = rows.entries[&key].root.clone();
            named(root.upcast_ref(), "send")
                .unwrap()
                .set_sensitive(ready && selection.is_some());
            pages.get_mut("nearby").unwrap().push(root);
            used.insert(key);
        }
        for transfer in &view.transfers {
            for page in ["incoming", "transfers"] {
                let incoming = transfer.direction == Direction::Inbound
                    && transfer.phase == Phase::AwaitingConsent;
                if page == "incoming" && !incoming {
                    continue;
                }
                let root =
                    self.cached_transfer(&mut rows, page, transfer, ready, page == "incoming");
                pages.get_mut(page).unwrap().push(root);
                used.insert((page.to_owned(), transfer.transfer.id().to_owned()));
            }
        }
        if let Some(transfer) = current {
            let incoming = transfer.direction == Direction::Inbound
                && transfer.phase == Phase::AwaitingConsent;
            let root = self.cached_transfer(&mut rows, "compact", transfer, ready, incoming);
            pages.get_mut("compact").unwrap().push(root);
            used.insert(("compact".into(), transfer.transfer.id().to_owned()));
        }
        for (page, title) in [
            (
                "nearby",
                "No nearby devices. Make the receiving device visible.",
            ),
            ("incoming", "No incoming requests"),
            ("transfers", "No transfers"),
            (
                "compact",
                if selection.is_some() {
                    "Files prepared · choose a recipient"
                } else {
                    "Ready to share"
                },
            ),
        ] {
            if pages[page].is_empty() {
                let key = (page.to_owned(), format!("empty:{}", selection.is_some()));
                let entry = rows.entries.entry(key.clone()).or_insert_with(|| {
                    let root = card();
                    root.append(&label(title));
                    if page == "compact" {
                        let button = gtk::Button::with_label(if selection.is_some() {
                            "Choose recipient"
                        } else {
                            "Send files…"
                        });
                        button.set_action_name(Some(if selection.is_some() {
                            "panel.nearby"
                        } else {
                            "panel.files"
                        }));
                        root.append(&button);
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
