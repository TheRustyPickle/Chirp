mod imp {
    use adw::subclass::prelude::*;
    use adw::ApplicationWindow;
    use gio::{ListStore, Settings};
    use glib::subclass::InitializingObject;
    use glib::{object_subclass, Binding, Propagation};
    use gtk::{
        gio, glib, Button, CompositeTemplate, EmojiChooser, Label, ListBox, ListView, Revealer,
        Stack, TextView,
    };
    use std::cell::{Cell, OnceCell, RefCell};
    use std::collections::{HashMap, HashSet};
    use std::rc::Rc;

    use crate::user::UserObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/therustypickle/chirp/window.xml")]
    pub struct Window {
        #[template_child]
        pub message_entry: TemplateChild<TextView>,
        #[template_child]
        pub message_list: TemplateChild<ListView>,
        #[template_child]
        pub send_button: TemplateChild<Button>,
        #[template_child]
        pub user_list: TemplateChild<ListBox>,
        #[template_child]
        pub stack: TemplateChild<Stack>,
        #[template_child]
        pub my_profile: TemplateChild<Button>,
        #[template_child]
        pub new_chat: TemplateChild<Button>,
        #[template_child]
        pub placeholder: TemplateChild<Label>,
        #[template_child]
        pub entry_revealer: TemplateChild<Revealer>,
        #[template_child]
        pub emoji_button: TemplateChild<Button>,
        #[template_child]
        pub emoji_chooser: TemplateChild<EmojiChooser>,
        pub users: OnceCell<ListStore>,
        pub chatting_with: Rc<RefCell<Option<UserObject>>>,
        pub own_profile: Rc<RefCell<Option<UserObject>>>,
        pub last_selected_user: Cell<i32>,
        pub bindings: RefCell<Vec<Binding>>,
        pub settings: OnceCell<Settings>,
        pub message_numbers: RefCell<HashMap<u64, HashSet<u64>>>,
    }

    #[object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "MainWindow";
        type Type = super::Window;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_settings();
            obj.setup_callbacks();
            obj.setup_users();
            obj.setup_actions();
        }
    }

    impl WindowImpl for Window {
        /// On window close save all existing user data to gschema
        fn close_request(&self) -> Propagation {
            self.obj().save_user_list();
            Propagation::Proceed
        }
    }

    impl WidgetImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::Application;
use chrono::{Local, NaiveDateTime, TimeZone};
use gio::{ActionGroup, ActionMap, ListStore, Settings, SimpleAction};
use glib::{clone, timeout_add_local_once, wrapper, Object};
use gtk::{
    gio, glib, Accessible, ApplicationWindow, Buildable, ConstraintTarget, ListBox, ListBoxRow,
    ListScrollFlags, Native, PositionType, Root, ShortcutManager, Widget,
};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use tracing::{debug, error, info};

use crate::encryption::{read_rsa_keys_from_file, read_rsa_public_from_string, stringify_rsa_keys};
use crate::message::MessageObject;
use crate::user::{UserObject, UserProfile, UserPrompt, UserRow};
use crate::utils::{generate_random_avatar_link, get_created_at_timing};
use crate::ws::{DecryptedMessageData, FullUserData, MessageData, RequestType, UserIDs};
use crate::APP_ID;

wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, ApplicationWindow, gtk::Window, Widget,
        @implements ActionGroup, ActionMap, Accessible, Buildable,
                    ConstraintTarget, Native,Root, ShortcutManager;
}

impl Window {
    pub fn new(app: &Application) -> Self {
        Object::builder().property("application", app).build()
    }

    fn setup_callbacks(&self) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("main");

        // If a new user in the user row is selected, this is activated
        imp.user_list
            .connect_row_activated(clone!(@weak self as window => move |listbox, row| {
                let last_index = window.imp().last_selected_user.get();
                let index = row.index();

                if last_index != index || last_index == 0 {
                    window.remove_selected_avatar_css(last_index, listbox);
                    window.add_selected_avatar_css(index, listbox);
                }

                let selected_chat = window.get_users_liststore()
                .item(index as u32)
                .unwrap()
                .downcast::<UserObject>()
                .unwrap();

                info!("Selected a new User from list");

                window.imp().last_selected_user.set(index);
                window.set_chatting_with(selected_chat);
                window.remove_last_binding();
                window.grab_focus();
                window.bind();
                window.scroll_to_bottom(window.get_chatting_with(), false);
            }));

        // The event on New Chat button clicked
        self.imp()
            .new_chat
            .connect_clicked(clone!(@weak self as window => move |_| {
                let prompt = UserPrompt::new("Start Chat").add_user(&window);
                prompt.present();
            }));

        // The event on Profile button clicked
        self.imp()
            .my_profile
            .connect_clicked(clone!(@weak self as window => move |_| {
                UserProfile::new(window.get_chatting_from(), &window);
            }));

        // The event on the send button beside the textview
        self.imp()
            .send_button
            .connect_clicked(clone!(@weak self as window => move |_| {
                window.send_message();
                window.grab_focus();
            }));

        // If the message typing space is empty, show the background text + disable the send button
        // else remove the text and enable the send button
        self.imp().message_entry.get().buffer().connect_changed(
            clone!(@weak self as window => move |buffer| {
                let char_count = buffer.char_count();
                let should_be_enabled = char_count != 0;

                if window.get_chatting_with().user_id() == 0 {
                    window.imp().send_button.set_sensitive(false);
                } else {
                    window.imp().send_button.set_sensitive(should_be_enabled);
                }

                if should_be_enabled {
                    window.imp().placeholder.set_visible(false);
                } else {
                    window.imp().placeholder.set_visible(true);
                }

            }),
        );

        // Timeout half a second before revealing the textview
        let window = self.clone();
        timeout_add_local_once(Duration::from_millis(500), move || {
            window.imp().entry_revealer.set_reveal_child(true);
        });

        // Set emoji chooser to visible on click
        self.imp()
            .emoji_button
            .connect_clicked(clone!(@weak self as window => move |_| {
                window.imp().emoji_chooser.set_position(PositionType::Top);
                window.imp().emoji_chooser.set_has_arrow(false);
                window.imp().emoji_chooser.set_visible(true);
            }));

        // Add the chosen emoji to the textview at the cursor point
        self.imp().emoji_chooser.connect_emoji_picked(
            clone!(@weak self as window => move |_, emoji| {
                let buffer = window.imp().message_entry.buffer();
                buffer.insert_at_cursor(emoji);
            }),
        );
    }

    fn setup_actions(&self) {
        // The action that is emitted on CTRL + Enter key. This is used for sending messages
        let send_message_action = SimpleAction::new("send-message", None);
        send_message_action.connect_activate(clone!(@weak self as window => move |_, _| {
            window.send_message();
            window.grab_focus();
        }));

        self.add_action(&send_message_action);
    }

    fn setup_settings(&self) {
        let settings = Settings::new(APP_ID);
        self.imp().settings.set(settings).unwrap();
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().unwrap()
    }

    // TODO use for when the app can take a location input
    // NOTE when saving it must end with a /
    fn _update_setting(&self, new_location: &str) {
        self.settings()
            .set_string("location", new_location)
            .unwrap();
    }

    fn empty_saved_user_list(&self) {
        self.settings().set_string("users", "").unwrap();
    }

    /// Save owner ID and Token in the predefined location in a json file
    pub fn save_user_data(&self) {
        let saving_location = self.settings().string("location");
        let user_data_path = format!("{}user_data.json", saving_location);

        info!("Saving new user id info on {}", user_data_path);
        let owner_id = self.get_chatting_from();
        let id_data = UserIDs::new_json(owner_id.user_id(), owner_id.user_token());

        let mut file = File::create(user_data_path).unwrap();
        file.write_all(id_data.as_bytes()).unwrap();
    }

    /// Check if any existing owner data is available
    fn check_saved_data(&self) -> Option<UserIDs> {
        info!("Checking for saved user data");
        let saving_location = self.settings().string("location");
        let user_data_path = format!("{}user_data.json", saving_location);

        if fs::metadata(&user_data_path).is_ok() {
            let mut file = File::open(user_data_path).unwrap();
            let mut file_contents = String::new();

            file.read_to_string(&mut file_contents)
                .expect("Failed to read file");

            let id_data = UserIDs::from_json(&file_contents);
            info!("Saved user data found");
            return Some(id_data);
        }

        info!("Failed to find any previously saved user data");
        None
    }

    /// Check if any existing rsa keys are available
    fn check_saved_keys(&self) -> Option<(RsaPublicKey, RsaPrivateKey)> {
        info!("Checking for saved RSA keys");
        let saving_location = self.settings().string("location");
        let public_location = format!("{}public_key.pem", saving_location);
        let private_location = format!("{}private_key.pem", saving_location);

        if fs::metadata(public_location).is_ok() && fs::metadata(private_location).is_ok() {
            let existing_keys = read_rsa_keys_from_file(saving_location.to_string());
            if let Ok((public, private)) = existing_keys {
                info!("Saved RSA Keys found");
                return Some((public, private));
            }
        }

        None
    }

    /// Get the saved data from gschema
    fn get_saved_user_data(&self) -> Vec<FullUserData> {
        let all_user_data = self.settings().string("users");
        if !all_user_data.is_empty() {
            serde_json::from_str(&all_user_data).unwrap()
        } else {
            Vec::new()
        }
    }

    // Save added UserObject data to gschema for later retrieval
    pub fn save_user_list(&self) {
        info!("Starting saving user list");
        let mut save_list = Vec::new();

        let user_list = self.get_users_liststore();
        for user_data in user_list.iter() {
            let user_object: UserObject = user_data.unwrap();
            debug!(
                "Saving user object {} {} {:?}",
                user_object.user_id(),
                user_object.name(),
                user_object.image_link()
            );
            let user_data = FullUserData::new(&user_object).empty_token();
            save_list.push(user_data)
        }

        let to_save = serde_json::to_string(&save_list).unwrap();
        self.settings().set_string("users", &to_save).unwrap();
    }

    /// Save the RSA keys to the saved location
    pub fn save_rsa_keys(&self) {
        info!("Starting saving RSA keys");
        let saving_location = self.settings().string("location").to_string();
        let owner_data = self.get_chatting_from();

        let public_key = owner_data.imp().rsa_public.get().unwrap();
        let private_key = owner_data.imp().rsa_private.get().unwrap();

        let (public_string, private_string) = stringify_rsa_keys(public_key, private_key);

        if fs::metadata(&saving_location).is_ok() {
            let public_location = format!("{}public_key.pem", saving_location);
            let private_location = format!("{}private_key.pem", saving_location);

            let mut file = File::create(public_location).unwrap();
            file.write_all(public_string.as_bytes()).unwrap();

            let mut file = File::create(private_location).unwrap();
            file.write_all(private_string.as_bytes()).unwrap();
        }
    }

    /// Bind the main window header bar's title to the selected chat
    fn bind(&self) {
        let mut bindings = self.imp().bindings.borrow_mut();
        let chatting_with = self.get_chatting_with();
        let title_binding = chatting_with
            .bind_property("name", self, "title")
            .transform_to(|_, name: String| Some(format!("Chirp - {}", name)))
            .sync_create()
            .build();
        bindings.push(title_binding);
    }

    /// Disconnect the last header bar title binding
    fn remove_last_binding(&self) {
        if let Some(binding) = self.imp().bindings.borrow_mut().pop() {
            binding.unbind();
        }
    }

    /// Setup owner profile and bind User model
    fn setup_users(&self) {
        let users = ListStore::new::<UserObject>();
        self.imp().users.set(users).expect("Could not set users");
        self.imp().last_selected_user.set(0);

        let user_store = self.get_users_liststore();
        let user_list = self.get_user_list();

        // Bind the model so if any user is added to user_store, start creating the new user row
        user_list.bind_model(
            Some(user_store),
            clone!(@weak self as window => @default-panic, move |obj| {
                let user_object = obj.downcast_ref().unwrap();
                let row = window.get_user_row(user_object, "user-inactive");
                row.upcast()
            }),
        );

        info!("Setting own profile");
        let saved_user_id = self.check_saved_data();
        let saved_keys = self.check_saved_keys();
        let data: UserObject = self.create_owner(saved_user_id.clone(), saved_keys.clone());

        self.imp().own_profile.replace(Some(data.clone()));

        // Select the first row we just added
        self.get_user_list().row_at_index(0).unwrap().activate();

        if saved_user_id.is_some() && saved_keys.is_some() {
            let id_data = saved_user_id.unwrap();
            let mut saved_users = self.get_saved_user_data();
            // If empty stop checking
            if saved_users.is_empty() {
                return;
            }

            let owner_data = saved_users.remove(0);
            // If it's not the same then the saved data is outdated or invalid
            if owner_data.user_id != id_data.user_id {
                error!("Invalid or outdated owner data found. Dismissing saved data");
                self.empty_saved_user_list();
                return;
            }

            data.set_name(owner_data.user_name);

            // Have to set the image link manually otherwise if new users are added
            // after this and the image is not loaded it would save None image link
            data.set_image_link(owner_data.image_link.clone());
            data.check_image_link(owner_data.image_link, false);

            for user_data in saved_users {
                self.create_user(user_data);
            }
        } else {
            self.empty_saved_user_list();
        }
    }

    /// Get the UserObject that is currently selected/chatting with
    pub fn get_chatting_with(&self) -> UserObject {
        self.imp().chatting_with.borrow().clone().unwrap()
    }

    /// Set chatting with the given user
    fn set_chatting_with(&self, user: UserObject) {
        info!("Setting chatting with {}", user.name());
        let message_factory = user.imp().message_factory.get().unwrap();
        let selection_model = user.imp().selection_model.get().unwrap();
        self.imp().message_list.set_factory(Some(message_factory));
        self.imp().message_list.set_model(Some(selection_model));
        user.add_queue_to_first(RequestType::GetLastMessageNumber(user.clone()));
        self.imp().chatting_with.replace(Some(user));
    }

    /// Get the UserObject of the owner/chatting from
    pub fn get_chatting_from(&self) -> UserObject {
        self.imp().own_profile.borrow().clone().unwrap()
    }

    /// Get the owner user id
    pub fn get_owner_id(&self) -> u64 {
        self.get_chatting_from().user_id()
    }

    /// Get the ListStore of where all UserObject is saved
    fn get_users_liststore(&self) -> &ListStore {
        self.imp().users.get().expect("User liststore is not set")
    }

    /// Get the ListStore of a user where all MessageObject is saved
    fn chatting_with_messages(&self) -> ListStore {
        self.get_chatting_with().messages()
    }

    /// Send the text on the Textview as a message
    fn send_message(&self) {
        if self.get_chatting_with().user_id() == 0 {
            return;
        }
        let buffer = self.imp().message_entry.buffer();
        let content = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .trim()
            .to_string();

        if content.is_empty() {
            info!("Empty text found");
            return;
        }

        let sender = self.get_chatting_from();
        let receiver = self.get_chatting_with();

        let receiver_id = receiver.user_id();
        let current_time = Local::now();
        let created_at = current_time.to_string();

        let message_timing = get_created_at_timing(&current_time.naive_local());

        let message = MessageObject::new(
            content,
            true,
            sender,
            receiver.clone(),
            message_timing,
            None,
        )
        .to_process(true);
        self.chatting_with_messages().append(&message);
        message.set_show_initial_message(false);
        buffer.set_text("");

        let send_message_data =
            MessageData::new_incomplete(created_at, self.get_owner_id(), receiver_id);

        // Receiver gets the queue because the receiver saves the message number variable
        // if it was sender, it would send the message number of owner_id@owner_id group which is invalid
        receiver.add_to_queue(RequestType::SendMessage(send_message_data, message.clone()));
        self.scroll_to_bottom(receiver, true);
    }

    /// Gets called when a message is received or when syncing previous message data
    pub fn receive_message(
        &self,
        message_data: DecryptedMessageData,
        other_user: UserObject,
        add_css: bool,
    ) -> Option<MessageObject> {
        // No need to receive an already existing message
        if self
            .imp()
            .message_numbers
            .borrow()
            .get(&other_user.user_id())
            .unwrap()
            .contains(&message_data.message_number)
        {
            return None;
        }

        self.imp()
            .message_numbers
            .borrow_mut()
            .get_mut(&other_user.user_id())
            .unwrap()
            .insert(message_data.message_number);

        let current_message_number = other_user.message_number();
        if current_message_number < message_data.message_number {
            // Less than current number means it's an old message
            other_user.set_message_number(other_user.message_number() + 1);
        }

        let (sender, receiver, is_send) =
            if self.get_chatting_from().user_id() == message_data.from_user {
                (self.get_chatting_from(), other_user.clone(), true)
            } else {
                (other_user.clone(), self.get_chatting_from(), false)
            };

        // The server sends the time in UTC
        let parsed_naive =
            NaiveDateTime::parse_from_str(&message_data.created_at, "%Y-%m-%d %H:%M:%S%.3f")
                .unwrap()
                .and_utc()
                .naive_utc();

        let created_at = Local.from_utc_datetime(&parsed_naive).naive_local();

        let message_timing = get_created_at_timing(&created_at);

        let message = MessageObject::new(
            message_data.message.unwrap(),
            is_send,
            sender.clone(),
            receiver.clone(),
            message_timing,
            Some(message_data.message_number),
        );

        // First case will only happen when syncing messages
        if current_message_number >= message_data.message_number {
            other_user.messages().insert(0, &message);
        } else {
            other_user.messages().append(&message);
        }

        // Pending message color should not be added when syncing messages
        if add_css {
            let target_user = if is_send { receiver } else { sender };

            if target_user != self.get_chatting_with() {
                self.add_pending_avatar_css(target_user)
            }
        }
        Some(message)
    }

    /// Create a row using a UserObject for the ListBox
    fn get_user_row(&self, data: &UserObject, css_name: &str) -> ListBoxRow {
        let user_row = UserRow::new(data.clone());
        user_row.imp().user_avatar.add_css_class(css_name);
        ListBoxRow::builder()
            .child(&user_row)
            .activatable(true)
            .selectable(false)
            .can_focus(false)
            .build()
    }

    /// Used during the startup of the app. Called only once to create the owner profile
    fn create_owner(
        &self,
        id_data: Option<UserIDs>,
        key_data: Option<(RsaPublicKey, RsaPrivateKey)>,
    ) -> UserObject {
        let data_exists = id_data.is_some() && key_data.is_some();

        let user_data = if data_exists {
            info!("Saved user data found");

            let (public_key, private_key) = key_data.unwrap();
            let data = id_data.unwrap();
            self.imp()
                .message_numbers
                .borrow_mut()
                .insert(data.user_id, HashSet::new());
            UserObject::new(
                "Me",
                None,
                None,
                Some(data.user_id),
                Some(data.user_token),
                self.clone(),
                Some(public_key.clone()),
                Some((public_key, private_key)),
            )
        } else {
            UserObject::new(
                "Me",
                Some(generate_random_avatar_link()),
                None,
                None,
                None,
                self.clone(),
                None,
                None,
            )
        };

        user_data.handle_ws();
        self.get_users_liststore().append(&user_data);

        user_data
    }

    /// Used to create all UserObject for the self's users ListStore except for the owner UserObject.
    /// Called when New Chat button is used or a message is received but the user was not added
    pub fn create_user(&self, user_data: FullUserData) -> UserObject {
        info!(
            "Creating new user with name: {}, id: {}",
            user_data.user_name, user_data.user_id
        );

        let rsa_public_key = read_rsa_public_from_string(user_data.rsa_public_key);
        let new_user_data = UserObject::new(
            &user_data.user_name,
            user_data.image_link,
            Some(&self.get_owner_name_color()),
            Some(user_data.user_id),
            None,
            self.clone(),
            Some(rsa_public_key),
            None,
        );

        // Every single user in the UserList of the client will have the owner User ID for reference
        // In case of connection  issues, bind is used so when the owner gets the data, all users will too.
        let chatting_from = self.get_chatting_from();
        chatting_from
            .bind_property("user-id", &new_user_data, "owner-id")
            .sync_create()
            .build();

        chatting_from
            .bind_property("user-token", &new_user_data, "user-token")
            .sync_create()
            .build();

        let public_key = chatting_from.imp().rsa_public.get().unwrap().clone();
        let private_key = chatting_from.imp().rsa_private.get().unwrap().clone();

        new_user_data.imp().rsa_public.set(public_key).unwrap();
        new_user_data.imp().rsa_private.set(private_key).unwrap();

        new_user_data.handle_ws();
        self.get_users_liststore().append(&new_user_data);
        self.save_user_list();
        self.imp()
            .message_numbers
            .borrow_mut()
            .insert(new_user_data.user_id(), HashSet::new());
        new_user_data
    }

    /// Get the users ListBox
    fn get_user_list(&self) -> ListBox {
        self.imp().user_list.get()
    }

    /// Get the color is that is being used for owner UserObject
    fn get_owner_name_color(&self) -> String {
        self.get_chatting_from().name_color()
    }

    /// Start focus on the textview
    fn grab_focus(&self) {
        self.imp().message_entry.grab_focus();
    }

    fn remove_selected_avatar_css(&self, index: i32, listbox: &ListBox) {
        if let Some(row_data) = listbox.row_at_index(index) {
            let user_row: UserRow = row_data.child().unwrap().downcast().unwrap();
            user_row.imp().user_avatar.remove_css_class("user-selected");
            user_row.imp().user_avatar.add_css_class("user-inactive");
            user_row.imp().user_avatar.remove_css_class("user-pending");
        }
    }

    fn add_selected_avatar_css(&self, index: i32, listbox: &ListBox) {
        if let Some(row_data) = listbox.row_at_index(index) {
            let user_row: UserRow = row_data.child().unwrap().downcast().unwrap();
            user_row.imp().user_avatar.add_css_class("user-selected");
            user_row.imp().user_avatar.remove_css_class("user-inactive");
            user_row.imp().user_avatar.remove_css_class("user-pending");
        }
    }

    pub fn add_pending_avatar_css(&self, target_user: UserObject) {
        let listbox = self.get_user_list();
        let total_user = self.get_users_liststore().n_items() as i32;

        for index in 0..total_user {
            if let Some(row_data) = listbox.row_at_index(index) {
                let user_row: UserRow = row_data.child().unwrap().downcast().unwrap();
                if user_row.imp().user_data.get().unwrap() == &target_user {
                    user_row.imp().user_avatar.remove_css_class("user-selected");
                    user_row.imp().user_avatar.remove_css_class("user-inactive");
                    user_row.imp().user_avatar.add_css_class("user-pending");
                    break;
                }
            }
        }
    }

    /// Iters through every UserObject and tries to reconnect to the WebSocket server
    pub fn reload_user_ws(&self) {
        info!("Reloading websocket connection");
        let user_list = self.get_users_liststore();

        for user_data in user_list.iter() {
            let user_data: UserObject = user_data.unwrap();
            user_data.user_ws().reload_manually();
        }
    }

    /// Find a UserObject based on the User ID
    pub fn find_user(&self, target_id: u64) -> Option<UserObject> {
        let user_list = self.get_users_liststore();
        for user_data in user_list.iter() {
            let user_data: UserObject = user_data.unwrap();
            if user_data.user_id() == target_id {
                return Some(user_data);
            }
        }
        None
    }

    /// Find and delete a UserObject based on the User ID
    pub fn delete_user(&self, target_id: u64) {
        let user_list = self.get_users_liststore();
        for (index, user_data) in user_list.iter().enumerate() {
            let user_data: UserObject = user_data.unwrap();
            if user_data.user_id() == target_id {
                if self.get_chatting_with().user_id() == target_id {
                    // If selected user is removed, select the owner user
                    self.get_user_list().row_at_index(0).unwrap().activate();
                }
                user_data.stop_signals();
                self.get_users_liststore().remove(index as u32);
                user_data
                    .user_ws()
                    .emit_by_name::<()>("stop-processing", &[&true]);
                self.save_user_list();
                break;
            }
        }
    }

    /// Scroll to the bottom of the ListView if the given user is selected
    pub fn scroll_to_bottom(&self, current_user: UserObject, reveal_message: bool) {
        if current_user == self.get_chatting_with() {
            let window = self.clone();

            // Small timeout because the GUI needs some time to add the item to model
            timeout_add_local_once(Duration::from_millis(200), move || {
                let model = window.imp().message_list.model().unwrap();

                let total_item = model.n_items();
                if total_item == 0 {
                    return;
                }
                let last_index = model.n_items() - 1;

                // If true the message animation will be loaded separately
                // The avatar is already pre-loaded
                if reveal_message {
                    let object: MessageObject = model.item(last_index).unwrap().downcast().unwrap();
                    if let Some(row) = object.target_row() {
                        row.imp().message_revealer.set_reveal_child(false);
                        timeout_add_local_once(
                            Duration::from_millis(100),
                            clone!(@weak row => move || {
                                row.imp().message_revealer.set_reveal_child(true);
                            }),
                        );
                    }
                }

                window
                    .imp()
                    .message_list
                    .scroll_to(last_index, ListScrollFlags::NONE, None);
            });
        }
    }
}
