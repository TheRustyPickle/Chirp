mod imp {
    use adw::{subclass::prelude::*, ActionRow, Avatar, Window};
    use gio::glib::Binding;
    use glib::object_subclass;
    use glib::subclass::InitializingObject;
    use gtk::{glib, Button, CompositeTemplate, Image};
    use std::cell::RefCell;

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

use crate::user::{UserObject, UserPrompt};
use crate::window;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::clone;
use glib::{wrapper, Object};
use gtk::{
    glib, Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager, Widget, Window,
};
use tracing::info;

wrapper! {
    pub struct UserProfile(ObjectSubclass<imp::UserProfile>)
    @extends Widget, Window,
    @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl UserProfile {
    pub fn new(user_data: UserObject, window: &window::Window) -> Self {
        let obj: UserProfile = Object::builder().build();
        obj.set_transient_for(Some(window));
        obj.set_modal(true);
        obj.set_visible(true);
        obj.bind(&user_data);
        obj.connect_button_signals();
        obj
    }

    fn bind(&self, user_data: &UserObject) {
        let mut bindings = self.imp().bindings.borrow_mut();
        let profile_avatar = self.imp().profile_avatar.get();
        let name_row = self.imp().name_row.get();
        let id_row = self.imp().id_row.get();
        let image_link_row = self.imp().image_link_row.get();
        let id_warning = self.imp().id_warning.get();

        let avatar_text_binding = user_data
            .bind_property("name", &profile_avatar, "text")
            .sync_create()
            .build();

        let avatar_image_binding = user_data
            .bind_property("image", &profile_avatar, "custom-image")
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
            let prompt = UserPrompt::new("Confirm").edit_name(&window);
            prompt.present();
        }));

        image_link_edit.connect_clicked(clone!(@weak self as window => move |_| {
            info!("Opening prompt to get new image link");
            let prompt = UserPrompt::new("Confirm").edit_image_link(&window);
            prompt.present();
        }));

        id_copy.connect_clicked(clone!(@weak self as window => move |_| {
            let text = window.imp().id_row.get().subtitle().unwrap();
            info!("Copying User ID {text} to clipboard.");
            window.clipboard().set(&text);
        }));

        image_link_copy.connect_clicked(clone!(@weak self as window => move |_| {
            let text = window.imp().image_link_row.get().subtitle().unwrap();
            info!("Copying Image Link {text} to clipboard.");
            window.clipboard().set(&text);
        }));

        image_link_reload.connect_clicked(clone!(@weak self as window => move |_| {
            info!("Updating Image Link with a new random link");
        }));
    }
}
