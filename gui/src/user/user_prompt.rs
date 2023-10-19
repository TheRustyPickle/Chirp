mod imp {
    use adw::subclass::prelude::*;
    use adw::Window;
    use glib::object_subclass;
    use glib::subclass::InitializingObject;
    use gtk::{glib, Button, CompositeTemplate, Entry, Label, Spinner};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/user_prompt.xml")]
    pub struct UserPrompt {
        #[template_child]
        pub prompt_text: TemplateChild<Label>,
        #[template_child]
        pub user_entry: TemplateChild<Entry>,
        #[template_child]
        pub confirm_button: TemplateChild<Button>,
        #[template_child]
        pub cancel_button: TemplateChild<Button>,
        #[template_child]
        pub loading_spinner: TemplateChild<Spinner>,
        #[template_child]
        pub error_text: TemplateChild<Label>,
    }

    #[object_subclass]
    impl ObjectSubclass for UserPrompt {
        const NAME: &'static str = "UserPrompt";
        type Type = super::UserPrompt;
        type ParentType = Window;

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

    impl AdwWindowImpl for UserPrompt {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::Toast;
use gio::glib::closure_local;
use glib::{clone, wrapper, Object};
use gtk::{
    glib, Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager, Widget, Window,
};
use tracing::{error, info};

use crate::user::{UserObject, UserProfile};
use crate::window;
use crate::ws::RequestType;

wrapper! {
    pub struct UserPrompt(ObjectSubclass<imp::UserPrompt>)
    @extends Widget, Window,
    @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl UserPrompt {
    pub fn new(confirm_name: &str) -> Self {
        let obj: UserPrompt = Object::builder().build();
        obj.imp().confirm_button.set_label(confirm_name);
        obj.imp()
            .cancel_button
            .connect_clicked(clone!(@weak obj as prompt => move |_| {
                prompt.destroy()
            }));
        obj.imp()
            .user_entry
            .connect_activate(clone!(@weak obj as prompt => move |_| {
                let confirm_button = prompt.imp().confirm_button.get();

                if confirm_button.is_sensitive() {
                    confirm_button.emit_clicked()
                }
            }));
        // Add the blue-entry initially so when this gets opened, the entry remains blue colored
        obj.imp().user_entry.add_css_class("blue-entry");

        obj
    }

    /// Bind the GtkEntry to work with an image link
    fn bind_image_link(&self) {
        self.imp()
            .user_entry
            .connect_changed(clone!(@weak self as prompt => move |entry| {
                let entry_text = entry.text();

                if entry_text.is_empty() {
                    entry.remove_css_class("blue-entry");
                    entry.add_css_class("error");
                    prompt.imp().confirm_button.set_sensitive(false);
                } else {
                    entry.remove_css_class("error");
                    entry.add_css_class("blue-entry");
                    prompt.imp().confirm_button.set_sensitive(true);
                }
                prompt.imp().error_text.set_label(&String::new());
            }));
    }

    /// Bind the GtkEntry to ensure input is u64 parsable
    fn bind_number(&self) {
        self.imp()
            .user_entry
            .connect_changed(clone!(@weak self as prompt => move |entry| {
                let entry_text = entry.text();

                let to_enable = !entry_text.is_empty() && entry_text.parse::<u64>().is_ok();
                prompt.imp().confirm_button.set_sensitive(to_enable);

                if !to_enable {
                    entry.remove_css_class("blue-entry");
                    entry.add_css_class("error");
                } else {
                    entry.remove_css_class("error");
                    entry.add_css_class("blue-entry");
                }
                prompt.imp().error_text.set_label(&String::new());
            }));
    }

    /// Bind the GtkEntry to ensure input length is below 250 chars
    fn bind_name(&self) {
        self.imp()
            .user_entry
            .connect_changed(clone!(@weak self as prompt => move |entry| {
                let entry_text = entry.text();

                if entry_text.is_empty() {
                    entry.remove_css_class("blue-entry");
                    entry.add_css_class("error");
                    prompt.imp().confirm_button.set_sensitive(false);
                } else if entry_text.len() > 250 {
                    entry.remove_css_class("blue-entry");
                    entry.add_css_class("error");
                    prompt.imp().confirm_button.set_sensitive(false);
                    prompt.imp().error_text.set_label(&String::from("Error: Name length must be below 250 letters"));
                    return;
                } else {
                    entry.remove_css_class("error");
                    entry.add_css_class("blue-entry");
                    prompt.imp().confirm_button.set_sensitive(true);
                }
                prompt.imp().error_text.set_label(&String::new());
            }));
    }

    /// Open prompt to handle number input for adding users
    pub fn add_user(self, window: &window::Window) -> Self {
        self.bind_number();
        self.set_transient_for(Some(window));
        self.set_modal(true);

        let user_data = window.get_chatting_from();
        let obj_clone = self.clone();
        user_data.connect_closure(
            "user-exists",
            false,
            closure_local!(move |_from: UserObject, exists: bool| {
                if !exists {
                    error!("Inputted User ID does not exists");
                    obj_clone.imp().loading_spinner.set_spinning(false);
                    obj_clone.set_buttons_sensitive();
                    obj_clone
                        .imp()
                        .error_text
                        .set_label(&format!("Error: User ID does not exist"));
                } else {
                    info!("Inputted User ID info found");
                    obj_clone.destroy()
                }
            }),
        );

        self.imp()
            .user_entry
            .get()
            .set_placeholder_text(Some("User ID"));
        self.imp()
            .prompt_text
            .set_label("Enter the User ID you want to chat with");

        self.imp().confirm_button.connect_clicked(clone!(@weak self as prompt, @weak window => move |_| {
            let entry_data = prompt.imp().user_entry.text();
            info!("Processing {} to add a new user", entry_data);
            window.get_chatting_from().add_to_queue(RequestType::GetUserData(entry_data.parse().unwrap()));
            prompt.imp().loading_spinner.set_spinning(true);
            prompt.set_buttons_insensitive();
        }));

        self
    }

    /// Open prompt to take a new name for the user
    pub fn edit_name(self, profile: &UserProfile, user_data: &UserObject) -> Self {
        self.bind_name();
        self.set_transient_for(Some(profile));
        self.set_modal(true);

        self.imp()
            .user_entry
            .get()
            .set_placeholder_text(Some("Name"));
        self.imp().prompt_text.set_label("Enter your new name");

        self.imp().confirm_button.connect_clicked(
            clone!(@weak self as prompt, @weak profile, @weak user_data => move |_| {
                let entry_data = prompt.imp().user_entry.text();
                info!("Updating name to: {}", entry_data);
                let over_lay = profile.imp().toast_overlay.get();
                let toast = Toast::builder()
                    .title(format!("Updating name to: {}", entry_data))
                    .timeout(1)
                    .build();
                over_lay.add_toast(toast);
                user_data.add_to_queue(RequestType::NameUpdated(entry_data.to_string()));
                prompt.destroy()
            }),
        );

        self
    }

    /// Open prompt to take a new image link for the user
    pub fn edit_image_link(self, profile: &UserProfile, user_data: &UserObject) -> Self {
        let is_owner = if user_data.user_id() == user_data.owner_id() {
            true
        } else {
            false
        };

        if is_owner {
            let obj_clone = self.clone();
            user_data.connect_closure(
                "image-modified",
                false,
                closure_local!(move |_from: UserObject, message: String| {
                    if !message.is_empty() {
                        error!("Failed to update image");
                        obj_clone.imp().loading_spinner.set_spinning(false);
                        obj_clone.set_buttons_sensitive();
                        obj_clone
                            .imp()
                            .error_text
                            .set_label(&format!("Error: {}", message));
                    } else {
                        info!("Image updated successfully");
                        obj_clone.destroy()
                    }
                }),
            );
        }

        self.bind_image_link();
        self.set_transient_for(Some(profile));
        self.set_modal(true);

        self.imp()
            .user_entry
            .get()
            .set_placeholder_text(Some("Image Link"));
        self.imp()
            .prompt_text
            .set_label("Enter the link to your new profile image");

        self.imp().confirm_button.connect_clicked(
            clone!(@weak self as prompt, @weak profile, @weak user_data => move |_| {
                let entry_data = prompt.imp().user_entry.text();
                info!("Updating image link to: {}", entry_data);
                let over_lay = profile.imp().toast_overlay.get();
                let toast = Toast::builder()
                    .title(format!("Starting updating image..."))
                    .timeout(1)
                    .build();
                over_lay.add_toast(toast);
                user_data.add_to_queue(RequestType::ImageUpdated(Some(entry_data.to_string())));
                prompt.imp().loading_spinner.set_spinning(true);
                prompt.set_buttons_insensitive();
            }),
        );

        self
    }

    /// Disable prompt buttons
    fn set_buttons_insensitive(&self) {
        self.imp().confirm_button.set_sensitive(false);
        self.imp().cancel_button.set_sensitive(false);
    }

    /// Enable prompt buttons
    fn set_buttons_sensitive(&self) {
        self.imp().confirm_button.set_sensitive(true);
        self.imp().cancel_button.set_sensitive(true);
    }
}
