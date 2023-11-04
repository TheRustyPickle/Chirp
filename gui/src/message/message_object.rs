mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::{derived_properties, object_subclass, Properties};
    use gtk::glib;
    use std::cell::{Cell, OnceCell, RefCell};

    use crate::message::MessageRow;
    use crate::user::UserObject;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MessageObject)]
    pub struct MessageObject {
        #[property(get, set)]
        pub message: OnceCell<String>,
        #[property(get, set)]
        pub is_send: OnceCell<bool>,
        #[property(get, set)]
        pub sent_from: OnceCell<UserObject>,
        #[property(get, set)]
        pub sent_to: OnceCell<UserObject>,
        #[property(get, set)]
        pub message_timing: OnceCell<String>,
        #[property(get, set)]
        pub message_number: OnceCell<u64>,
        #[property(get, set)]
        pub target_row: RefCell<Option<MessageRow>>,
        #[property(get, set)]
        pub must_process: Cell<bool>,
        #[property(get, set)]
        pub show_initial_message: Cell<bool>,
    }

    #[object_subclass]
    impl ObjectSubclass for MessageObject {
        const NAME: &'static str = "MessageObject";
        type Type = super::MessageObject;
    }

    #[derived_properties]
    impl ObjectImpl for MessageObject {}
}

use glib::{wrapper, Object};
use gtk::glib;

use crate::user::UserObject;

wrapper! {
    pub struct MessageObject(ObjectSubclass<imp::MessageObject>);
}

impl MessageObject {
    pub fn new(
        message: String,
        is_send: bool,
        sent_from: UserObject,
        sent_to: UserObject,
        message_timing: String,
        message_number: Option<u64>,
    ) -> Self {
        let obj: MessageObject = Object::builder()
            .property("is-send", is_send)
            .property("message", message)
            .property("sent-from", sent_from)
            .property("sent-to", sent_to)
            .property("message-timing", message_timing)
            .property("must-process", false)
            .property("show-initial-message", true)
            .build();

        if let Some(num) = message_number {
            obj.set_message_number(num)
        }

        obj
    }

    /// Sets the status of whether this needs to be processed. Utilized
    /// by MessageRow to determine whether to show the spinner
    pub fn to_process(self, state: bool) -> Self {
        self.set_must_process(state);
        if let Some(row) = self.target_row() {
            if state {
                row.disable_delete_message()
            } else {
                row.enable_delete_message()
            }
        };
        self
    }
}
