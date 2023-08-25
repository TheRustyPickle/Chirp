mod imp {

    use adw::subclass::prelude::*;
    use adw::{ApplicationWindow, Leaflet};
    use gio::ListStore;
    use glib::object_subclass;
    use glib::subclass::InitializingObject;
    use gtk::{gio, glib, Button, CompositeTemplate, ListBox, Stack, TextView};
    use std::cell::{OnceCell, RefCell};
    use std::rc::Rc;

    use crate::user_data::UserObject;

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
        // `NAME` needs to match `class` attribute of template
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
            // Call "constructed" on parent
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
use adw::Application;
use adw::{prelude::*, NavigationDirection};
use gio::{ActionGroup, ActionMap, ListStore, SimpleAction};
use glib::{clone, wrapper, Object};
use gtk::prelude::*;
use gtk::{
    gio, glib, Accessible, ApplicationWindow, Buildable, ConstraintTarget, ListBox, Native, Root,
    ShortcutManager, Widget,
};

use crate::message_data::MessageObject;
use crate::message_row::MessageRow;
use crate::user_data::UserObject;
use crate::user_row::UserRow;
use crate::utils::{generate_dicebear_link, generate_robohash_link};

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
                if !leaflet.is_child_transition_running() {
                    if leaflet.is_folded() {
                        leaflet.navigate(NavigationDirection::Forward);
                        leaflet.navigate(NavigationDirection::Forward);
                    }
                }

            }));

        imp.user_list
            .connect_row_activated(clone!(@weak self as window => move |_, row| {
                let index = row.index();
                let selected_chat = window.get_users_liststore()
                .item(index as u32)
                .expect("There should be an item here")
                .downcast::<UserObject>()
                .expect("It should be an UserObject");
                window.set_chatting_with(selected_chat);
            }));

        self.imp()
            .new_chat
            .connect_clicked(clone!(@weak self as window => move |_| {
                let user_data = window.create_user("test user");
                let user_row = UserRow::new(user_data);
                user_row.bind();
                window.get_user_list().append(&user_row);
            }));
    }

    fn setup_actions(&self) {
        let button_action = SimpleAction::new("send-message", None);
        button_action.connect_activate(clone!(@weak self as window => move |_, _| {
            window.send_message();
            window.receive_message("Bot message received. A very long message is about to be sent to test how the ui is doing on handling the wrapping and the size.");
        }));

        self.add_action(&button_action);
    }

    fn setup_users(&self) {
        let users = ListStore::new::<UserObject>();
        self.imp().users.set(users).expect("Could not set users");

        let data = self.create_user("Me");

        let user_clone_1 = data.clone();
        let user_clone_2 = data.clone();

        self.imp().own_profile.replace(Some(data));

        let user_row = UserRow::new(user_clone_1);
        user_row.bind();
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
        let message_list = user.messages();
        self.imp().message_list.bind_model(
            Some(&message_list),
            clone!(@weak self as window => @default-panic, move |obj| {
                let message_data = obj.downcast_ref().expect("No MessageData here");
                let row = window.create_message(message_data);
                row.upcast()
            }),
        );

        self.imp().chatting_with.replace(Some(user));
    }

    fn get_chatting_from(&self) -> UserObject {
        self.imp()
            .own_profile
            .borrow()
            .clone()
            .expect("Expected an UserObject")
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
            return;
        }
        buffer.set_text("");

        let sender = self.get_chatting_from();
        let receiver = self.get_chatting_with();
        let message = MessageObject::new(content, true, sender, receiver);

        self.chatting_with_messages().append(&message);
        self.create_message(&message);
    }

    fn receive_message(&self, content: &str) {
        let sender = self.get_chatting_from();
        let receiver = self.get_chatting_with();

        if sender == receiver {
            return;
        }

        let message = MessageObject::new(content.to_string(), false, sender, receiver);

        self.chatting_with_messages().append(&message);
        self.create_message(&message);
    }

    fn create_message(&self, data: &MessageObject) -> MessageRow {
        let row = MessageRow::new(data.clone());
        row.bind();
        row
    }

    fn create_user(&self, name: &str) -> UserObject {
        let messages = ListStore::new::<MessageObject>();
        let user_data = UserObject::new(name, Some(generate_dicebear_link()), messages);
        self.get_users_liststore().append(&user_data);
        user_data
    }

    fn get_message_list(&self) -> ListBox {
        self.imp().message_list.get()
    }

    fn get_user_list(&self) -> ListBox {
        self.imp().user_list.get()
    }
}
