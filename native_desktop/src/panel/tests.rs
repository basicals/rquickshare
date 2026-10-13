use super::*;
use rqs_lib::service::{
    client::state::{FileDetail, StateView},
    ResourceUsage,
};

#[test]
fn given_complete_view_and_busy_flags_when_controls_are_projected_then_only_synchronized_live_idle_state_is_mutable(
) {
    // Arrange
    let mut state = PanelState {
        view: Some(Arc::new(StateView {
            epoch: 1,
            revision: 1,
            lifecycle: Lifecycle::Running,
            synchronized: true,
            resources: ResourceUsage::default(),
            endpoints: vec![],
            selections: vec![],
            transfers: vec![],
            warnings: vec![],
            notices: vec![],
        })),
        ..Default::default()
    };
    // Act / Assert
    assert!(controls_ready(&state, false));
    assert!(!controls_ready(&state, true));
    for flag in 0..3 {
        state.busy = flag == 0;
        state.pending = flag == 1;
        state.disconnected = flag == 2;
        assert!(!controls_ready(&state, false));
    }
    state.busy = false;
    state.pending = false;
    state.disconnected = false;
    for lifecycle in [Lifecycle::Draining, Lifecycle::Stopped] {
        Arc::make_mut(state.view.as_mut().unwrap()).lifecycle = lifecycle;
        assert!(!controls_ready(&state, false));
    }
    Arc::make_mut(state.view.as_mut().unwrap()).lifecycle = Lifecycle::Running;
    Arc::make_mut(state.view.as_mut().unwrap()).synchronized = false;
    assert!(!controls_ready(&state, false));
    state.view = None;
    assert!(!controls_ready(&state, false));
}

fn labels(widget: &impl IsA<gtk::Widget>) -> Vec<gtk::Label> {
    let widget = widget.as_ref();
    let mut found = Vec::new();
    if let Some(label) = widget.downcast_ref::<gtk::Label>() {
        found.push(label.clone());
    }
    let mut next = widget.first_child();
    while let Some(child) = next {
        found.extend(labels(&child));
        next = child.next_sibling();
    }
    found
}

#[test]
fn given_actual_builder_and_untrusted_details_when_rendered_then_preview_is_inert_manifest_is_complete_plain_text_and_close_hides(
) {
    // Arrange: this single GTK test runs on an isolated display/session bus.
    gtk::init().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let application = gtk::Application::new(
        Some("io.github.morozhenko_apps.RQuickShare.WidgetTest"),
        gio::ApplicationFlags::NON_UNIQUE,
    );
    application.register(gio::Cancellable::NONE).unwrap();
    let panel = Panel::new(
        &application,
        None,
        runtime.handle().clone(),
        Arc::new(Ingress::new()),
        false,
    );
    // Act / Assert
    assert_eq!(panel.model.borrow().mode, Mode::Compact);
    assert!(panel.box_widget("compact").get_visible());
    assert!(!panel.pages.get_visible());
    assert!(!panel.box_widget("source").get_visible());
    let permit = panel.ingress.gate.try_acquire().unwrap();
    panel
        .actions
        .activate_action("mode", Some(&"full".to_variant()));
    assert_eq!(panel.model.borrow().mode, Mode::Full);
    assert!(panel.pages.get_visible() && panel.box_widget("source").get_visible());
    assert!(!panel.box_widget("compact").get_visible());
    assert!(*panel.ingress.gate.subscribe().borrow());
    panel
        .actions
        .activate_action("mode", Some(&"invalid".to_variant()));
    assert_eq!(panel.model.borrow().mode, Mode::Full);
    drop(permit);
    assert!(!*panel.ingress.gate.subscribe().borrow());
    panel.actions.activate_action("files", None);
    assert!(panel.model.borrow().dialog.is_none());
    for name in ["files", "folder", "discard", "recover", "abort"] {
        assert!(!panel.button(name).is_sensitive(), "{name}");
    }
    for status in [DetailsStatus::Unavailable, DetailsStatus::Unsupported] {
        let row = details_card(&TransferDetails {
            status,
            ..Default::default()
        });
        let text = labels(&row)
            .iter()
            .map(|label| label.text().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains(if status == DetailsStatus::Unavailable {
            "Waiting"
        } else {
            "cannot be reviewed safely"
        }));
        assert!(!text.contains("Code:"));
    }
    let details = TransferDetails {
        status: DetailsStatus::Ready,
        sender: "<b>Unverified phone</b>".into(),
        pin: "0123".into(),
        total_bytes: 256,
        files: (0..256)
            .map(|index| FileDetail {
                name: format!("<file-{index}>.txt"),
                size: 1,
            })
            .collect::<Vec<_>>()
            .into(),
        ..Default::default()
    };
    let row = details_card(&details);
    assert!(labels(&row)
        .iter()
        .any(|label| label.text() == format!("{} · advertised device", details.sender)));
    assert!(labels(&row).iter().all(|label| !label.uses_markup()));
    assert!(labels(&row)
        .iter()
        .any(|label| label.text() == "Code: 0123"));
    let expander = row
        .last_child()
        .unwrap()
        .downcast::<gtk::Expander>()
        .unwrap();
    assert!(expander.child().is_none());
    expander.set_expanded(true);
    let file_labels = labels(&expander.child().unwrap());
    assert_eq!(file_labels.len(), 256);
    for (index, label) in file_labels.iter().enumerate() {
        assert_eq!(label.text(), format!("<file-{index}>.txt · 1 bytes"));
        assert!(!label.uses_markup());
    }
    expander.set_expanded(false);
    assert!(expander.child().is_none());
    panel.set_mode(Mode::Compact);
    panel.preview("incoming");
    let compact_text = labels(&panel.box_widget("compact"))
        .iter()
        .map(|label| label.text().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(compact_text.contains("Code: 4821"));
    assert!(compact_text.contains("1 file · 12345 bytes"));
    assert!(!compact_text.contains("Example.pdf"));
    panel.actions.activate_action("settings", None);
    let settings = panel.settings.borrow().as_ref().unwrap().clone();
    assert!(settings.is_visible());
    let appearance = panel.appearance.borrow().as_ref().unwrap().clone();
    appearance.set_selected(1);
    assert_eq!(panel.model.borrow().mode, Mode::Full);
    assert!(labels(&panel.box_widget("incoming"))
        .iter()
        .any(|label| label.text() == "Code: 4821"));
    assert_eq!(
        panel
            .actions
            .lookup_action("mode")
            .unwrap()
            .state()
            .unwrap()
            .str(),
        Some("full")
    );
    panel.set_mode(Mode::Compact);
    assert_eq!(appearance.selected(), 0);
    panel.set_mode(Mode::Full);
    assert!(!panel.box_widget("source").get_visible());
    panel.pages.set_visible_child_name("nearby");
    assert!(panel.box_widget("source").get_visible());
    panel.pages.set_visible_child_name("incoming");
    assert!(!panel.box_widget("source").get_visible());
    panel.set_mode(Mode::Compact);
    settings.close();
    assert!(!settings.is_visible());
    panel.actions.activate_action("settings", None);
    assert_eq!(*panel.settings.borrow().as_ref().unwrap(), settings);
    assert!(settings.is_visible());
    assert_eq!(
        panel.pages.visible_child_name().as_deref(),
        Some("incoming")
    );
    panel.window.present();
    assert!(panel.window.is_visible());
    panel.window.close();
    assert!(!panel.window.is_visible());
    assert!(!panel.model.borrow().closing);
    panel.submit(|| panic!("preview must not admit actions"));
    panel.window.present();
    assert!(panel.window.is_visible());
    panel.safety_update(crate::desktop_guard::SafetyState {
        locked: Some(true),
        inhibited: Some(false),
        closed: false,
    });
    assert!(!panel.window.is_visible() && !settings.is_visible());
    panel.safety_update(crate::desktop_guard::SafetyState {
        locked: Some(false),
        inhibited: Some(false),
        closed: false,
    });
    assert!(!panel.window.is_visible());
    panel.window.present();
    panel.safety_update(crate::desktop_guard::SafetyState::default());
    assert!(!panel.window.is_visible());
    panel.close();
    assert!(panel.model.borrow().closing);
    assert!(panel.settings.borrow().is_none());
    assert!(panel.appearance.borrow().is_none());
    assert!(panel.context.parent().is_none());
    assert!(!settings.is_visible());
    panel.close(); // Explicit Quit and the application cleanup may both close the panel.
}
