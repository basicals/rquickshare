//! A single-child snapshot effect; the overlay owns the interactive curtain.
use gtk::{glib, prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};

mod imp {
    use super::*;
    #[derive(Default)]
    pub struct Backdrop {
        pub child: RefCell<Option<gtk::Widget>>,
        pub blurred: Cell<bool>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Backdrop {
        const NAME: &'static str = "RqsControlBackdrop";
        type Type = super::Backdrop;
        type ParentType = gtk::Widget;
    }
    impl ObjectImpl for Backdrop {
        fn dispose(&self) {
            if let Some(child) = self.child.borrow_mut().take() {
                child.unparent();
            }
        }
    }
    impl WidgetImpl for Backdrop {
        fn measure(&self, orientation: gtk::Orientation, size: i32) -> (i32, i32, i32, i32) {
            self.child
                .borrow()
                .as_ref()
                .map_or((0, 0, -1, -1), |child| child.measure(orientation, size))
        }
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(child) = self.child.borrow().as_ref() {
                child.allocate(width, height, baseline, None);
            }
        }
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if let Some(child) = self.child.borrow().as_ref() {
                if self.blurred.get() {
                    snapshot.push_blur(4.0);
                }
                self.obj().snapshot_child(child, snapshot);
                if self.blurred.get() {
                    snapshot.pop();
                }
            }
        }
    }
}
glib::wrapper! {
    pub struct Backdrop(ObjectSubclass<imp::Backdrop>)
        @extends gtk::Widget, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl Backdrop {
    pub fn new(child: &impl IsA<gtk::Widget>) -> Self {
        let this: Self = glib::Object::new();
        child.set_parent(&this);
        this.imp().child.replace(Some(child.clone().upcast()));
        this
    }
    pub fn set_blurred(&self, blurred: bool) {
        if self.imp().blurred.replace(blurred) != blurred {
            if let Some(child) = self.imp().child.borrow().as_ref() {
                child.set_sensitive(!blurred);
            }
            self.queue_draw();
        }
    }
}
