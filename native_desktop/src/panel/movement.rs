//! Header-only movement of the existing layer surface; no global pointer capture.
use super::*;

pub(super) fn clamp_position(
    position: (i32, i32),
    size: (i32, i32),
    monitor: (i32, i32),
) -> Option<(i32, i32)> {
    if size.0 <= 0 || size.1 <= 0 || monitor.0 < size.0 || monitor.1 < size.1 {
        return None;
    }
    Some((
        position.0.clamp(0, monitor.0 - size.0),
        position.1.clamp(0, monitor.1 - size.1),
    ))
}
impl Panel {
    pub(super) fn clamp_moved_surface(&self, height: i32) {
        if !self.layer || !self.model.borrow().dragged {
            return;
        }
        let Some(monitor) = self.window.monitor() else {
            return;
        };
        let geometry = monitor.geometry();
        if let Some((left, top)) = clamp_position(
            (
                self.window.margin(LayerEdge::Left),
                self.window.margin(LayerEdge::Top),
            ),
            (420, height),
            (geometry.width(), geometry.height()),
        ) {
            self.window.set_margin(LayerEdge::Left, left);
            self.window.set_margin(LayerEdge::Top, top);
        }
    }
    pub(super) fn configure_movement(self: &Rc<Self>) {
        let handle = self.box_widget("drag-handle");
        handle.set_cursor_from_name(Some("grab"));
        let drag = gtk::GestureDrag::new();
        drag.set_button(1);
        let weak = Rc::downgrade(self);
        drag.connect_drag_begin(move |gesture, x, y| {
            if let Some(panel) = weak.upgrade() {
                if !panel.layer
                    || panel.model.borrow().closing
                    || (panel.model.borrow().safety_active
                        && !panel.model.borrow().safety.manual_allowed())
                {
                    gesture.set_state(gtk::EventSequenceState::Denied);
                    return;
                }
                let mut target = panel.box_widget("root").pick(x, y, gtk::PickFlags::DEFAULT);
                while let Some(widget) = target {
                    if widget.is::<gtk::Button>()
                        || widget.is::<gtk::CheckButton>()
                        || widget.is::<gtk::Expander>()
                        || widget.is::<gtk::ScrolledWindow>()
                    {
                        gesture.set_state(gtk::EventSequenceState::Denied);
                        return;
                    }
                    target = widget.parent();
                }
                panel.mark_manual();
                panel.model.borrow_mut().dragged = true;
                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        });
        let weak = Rc::downgrade(self);
        drag.connect_drag_update(move |_, x, y| {
            let Some(panel) = weak.upgrade() else { return };
            if !panel.layer || !panel.window.is_visible() {
                return;
            }
            let Some(monitor) = panel.window.monitor() else {
                return;
            };
            let geometry = monitor.geometry();
            let (width, height) = (panel.window.width(), panel.window.height());
            let left = if panel.window.is_anchor(LayerEdge::Left) {
                panel.window.margin(LayerEdge::Left)
            } else {
                geometry.width() - width - panel.window.margin(LayerEdge::Right)
            };
            let top = if panel.window.is_anchor(LayerEdge::Top) {
                panel.window.margin(LayerEdge::Top)
            } else {
                geometry.height() - height - panel.window.margin(LayerEdge::Bottom)
            };
            // Event offsets are surface-local: account for the surface moving under the pointer.
            let Some((left, top)) = clamp_position(
                (
                    left.saturating_add(x.round() as i32),
                    top.saturating_add(y.round() as i32),
                ),
                (width, height),
                (geometry.width(), geometry.height()),
            ) else {
                return;
            };
            for edge in [
                LayerEdge::Left,
                LayerEdge::Right,
                LayerEdge::Top,
                LayerEdge::Bottom,
            ] {
                panel.window.set_anchor(edge, false);
                panel.window.set_margin(edge, 0);
            }
            panel.window.set_anchor(LayerEdge::Left, true);
            panel.window.set_anchor(LayerEdge::Top, true);
            panel.window.set_margin(LayerEdge::Left, left);
            panel.window.set_margin(LayerEdge::Top, top);
        });
        self.box_widget("root").add_controller(drag);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_monitor_and_card_boundaries_when_moved_then_every_position_is_clamped_or_rejected() {
        for size in [(420, 120), (420, 500), (1920, 1080)] {
            for position in [
                (i32::MIN, i32::MIN),
                (-1, -1),
                (0, 0),
                (1, 1),
                (1920, 1080),
                (i32::MAX, i32::MAX),
            ] {
                let (x, y) = clamp_position(position, size, (1920, 1080)).unwrap();
                assert_eq!(x, position.0.clamp(0, 1920 - size.0));
                assert_eq!(y, position.1.clamp(0, 1080 - size.1));
            }
        }
        for (size, monitor) in [
            ((0, 1), (1920, 1080)),
            ((1, 0), (1920, 1080)),
            ((1921, 1), (1920, 1080)),
            ((1, 1081), (1920, 1080)),
            ((1, 1), (0, 0)),
        ] {
            assert_eq!(clamp_position((0, 0), size, monitor), None);
        }
    }
}
