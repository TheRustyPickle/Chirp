mod imp {
    use adw::{subclass::prelude::*, ActionRow, Avatar, ToastOverlay, Window};
    use glib::subclass::InitializingObject;
    use glib::{object_subclass, Binding};
    use gtk::{glib, Button, CompositeTemplate, Image, Label, Switch};
    use std::cell::{OnceCell, RefCell};

    use crate::user::UserObject;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/user_profile.xml")]
    pub struct UserProfile {
        #[template_child]
        pub toast_overlay: TemplateChild<ToastOverlay>,
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
        pub image_link_delete: TemplateChild<Button>,
        #[template_child]
        pub conn_row: TemplateChild<ActionRow>,
        #[template_child]
        pub conn_switch: TemplateChild<Switch>,
        #[template_child]
        pub conn_timer: TemplateChild<Label>,
        #[template_child]
        pub conn_reload: TemplateChild<Button>,
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

use std::env;

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::Toast;
use glib::closure_local;
use glib::{clone, timeout_add_seconds_local_once, wrapper, Object};
use gtk::{
    glib, Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager, Widget, Window,
};
use soup::WebsocketConnection;
use tracing::info;

use crate::user::{UserObject, UserPrompt};
use crate::window;
use crate::ws::RequestType;

wrapper! {
    pub struct UserProfile(ObjectSubclass<imp::UserProfile>)
    @extends Widget, Window,
    @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl UserProfile {
    pub fn new(user_data: UserObject, window: &window::Window, is_owner: bool) -> Self {
        let obj: UserProfile = Object::builder().build();

        if is_owner {
            let obj_clone = obj.clone();
            user_data.connect_closure(
                "image-modified",
                false,
                closure_local!(move |_from: UserObject, message: String| {
                    if !message.is_empty() {
                        let toast_overlay = obj_clone.imp().toast_overlay.get();
                        let toast = Toast::builder()
                            .title(format!("Failed to update image: {}", message))
                            .timeout(2)
                            .build();
                        toast_overlay.add_toast(toast);
                    }
                }),
            );
        }

        obj.imp().user_data.set(user_data).unwrap();
        obj.set_transient_for(Some(window));
        obj.set_modal(true);
        obj.set_visible(true);
        obj.imp()
            .conn_row
            .set_subtitle(&env::var("WEBSOCKET_URL").unwrap());
        obj.bind();

        if !is_owner {
            obj.hide_editing_buttons();
        }

        obj.connect_button_signals(window);
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
        let image_link_delete_button = self.imp().image_link_delete.get();
        let image_link_copy_button = self.imp().image_link_copy.get();
        let conn_switch = self.imp().conn_switch.get();
        let conn_reload = self.imp().conn_reload.get();
        let conn_timer = self.imp().conn_timer.get();

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

        let image_delete_biding = user_data
            .bind_property("image-link", &image_link_delete_button, "sensitive")
            .transform_to(|_, link: Option<String>| {
                if link.is_some() {
                    Some(true.to_value())
                } else {
                    Some(false.to_value())
                }
            })
            .sync_create()
            .build();

        let image_copy_biding = user_data
            .bind_property("image-link", &image_link_copy_button, "sensitive")
            .transform_to(|_, link: Option<String>| {
                if link.is_some() {
                    Some(true.to_value())
                } else {
                    Some(false.to_value())
                }
            })
            .sync_create()
            .build();

        let conn_status_binding = user_data
            .user_ws()
            .bind_property("ws-conn", &conn_switch, "active")
            .transform_to(|_, link: Option<WebsocketConnection>| {
                if link.is_some() {
                    Some(true.to_value())
                } else {
                    Some(false.to_value())
                }
            })
            .sync_create()
            .build();

        let conn_reload_binding = user_data
            .user_ws()
            .bind_property("ws-conn", &conn_reload, "visible")
            .transform_to(|_, link: Option<WebsocketConnection>| {
                if link.is_some() {
                    Some(false.to_value())
                } else {
                    Some(true.to_value())
                }
            })
            .sync_create()
            .build();

        let conn_timer_label_binding = user_data
            .user_ws()
            .bind_property("reconnecting-timer", &conn_timer, "label")
            .sync_create()
            .build();

        let conn_timer_visible_binding = user_data
            .user_ws()
            .bind_property("ws-conn", &conn_timer, "visible")
            .transform_to(|_, link: Option<WebsocketConnection>| {
                if link.is_some() {
                    Some(false.to_value())
                } else {
                    Some(true.to_value())
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
        bindings.push(image_delete_biding);
        bindings.push(image_copy_biding);
        bindings.push(conn_status_binding);
        bindings.push(conn_reload_binding);
        bindings.push(conn_timer_label_binding);
        bindings.push(conn_timer_visible_binding);
    }

    fn hide_editing_buttons(&self) {
        self.imp().name_edit.set_visible(false);
        self.imp().image_link_edit.set_visible(false);
        self.imp().image_link_reload.set_visible(false);
        self.imp().image_link_delete.set_visible(false);
        self.imp().conn_row.set_visible(false);

        let user_data = self.imp().user_data.get().unwrap();
        user_data
            .bind_property("name", self, "title")
            .transform_to(|_, name: String| Some(format!("Profile - {}", name)))
            .sync_create()
            .build();
    }

    fn connect_button_signals(&self, window: &window::Window) {
        let name_edit = self.imp().name_edit.get();
        let image_link_edit = self.imp().image_link_edit.get();
        let id_copy = self.imp().id_copy.get();
        let image_link_copy = self.imp().image_link_copy.get();
        let image_link_reload = self.imp().image_link_reload.get();
        let image_link_delete = self.imp().image_link_delete.get();
        let conn_reload = self.imp().conn_reload.get();

        name_edit.connect_clicked(clone!(@weak self as profile => move |_| {
            info!("Opening prompt to get new name");
            let user_data = profile.imp().user_data.get().unwrap();
            let prompt = UserPrompt::new("Confirm").edit_name(&profile, user_data);
            prompt.present();
        }));

        image_link_edit.connect_clicked(clone!(@weak self as profile => move |_| {
            info!("Opening prompt to get new image link");
            let user_data = profile.imp().user_data.get().unwrap();
            let prompt = UserPrompt::new("Confirm").edit_image_link(&profile, user_data);
            prompt.present();
        }));

        id_copy.connect_clicked(clone!(@weak self as profile => move |_| {
            let text = profile.imp().id_row.get().subtitle().unwrap();
            info!("Copying User ID {text} to clipboard.");

            profile.clipboard().set(&text);

            let toast_overlay = profile.imp().toast_overlay.get();
            let toast = Toast::builder()
                .title("User ID has been copied to clipboard".to_string())
                .timeout(1)
                .build();
            toast_overlay.add_toast(toast);
        }));

        image_link_copy.connect_clicked(clone!(@weak self as profile => move |_| {
            let text = profile.imp().image_link_row.get().subtitle().unwrap();
            info!("Copying Image Link {text} to clipboard.");

            profile.clipboard().set(&text);

            let toast_overlay = profile.imp().toast_overlay.get();
            let toast = Toast::builder()
                .title("Image Link has been copied to clipboard")
                .timeout(1)
                .build();
            toast_overlay.add_toast(toast);
        }));

        image_link_reload.connect_clicked(clone!(@weak self as profile => move |_| {
            info!("Updating Image Link with a new random link");

            let user_data = profile.imp().user_data.get().unwrap();
            user_data.set_random_image();

            let toast_overlay = profile.imp().toast_overlay.get();
            let toast = Toast::builder()
                .title("Generating a new random image...")
                .timeout(1)
                .build();
            toast_overlay.add_toast(toast);
        }));

        image_link_delete.connect_clicked(clone!(@weak self as profile => move |_| {
            info!("Removing user image");

            let user_data = profile.imp().user_data.get().unwrap();
            user_data.remove_image();
            user_data.add_to_queue(RequestType::ImageUpdated(None));
        }));

        conn_reload.connect_clicked(clone!(@weak self as profile, @weak window => move |_| {
            info!("Reloading websocket connection");
            profile.imp().conn_reload.set_sensitive(false);
            window.reload_user_ws();
            let toast_overlay = profile.imp().toast_overlay.get();
            let toast = Toast::builder()
                .title("Reloading websocket connection...")
                .timeout(1)
                .build();
            toast_overlay.add_toast(toast);
            timeout_add_seconds_local_once(5, move || {
                profile.imp().conn_reload.set_sensitive(true);
            });
        }));
    }
}
