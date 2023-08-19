mod imp {

    use adw::subclass::prelude::*;
    use gio::ListStore;
    use glib::subclass::InitializingObject;
    use gtk::{gio, glib, CompositeTemplate, Entry, ListBox};
    use std::{
        cell::{OnceCell, RefCell},
        rc::Rc,
    };

    use crate::user_data::UserObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/therustypickle/chirp/window.xml")]
    pub struct Window {
        #[template_child]
        pub message_box: TemplateChild<Entry>,
        #[template_child]
        pub message_list: TemplateChild<ListBox>,
        pub users: OnceCell<ListStore>,
        pub chatting_with: Rc<RefCell<Option<UserObject>>>,
        pub own_profile: Rc<RefCell<Option<UserObject>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MainWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

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
        }
    }

    impl WindowImpl for Window {}

    impl WidgetImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::Application;
use gio::ListStore;
use glib::{clone, Object};
use gtk::{gio, glib, ListBox};

use crate::message_data::MessageObject;
use crate::message_row::MessageRow;
use crate::user_data::UserObject;
use crate::utils::generate_avatar_link;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new(app: &Application) -> Self {
        Object::builder().property("application", app).build()
    }

    fn setup_callbacks(&self) {
        let imp = self.imp();

        imp.message_box
            .connect_activate(clone!(@weak self as window => move |_| {
                window.new_message();
                window.new_receive_message("Bot message received");
            }));

        imp.message_box
            .connect_icon_release(clone!(@weak self as window => move |_, _| {
                window.new_message();
            }));
    }

    fn setup_users(&self) {
        let users = ListStore::new::<UserObject>();
        self.imp().users.set(users).expect("Could not set users");

        let data = self.create_user("Me");
        self.imp().own_profile.replace(Some(data));
        let data = self.create_user("Bot reply");
        self.imp().chatting_with.replace(Some(data));
    }

    fn get_chatting_with(&self) -> UserObject {
        self.imp()
            .chatting_with
            .borrow()
            .clone()
            .expect("Expected an UserObject")
    }

    fn get_chatting_from(&self) -> UserObject {
        self.imp()
            .own_profile
            .borrow()
            .clone()
            .expect("Expected an UserObject")
    }

    fn get_all_users(&self) -> ListStore {
        self.imp()
            .users
            .get()
            .expect("User liststore is not set")
            .clone()
    }

    fn chatting_with_messages(&self) -> ListStore {
        self.get_chatting_with().messages()
    }

    fn new_message(&self) {
        let buffer = self.imp().message_box.buffer();
        let content = buffer.text().to_string();
        if content.is_empty() {
            return;
        }
        buffer.set_text("");

        let sender = self.get_chatting_from();
        let receiver = self.get_chatting_with();
        let message = MessageObject::new(content, true, sender, receiver);

        self.chatting_with_messages().append(&message);

        let row = MessageRow::new(message);
        row.bind();
        self.get_message_list().append(&row);
    }

    fn new_receive_message(&self, content: &str) {
        let sender = self.get_chatting_from();
        let receiver = self.get_chatting_with();
        let message = MessageObject::new(content.to_string(), false, sender, receiver);

        self.chatting_with_messages().append(&message);

        let row = MessageRow::new(message);
        row.bind();
        self.get_message_list().append(&row);
    }

    fn create_user(&self, name: &str) -> UserObject {
        let messages = ListStore::new::<MessageObject>();
        let user_data = UserObject::new(name, Some(generate_avatar_link()), messages);
        self.get_all_users().append(&user_data);
        user_data
    }

    fn get_message_list(&self) -> ListBox {
        self.imp().message_list.get()
    }
}
