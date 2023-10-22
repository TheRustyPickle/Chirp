mod imp {
    use adw::subclass::prelude::*;
    use adw::Avatar;
    use glib::subclass::InitializingObject;
    use glib::{object_subclass, Binding};
    use gtk::{glib, Box, Button, CompositeTemplate, Label, PopoverMenu, Revealer, Spinner};
    use std::cell::RefCell;

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
        #[template_child]
        pub message_menu: TemplateChild<PopoverMenu>,
        #[template_child]
        pub message_time: TemplateChild<Label>,
        #[template_child]
        pub processing_spinner: TemplateChild<Spinner>,
        pub bindings: RefCell<Vec<Binding>>,
        pub message_data: RefCell<Option<MessageObject>>,
    }

    #[object_subclass]
    impl ObjectSubclass for MessageRow {
        const NAME: &'static str = "MessageRow";
        type Type = super::MessageRow;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("message-row.copy", None, move |row, _, _| {
                row.copy_message()
            });
            klass.install_action("message-row.delete", None, move |row, _, _| {
                row.delete_message()
            });
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
use gdk::{Cursor, Rectangle};
use glib::{clone, timeout_add_local_once, wrapper, Object};
use gtk::{
    gdk, glib, Accessible, Box, Buildable, ConstraintTarget, GestureClick, Orientable,
    RevealerTransitionType, Widget,
};
use std::time::Duration;
use tracing::info;

use crate::message::MessageObject;
use crate::user::UserProfile;
use crate::window::Window;
use crate::ws::RequestType;

wrapper! {
    pub struct MessageRow(ObjectSubclass<imp::MessageRow>)
    @extends Box, Widget,
    @implements Accessible, Buildable, ConstraintTarget, Orientable;
}

impl MessageRow {
    pub fn update(&self, object: &MessageObject, window: &Window) {
        object.set_target_row(self.clone());
        let revealer = self.imp().message_revealer.get();

        let new_cursor = Cursor::builder().name("pointer").build();

        if object.is_send() {
            let sender = self.imp().sender.get();
            sender.set_cursor(Some(&new_cursor));
            sender.set_visible(true);
            self.imp().receiver_avatar_button.set_visible(false);
            self.imp().sent_by.set_xalign(1.0);
            self.imp().message.set_xalign(1.0);
            self.imp().message_content.add_css_class("message-row-sent");
            self.imp().placeholder.set_visible(true);
            revealer.set_transition_type(RevealerTransitionType::SlideLeft);
        } else {
            let receiver = self.imp().receiver.get();
            receiver.set_cursor(Some(&new_cursor));
            receiver.set_visible(true);
            self.imp().sender_avatar_button.set_visible(false);
            self.imp().sent_by.set_xalign(0.0);
            self.imp().message.set_xalign(0.0);
            self.imp()
                .message_content
                .add_css_class("message-row-received");
            revealer.set_transition_type(RevealerTransitionType::SlideRight)
        }

        self.imp().message_data.replace(Some(object.clone()));

        self.bind();
        self.connect_button_signals(window);

        // The transition must start after it gets added to the ListBox thus a small timer
        timeout_add_local_once(Duration::from_millis(50), move || {
            revealer.set_reveal_child(true);
        });
    }

    pub fn new_empty() -> Self {
        Object::builder().build()
    }

    pub fn stop_signals(&self) {
        for binding in self.imp().bindings.take() {
            binding.unbind();
        }
    }

    fn bind(&self) {
        let mut bindings = self.imp().bindings.borrow_mut();

        let sent_by = self.imp().sent_by.get();
        let message = self.imp().message.get();
        let message_time = self.imp().message_time.get();
        let spinner = self.imp().processing_spinner.get();
        let message_object = self.imp().message_data.borrow().clone().unwrap();
        let is_sent = message_object.is_send();
        let sender = message_object.sent_from();

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

        let image_binding = sender
            .bind_property("small-image", &sender_avatar, "custom-image")
            .sync_create()
            .build();

        let message_binding = message_object
            .bind_property("message", &message, "label")
            .sync_create()
            .build();

        let spinner_visible_binding = message_object
            .bind_property("must-process", &spinner, "visible")
            .sync_create()
            .build();

        let spinner_spinning_binding = message_object
            .bind_property("must-process", &spinner, "spinning")
            .sync_create()
            .build();

        let message_timing_binding = message_object
            .bind_property("message-timing", &message_time, "label")
            .sync_create()
            .build();

        bindings.push(sent_by_binding);
        bindings.push(avatar_fallback_binding);
        bindings.push(image_binding);
        bindings.push(message_binding);
        bindings.push(spinner_visible_binding);
        bindings.push(spinner_spinning_binding);
        bindings.push(message_timing_binding);
    }

    fn connect_button_signals(&self, window: &Window) {
        let sender_button = self.imp().sender_avatar_button.get();
        let receiver_button = self.imp().receiver_avatar_button.get();

        let sent_from = self
            .imp()
            .message_data
            .borrow()
            .clone()
            .unwrap()
            .sent_from();

        sender_button.connect_clicked(clone!(@weak window, @weak sent_from => move |_| {
            UserProfile::new(sent_from, &window);
        }));

        receiver_button.connect_clicked(clone!(@weak window, @weak sent_from => move |_| {
            UserProfile::new(sent_from, &window);
        }));

        let gesture = GestureClick::new();
        gesture.set_button(3);
        self.imp().message_content.add_controller(gesture.clone());

        gesture.connect_pressed(
            clone!(@weak self as row => move |_, _, x_position, y_position|{
                let popover = row.imp().message_menu.get();
                let position = Rectangle::new(x_position as i32, y_position as i32 + 10, -1, -1);
                popover.set_pointing_to(Some(&position));
                popover.set_visible(true);
            }),
        );
    }

    fn delete_message(&self) {
        info!("Deleting a message from the UI");
        let message_data = self.imp().message_data.borrow().clone().unwrap();

        let other_user =
            if message_data.sent_from().user_id() == message_data.sent_from().owner_id() {
                message_data.sent_to()
            } else {
                message_data.sent_from()
            };

        let message_number = message_data.message_number();

        other_user.add_to_queue(RequestType::DeleteMessage(
            other_user.user_id(),
            message_number,
        ));
        self.imp()
            .message_data
            .borrow()
            .clone()
            .unwrap()
            .to_process(true);
    }

    fn copy_message(&self) {
        info!("Copying message text to clipboard");
        let text = self.imp().message_data.borrow().clone().unwrap().message();
        self.clipboard().set(&text);
    }
}
