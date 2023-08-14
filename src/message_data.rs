mod imp {
    use std::cell::RefCell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::Properties;
    use gtk::glib;

    use super::MessageData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MessageObject)]
    pub struct MessageObject {
        #[property(name = "message", get, set, type = String, member = message)]
        #[property(name = "sent-by", get, set, type = String, member = sent_by)]
        pub data: RefCell<MessageData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MessageObject {
        const NAME: &'static str = "ChirpMessageObject";
        type Type = super::MessageObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MessageObject {}
}

use glib::Object;
use gtk::{glib, IconSize, Image};

glib::wrapper! {
    pub struct MessageObject(ObjectSubclass<imp::MessageObject>);
}

impl MessageObject {
    pub fn new(sent_by: String, message: String) -> Self {
        let placeholder_image = Image::from_icon_name("image-x-generic");
        placeholder_image.set_icon_size(IconSize::Large);

        Object::builder()
            .property("sent-by", sent_by)
            .property("message", message)
            .build()
    }
}

#[derive(Default, Clone)]
pub struct MessageData {
    pub sent_by: String,
    pub message: String,
}
