use super::*;
use rqs_lib::service::client::state::FileDetail;
fn details() -> TransferDetails {
    TransferDetails {
        status: DetailsStatus::Ready,
        kind: PayloadKind::Files,
        sender: "phone".into(),
        total_bytes: 8,
        transferred_bytes: 8,
        files: vec![
            FileDetail {
                name: "<b>untrusted</b>.txt".into(),
                size: 8,
            },
            FileDetail {
                name: "nested/empty".into(),
                size: 0,
            },
        ]
        .into(),
        ..Default::default()
    }
}
#[test]
fn given_reveal_identity_when_epoch_revision_index_or_completion_differs_then_external_action_is_not_authorized(
) {
    // Arrange / Act / Assert
    let details = details();
    let observed = Observation {
        epoch: 123,
        id: "transfer",
        revision: Some(7),
        direction: Direction::Inbound,
        phase: Phase::Finished,
        details: &details,
    };
    assert!(matches(123, "transfer", 7, 0, &observed));
    assert!(matches(123, "transfer", 7, 1, &observed));
    for (epoch, id, revision, index) in [
        (124, "transfer", 7, 0),
        (123, "other", 7, 0),
        (123, "transfer", 0, 0),
        (123, "transfer", 8, 0),
        (123, "transfer", 7, 2),
        (123, "transfer", 7, u32::MAX),
    ] {
        assert!(!matches(epoch, id, revision, index, &observed));
    }
    for (direction, phase, revision) in [
        (Direction::Outbound, Phase::Finished, Some(7)),
        (Direction::Inbound, Phase::Receiving, Some(7)),
        (Direction::Inbound, Phase::Failed, Some(7)),
        (Direction::Inbound, Phase::Finished, None),
    ] {
        assert!(!matches(
            123,
            "transfer",
            7,
            0,
            &Observation {
                direction,
                phase,
                revision,
                ..observed
            }
        ));
    }
    for invalid in [
        TransferDetails {
            status: DetailsStatus::Unsupported,
            ..details.clone()
        },
        TransferDetails {
            kind: PayloadKind::Text,
            ..details.clone()
        },
        TransferDetails {
            transferred_bytes: 7,
            ..details.clone()
        },
    ] {
        assert!(!matches(
            123,
            "transfer",
            7,
            0,
            &Observation {
                details: &invalid,
                ..observed
            }
        ));
    }
}
pub(in crate::panel) fn verify_reveal_smoke() {
    // Arrange: actual shared file-button adapter; command boundary is the external dependency.
    let calls = Rc::new(RefCell::new(Vec::new()));
    let capture = calls.clone();
    let action: Rc<FileAction> = Rc::new(move |row, index| {
        let calls = capture.clone();
        row.append(&file_button(index, true, move |index| {
            calls.borrow_mut().push(index)
        }));
    });
    let row = details_card_mode_with_files(&details(), true, Some(action));
    let expander = row
        .last_child()
        .unwrap()
        .downcast::<gtk::Expander>()
        .unwrap();
    assert!(expander.child().is_none() && calls.borrow().is_empty());
    expander.set_expanded(true);
    assert!(calls.borrow().is_empty()); // Rendering and expansion cannot acquire or launch.
    let list = expander.child().unwrap().downcast::<gtk::Box>().unwrap();
    let first = list.first_child().unwrap().downcast::<gtk::Box>().unwrap();
    let label = first
        .first_child()
        .unwrap()
        .downcast::<gtk::Label>()
        .unwrap();
    assert!(!label.uses_markup());
    assert_eq!(label.text(), "<b>untrusted</b>.txt · 8 bytes");
    let button = first
        .last_child()
        .unwrap()
        .downcast::<gtk::Button>()
        .unwrap();
    assert_eq!(button.label().as_deref(), Some("Show in folder"));
    button.emit_clicked();
    let second = first
        .next_sibling()
        .unwrap()
        .downcast::<gtk::Box>()
        .unwrap();
    second
        .last_child()
        .unwrap()
        .downcast::<gtk::Button>()
        .unwrap()
        .emit_clicked();
    assert_eq!(*calls.borrow(), vec![0, 1]);
    expander.set_expanded(false);
    assert!(expander.child().is_none());
    assert_eq!(*calls.borrow(), vec![0, 1]);
}
