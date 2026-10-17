//! Display-only interpolation of a checked daemon snapshot; never expires visibility.
use super::*;
use std::time::{Duration, Instant};

fn remaining_seconds(model: &Model, now: Instant) -> Option<u64> {
    if !temporary_receiving(model) {
        return None;
    }
    let view = model.state.visibility.as_ref()?;
    if u128::from_str_radix(&view.epoch, 16).ok()? != model.state.view.as_ref()?.epoch
        || view.remaining_ms > 60_000
    {
        return None;
    }
    let elapsed = now.checked_duration_since(model.state.visibility_sampled_at?)?;
    let remaining = Duration::from_millis(view.remaining_ms).saturating_sub(elapsed);
    Some(remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0))
}

impl Panel {
    fn refresh_countdown(&self, now: Instant) -> bool {
        let model = self.model.borrow();
        let shown = self.window.is_visible()
            && !model.closing
            && model.drawer == Drawer::Open
            && temporary_receiving(&model);
        self.box_widget("receive-indicator").set_visible(shown);
        let spinner = self
            .builder
            .object::<gtk::Spinner>("receive-spinner")
            .unwrap();
        if !shown {
            spinner.stop();
            return false;
        }
        spinner.start();
        let seconds = remaining_seconds(&model, now);
        self.text(
            "receive-countdown",
            &match seconds {
                Some(seconds) if seconds > 0 => format!(
                    "Visible to nearby devices · {}:{:02}",
                    seconds / 60,
                    seconds % 60
                ),
                _ => "Updating visibility…".into(),
            },
        );
        seconds.is_some_and(|seconds| seconds > 0)
    }
    pub(super) fn sync_countdown(self: &Rc<Self>) {
        let active = self.refresh_countdown(Instant::now());
        if !active {
            if let Some(tick) = self.visibility_tick.borrow_mut().take() {
                tick.remove();
            }
        } else if self.visibility_tick.borrow().is_none() {
            let weak = Rc::downgrade(self);
            let tick = glib::timeout_add_local(Duration::from_secs(1), move || {
                let Some(panel) = weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                if panel.refresh_countdown(Instant::now()) {
                    glib::ControlFlow::Continue
                } else {
                    panel.visibility_tick.borrow_mut().take();
                    glib::ControlFlow::Break
                }
            });
            self.visibility_tick.replace(Some(tick));
        }
    }
}

#[cfg(test)]
pub(super) fn verify(panel: &Rc<Panel>) {
    let now = Instant::now();
    panel.model.borrow_mut().state.visibility_sampled_at = Some(now);
    // Arrange / Act / Assert: elapsed time is recomputed, never decremented or renewed per render.
    for (elapsed, expected) in [
        (0, 60),
        (1, 60),
        (1000, 59),
        (59_000, 1),
        (59_999, 1),
        (60_000, 0),
        (61_000, 0),
    ] {
        assert_eq!(
            remaining_seconds(&panel.model.borrow(), now + Duration::from_millis(elapsed)),
            Some(expected)
        );
    }
    assert_eq!(
        remaining_seconds(&panel.model.borrow(), now - Duration::from_millis(1)),
        None
    );
    let epoch = panel
        .model
        .borrow()
        .state
        .visibility
        .as_ref()
        .unwrap()
        .epoch
        .clone();
    panel
        .model
        .borrow_mut()
        .state
        .visibility
        .as_mut()
        .unwrap()
        .epoch = "bad".into();
    assert_eq!(remaining_seconds(&panel.model.borrow(), now), None);
    panel
        .model
        .borrow_mut()
        .state
        .visibility
        .as_mut()
        .unwrap()
        .epoch = epoch;
    panel.model.borrow_mut().state.visibility_sampled_at = None;
    assert_eq!(remaining_seconds(&panel.model.borrow(), now), None);
    panel.model.borrow_mut().state.visibility_sampled_at = Some(now);
    panel
        .model
        .borrow_mut()
        .state
        .visibility
        .as_mut()
        .unwrap()
        .remaining_ms = 60_001;
    assert_eq!(remaining_seconds(&panel.model.borrow(), now), None);
    panel
        .model
        .borrow_mut()
        .state
        .visibility
        .as_mut()
        .unwrap()
        .remaining_ms = 60_000;
    panel.set_drawer(Drawer::Open);
    panel.window.present();
    panel.sync_countdown();
    assert!(panel.box_widget("receive-indicator").get_visible());
    assert!(panel.visibility_tick.borrow().is_some());
    assert!(panel
        .builder
        .object::<gtk::Label>("receive-countdown")
        .unwrap()
        .text()
        .contains("1:00"));
    panel.set_drawer(Drawer::Closed);
    assert!(!panel.box_widget("receive-indicator").get_visible());
    assert!(panel.visibility_tick.borrow().is_none());
    panel.set_drawer(Drawer::Open);
    assert!(panel.visibility_tick.borrow().is_some());
    panel.hide();
    panel.sync_countdown();
    assert!(panel.visibility_tick.borrow().is_none());
    panel.window.present();
    panel.sync_countdown();
    assert!(panel.visibility_tick.borrow().is_some());
}
