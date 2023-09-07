mod imp {
    use adw::{subclass::prelude::*, MessageDialog};
    use glib::object_subclass;
    use glib::subclass::InitializingObject;
    use gtk::{glib, CompositeTemplate, Entry};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/user_prompt.xml")]
    pub struct UserPrompt {
        #[template_child]
        pub id_entry: TemplateChild<Entry>,
    }

    #[object_subclass]
    impl ObjectSubclass for UserPrompt {
        const NAME: &'static str = "UserPrompt";
        type Type = super::UserPrompt;
        type ParentType = MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UserPrompt {}

    impl WidgetImpl for UserPrompt {}

    impl WindowImpl for UserPrompt {}

    impl MessageDialogImpl for UserPrompt {}
}

use adw::subclass::prelude::*;
use adw::MessageDialog;
use adw::{prelude::*, ResponseAppearance};
use gio::glib::clone;
use glib::{wrapper, Object};
use gtk::{
    glib, Accessible, Buildable, ConstraintTarget, Native, Orientable, Root, ShortcutManager,
    Widget, Window,
};
use tracing::info;

use crate::window;

wrapper! {
    pub struct UserPrompt(ObjectSubclass<imp::UserPrompt>)
    @extends MessageDialog, Widget, Window,
    @implements Accessible, Buildable, ConstraintTarget, Orientable, Native, Root, ShortcutManager;
}

impl UserPrompt {
    pub fn new(window: &window::Window) -> Self {
        let obj: UserPrompt = Object::builder().build();
        obj.add_responses(&[("cancel", "Cancel"), ("chat", "Start Chat")]);
        obj.set_response_enabled("chat", false);
        obj.set_response_appearance("chat", ResponseAppearance::Suggested);
        obj.set_transient_for(Some(window));
        obj.imp().id_entry.add_css_class("blue-entry");
        obj.imp()
            .id_entry
            .connect_changed(clone!(@weak obj as prompt => move |entry| {
                let text = entry.text();
                let empty = text.is_empty();

                prompt.set_response_enabled("chat", !empty);

                if empty {
                    entry.remove_css_class("blue-entry");
                    entry.add_css_class("error");
                } else {
                    entry.remove_css_class("error");
                    entry.add_css_class("blue-entry");
                }
            }));

        let entry = obj.imp().id_entry.get();

        obj.connect_response(
            None,
            clone!(@weak window, @weak entry => move |dialog, response| {
                if response != "chat" {
                    return;
                }
                let entry_data = entry.text();
                info!("Entry data: {}", entry_data);
                let conn = window.get_chatting_from().user_ws();
                conn.get_user_data(entry_data.parse().unwrap());
                dialog.destroy();
            }),
        );

        obj
    }

    pub fn bind(&self) {}
}
