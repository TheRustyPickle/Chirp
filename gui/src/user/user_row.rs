mod imp {
    use adw::{subclass::prelude::*, Avatar};
    use glib::subclass::InitializingObject;
    use glib::{object_subclass, Binding};
    use gtk::{glib, Box, CompositeTemplate, Label, Popover, PopoverMenu, Revealer};
    use std::cell::{Cell, OnceCell, RefCell};

    use crate::user::UserObject;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/therustypickle/chirp/user_row.xml")]
    pub struct UserRow {
        #[template_child]
        pub user_revealer: TemplateChild<Revealer>,
        #[template_child]
        pub user_avatar: TemplateChild<Avatar>,
        #[template_child]
        pub user_popover: TemplateChild<Popover>,
        #[template_child]
        pub popover_label: TemplateChild<Label>,
        #[template_child]
        pub user_menu: TemplateChild<PopoverMenu>,
        pub popover_visible: Cell<bool>,
        pub bindings: RefCell<Vec<Binding>>,
        pub user_data: OnceCell<UserObject>,
    }

    #[object_subclass]
    impl ObjectSubclass for UserRow {
        const NAME: &'static str = "UserRow";
        type Type = super::UserRow;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("user-row.profile", None, move |row, _, _| {
                row.view_profile()
            });
            klass.install_action("user-row.delete", None, move |row, _, _| row.delete_user());
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UserRow {}

    impl WidgetImpl for UserRow {}

    impl BoxImpl for UserRow {}
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use gdk::{Cursor, Rectangle};
use glib::{clone, timeout_add_local_once, wrapper, Object};
use gtk::{
    gdk, glib, Accessible, Box, Buildable, ConstraintTarget, EventControllerMotion, GestureClick,
    Orientable, Widget,
};
use std::time::Duration;
use tracing::info;

use crate::user::{UserObject, UserProfile};
use crate::window::Window;

wrapper! {
    pub struct UserRow(ObjectSubclass<imp::UserRow>)
    @extends Box, Widget,
    @implements Accessible, Buildable, ConstraintTarget, Orientable;
}

impl UserRow {
    pub fn new(object: UserObject) -> Self {
        let row: UserRow = Object::builder().build();
        row.imp().popover_visible.set(false);

        let motion = EventControllerMotion::new();
        row.imp().user_avatar.get().add_controller(motion.clone());

        let new_cursor = Cursor::builder().name("pointer").build();
        row.imp().user_avatar.set_cursor(Some(&new_cursor));

        // NOTE couldn't use clone! here as gtk was giving me children left error on exit. couldn't find a solution
        let row_clone = row.clone();
        motion.connect_enter(move |_, _, _| {
            if !row_clone.imp().popover_visible.get() {
                let popover = row_clone.imp().user_popover.get();
                let position = row_clone
                    .compute_bounds(&row_clone.imp().user_avatar.get())
                    .unwrap();
                let popover_text = row_clone.imp().user_data.get().unwrap().name();

                let x_position = position.x() as i32 + 45;
                let y_position = position.y() as i32 + 25;

                let position = Rectangle::new(x_position, y_position, -1, -1);

                popover.set_pointing_to(Some(&position));
                row_clone.imp().popover_label.set_label(&popover_text);

                popover.set_visible(true);
                row_clone.imp().popover_visible.set(true);
            }
        });

        motion.connect_leave(clone!(@weak row => move |_| {
            if row.imp().popover_visible.get() {
                row.imp().user_popover.get().set_visible(false);
                row.imp().popover_visible.set(false);
            }
        }));

        let gesture = GestureClick::new();
        gesture.set_button(3);
        row.imp().user_avatar.add_controller(gesture.clone());

        gesture.connect_pressed(clone!(@weak row => move |_, _, x_position, y_position|{
            let popover = row.imp().user_menu.get();
            let position = Rectangle::new(x_position as i32, y_position as i32 + 10, -1, -1);
            popover.set_pointing_to(Some(&position));
            popover.set_visible(true);
        }));

        let is_owner = if object.user_id() == object.owner_id() {
            true
        } else {
            false
        };

        // Prevent delete button from working on owner row
        if is_owner {
            row.action_set_enabled("user-row.delete", false);
        }

        row.imp().user_data.set(object).unwrap();
        row.bind();

        // The transition must start after it gets added to the ListBox thus a small timer
        let revealer = row.imp().user_revealer.get();
        timeout_add_local_once(Duration::from_millis(50), move || {
            revealer.set_reveal_child(true);
        });

        row
    }

    pub fn bind(&self) {
        let mut bindings = self.imp().bindings.borrow_mut();
        let user_avatar = self.imp().user_avatar.get();

        let user_object = self.imp().user_data.get().unwrap();

        let avatar_text_binding = user_object
            .bind_property("name", &user_avatar, "text")
            .sync_create()
            .build();

        let avatar_image_binding = user_object
            .bind_property("small-image", &user_avatar, "custom-image")
            .sync_create()
            .build();
        bindings.push(avatar_image_binding);

        bindings.push(avatar_text_binding);
    }

    fn view_profile(&self) {
        info!("Opening user profile");
        let root = self.root().unwrap();
        let main_window = root.downcast_ref::<Window>().unwrap();
        let user_data = self.imp().user_data.get().unwrap();

        UserProfile::new(user_data.clone(), main_window);
    }

    fn delete_user(&self) {
        info!("Deleting a user row");
        let root = self.root().unwrap();
        let main_window = root.downcast_ref::<Window>().unwrap().clone();
        let user_data = self.imp().user_data.get().unwrap();
        let user_id = user_data.user_id();

        self.imp().user_revealer.set_reveal_child(false);
        timeout_add_local_once(Duration::from_millis(500), move || {
            main_window.delete_user(user_id)
        });
    }
}
