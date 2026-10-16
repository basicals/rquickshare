use super::*;
use rqs_lib::service::client::{
    state::{Endpoint, StateView},
    OperationKind, Receipt,
};

fn choices(panel: &Panel) -> Vec<gtk::CheckButton> {
    fn collect(widget: &gtk::Widget, result: &mut Vec<gtk::CheckButton>) {
        if let Some(choice) = widget.downcast_ref::<gtk::CheckButton>() {
            result.push(choice.clone());
        }
        let mut child = widget.first_child();
        while let Some(widget) = child {
            collect(&widget, result);
            child = widget.next_sibling();
        }
    }
    let mut result = vec![];
    collect(panel.box_widget("nearby").upcast_ref(), &mut result);
    result
}
pub(in crate::panel) fn verify_ui(panel: &Rc<Panel>) {
    // Arrange: real widgets on the existing single GTK smoke thread; no send handle.
    let (previous, preview, mode, safety, safety_active, page) = {
        let model = panel.model.borrow();
        (
            model.state.clone(),
            model.preview,
            model.mode,
            model.safety,
            model.safety_active,
            panel.pages.visible_child_name(),
        )
    };
    let view = StateView {
        epoch: 1,
        revision: 1,
        lifecycle: Lifecycle::Running,
        synchronized: true,
        resources: Default::default(),
        sources: vec![],
        selections: vec![],
        transfers: vec![],
        warnings: vec![],
        notices: vec![],
        endpoints: (0..12)
            .map(|n| Endpoint {
                id: format!("peer{n:02}"),
                name: format!("<Phone {n}>"),
            })
            .collect(),
    };
    {
        let mut model = panel.model.borrow_mut();
        model.preview = false;
        model.safety_active = false;
        model.state = PanelState {
            view: Some(Arc::new(view.clone())),
            ..Default::default()
        };
        model.recipients.clear();
    }
    panel.set_mode(Mode::Full);
    panel.pages.set_visible_child_name("nearby");
    panel.render();
    // Act: toggling actual widgets does not require or submit a prepared source.
    for choice in choices(panel).into_iter().take(10) {
        choice.set_active(true);
    }
    // Assert: exact limit, explicit Send remains disabled without a source.
    assert_eq!(panel.model.borrow().recipients.len(), 10);
    assert!(!panel.model.borrow().waiting);
    assert!(panel.model.borrow().notice.is_none());
    assert!(choices(panel)
        .iter()
        .skip(10)
        .all(|choice| !choice.is_sensitive()));
    choices(panel)[10].set_active(true);
    assert!(!choices(panel)[10].is_active());
    assert_eq!(panel.model.borrow().recipients.len(), 10);
    assert!(!panel
        .actions
        .lookup_action("send-many")
        .unwrap()
        .is_enabled());
    assert!(send_ready(&panel.model.borrow(), false, true));
    assert!(!send_ready(&panel.model.borrow(), true, true));
    // A disappeared selection stays visible and blocks Send until explicit uncheck.
    let mut unavailable = view.clone();
    unavailable.endpoints.remove(0);
    panel.update(PanelState {
        view: Some(Arc::new(unavailable)),
        ..Default::default()
    });
    assert_eq!(choices(panel).len(), 12);
    assert!(choices(panel)[0].is_active());
    assert!(!send_ready(&panel.model.borrow(), false, true));
    assert_eq!(
        panel
            .builder
            .object::<gtk::Label>("recipient-summary")
            .unwrap()
            .text(),
        "10 selected · 1 unavailable"
    );
    choices(panel)[0].set_active(false);
    assert_eq!(panel.model.borrow().recipients.len(), 9);
    assert_eq!(choices(panel).len(), 11);
    assert!(send_ready(&panel.model.borrow(), false, true));
    // Preview, pending, lock, busy, disconnect and close never admit new choices.
    for case in 0..7 {
        {
            let mut model = panel.model.borrow_mut();
            model.preview = case == 0;
            model.state.pending = case == 1;
            model.safety_active = case == 2;
            model.safety.locked = Some(case == 2);
            model.waiting = case == 3;
            model.state.disconnected = case == 4;
            model.closing = case == 5;
            model.state.busy = case == 6;
        }
        choices(panel).last().unwrap().set_active(true);
        assert_eq!(panel.model.borrow().recipients.len(), 9, "case {case}");
        assert!(!choices(panel).last().unwrap().is_active());
        assert!(!send_ready(&panel.model.borrow(), false, true));
    }
    {
        let mut model = panel.model.borrow_mut();
        model.waiting = false;
        model.closing = false;
        model.preview = false;
        model.state.busy = false;
        model.state.disconnected = false;
    }
    assert!(confirmed(&Ok(ActionResult::Transfers(vec![]))));
    assert!(confirmed(&Ok(ActionResult::Recovered(Some(
        Outcome::Operation(Receipt {
            sequence: 1,
            operation: OperationKind::SendMany,
            result: OperationValue::Transfers(vec![]),
        })
    )))));
    assert!(!confirmed(&Err(ControllerError::Client(
        rqs_lib::service::client::ClientError::AcknowledgementPending
    ))));
    assert!(!confirmed(&Ok(ActionResult::Completed)));
    let mut restarted = view;
    restarted.epoch = 2;
    panel.update(PanelState {
        view: Some(Arc::new(restarted)),
        ..Default::default()
    });
    assert!(panel.model.borrow().recipients.is_empty());
    assert!(choices(panel).iter().all(|choice| !choice.is_active()));
    {
        let mut model = panel.model.borrow_mut();
        model.state = previous;
        model.preview = preview;
        model.safety = safety;
        model.safety_active = safety_active;
    }
    panel.set_mode(mode);
    if let Some(page) = page {
        panel.pages.set_visible_child_name(&page);
    }
    panel.render();
}
