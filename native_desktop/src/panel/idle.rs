//! Card-only dismissal after approved idle time; never changes daemon work.
use super::*;
use std::time::{Duration, Instant};
const DELAY: Duration = Duration::from_secs(15);

#[derive(Default)]
pub(super) struct Idle {
    inside: bool,
    deadline: Option<Instant>,
    timer: Option<glib::SourceId>,
}
fn deadline(
    previous: Option<Instant>,
    now: Instant,
    eligible: bool,
    activity: bool,
) -> Option<Instant> {
    if !eligible {
        None
    } else if activity {
        Some(now + DELAY)
    } else {
        previous.or(Some(now + DELAY))
    }
}
fn free(model: &Model) -> bool {
    !model.preview
        && !model.closing
        && model.notice.is_none()
        && (!model.safety_active || model.safety.manual_allowed())
        && !model.waiting
        && !model.input_active
        && model.strip_target.is_none()
        && model.dialog.is_none()
        && !model.ingress.busy
        && !model.desktop.staging
        && !model.state.disconnected
        && !model.state.busy
        && !model.state.pending
        && !model.state.text_pending
        && !model.state.settings_pending
        && !model.state.visibility_pending
        && model.state.view.as_ref().is_some_and(|view| {
            view.synchronized
                && view.lifecycle == Lifecycle::Running
                && view.selections.is_empty()
                && view.sources.is_empty()
                && view.transfers.iter().all(|row| row.phase.is_terminal())
        })
}
impl Panel {
    fn idle_eligible(&self) -> bool {
        self.window.is_visible()
            && !self.idle.borrow().inside
            && free(&self.model.borrow())
            && !*self.ingress.gate.subscribe().borrow()
            && !self.context.is_visible()
            && !self.sending.is_visible()
            && self.reveal.borrow().is_none()
            && self.results.borrow().pending_empty()
            && !self
                .settings
                .borrow()
                .as_ref()
                .is_some_and(|w| w.is_visible())
            && !self.about.borrow().as_ref().is_some_and(|w| w.is_visible())
            && !self
                .text_review
                .borrow()
                .as_ref()
                .is_some_and(|w| w.visible())
            && !self.composer.borrow().as_ref().is_some_and(|w| w.visible())
            && !self.image.borrow().as_ref().is_some_and(|w| w.visible())
    }
    pub(super) fn stop_idle(&self) {
        let mut idle = self.idle.borrow_mut();
        if let Some(timer) = idle.timer.take() {
            timer.remove();
        }
        idle.deadline = None;
    }
    pub(super) fn sync_idle(self: &Rc<Self>) {
        self.update_idle(false);
    }
    fn update_idle(self: &Rc<Self>, activity: bool) {
        let now = Instant::now();
        let eligible = self.idle_eligible();
        let due = {
            let mut idle = self.idle.borrow_mut();
            let next = deadline(idle.deadline, now, eligible, activity);
            if next == idle.deadline && idle.timer.is_some() {
                return;
            }
            if let Some(timer) = idle.timer.take() {
                timer.remove();
            }
            idle.deadline = next;
            next
        };
        if let Some(due) = due {
            let weak = Rc::downgrade(self);
            let timer =
                glib::timeout_add_local_once(due.saturating_duration_since(now), move || {
                    if let Some(panel) = weak.upgrade() {
                        panel.idle.borrow_mut().timer.take();
                        if panel.idle_eligible() && Instant::now() >= due {
                            panel.hide();
                        } else {
                            panel.sync_idle();
                        }
                    }
                });
            self.idle.borrow_mut().timer = Some(timer);
        }
    }
    pub(super) fn watch_idle_widget(self: &Rc<Self>, widget: &impl IsA<gtk::Widget>) {
        let weak = Rc::downgrade(self);
        widget.connect_visible_notify(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.stop_idle();
                let weak = Rc::downgrade(&panel);
                // Visibility may change while its owning model is being updated.
                glib::idle_add_local_once(move || {
                    if let Some(panel) = weak.upgrade() {
                        panel.update_idle(true);
                    }
                });
            }
        });
    }
    pub(super) fn configure_idle(self: &Rc<Self>) {
        self.watch_idle_widget(&self.window);
        self.watch_idle_widget(&self.context);
        self.watch_idle_widget(&self.sending);
        let motion = gtk::EventControllerMotion::new();
        motion.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        motion.connect_enter(move |_, _, _| {
            if let Some(panel) = weak.upgrade() {
                panel.idle.borrow_mut().inside = true;
                panel.update_idle(true);
            }
        });
        let weak = Rc::downgrade(self);
        motion.connect_leave(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.idle.borrow_mut().inside = false;
                panel.update_idle(true);
            }
        });
        self.window.add_controller(motion);
        let input = gtk::EventControllerLegacy::new();
        input.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        input.connect_event(move |_, event| {
            if matches!(
                event.event_type(),
                gtk::gdk::EventType::ButtonPress
                    | gtk::gdk::EventType::KeyPress
                    | gtk::gdk::EventType::Scroll
                    | gtk::gdk::EventType::TouchBegin
            ) {
                if let Some(panel) = weak.upgrade() {
                    panel.update_idle(true);
                }
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(input);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_idle_deadline_when_redrawn_interacted_or_blocked_then_only_free_inactivity_can_expire()
    {
        // Arrange
        let now = Instant::now();
        let due = deadline(None, now, true, false).unwrap();
        // Act / Assert: redraws never postpone it, activity resets it, blockers remove it.
        assert_eq!(due, now + DELAY);
        for millis in [0, 1, 14_999, 15_000, 15_001] {
            let sample = now + Duration::from_millis(millis);
            assert_eq!(deadline(Some(due), sample, true, false), Some(due));
            assert_eq!(sample >= due, millis >= 15_000);
            assert_eq!(
                deadline(Some(due), sample, true, true),
                Some(sample + DELAY)
            );
            assert_eq!(deadline(Some(due), sample, false, false), None);
            assert_eq!(deadline(Some(due), sample, false, true), None);
        }
        assert_eq!(
            deadline(None, now + DELAY, true, false),
            Some(now + DELAY + DELAY)
        );
    }
    #[test]
    fn given_unfinished_or_unhealthy_ui_when_idle_is_checked_then_it_is_never_free() {
        use rqs_lib::service::client::state::StateView;
        // Arrange: healthy, synchronized empty model.
        let base = || Model {
            state: PanelState {
                view: Some(Arc::new(StateView {
                    epoch: 1,
                    revision: 1,
                    lifecycle: Lifecycle::Running,
                    synchronized: true,
                    sources: vec![],
                    selections: vec![],
                    endpoints: vec![],
                    transfers: vec![],
                    resources: Default::default(),
                    warnings: vec![],
                    notices: vec![],
                })),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(free(&base()));
        // Act / Assert: every independent local operation/recovery flag protects the card.
        for flag in 0..17 {
            let mut model = base();
            match flag {
                0 => model.preview = true,
                1 => model.closing = true,
                2 => model.waiting = true,
                3 => model.input_active = true,
                4 => model.strip_target = Some("peer".into()),
                5 => model.dialog = Some(gio::Cancellable::new()),
                6 => model.ingress.busy = true,
                7 => model.desktop.staging = true,
                8 => model.state.disconnected = true,
                9 => model.state.busy = true,
                10 => model.state.pending = true,
                11 => model.state.text_pending = true,
                12 => model.state.settings_pending = true,
                13 => model.state.visibility_pending = true,
                14 => model.notice = Some("Operation failed"),
                15 => Arc::make_mut(model.state.view.as_mut().unwrap()).synchronized = false,
                16 => {
                    Arc::make_mut(model.state.view.as_mut().unwrap()).lifecycle = Lifecycle::Stopped
                }
                _ => unreachable!(),
            }
            assert!(!free(&model), "guard {flag}");
        }
        let mut missing = base();
        missing.state.view = None;
        assert!(!free(&missing));
    }
}
