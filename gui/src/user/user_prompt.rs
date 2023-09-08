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

use crate::user::UserProfile;
use crate::window;

wrapper! {
    pub struct UserPrompt(ObjectSubclass<imp::UserPrompt>)
    @extends MessageDialog, Widget, Window,
    @implements Accessible, Buildable, ConstraintTarget, Orientable, Native, Root, ShortcutManager;
}

impl UserPrompt {
    pub fn new(accept_name: &str) -> Self {
        let obj: UserPrompt = Object::builder().build();
        obj.add_responses(&[("cancel", "Cancel"), ("accept", accept_name)]);
        obj.set_response_enabled("accept", false);
        obj.set_response_appearance("accept", ResponseAppearance::Suggested);
        obj.imp().id_entry.add_css_class("blue-entry");

        obj.imp()
            .id_entry
            .connect_changed(clone!(@weak obj as prompt => move |entry| {
                let text = entry.text();
                let empty = text.is_empty();

                prompt.set_response_enabled("accept", !empty);

                if empty {
                    entry.remove_css_class("blue-entry");
                    entry.add_css_class("error");
                } else {
                    entry.remove_css_class("error");
                    entry.add_css_class("blue-entry");
                }
            }));

        obj
    }

    pub fn add_user(self, window: &window::Window) -> Self {
        self.set_transient_for(Some(window));
        let entry = self.imp().id_entry.get();

        entry.set_placeholder_text(Some("User ID"));
        self.set_body("Enter the User ID you want to chat with");

        self.connect_response(
            None,
            clone!(@weak window, @weak entry => move |dialog, response| {
                if response != "accept" {
                    return;
                }
                // TODO parse number properly
                let entry_data = entry.text();
                info!("Entry data: {}", entry_data);
                let conn = window.get_chatting_from().user_ws();
                conn.get_user_data(entry_data.parse().unwrap());
                dialog.destroy();
            }),
        );

        self
    }

    pub fn edit_name(self, window: &UserProfile) -> Self {
        self.set_transient_for(Some(window));
        let entry = self.imp().id_entry.get();

        entry.set_placeholder_text(Some("Name"));
        self.set_body("Enter your new name");

        self.connect_response(
            None,
            clone!(@weak window, @weak entry => move |dialog, response| {
                if response != "accept" {
                    return;
                }
                let entry_data = entry.text();
                info!("Entry data: {}", entry_data);
                info!("Have to update name");
                dialog.destroy();
            }),
        );

        self
    }

    pub fn edit_image_link(self, window: &UserProfile) -> Self {
        self.set_transient_for(Some(window));
        let entry = self.imp().id_entry.get();

        entry.set_placeholder_text(Some("Direct Image Link"));
        self.set_body("Enter your new image link");

        self.connect_response(
            None,
            clone!(@weak window, @weak entry => move |dialog, response| {
                if response != "accept" {
                    return;
                }
                let entry_data = entry.text();
                info!("Entry data: {}", entry_data);
                info!("Have to update image link");
                dialog.destroy();
            }),
        );

        self
    }
}
