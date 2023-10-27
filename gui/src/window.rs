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
    ListItem, ListScrollFlags, Native, NoSelection, PositionType, Root, ShortcutManager,
    SignalListItemFactory, Widget,
};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use tracing::{debug, error, info};

use crate::message::{MessageObject, MessageRow};
use crate::user::{UserObject, UserProfile, UserPrompt, UserRow};
use crate::utils::{
    generate_random_avatar_link, get_created_at_timing, read_rsa_keys, stringify_rsa_keys,
};
use crate::ws::{FullUserData, MessageData, RequestType, UserIDs};
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

                if last_index != index || last_index == index {
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
        let saving_location = self.settings().string("location");
        let user_data_path = format!("{}user_data.json", saving_location);

        if fs::metadata(user_data_path.to_owned()).is_ok() {
            let mut file = File::open(user_data_path).unwrap();
            let mut file_contents = String::new();

            file.read_to_string(&mut file_contents)
                .expect("Failed to read file");

            let id_data = UserIDs::from_json(&file_contents);
            return Some(id_data);
        }

        info!("Failed to find any previously saved user data");
        None
    }

    fn check_saved_keys(&self) -> Option<(RsaPublicKey, RsaPrivateKey)> {
        let saving_location = self.settings().string("location");
        let public_location = format!("{}public_key.pem", saving_location);
        let private_location = format!("{}private_key.pem", saving_location);

        if fs::metadata(public_location.to_owned()).is_ok()
            && fs::metadata(private_location.to_owned()).is_ok()
        {
            let existing_keys = read_rsa_keys(saving_location.to_string());
            if let Ok((public, private)) = existing_keys {
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

    pub fn save_rsa_keys(&self) {
        info!("Starting saving RSA keys");
        let saving_location = self.settings().string("location").to_string();
        let owner_data = self.get_chatting_from();

        let public_key = owner_data.imp().rsa_public.get().unwrap();
        let private_key = owner_data.imp().rsa_private.get().unwrap();

        let (public_string, private_string) = stringify_rsa_keys(public_key, private_key);

        if fs::metadata(saving_location.to_owned()).is_ok() {
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
        let data: UserObject = self.create_owner(saved_user_id.clone(), saved_keys);

        self.imp().own_profile.replace(Some(data.clone()));

        // Select the first row we just added
        self.get_user_list().row_at_index(0).unwrap().activate();

        if let Some(id_data) = saved_user_id {
            let mut saved_users = self.get_saved_user_data();
            // If empty stop checking
            if saved_users.is_empty() {
                return;
            }

            let owner_data = saved_users.remove(0);
            // If it's not the same then the saved data is outdated or invalid
            if owner_data.user_id != id_data.user_id {
                error!("Invalid or outdated owner data found. Dismissing saved data");
                return;
            }

            data.set_name(owner_data.user_name);

            // Have to set the image link manually otherwise if new users are added
            // after this and the image is not loaded it would save None image link
            data.set_image_link(owner_data.image_link.clone());
            data.check_image_link(owner_data.image_link, false);

            for user_data in saved_users {
                self.create_user(user_data)
            }
        }
    }

    /// Get the UserObject that is currently selected/chatting with
    pub fn get_chatting_with(&self) -> UserObject {
        self.imp().chatting_with.borrow().clone().unwrap()
    }

    /// Set chatting with the given user
    fn set_chatting_with(&self, user: UserObject) {
        info!("Setting chatting with {}", user.name());
        user.add_queue_to_first(RequestType::GetLastMessageNumber(user.clone()));
        let message_list = user.messages();
        let selection_model = NoSelection::new(Some(message_list));
        self.imp().message_list.set_model(Some(&selection_model));

        // Bind user chatting with liststore with the factory so if any new message gets added there
        // new message row creation also starts
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let message_row = MessageRow::new_empty();
            list_item
                .downcast_ref::<ListItem>()
                .unwrap()
                .set_child(Some(&message_row));
            let list_item = list_item.downcast_ref::<ListItem>().unwrap();
            list_item.set_activatable(false);
            list_item.set_selectable(false);
            list_item.set_focusable(false);
        });

        factory.connect_bind(clone!(@weak self as window => move |_, item| {
            let list_item = item
            .downcast_ref::<ListItem>()
            .unwrap();

            let message_object = list_item
                .item()
                .and_downcast::<MessageObject>()
                .unwrap();

            let message_row = list_item
                .child()
                .and_downcast::<MessageRow>()
                .unwrap();
            message_row.update(&message_object, &window);
        }));

        factory.connect_unbind(move |_, list_item| {
            let message_row = list_item
                .downcast_ref::<ListItem>()
                .unwrap()
                .child()
                .and_downcast::<MessageRow>()
                .unwrap();

            message_row.stop_signals();
        });

        self.imp().message_list.set_factory(Some(&factory));
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
            content.to_owned(),
            true,
            sender,
            receiver.clone(),
            message_timing,
            None,
        )
        .to_process(true);
        self.chatting_with_messages().append(&message);
        buffer.set_text("");

        let send_message_data =
            MessageData::new_incomplete(created_at, self.get_owner_id(), receiver_id, content);

        // Receiver gets the queue because the receiver saves the message number variable
        // if it was sender, it would send the message number of owner_id@owner_id group which is invalid
        receiver.add_to_queue(RequestType::SendMessage(send_message_data, message.clone()));
    }

    /// Gets called when a message is received or when syncing previous message data
    pub fn receive_message(
        &self,
        message_data: MessageData,
        other_user: UserObject,
        add_css: bool,
    ) {
        // No need to receive an already existing message
        if self
            .imp()
            .message_numbers
            .borrow()
            .get(&other_user.user_id())
            .unwrap()
            .contains(&message_data.message_number)
        {
            return;
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

        // Will always proceed to this block if syncing messages
        if current_message_number >= message_data.message_number {
            // If 0 element, no checks to be done. Append it to the list
            let element_num = other_user.messages().n_items();
            if element_num < 1 {
                other_user.messages().append(&message);
            } else {
                other_user
                    .messages()
                    .insert_sorted(&message, |obj_1, obj_2| {
                        let message_content_1: MessageObject = obj_1.clone().downcast().unwrap();
                        let message_content_2: MessageObject = obj_2.clone().downcast().unwrap();
                        let msg_num_1 = message_content_1.imp().message_number.get();
                        let msg_num_2 = message_content_2.imp().message_number.get();

                        match (msg_num_1, msg_num_2) {
                            (Some(num_a), Some(num_b)) => num_a.cmp(&num_b),
                            (Some(_), None) => Ordering::Less,
                            (None, Some(_)) => Ordering::Greater,
                            (None, None) => Ordering::Equal,
                        }
                    });
            }
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
            )
        } else {
            UserObject::new(
                "Me",
                Some(generate_random_avatar_link()),
                None,
                None,
                None,
                self.clone(),
            )
        };

        user_data.handle_ws();
        self.get_users_liststore().append(&user_data);

        user_data
    }

    /// Used to create all UserObject for the self's users ListStore except for the owner UserObject.
    /// Called when New Chat button is used or a message is received but the user was not added
    pub fn create_user(&self, user_data: FullUserData) {
        info!(
            "Creating new user with name: {}, id: {}",
            user_data.user_name, user_data.user_id
        );

        let new_user_data = UserObject::new(
            &user_data.user_name,
            user_data.image_link,
            Some(&self.get_owner_name_color()),
            Some(user_data.user_id),
            None,
            self.clone(),
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

        new_user_data.handle_ws();
        self.get_users_liststore().append(&new_user_data);
        self.save_user_list();
        self.imp()
            .message_numbers
            .borrow_mut()
            .insert(new_user_data.user_id(), HashSet::new());
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

    fn add_pending_avatar_css(&self, target_user: UserObject) {
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

    /// Scroll to the bottom of the ListView if the given is selected
    pub fn scroll_to_bottom(&self, current_user: UserObject) {
        if current_user == self.get_chatting_with() {
            let model = self.imp().message_list.model().unwrap();

            let last_index = model.n_items() - 1;

            self.imp()
                .message_list
                .scroll_to(last_index, ListScrollFlags::NONE, None);
        }
    }
}
