mod imp {
    use std::cell::{OnceCell, RefCell};

    use adw::{subclass::prelude::*, Avatar};
    use glib::Binding;
    use gtk::{glib, Box, CompositeTemplate, Label};

    use crate::message_data::MessageObject;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/message_row.xml")]
    pub struct MessageRow {
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
        pub bindings: RefCell<Vec<Binding>>,
        pub message_data: OnceCell<MessageObject>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MessageRow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MessageRow";
        type Type = super::MessageRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for MessageRow {}

    // Trait shared by all widgets
    impl WidgetImpl for MessageRow {}

    // Trait shared by all boxes
    impl BoxImpl for MessageRow {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::closure_local;
use glib::Object;
use gtk::gdk::Paintable;
use gtk::glib;

use crate::message_data::MessageObject;
use crate::user_data::UserObject;

glib::wrapper! {
    pub struct MessageRow(ObjectSubclass<imp::MessageRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl MessageRow {
    pub fn new(object: MessageObject) -> Self {
        let row: MessageRow = Object::builder().build();

        if object.is_send() {
            row.imp().sender.set_visible(true);
            row.imp().sent_by.set_xalign(1.0);
            row.imp().message.set_xalign(1.0);
            row.imp().message_content.add_css_class("message-row-sent");
            row.imp().placeholder.set_visible(true);
        } else {
            row.imp().receiver.set_visible(true);
            row.imp().sent_by.set_xalign(0.0);
            row.imp().message.set_xalign(0.0);
            row.imp()
                .message_content
                .add_css_class("message-row-received")
        }

        let row_clone_1 = row.clone();

        let row_clone_2 = row.clone();

        let sent_from = object.sent_from().unwrap();
        let sent_to = object.sent_to().unwrap();

        sent_from.connect_closure(
            "updating-image",
            false,
            closure_local!(move |_from: UserObject, status: Paintable| {
                let sender = row_clone_1.imp().sender.get();
                //sender.set_paintable(Some(&status));
                sender.set_custom_image(Some(&status))
            }),
        );

        sent_to.connect_closure(
            "updating-image",
            false,
            closure_local!(move |_from: UserObject, status: Paintable| {
                let receiver = row_clone_2.imp().receiver.get();
                receiver.set_custom_image(Some(&status))
            }),
        );

        row.imp().message_data.set(object).unwrap();
        row
    }

    pub fn bind(&self) {
        let mut bindings = self.imp().bindings.borrow_mut();
        let sent_by = self.imp().sent_by.get();
        let message = self.imp().message.get();

        let message_object = self.imp().message_data.get().unwrap();
        let is_sent = message_object.is_send();

        if is_sent {
            let sent_from = self.imp().message_data.get().unwrap().sent_from().unwrap();
            let image = sent_from.image();
            let sender = self.imp().sender.get();

            let sent_by_binding = sent_from
                .bind_property("name", &sent_by, "label")
                .sync_create()
                .build();

            let avatar_fallback_binding = sent_from
                .bind_property("name", &sender, "text")
                .sync_create()
                .build();

            bindings.push(sent_by_binding);
            bindings.push(avatar_fallback_binding);

            if image.is_some() {
                let image_binding = sent_from
                    .bind_property("image", &sender, "custom-image")
                    .sync_create()
                    .build();
                bindings.push(image_binding);
            }
        } else {
            let sent_to = self.imp().message_data.get().unwrap().sent_to().unwrap();
            let image = sent_to.image();
            let receiver = self.imp().receiver.get();

            let sent_by_binding = sent_to
                .bind_property("name", &sent_by, "label")
                .sync_create()
                .build();

            let avatar_fallback_binding = sent_to
                .bind_property("name", &receiver, "text")
                .sync_create()
                .build();

            bindings.push(sent_by_binding);
            bindings.push(avatar_fallback_binding);

            if image.is_some() {
                let image_binding = sent_to
                    .bind_property("image", &receiver, "custom-image")
                    .sync_create()
                    .build();
                bindings.push(image_binding);
            }
        }

        let message_binding = message_object
            .bind_property("message", &message, "label")
            .sync_create()
            .build();

        bindings.push(message_binding);
    }
}
