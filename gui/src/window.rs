mod imp {
    use adw::subclass::prelude::*;
    use adw::{ApplicationWindow, Leaflet};
    use gio::ListStore;
    use glib::object_subclass;
    use glib::subclass::InitializingObject;
    use gtk::{gio, glib, Button, CompositeTemplate, ListBox, Stack, TextView};
    use std::cell::{OnceCell, RefCell};
    use std::rc::Rc;

    use crate::user::UserObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/therustypickle/chirp/window.xml")]
    pub struct Window {
        #[template_child]
        pub leaflet: TemplateChild<Leaflet>,
        #[template_child]
        pub message_box: TemplateChild<TextView>,
        #[template_child]
        pub message_list: TemplateChild<ListBox>,
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
        pub users: OnceCell<ListStore>,
        pub chatting_with: Rc<RefCell<Option<UserObject>>>,
        pub own_profile: Rc<RefCell<Option<UserObject>>>,
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
            obj.setup_callbacks();
            obj.setup_users();
            obj.setup_actions();
        }
    }

    impl WindowImpl for Window {}

    impl WidgetImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}

use adw::subclass::prelude::*;
use adw::{prelude::*, Application, NavigationDirection};
use gio::{ActionGroup, ActionMap, ListStore, SimpleAction};
use glib::{clone, wrapper, ControlFlow, Object, Receiver};
use gtk::{
    gio, glib, Accessible, ApplicationWindow, Buildable, ConstraintTarget, ListBox, Native, Root,
    ShortcutManager, Widget,
};
use tracing::info;

use crate::message::{MessageObject, MessageRow};
use crate::user::{FullUserData, UserObject, UserProfile, UserPrompt, UserRow};
use crate::utils::{generate_dicebear_link, generate_robohash_link};
use crate::ws::WSObject;

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
        imp.message_box.grab_focus();
        imp.stack.set_visible_child_name("main");
        imp.leaflet
            .connect_folded_notify(clone!(@weak self as window => move |leaflet| {
                if !leaflet.is_child_transition_running() && leaflet.is_folded() {
                    leaflet.navigate(NavigationDirection::Forward);
                    leaflet.navigate(NavigationDirection::Forward);
                }

            }));

        imp.user_list
            .connect_row_activated(clone!(@weak self as window => move |_, row| {
                let index = row.index();
                let selected_chat = window.get_users_liststore()
                .item(index as u32)
                .unwrap()
                .downcast::<UserObject>()
                .unwrap();

                info!("Selected a new User from list");
                let selected_user_id = selected_chat.user_id();
                window.get_chatting_from().user_ws().update_chatting_with(selected_user_id);
                window.set_chatting_with(selected_chat);
            }));

        self.imp()
            .new_chat
            .connect_clicked(clone!(@weak self as window => move |_| {
                let prompt = UserPrompt::new("Start Chat").add_user(&window);
                prompt.present();
            }));

        self.imp()
            .my_profile
            .connect_clicked(clone!(@weak self as window => move |_| {
                UserProfile::new(window.get_chatting_from(), &window);
            }));
    }

    fn setup_actions(&self) {
        let button_action = SimpleAction::new("send-message", None);
        button_action.connect_activate(clone!(@weak self as window => move |_, _| {
            window.send_message();
            window.grab_focus();
        }));

        self.add_action(&button_action);
    }

    fn setup_users(&self) {
        let users = ListStore::new::<UserObject>();
        self.imp().users.set(users).expect("Could not set users");

        let data: UserObject = self.create_owner("Me");

        let user_clone_1 = data.clone();
        let user_clone_2 = data.clone();

        info!("Setting own profile");
        self.imp().own_profile.replace(Some(data));
        let user_row = UserRow::new(user_clone_1);
        self.get_user_list().append(&user_row);

        self.set_chatting_with(user_clone_2);

        if let Some(row) = self.get_user_list().row_at_index(0) {
            self.get_user_list().select_row(Some(&row));
        }
    }

    fn get_chatting_with(&self) -> UserObject {
        self.imp()
            .chatting_with
            .borrow()
            .clone()
            .expect("Expected an UserObject")
    }

    fn set_chatting_with(&self, user: UserObject) {
        info!("Setting chatting with {}", user.name());
        self.set_title(Some(&format!("Chirp - {}", user.name())));
        let message_list = user.messages();
        info!("Binding model");
        self.imp().message_list.bind_model(
            Some(&message_list),
            clone!(@weak self as window => @default-panic, move |obj| {
                let message_data = obj.downcast_ref().expect("No MessageObject here");
                let row = window.create_message(message_data);
                window.grab_focus();
                row.upcast()
            }),
        );

        self.imp().chatting_with.replace(Some(user));
    }

    pub fn get_chatting_from(&self) -> UserObject {
        self.imp()
            .own_profile
            .borrow()
            .clone()
            .expect("Expected an UserObject")
    }

    pub fn get_owner_id(&self) -> u64 {
        self.get_chatting_from().user_id()
    }

    fn get_users_liststore(&self) -> ListStore {
        self.imp()
            .users
            .get()
            .expect("User liststore is not set")
            .clone()
    }

    fn chatting_with_messages(&self) -> ListStore {
        self.get_chatting_with().messages()
    }

    fn send_message(&self) {
        let buffer = self.imp().message_box.buffer();
        let content = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .trim()
            .to_string();

        if content.is_empty() {
            info!("Empty text found");
            return;
        }

        // NOTE dummy (number) will create a dummy user on the server
        if content.starts_with("dummy") {
            info!("Creating dummy user");
            let dummy_type: Vec<&str> = content.splitn(2, ' ').collect();
            self.create_dummy_user(dummy_type[1].parse().unwrap());
            return;
        }

        info!("Sending new text message: {}", content);

        if let Some(conn) = self.get_chatting_from().user_ws().ws_conn() {
            conn.send_text(&content);
        }

        buffer.set_text("");

        let sender = self.get_chatting_from();
        let receiver = self.get_chatting_with();
        let message = MessageObject::new(content, true, sender, receiver);

        self.chatting_with_messages().append(&message);
    }

    fn receive_message(&self, content: &str, sender: UserObject) {
        info!(
            "Receiving message from {} with content: {}",
            sender.name(),
            content
        );
        let receiver = self.get_chatting_with();

        if sender == self.get_chatting_from() {
            info!("Both sender and receiver are the same. Stopping sending");
            return;
        }

        let message = MessageObject::new(content.to_string(), false, sender.clone(), receiver);

        sender.messages().append(&message);
    }

    fn create_message(&self, data: &MessageObject) -> MessageRow {
        let row = MessageRow::new(data.clone());
        row
    }

    fn create_owner(&self, name: &str) -> UserObject {
        let messages = ListStore::new::<MessageObject>();
        let ws = WSObject::new();
        let user_data = UserObject::new(
            name,
            Some(generate_dicebear_link()),
            messages,
            None,
            ws,
            None,
        );

        // It's a new user + owner so the ID will be generated on the server side
        let receiver = user_data.handle_ws(0);
        self.handle_ws_message(user_data.clone(), receiver);

        self.get_users_liststore().append(&user_data);

        user_data
    }

    fn create_dummy_user(&self, image_type: u8) {
        let messages = ListStore::new::<MessageObject>();
        let ws = WSObject::new();
        let image_link = if image_type == 0 {
            generate_robohash_link()
        } else {
            generate_dicebear_link()
        };
        let user_data = UserObject::new("Dummy user", Some(image_link), messages, None, ws, None);

        let receiver = user_data.handle_ws(self.get_owner_id());
        self.handle_ws_message(user_data.clone(), receiver);
    }

    fn handle_ws_message(&self, user: UserObject, receiver: Receiver<String>) {
        receiver.attach(None, clone!(@weak user as user_object, @weak self as window => @default-return ControlFlow::Break, move |response| {
            let response_data: Vec<&str> = response.splitn(2, ' ').collect();
            match response_data[0] {
                "/get-user-data" => {
                    let user_data: FullUserData = serde_json::from_str(response_data[1]).unwrap();
                    let user = window.create_user(user_data);
                    let user_row = UserRow::new(user);
                    window.get_user_list().append(&user_row);
                }
                _ => window.receive_message(&response, user_object),
            }
            ControlFlow::Continue
        }));
    }

    fn create_user(&self, user_data: FullUserData) -> UserObject {
        info!(
            "Creating new user with name: {}, id: {}",
            user_data.name, user_data.id
        );
        let messages = ListStore::new::<MessageObject>();
        let ws = WSObject::new();

        let new_user_data = UserObject::new(
            &user_data.name,
            user_data.image_link,
            messages,
            Some(&self.get_owner_name_color()),
            ws,
            Some(user_data.id),
        );

        let receiver = new_user_data.handle_ws(self.get_owner_id());
        self.handle_ws_message(new_user_data.clone(), receiver);
        self.get_users_liststore().append(&new_user_data);
        new_user_data
    }

    fn get_user_list(&self) -> ListBox {
        self.imp().user_list.get()
    }

    fn get_owner_name_color(&self) -> String {
        self.get_chatting_from().name_color()
    }

    fn grab_focus(&self) {
        self.imp().message_box.grab_focus();
    }
}
