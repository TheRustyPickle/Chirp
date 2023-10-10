mod imp {
    use adw::{subclass::prelude::*, Avatar};
    use glib::subclass::InitializingObject;
    use glib::{object_subclass, Binding};
    use gtk::{glib, Box, Button, CompositeTemplate, Label, Revealer};
    use std::cell::{OnceCell, RefCell};

    use crate::message::MessageObject;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/message_row.xml")]
    pub struct MessageRow {
        #[template_child]
        pub message_revealer: TemplateChild<Revealer>,
        #[template_child]
        pub message_content: TemplateChild<Box>,
        #[template_child]
        pub placeholder: TemplateChild<Label>,
        #[template_child]
        pub sent_by: TemplateChild<Label>,
        #[template_child]
        pub message: TemplateChild<Label>,
        #[template_child]
        pub sender: TemplateChild<Avatar>,
        #[template_child]
        pub receiver: TemplateChild<Avatar>,
        #[template_child]
        pub sender_avatar_button: TemplateChild<Button>,
        #[template_child]
        pub receiver_avatar_button: TemplateChild<Button>,
        pub bindings: RefCell<Vec<Binding>>,
        pub message_data: OnceCell<MessageObject>,
    }

    #[object_subclass]
    impl ObjectSubclass for MessageRow {
        const NAME: &'static str = "MessageRow";
        type Type = super::MessageRow;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MessageRow {}

    impl WidgetImpl for MessageRow {}

    impl BoxImpl for MessageRow {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use gdk::Cursor;
use glib::{clone, timeout_add_local_once, wrapper, Object};
use gtk::{
    gdk, glib, Accessible, Box, Buildable, ConstraintTarget, Orientable, RevealerTransitionType,
    Widget,
};
use std::time::Duration;

use crate::message::MessageObject;
use crate::user::UserProfile;
use crate::window::Window;

wrapper! {
    pub struct MessageRow(ObjectSubclass<imp::MessageRow>)
    @extends Box, Widget,
    @implements Accessible, Buildable, ConstraintTarget, Orientable;
}

impl MessageRow {
    pub fn new(object: MessageObject, window: &Window) -> Self {
        let row: MessageRow = Object::builder().build();
        let revealer = row.imp().message_revealer.get();

        let new_cursor = Cursor::builder().name("pointer").build();

        if object.is_send() {
            let sender = row.imp().sender.get();
            sender.set_cursor(Some(&new_cursor));
            sender.set_visible(true);
            row.imp().receiver_avatar_button.set_visible(false);
            row.imp().sent_by.set_xalign(1.0);
            row.imp().message.set_xalign(1.0);
            row.imp().message_content.add_css_class("message-row-sent");
            row.imp().placeholder.set_visible(true);
            revealer.set_transition_type(RevealerTransitionType::SlideLeft)
        } else {
            let receiver = row.imp().receiver.get();
            receiver.set_cursor(Some(&new_cursor));
            receiver.set_visible(true);
            row.imp().sender_avatar_button.set_visible(false);
            row.imp().sent_by.set_xalign(0.0);
            row.imp().message.set_xalign(0.0);
            row.imp()
                .message_content
                .add_css_class("message-row-received");
            revealer.set_transition_type(RevealerTransitionType::SlideRight)
        }

        row.imp().message_data.set(object).unwrap();
        row.bind();
        row.connect_button_signals(window);

        // The transition must start after it gets added to the ListBox thus a small timer
        timeout_add_local_once(Duration::from_millis(50), move || {
            revealer.set_reveal_child(true);
        });

        row
    }

    fn bind(&self) {
        let mut bindings = self.imp().bindings.borrow_mut();

        let sent_by = self.imp().sent_by.get();
        let message = self.imp().message.get();

        let message_object = self.imp().message_data.get().unwrap();
        let is_sent = message_object.is_send();

        let sender = self.imp().message_data.get().unwrap().sent_from();

        sent_by.add_css_class(&format!("sender-{}", sender.name_color()));

        let sender_avatar = if is_sent {
            self.imp().sender.get()
        } else {
            self.imp().receiver.get()
        };

        let sent_by_binding = sender
            .bind_property("name", &sent_by, "label")
            .sync_create()
            .build();

        let avatar_fallback_binding = sender
            .bind_property("name", &sender_avatar, "text")
            .sync_create()
            .build();

        bindings.push(sent_by_binding);
        bindings.push(avatar_fallback_binding);

        let image_binding = sender
            .bind_property("small-image", &sender_avatar, "custom-image")
            .sync_create()
            .build();
        bindings.push(image_binding);

        let message_binding = message_object
            .bind_property("message", &message, "label")
            .sync_create()
            .build();

        bindings.push(message_binding);
    }

    fn connect_button_signals(&self, window: &Window) {
        let sender_button = self.imp().sender_avatar_button.get();
        let receiver_button = self.imp().receiver_avatar_button.get();

        let sent_from = self.imp().message_data.get().unwrap().sent_from();

        sender_button.connect_clicked(clone!(@weak window, @weak sent_from => move |_| {
            UserProfile::new(sent_from, &window, true);
        }));

        receiver_button.connect_clicked(clone!(@weak window, @weak sent_from => move |_| {
            UserProfile::new(sent_from, &window, false);
        }));
    }
}
