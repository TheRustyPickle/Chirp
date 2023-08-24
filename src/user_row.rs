mod imp {
    use adw::{subclass::prelude::*, Avatar};
    use glib::subclass::InitializingObject;
    use glib::{object_subclass, Binding};
    use gtk::{glib, Box, CompositeTemplate};
    use std::cell::{OnceCell, RefCell};

    use crate::user_data::UserObject;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/user_row.xml")]
    pub struct UserRow {
        #[template_child]
        pub user_avatar: TemplateChild<Avatar>,
        pub bindings: RefCell<Vec<Binding>>,
        pub user_data: OnceCell<UserObject>,
    }

    #[object_subclass]
    impl ObjectSubclass for UserRow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "UserRow";
        type Type = super::UserRow;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for UserRow {}

    // Trait shared by all widgets
    impl WidgetImpl for UserRow {}

    // Trait shared by all boxes
    impl BoxImpl for UserRow {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::closure_local;
use glib::{wrapper, Object};
use gtk::gdk::Paintable;
use gtk::{glib, Accessible, Box, Buildable, ConstraintTarget, Orientable, Widget};

use crate::user_data::UserObject;

wrapper! {
    pub struct UserRow(ObjectSubclass<imp::UserRow>)
    @extends Box, Widget,
    @implements Accessible, Buildable, ConstraintTarget, Orientable;
}

impl UserRow {
    pub fn new(object: UserObject) -> Self {
        let row: UserRow = Object::builder().build();

        row.imp().user_data.set(object).unwrap();
        row
    }

    pub fn bind(&self) {
        let mut bindings = self.imp().bindings.borrow_mut();
    }
}
