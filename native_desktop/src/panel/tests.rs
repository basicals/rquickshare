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
            sources: vec![],
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
    for flag in 0..4 {
        state.busy = flag == 0;
        state.pending = flag == 1;
        state.disconnected = flag == 2;
        state.text_pending = flag == 3;
        assert!(!controls_ready(&state, false));
    }
    state.busy = false;
    state.pending = false;
    state.disconnected = false;
    state.text_pending = false;
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

fn expand_details(widget: &impl IsA<gtk::Widget>) {
    let widget = widget.as_ref();
    if let Some(expander) = widget.downcast_ref::<gtk::Expander>() {
        expander.set_expanded(true);
    }
    let mut next = widget.first_child();
    while let Some(child) = next {
        expand_details(&child);
        next = child.next_sibling();
    }
}

#[test]
fn given_actual_builder_and_untrusted_details_when_rendered_then_preview_is_inert_manifest_is_complete_plain_text_and_close_hides(
) {
    // Arrange: this single GTK test runs on an isolated display/session bus.
    gtk::init().unwrap();
    let css_errors = Rc::new(RefCell::new(Vec::new()));
    let observed = css_errors.clone();
    let css = gtk::CssProvider::new();
    css.connect_parsing_error(move |_, _, error| observed.borrow_mut().push(error.to_string()));
    css.load_from_string(include_str!("../../ui/panel.css"));
    assert!(css_errors.borrow().is_empty(), "{:?}", css_errors.borrow());
    rows::verify_keyed_layout();
    reveal::tests::verify_reveal_smoke();
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
    assert_eq!(panel.model.borrow().drawer, Drawer::Closed);
    assert!(panel.box_widget("compact").get_visible());
    assert!(!panel.pages.get_visible());
    assert!(!panel.box_widget("source").get_visible());
    for id in ["menu", "badge", "visibility", "tabs"] {
        assert!(panel.builder.object::<gtk::Widget>(id).is_none(), "{id}");
    }
    assert!(panel.window.has_css_class("rqs-card-window"));
    assert!(panel.box_widget("root").has_css_class("rqs-card-body"));
    assert_eq!(panel.box_widget("root").overflow(), gtk::Overflow::Hidden);
    assert_eq!(
        panel.button("quick-hide").action_name().as_deref(),
        Some("panel.hide")
    );
    for text in labels(&panel.window) {
        assert!(!["rQuickShare", "Ready", "Ready to share"].contains(&text.text().as_str()));
        assert!(!text.text().starts_with("Choose files"));
    }
    let permit = panel.ingress.gate.try_acquire().unwrap();
    panel.window.present();
    panel.button("quick-hide").emit_clicked();
    assert!(!panel.window.is_visible() && !panel.model.borrow().closing);
    assert!(*panel.ingress.gate.subscribe().borrow());
    panel.button("quick-devices").emit_clicked();
    assert_eq!(panel.model.borrow().drawer, Drawer::Open);
    assert!(panel.pages.get_visible());
    assert!(!panel.box_widget("source").get_visible());
    for removed in ["header", "mode", "settings-button", "close", "send-entry"] {
        assert!(panel.builder.object::<gtk::Widget>(removed).is_none());
    }
    assert!(panel.box_widget("compact").get_visible());
    assert_eq!(
        panel.box_widget("device-drawer").next_sibling(),
        Some(panel.box_widget("compact").upcast())
    );
    let scroll = panel
        .builder
        .object::<gtk::ScrolledWindow>("nearby-scroll")
        .unwrap();
    assert_eq!(scroll.min_content_height(), 0);
    assert_eq!(scroll.max_content_height(), 300);
    assert!(panel.box_widget("compact").get_visible());
    assert!(*panel.ingress.gate.subscribe().borrow());
    panel.button("quick-devices").emit_clicked();
    assert_eq!(panel.drawer(), Drawer::Closed);
    assert!(!panel.box_widget("device-drawer").get_visible());
    panel.button("quick-devices").emit_clicked();
    assert_eq!(panel.drawer(), Drawer::Open);
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
    for kind in [
        rqs_lib::sessions::details::PayloadKind::Text,
        rqs_lib::sessions::details::PayloadKind::Url,
    ] {
        let text_details = TransferDetails {
            kind,
            status: DetailsStatus::Ready,
            sender: "phone".into(),
            pin: "0123".into(),
            total_bytes: 3,
            ..Default::default()
        };
        for full in [false, true] {
            let row = details_card_mode(&text_details, full);
            let texts = labels(&row)
                .iter()
                .map(|label| label.text().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            assert!(
                texts.contains(if kind == rqs_lib::sessions::details::PayloadKind::Url {
                    "Link · 3 bytes"
                } else {
                    "Text · 3 bytes"
                })
            );
            assert!(texts.contains("Code: 0123"));
            assert!(!texts.contains("files") && !texts.contains("Review all files"));
            assert!(labels(&row).iter().all(|label| !label.uses_markup()));
        }
    }
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
    panel.set_drawer(Drawer::Closed);
    panel.preview("incoming");
    let compact_text = labels(&panel.box_widget("compact"))
        .iter()
        .map(|label| label.text().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(panel.box_widget("compact").observe_children().n_items(), 4);
    let quick = panel.box_widget("compact").observe_children();
    for (index, expected) in ["quick-receive", "quick-send", "quick-devices", "quick-hide"]
        .iter()
        .enumerate()
    {
        assert_eq!(
            quick
                .item(index as u32)
                .unwrap()
                .downcast::<gtk::Widget>()
                .unwrap(),
            panel.button(expected).upcast::<gtk::Widget>()
        );
    }

    for action in ["Receive", "Send", "Hide", "Devices"] {
        assert!(compact_text.contains(action));
    }
    assert!(!compact_text.contains("Code: 4821"));
    expand_details(&panel.box_widget("incoming"));
    let reviewed = labels(&panel.box_widget("incoming"))
        .iter()
        .map(|label| label.text().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(reviewed.contains("Code: 4821") && reviewed.contains("Example.pdf"));
    assert!(!compact_text.contains("Example.pdf"));
    panel.actions.activate_action("settings", None);
    let settings = panel.settings.borrow().as_ref().unwrap().clone();
    assert!(settings.is_visible());
    panel.actions.activate_action("about", None);
    let about = panel.about.borrow().as_ref().unwrap().clone();
    assert!(about.is_visible());
    assert_eq!(about.program_name().as_deref(), Some("rQuickShare"));
    assert_eq!(
        about.logo_icon_name().as_deref(),
        Some(crate::tray::ICON_NAME)
    );
    about.close();
    assert!(!about.is_visible());
    panel.actions.activate_action("about", None);
    assert_eq!(*panel.about.borrow().as_ref().unwrap(), about);
    assert!(about.is_visible());
    assert_eq!(panel.drawer(), Drawer::Closed);
    settings::verify_editor_smoke(panel.settings_owner.borrow().as_ref().unwrap());
    assert!(labels(&panel.box_widget("incoming"))
        .iter()
        .any(|label| label.text() == "Code: 4821"));
    assert_eq!(
        panel
            .actions
            .lookup_action("devices")
            .unwrap()
            .state()
            .unwrap()
            .get::<bool>(),
        Some(true)
    );
    panel.set_drawer(Drawer::Closed);
    panel.set_drawer(Drawer::Open);
    assert!(panel.pages.get_visible()); // Full content stays configured while Cancel hides its parent.
    assert!(panel.builder.object::<gtk::Widget>("tabs").is_none());
    assert_eq!(panel.pages.visible_child_name().as_deref(), Some("nearby"));
    panel.pages.set_visible_child_name("incoming");
    panel.set_drawer(Drawer::Closed);
    settings.close();
    assert!(!settings.is_visible());
    panel.actions.activate_action("settings", None);
    assert_eq!(*panel.settings.borrow().as_ref().unwrap(), settings);
    assert!(settings.is_visible());
    tray::verify_tray_projection(&panel);
    recipients::tests::verify_ui(&panel);
    image::tests::verify_image_smoke(&panel, &application);
    compose::tests::verify_composer_smoke(&panel, &application);
    text::tests::verify_review_smoke(&panel, &application);
    panel.safety_update(crate::desktop_guard::SafetyState {
        locked: Some(false),
        inhibited: Some(false),
        closed: false,
    });
    assert_eq!(
        panel.pages.visible_child_name().as_deref(),
        Some("nearby") // The mockup keeps requests in the single device list.
    );
    panel.window.present();
    assert!(panel.window.is_visible());
    assert!(panel.activate_at((1, 1)));
    assert!(!panel.window.is_visible());
    assert!(panel.activate_at((1, 1)));
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
    // Explicit unavailable backend is distinct from preview and has visible recovery.
    {
        let mut model = panel.model.borrow_mut();
        model.preview = false;
        model.preview_page = None;
        model.state.disconnected = true;
    }
    panel.render();
    assert_eq!(
        panel.builder.object::<gtk::Label>("status").unwrap().text(),
        "Transfer service unavailable. Check installation, then reconnect."
    );
    assert!(panel.reconnect_button.get_visible());
    assert!(panel.box_widget("recovery").get_visible());
    assert!(!panel.button("recover").get_visible() && !panel.button("abort").get_visible());
    assert!(!panel.button("files").is_sensitive());
    assert!(panel
        .actions
        .lookup_action("reconnect")
        .unwrap()
        .is_enabled());
    panel.actions.activate_action("reconnect", None);
    assert!(panel.reconnect_requested());
    panel.close();
    assert!(panel.model.borrow().closing);
    assert!(panel.settings.borrow().is_none());
    assert!(panel.about.borrow().is_none());
    assert!(!about.is_visible());
    assert!(panel.settings_owner.borrow().is_none());
    assert!(panel.context.parent().is_none());
    assert!(!settings.is_visible());
    panel.close(); // Explicit Quit and the application cleanup may both close the panel.
}

// Optional actual-widget frames; no connected writer, timer-based state or daemon transfer fixture.
pub(super) fn capture_card(panel: &Rc<Panel>, name: &str) {
    use gtk::gdk::prelude::PaintableExt;
    let Some(output) = std::env::var_os("RQS_CARD_PREVIEW_OUTPUT") else {
        return;
    };
    let output = std::path::PathBuf::from(output);
    assert!(output.is_absolute());
    std::fs::create_dir_all(&output).unwrap();
    let destination = output.join(format!("{name}.png"));
    panel.window.present();
    panel.window.queue_draw();
    let finished = Rc::new(Cell::new(false));
    let captured = finished.clone();
    let loop_ = glib::MainLoop::new(None, false);
    let completed = loop_.clone();
    let clock = panel.window.frame_clock().unwrap();
    let window = panel.window.clone();
    // Snapshot only after GTK has painted the new layout, not the previous frame cached at tick time.
    let paint = clock.connect_after_paint(move |_| {
        if window.width() <= 0 || window.height() != window.default_height() {
            return;
        }
        let paintable = gtk::WidgetPaintable::new(Some(&window));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, window.width().into(), window.height().into());
        let Some(node) = snapshot.to_node() else {
            return;
        };
        window
            .renderer()
            .unwrap()
            .render_texture(&node, None)
            .save_to_png(&destination)
            .unwrap();
        captured.set(true);
        completed.quit();
    });
    let expired = Rc::new(Cell::new(false));
    let timeout_fired = expired.clone();
    let deadline = loop_.clone();
    let timeout = glib::timeout_add_local_once(std::time::Duration::from_secs(10), move || {
        timeout_fired.set(true);
        deadline.quit();
    });
    loop_.run();
    if !expired.get() {
        timeout.remove();
    }
    clock.disconnect(paint);
    assert!(
        finished.get(),
        "actual card frame did not become renderable"
    );
}
