mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gio::ListStore;
    use glib::{derived_properties, object_subclass, Properties};
    use gtk::{glib, NoSelection, SignalListItemFactory};
    use std::cell::{Cell, OnceCell, RefCell};
    use std::collections::HashMap;

    use crate::message::MessageObject;
    use crate::user::UserObject;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MessageRenderer)]
    pub struct MessageRenderer {
        pub saved_messages: RefCell<HashMap<u64, MessageObject>>,
        #[property(get, set)]
        pub message_liststore: OnceCell<ListStore>,
        #[property(get, set)]
        pub is_syncing: Cell<bool>,
        #[property(get, set)]
        pub message_number: Cell<u64>,
        #[property(get, set)]
        pub synced_till: Cell<u64>,
        #[property(get, set)]
        pub belongs_to: OnceCell<UserObject>,
        #[property(get, set)]
        pub message_factory: OnceCell<SignalListItemFactory>,
        #[property(get, set)]
        pub selection_model: OnceCell<NoSelection>,
        #[property(get, set)]
        pub became_inactive: Cell<bool>,
    }

    #[object_subclass]
    impl ObjectSubclass for MessageRenderer {
        const NAME: &'static str = "MessageRenderer";
        type Type = super::MessageRenderer;
    }

    #[derived_properties]
    impl ObjectImpl for MessageRenderer {}
}

use gio::subclass::prelude::ObjectSubclassIsExt;
use gio::{glib::SignalHandlerId, ListStore};
use glib::{clone, wrapper, Object};
use gtk::prelude::*;
use gtk::{glib, ListItem, NoSelection, SignalListItemFactory};
use std::cell::RefMut;

use crate::message::{MessageObject, MessageRow};
use crate::user::UserObject;

wrapper! {
    pub struct MessageRenderer(ObjectSubclass<imp::MessageRenderer>);
}

impl MessageRenderer {
    pub fn new(belongs_to: UserObject, signal_list: RefMut<Vec<SignalHandlerId>>) -> Self {
        let liststore = ListStore::new::<MessageObject>();
        let factory = SignalListItemFactory::new();
        let selection_model = NoSelection::new(Some(liststore.clone()));

        let obj: MessageRenderer = Object::builder()
            .property("message-number", 0_u64)
            .property("synced-till", 0_u64)
            .property("message-liststore", liststore)
            .property("is-syncing", false)
            .property("message-factory", factory)
            .property("selection-model", selection_model)
            .property("belongs-to", belongs_to)
            .property("became-inactive", false)
            .build();

        obj.start_factory(signal_list);

        obj
    }

    /// Connect the message ListStore with the ListView factory
    fn start_factory(&self, mut signal_list: RefMut<Vec<SignalHandlerId>>) {
        let factory = self.imp().message_factory.get().unwrap();
        let window = self.belongs_to().main_window();

        let factory_setup_signal = factory.connect_setup(move |_, list_item| {
            let message_row = MessageRow::new_empty();
            let list_item = list_item.downcast_ref::<ListItem>().unwrap();
            list_item.set_child(Some(&message_row));
            list_item.set_activatable(false);
            list_item.set_selectable(false);
            list_item.set_focusable(false);
        });

        let factory_bind_signal = factory.connect_bind(clone!(@weak window => move |_, item| {
            let list_item = item
            .downcast_ref::<ListItem>()
            .unwrap();

            let message_object = list_item
                .item()
                .and_downcast::<MessageObject>()
                .unwrap();

            let message_row = list_item
                .child()
                .and_downcast::<MessageRow>()
                .unwrap();
            message_row.update(&message_object, &window);
        }));

        let factory_unbind_signal = factory.connect_unbind(move |_, list_item| {
            let message_row = list_item
                .downcast_ref::<ListItem>()
                .unwrap()
                .child()
                .and_downcast::<MessageRow>()
                .unwrap();

            message_row.stop_signals();
        });

        signal_list.push(factory_setup_signal);
        signal_list.push(factory_bind_signal);
        signal_list.push(factory_unbind_signal);
    }

    /// Save a message data for future loading
    pub fn save_message(&self, message: MessageObject, number: u64) {
        self.imp()
            .saved_messages
            .borrow_mut()
            .insert(number, message);
    }

    /// Clears the user's Message ListStore. Used before another user is set as active
    pub fn user_inactive(&self) {
        self.message_liststore().remove_all();
        if self.is_syncing() {
            self.set_became_inactive(true)
        }
    }

    /// Reloads the saved messages to the ListStore. Used after the user is set as active
    pub fn user_active(&self) {
        let message_list = self.message_liststore();
        let saved_messages = self.imp().saved_messages.borrow();

        let mut total_to_add = 100;
        let mut last_num = self.message_number();

        while total_to_add > 0 && last_num > 0 {
            if let Some(message) = saved_messages.get(&last_num) {
                message_list.append(message);
                total_to_add += 1;
            }
            last_num -= 1;
        }
    }
}
