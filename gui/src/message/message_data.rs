mod imp {
    use std::cell::RefCell;

    use crate::user::UserObject;
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{derived_properties, object_subclass, Properties};
    use gtk::glib;

    use super::MessageData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MessageObject)]
    pub struct MessageObject {
        #[property(name = "message", get, set, type = String, member = message)]
        #[property(name = "is-send", get, set, type = bool, member = is_send)]
        pub data: RefCell<MessageData>,
        #[property(get, set)]
        pub sent_from: RefCell<Option<UserObject>>,
        #[property(get, set)]
        pub sent_to: RefCell<Option<UserObject>>,
    }

    #[object_subclass]
    impl ObjectSubclass for MessageObject {
        const NAME: &'static str = "MessageObject";
        type Type = super::MessageObject;
    }

    #[derived_properties]
    impl ObjectImpl for MessageObject {}
}

use glib::wrapper;
use glib::Object;
use gtk::glib;

use crate::user::UserObject;

wrapper! {
    pub struct MessageObject(ObjectSubclass<imp::MessageObject>);
}

impl MessageObject {
    pub fn new(message: String, is_send: bool, sent_from: UserObject, sent_to: UserObject) -> Self {
        let obj: MessageObject = Object::builder()
            .property("is-send", is_send)
            .property("message", message)
            .property("sent-from", sent_from)
            .property("sent-to", sent_to)
            .build();
        obj
    }
}

#[derive(Default, Clone)]
pub struct MessageData {
    pub message: String,
    pub is_send: bool,
}
