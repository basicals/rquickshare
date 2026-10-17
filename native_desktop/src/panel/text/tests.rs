use super::*;

fn key() -> TextKey {
    TextKey {
        epoch: 123,
        id: "transfer".into(),
        revision: 7,
        kind: PayloadKind::Text,
        bytes: 3,
    }
}
fn observed() -> Observation<'static> {
    Observation {
        epoch: 123,
        id: "transfer",
        revision: Some(7),
        direction: Direction::Inbound,
        phase: Phase::Finished,
        kind: PayloadKind::Text,
        bytes: 3,
    }
}
fn unlocked() -> SafetyState {
    SafetyState {
        locked: Some(false),
        inhibited: Some(false),
        closed: false,
    }
}
#[test]
fn given_copy_identity_and_lock_inputs_when_checked_then_only_current_finished_unlocked_text_is_allowed(
) {
    // Arrange / Act / Assert
    let key = key();
    assert!(permitted(&key, Some(&observed()), unlocked(), true));
    assert!(!permitted(&key, None, unlocked(), true));
    assert!(!permitted(&key, Some(&observed()), unlocked(), false));
    for safety in [
        SafetyState::default(),
        SafetyState {
            locked: Some(true),
            ..unlocked()
        },
        SafetyState {
            closed: true,
            ..unlocked()
        },
    ] {
        assert!(!permitted(&key, Some(&observed()), safety, true));
    }
    for changed in [
        Observation {
            epoch: 124,
            ..observed()
        },
        Observation {
            id: "another",
            ..observed()
        },
        Observation {
            revision: None,
            ..observed()
        },
        Observation {
            revision: Some(8),
            ..observed()
        },
        Observation {
            direction: Direction::Outbound,
            ..observed()
        },
        Observation {
            kind: PayloadKind::Files,
            ..observed()
        },
        Observation {
            kind: PayloadKind::Url,
            ..observed()
        },
        Observation {
            bytes: 2,
            ..observed()
        },
        Observation {
            bytes: 4,
            ..observed()
        },
    ] {
        assert!(!permitted(&key, Some(&changed), unlocked(), true));
    }
    for phase in [
        Phase::Queued,
        Phase::Active,
        Phase::AwaitingConsent,
        Phase::Receiving,
        Phase::Sending,
        Phase::CancelRequested,
        Phase::Rejected,
        Phase::Cancelled,
        Phase::Failed,
        Phase::Disconnected,
    ] {
        assert!(!permitted(
            &key,
            Some(&Observation {
                phase,
                ..observed()
            }),
            unlocked(),
            true
        ));
    }
    let mut changed = key.clone();
    changed.revision = 0;
    assert!(!permitted(
        &changed,
        Some(&Observation {
            revision: Some(0),
            ..observed()
        }),
        unlocked(),
        true
    ));
    changed = key;
    changed.kind = PayloadKind::Files;
    assert!(!permitted(
        &changed,
        Some(&Observation {
            kind: PayloadKind::Files,
            ..observed()
        }),
        unlocked(),
        true
    ));
}
fn clipboard_text(clipboard: &gtk::gdk::Clipboard) -> String {
    glib::MainContext::default()
        .block_on(clipboard.read_text_future())
        .unwrap()
        .unwrap()
        .to_string()
}
pub(in crate::panel) fn verify_review_smoke(panel: &Rc<Panel>, application: &gtk::Application) {
    // Arrange: only this test display's clipboard is used; no desktop connection.
    let display = gtk::prelude::WidgetExt::display(&panel.window);
    let clipboard = display.clipboard();
    let primary = display.primary_clipboard();
    clipboard.set_text("existing clipboard");
    primary.set_text("existing selection");
    let ready = Rc::new(Cell::new(true));
    let safety = Rc::new(Cell::new(unlocked()));
    let current_epoch = Rc::new(Cell::new(123));
    let body = Arc::new(
        ReceivedText::new(
            PayloadKind::Text,
            "<b>private</b>\nhttps://example.invalid/".into(),
        )
        .unwrap(),
    );
    let weak_body = Arc::downgrade(&body);
    let mut key = key();
    key.bytes = body.text().len() as u64;
    let check_ready = ready.clone();
    let check_safety = safety.clone();
    let check_epoch = current_epoch.clone();
    let review = Review::new(Some(application), key, body, move |key| {
        permitted(
            key,
            Some(&Observation {
                epoch: check_epoch.get(),
                bytes: key.bytes,
                ..observed()
            }),
            check_safety.get(),
            check_ready.get(),
        )
    });
    // Act / Assert: creating/updating/showing never claims either clipboard.
    review.window.present();
    assert_eq!(
        review.text.text(),
        "<b>private</b>\nhttps://example.invalid/"
    );
    assert!(!review.text.uses_markup() && !review.text.is_selectable());
    review.refresh();
    assert_eq!(clipboard_text(&clipboard), "existing clipboard");
    assert_eq!(clipboard_text(&primary), "existing selection");
    // A stale enabled button must still revalidate before touching clipboard.
    safety.set(SafetyState {
        locked: Some(true),
        ..unlocked()
    });
    review.copy.emit_clicked();
    assert_eq!(clipboard_text(&clipboard), "existing clipboard");
    safety.set(unlocked());
    current_epoch.set(124);
    review.copy.emit_clicked();
    assert_eq!(clipboard_text(&clipboard), "existing clipboard");
    current_epoch.set(123);
    ready.set(false);
    review.copy.emit_clicked();
    assert_eq!(clipboard_text(&clipboard), "existing clipboard");
    ready.set(true);
    review.copy.emit_clicked();
    assert_eq!(clipboard_text(&clipboard), review.text.text());
    assert_eq!(clipboard_text(&primary), "existing selection");
    assert_eq!(review.status.text(), "Copied to this clipboard");
    let copied = clipboard_text(&clipboard);
    // Real panel lock handling destroys the owned surface and body, preserving explicit copy.
    panel.text_review.replace(Some(review.clone()));
    panel.safety_update(SafetyState {
        locked: Some(true),
        ..unlocked()
    });
    assert!(review.closed.get() && !review.window.is_visible());
    assert!(panel.text_review.borrow().is_none());
    assert!(review.body.borrow().is_none() && review.text.text().is_empty());
    assert!(weak_body.upgrade().is_none());
    review.copy.emit_clicked();
    assert_eq!(clipboard_text(&clipboard), copied);
    assert_eq!(clipboard_text(&primary), "existing selection");
}
