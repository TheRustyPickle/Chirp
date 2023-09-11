mod imp {
    use adw::{subclass::prelude::*, ActionRow, Avatar, Window};
    use glib::subclass::InitializingObject;
    use glib::{object_subclass, Binding};
    use gtk::{glib, Button, CompositeTemplate, Image, Label, Revealer};
    use std::cell::{OnceCell, RefCell};

    use crate::user::UserObject;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/user_profile.xml")]
    pub struct UserProfile {
        #[template_child]
        pub profile_avatar: TemplateChild<Avatar>,
        #[template_child]
        pub name_row: TemplateChild<ActionRow>,
        #[template_child]
        pub name_edit: TemplateChild<Button>,
        #[template_child]
        pub id_row: TemplateChild<ActionRow>,
        #[template_child]
        pub id_warning: TemplateChild<Image>,
        #[template_child]
        pub id_copy: TemplateChild<Button>,
        #[template_child]
        pub image_link_row: TemplateChild<ActionRow>,
        #[template_child]
        pub image_link_copy: TemplateChild<Button>,
        #[template_child]
        pub image_link_reload: TemplateChild<Button>,
        #[template_child]
        pub image_link_edit: TemplateChild<Button>,
        #[template_child]
        pub copy_revealer: TemplateChild<Revealer>,
        #[template_child]
        pub copy_info: TemplateChild<Label>,
        pub user_data: OnceCell<UserObject>,
        pub bindings: RefCell<Vec<Binding>>,
    }

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

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::{clone, timeout_add_seconds_local_once, wrapper, Object};
use gtk::{
    glib, Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager, Widget, Window,
};
use tracing::info;

use crate::user::{UserObject, UserPrompt};
use crate::window;

wrapper! {
    pub struct UserProfile(ObjectSubclass<imp::UserProfile>)
    @extends Widget, Window,
    @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl UserProfile {
    pub fn new(user_data: UserObject, window: &window::Window) -> Self {
        let obj: UserProfile = Object::builder().build();
        obj.imp().user_data.set(user_data).unwrap();
        obj.set_transient_for(Some(window));
        obj.set_modal(true);
        obj.set_visible(true);
        obj.bind();
        obj.connect_button_signals();
        obj
    }

    fn bind(&self) {
        let mut bindings = self.imp().bindings.borrow_mut();
        let profile_avatar = self.imp().profile_avatar.get();
        let name_row = self.imp().name_row.get();
        let id_row = self.imp().id_row.get();
        let image_link_row = self.imp().image_link_row.get();
        let id_warning = self.imp().id_warning.get();
        let user_data = self.imp().user_data.get().unwrap();

        let avatar_text_binding = user_data
            .bind_property("name", &profile_avatar, "text")
            .sync_create()
            .build();

        let avatar_image_binding = user_data
            .bind_property("big-image", &profile_avatar, "custom-image")
            .sync_create()
            .build();

        let name_subtitle_binding = user_data
            .bind_property("name", &name_row, "subtitle")
            .sync_create()
            .build();

        let id_subtitle_binding = user_data
            .bind_property("user-id", &id_row, "subtitle")
            .sync_create()
            .build();

        let image_link_subtitle_binding = user_data
            .bind_property("image-link", &image_link_row, "subtitle")
            .sync_create()
            .build();

        let id_warning_binding = user_data
            .bind_property("user-id", &id_warning, "visible")
            .transform_to(|_, number: u64| {
                if number == 0 {
                    Some(true.to_value())
                } else {
                    Some(false.to_value())
                }
            })
            .sync_create()
            .build();

        bindings.push(avatar_text_binding);
        bindings.push(avatar_image_binding);
        bindings.push(name_subtitle_binding);
        bindings.push(id_subtitle_binding);
        bindings.push(id_warning_binding);
        bindings.push(image_link_subtitle_binding);
    }

    fn connect_button_signals(&self) {
        let name_edit = self.imp().name_edit.get();
        let image_link_edit = self.imp().image_link_edit.get();
        let id_copy = self.imp().id_copy.get();
        let image_link_copy = self.imp().image_link_copy.get();
        let image_link_reload = self.imp().image_link_reload.get();

        name_edit.connect_clicked(clone!(@weak self as window => move |_| {
            info!("Opening prompt to get new name");
            let user_data = window.imp().user_data.get().unwrap();
            let prompt = UserPrompt::new("Confirm").edit_name(&window, user_data);
            prompt.present();
        }));

        image_link_edit.connect_clicked(clone!(@weak self as window => move |_| {
            info!("Opening prompt to get new image link");
            let user_data = window.imp().user_data.get().unwrap();
            let prompt = UserPrompt::new("Confirm").edit_image_link(&window, user_data);
            prompt.present();
        }));

        id_copy.connect_clicked(clone!(@weak self as window => move |_| {
            let text = window.imp().id_row.get().subtitle().unwrap();
            info!("Copying User ID {text} to clipboard.");

            window.clipboard().set(&text);
            window.start_revealer(&format!("User ID {text} has been copied"));
        }));

        image_link_copy.connect_clicked(clone!(@weak self as window => move |_| {
            let text = window.imp().image_link_row.get().subtitle().unwrap();
            info!("Copying Image Link {text} to clipboard.");

            window.clipboard().set(&text);
            window.start_revealer("Image Link has been copied");
        }));

        image_link_reload.connect_clicked(clone!(@weak self as window => move |_| {
            info!("Updating Image Link with a new random link");
            window.start_revealer("Starting updating image...");

            let user_data = window.imp().user_data.get().unwrap();
            user_data.set_random_image();
        }));
    }

    pub fn start_revealer(&self, label_text: &str) {
        self.imp().copy_info.get().set_label(label_text);

        let revealer = self.imp().copy_revealer.get();
        revealer.set_visible(true);
        revealer.set_reveal_child(true);

        timeout_add_seconds_local_once(1, move || {
            revealer.set_reveal_child(false);
        });
    }
}
