mod imp {
    use crate::APP_ID;

    use std::cell::{OnceCell, RefCell};
    use std::fs::File;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use adw::Leaflet;
    use gio::{ListStore, Settings};
    use glib::subclass::InitializingObject;
    use gtk::ffi::GtkEntry;
    use gtk::glib::SignalHandlerId;
    use gtk::{gio, glib, Button, CompositeTemplate, Entry, FilterListModel, ListBox, Stack};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/therustypickle/chirp/window.xml")]
    pub struct Window {
        #[template_child]
        pub message_box: TemplateChild<Entry>,
        #[template_child]
        pub message_list: TemplateChild<ListBox>,
        pub messages: OnceCell<ListStore>,
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
            obj.setup_message_list();
        }
    }

    impl WindowImpl for Window {}

    impl WidgetImpl for Window {}

    impl ApplicationWindowImpl for Window {}

    impl AdwApplicationWindowImpl for Window {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{ActionRow, Application, MessageDialog, NavigationDirection, ResponseAppearance};
use gio::{ListStore, Settings};
use glib::{clone, Object};
use gtk::{
    gio, glib, pango, Align, CheckButton, CustomFilter, Entry, FilterListModel, Label, ListBox,
    ListBoxRow, NoSelection, SelectionMode,
};

use crate::message_data::MessageObject;
use crate::message_row::MessageRow;

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
                window.new_message(true);
            }));

        imp.message_box
            .connect_icon_release(clone!(@weak self as window => move |_, _| {
                window.new_message(true);
            }));
    }

    fn new_message(&self, is_send: bool) {
        let message_buffer = self.imp().message_box.buffer();
        let message_text = message_buffer.text().to_string();

        if message_text.is_empty() {
            return;
        }

        message_buffer.set_text("");

        let message_object = MessageObject::new("Me".to_string(), message_text.clone());

        let message_row = MessageRow::new(is_send);
        message_row.bind(&message_object);

        if is_send {
            message_row.add_css_class("sent-message")
        } else {
            message_row.add_css_class("received-message")
        }

        self.get_message_list().append(&message_row);
    }

    fn get_messages(&self) -> ListStore {
        self.imp().messages.get().unwrap().clone()
    }

    fn get_message_list(&self) -> ListBox {
        self.imp().message_list.clone()
    }

    fn setup_message_list(&self) {
        let list = ListStore::new::<MessageObject>();

        self.imp()
            .messages
            .set(list.clone())
            .expect("Could not set collections");

            self.imp().message_box.buffer().set_text("Initial message");
        self.new_message(false)
    }
}
