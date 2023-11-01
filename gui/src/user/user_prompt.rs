mod imp {
    use adw::subclass::prelude::*;
    use adw::Window;
    use glib::subclass::InitializingObject;
    use glib::{object_subclass, Propagation, SignalHandlerId};
    use gtk::{glib, Button, CompositeTemplate, Entry, Label, Spinner};
    use std::cell::RefCell;

    use crate::user::UserObject;

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
        pub signal_ids: RefCell<Vec<SignalHandlerId>>,
        pub user_data: RefCell<Option<UserObject>>,
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

    impl WindowImpl for UserPrompt {
        fn close_request(&self) -> Propagation {
            self.obj().stop_signals();
            Propagation::Proceed
        }
    }

    impl AdwWindowImpl for UserPrompt {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::Toast;
use glib::{clone, closure_local, wrapper, Object};
use gtk::{
    glib, Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager, Widget, Window,
};
use tracing::{debug, error, info};

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
                prompt.close()
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

    pub fn stop_signals(&self) {
        let user_data = self.imp().user_data.take();
        if let Some(user_object) = user_data {
            for signal in self.imp().signal_ids.take() {
                user_object.disconnect(signal);
                debug!("A signal in UserPrompt was disconnected");
            }
        }
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
                prompt.imp().error_text.set_label("");
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
                prompt.imp().error_text.set_label("");
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
                prompt.imp().error_text.set_label("");
            }));
    }

    /// Open prompt to handle number input for adding users
    pub fn add_user(self, window: &window::Window) -> Self {
        self.bind_number();
        self.set_transient_for(Some(window));
        self.set_modal(true);

        let user_data = window.get_chatting_from();
        self.imp().user_data.replace(Some(user_data.clone()));

        let obj_clone = self.clone();
        let user_exist_signal = user_data.connect_closure(
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
                        .set_label("Error: User ID does not exist");
                } else {
                    info!("Inputted User ID info found");
                    obj_clone.close()
                }
            }),
        );
        self.imp().signal_ids.borrow_mut().push(user_exist_signal);

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
                prompt.close()
            }),
        );

        self
    }

    /// Open prompt to take a new image link for the user
    pub fn edit_image_link(self, profile: &UserProfile, user_data: &UserObject) -> Self {
        let is_owner = if user_data.user_id() == user_data.owner_id() {
            self.imp().user_data.replace(Some(user_data.clone()));
            true
        } else {
            false
        };

        if is_owner {
            let obj_clone = self.clone();
            let image_modified_signal = user_data.connect_closure(
                "image-modified",
                false,
                closure_local!(move |_from: UserObject,
                                     error_message: String,
                                     _image_link: String| {
                    if !error_message.is_empty() {
                        error!("Failed to update image");
                        obj_clone.imp().loading_spinner.set_spinning(false);
                        obj_clone.set_buttons_sensitive();
                        obj_clone
                            .imp()
                            .error_text
                            .set_label(&format!("Error: {}", error_message));
                    } else {
                        info!("Image updated successfully");
                        obj_clone.close()
                    }
                }),
            );
            self.imp()
                .signal_ids
                .borrow_mut()
                .push(image_modified_signal);
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
                    .title("Starting updating image...".to_string())
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
