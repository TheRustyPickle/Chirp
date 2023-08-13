mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::Binding;
    use gtk::{glib, CheckButton, CompositeTemplate, Label, Image};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/message_row.xml")]
    pub struct MessageRow {
        
        #[template_child]
        pub sent_by: TemplateChild<Label>,
        #[template_child]
        pub message: TemplateChild<Label>,
        pub bindings: RefCell<Vec<Binding>>,
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
use glib::Object;
use gtk::{glib, pango, Image, IconSize, Align};
use pango::{AttrInt, AttrList};

use crate::message_data::MessageObject;

glib::wrapper! {
    pub struct MessageRow(ObjectSubclass<imp::MessageRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl MessageRow {
    pub fn new(is_send: bool) -> Self {
        let row: MessageRow = Object::builder().build();

        if is_send {
            let sender_image = Image::from_icon_name("image-x-generic");
            sender_image.set_icon_size(IconSize::Large);
            sender_image.set_halign(Align::End);
            row.append(&sender_image);
            
            row.imp().sent_by.set_halign(Align::End);
            row.imp().message.set_halign(Align::End);
        } else {
            let sender_image = Image::from_icon_name("image-x-generic");
            sender_image.set_icon_size(IconSize::Large);
            sender_image.set_halign(Align::Start);
            row.prepend(&sender_image);
            
            row.imp().sent_by.set_halign(Align::Start);
            row.imp().message.set_halign(Align::Start);
        }

        row
    }

    pub fn bind(&self, message_object: &MessageObject) {
        let sent_by = self.imp().sent_by.get();
        let message = self.imp().message.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let sent_by_binding = message_object
            .bind_property("sent_by", &sent_by, "label")
            .sync_create()
            .build();

        let message_binding = message_object
            .bind_property("message", &message, "label")
            .sync_create()
            .build();

        bindings.push(sent_by_binding);

        bindings.push(message_binding);
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
