mod imp {
    use adw::subclass::prelude::*;
    use adw::ApplicationWindow;
    use gio::ListStore;
    use glib::object_subclass;
    use glib::subclass::InitializingObject;
    use gtk::{gio, glib, Button, CompositeTemplate, ListBox, ScrolledWindow, Stack, TextView};
    use std::cell::{OnceCell, RefCell};
    use std::rc::Rc;

    use crate::user::UserObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/therustypickle/chirp/window.xml")]
    pub struct Window {
        #[template_child]
        pub message_scroller: TemplateChild<ScrolledWindow>,
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
            obj.setup_binding();
        }
    }

    impl WindowImpl for Window {}

    impl WidgetImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}

use std::time::Duration;

use adw::subclass::prelude::*;
use adw::{prelude::*, Application};
use gio::{ActionGroup, ActionMap, ListStore, SimpleAction};
use glib::{clone, timeout_add_local_once, wrapper, ControlFlow, Object, Receiver};
use gtk::{
    gio, glib, Accessible, ApplicationWindow, Buildable, ConstraintTarget, ListBox, Native, Root,
    ShortcutManager, Widget,
};
use tracing::info;

use crate::message::{MessageObject, MessageRow};
use crate::user::{
    FullUserData, RequestType, SendMessageData, UserObject, UserProfile, UserPrompt, UserRow,
};
use crate::utils::generate_random_avatar_link;

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
                window.get_chatting_from().add_to_queue(RequestType::UpdateChattingWith(selected_user_id));
                window.set_chatting_with(selected_chat);
                window.setup_binding();
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

    fn setup_binding(&self) {
        let chatting_with = self.get_chatting_with();
        chatting_with
            .bind_property("name", self, "title")
            .transform_to(|_, name: String| Some(format!("Chirp - {}", name)))
            .sync_create()
            .build();
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
        let message_list = user.messages();
        self.imp().message_list.bind_model(
            Some(&message_list),
            clone!(@weak self as window => @default-panic, move |obj| {
                let message_data = obj.downcast_ref().expect("No MessageObject here");
                let row = window.get_message_row(message_data);
                window.grab_focus();
                window.scroll_to_bottom();
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

        info!("Sending new text message: {}", content);

        let sender = self.get_chatting_from();
        let receiver = self.get_chatting_with();

        self.get_chatting_from()
            .add_to_queue(RequestType::SendMessage(SendMessageData::new(
                sender.user_id(),
                receiver.user_id(),
                content.to_string(),
            )));

        buffer.set_text("");
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

    fn get_message_row(&self, data: &MessageObject) -> MessageRow {
        MessageRow::new(data.clone())
    }

    fn create_owner(&self, name: &str) -> UserObject {
        // It's a new user + owner so the ID will be generated on the server side
        let user_data = UserObject::new(name, Some(generate_random_avatar_link()), None, None);
        user_data.handle_ws(self.clone());
        self.get_users_liststore().append(&user_data);

        user_data
    }

    pub fn handle_ws_message(&self, user: &UserObject, receiver: Receiver<String>) {
        receiver.attach(None, clone!(@weak user as user_object, @weak self as window => @default-return ControlFlow::Break, move |response| {
            let response_data: Vec<&str> = response.splitn(2, ' ').collect();
            match response_data[0] {
                "/get-user-data" => {
                    let user_data: FullUserData = serde_json::from_str(response_data[1]).unwrap();
                    let user = window.create_user(user_data);
                    let user_row = UserRow::new(user);
                    window.get_user_list().append(&user_row);
                }
                "/message" => window.receive_message(response_data[1], user_object),
                "/update-user-id" => {
                    let chatting_from = window.get_chatting_from();
                    if user_object == chatting_from {
                        let id = response_data[1].parse::<u64>().unwrap();
                        chatting_from.set_owner_id(id);
                    }
                }
                _ => {}
            }
            ControlFlow::Continue
        }));
    }

    fn create_user(&self, user_data: FullUserData) -> UserObject {
        info!(
            "Creating new user with name: {}, id: {}",
            user_data.name, user_data.id
        );

        let new_user_data = UserObject::new(
            &user_data.name,
            user_data.image_link,
            Some(&self.get_owner_name_color()),
            Some(user_data.id),
        );

        // Every single user in the UserList of the client will have the owner User ID for reference
        let chatting_from = self.get_chatting_from();
        chatting_from
            .bind_property("user-id", &new_user_data, "owner-id")
            .sync_create()
            .build();

        new_user_data.handle_ws(self.clone());
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

    /// Takes the highest value of the scrollbar and sets it
    fn scroll_to_bottom(&self) {
        timeout_add_local_once(
            Duration::from_millis(100),
            clone!(@weak self as window => move || {
                let scroller_bar = window.imp().message_scroller.get();
                let upper = scroller_bar.vadjustment().upper();
                let vadjust = scroller_bar.vadjustment();
                vadjust.set_value(upper);
                scroller_bar.set_vadjustment(Some(&vadjust));
            }),
        );
    }
}
