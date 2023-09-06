mod imp {
    use adw::{subclass::prelude::*, Window};
    use glib::object_subclass;
    use glib::subclass::InitializingObject;
    use gtk::{glib, CompositeTemplate};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/user_profile.xml")]
    pub struct UserProfile {}

    #[object_subclass]
    impl ObjectSubclass for UserProfile {
        const NAME: &'static str = "UserProfile";
        type Type = super::UserProfile;
        type ParentType = Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UserProfile {}

    impl WidgetImpl for UserProfile {}

    impl WindowImpl for UserProfile {}

    impl AdwWindowImpl for UserProfile {}

    impl MessageDialogImpl for UserProfile {}
}

use adw::subclass::prelude::*;
use adw::{prelude::*, ResponseAppearance, ActionRow};
use gio::glib::clone;
use glib::{wrapper, Object};
use gtk::{
    glib, Accessible, Buildable, ConstraintTarget, Native, Orientable, Root, ShortcutManager,
    Widget, Window, CheckButton, Align,
};
use tracing::info;

wrapper! {
    pub struct UserProfile(ObjectSubclass<imp::UserProfile>)
    @extends Widget, Window,
    @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl UserProfile {
    pub fn new() -> Self {
        let obj: UserProfile = Object::builder().build();
        obj
    }

    pub fn bind(&self) {}
}
