//! One horizontal size owner; opening the drawer changes only vertical content.
use gtk::{glib, prelude::*, subclass::prelude::*};
use std::cell::RefCell;

pub(super) const WIDTH: i32 = 420;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Surface {
        pub child: RefCell<Option<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Surface {
        const NAME: &'static str = "RqsCardSurface";
        type Type = super::Surface;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Surface {
        fn dispose(&self) {
            if let Some(child) = self.child.borrow_mut().take() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Surface {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, size: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                return (WIDTH, WIDTH, -1, -1);
            }
            self.child
                .borrow()
                .as_ref()
                .map_or((0, 0, -1, -1), |child| {
                    child.measure(orientation, if size < 0 { WIDTH } else { size })
                })
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(child) = self.child.borrow().as_ref() {
                child.allocate(width, height, baseline, None);
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if let Some(child) = self.child.borrow().as_ref() {
                self.obj().snapshot_child(child, snapshot);
            }
        }
    }
}

glib::wrapper! {
    pub struct Surface(ObjectSubclass<imp::Surface>)
        @extends gtk::Widget, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Surface {
    pub fn new(child: &impl IsA<gtk::Widget>) -> Self {
        let surface: Self = glib::Object::new();
        child.set_parent(&surface);
        surface.imp().child.replace(Some(child.clone().upcast()));
        surface
    }
}
