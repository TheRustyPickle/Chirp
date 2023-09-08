mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gio::ListStore;
    use glib::{derived_properties, object_subclass, Properties};
    use gtk::gdk::Paintable;
    use gtk::glib;
    use std::cell::{Cell, OnceCell, RefCell};

    use crate::ws::WSObject;

    use super::UserData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::UserObject)]
    pub struct UserObject {
        #[property(name = "image", get, set, type = Option<Paintable>, member = image)]
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "name-color", get, set, type = String, member = name_color)]
        #[property(name = "image-link", get, set, type = Option<String>, member = image_link)]
        pub data: RefCell<UserData>,
        #[property(get, set)]
        pub messages: OnceCell<ListStore>,
        #[property(get, set)]
        pub user_ws: OnceCell<WSObject>,
        #[property(get, set)]
        pub user_id: Cell<u64>,
    }

    #[object_subclass]
    impl ObjectSubclass for UserObject {
        const NAME: &'static str = "UserObject";
        type Type = super::UserObject;
    }

    #[derived_properties]
    impl ObjectImpl for UserObject {}
}

use adw::prelude::*;
use gio::glib::{clone, closure_local, MainContext, Priority, Receiver, Sender};
use gio::subclass::prelude::ObjectSubclassIsExt;
use gio::{spawn_blocking, ListStore};
use glib::{Bytes, ControlFlow, Object};
use gtk::gdk::{pixbuf_get_from_texture, Paintable, Texture};
use gtk::{glib, Image};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::utils::{get_avatar, get_random_color};
use crate::ws::WSObject;

glib::wrapper! {
    pub struct UserObject(ObjectSubclass<imp::UserObject>);
}

impl UserObject {
    pub fn new(
        name: &str,
        image_link: Option<String>,
        messages: ListStore,
        color_to_ignore: Option<&str>,
        user_ws: WSObject,
    ) -> Self {
        let random_color = get_random_color(color_to_ignore);

        let obj: UserObject = Object::builder()
            .property("name", name)
            .property("image-link", image_link.clone())
            .property("messages", messages)
            .property("name-color", random_color)
            .build();

        if let Some(image_link) = image_link {
            info!("Starting a new channel to update image");
            let (sender, receiver) = MainContext::channel(Priority::default());
            obj.set_user_image(receiver);
            spawn_blocking(move || {
                info!("image link: {:?}", image_link);
                let avatar = get_avatar(image_link);
                sender.send(avatar).unwrap();
            });
        }
        obj.set_user_id(0);
        obj.set_user_ws(user_ws);
        obj
    }

    pub fn new_with_id(
        id: u64,
        name: &str,
        image_link: Option<String>,
        messages: ListStore,
        color_to_ignore: Option<&str>,
        user_ws: WSObject,
    ) -> Self {
        let random_color = get_random_color(color_to_ignore);

        let obj: UserObject = Object::builder()
            .property("name", name)
            .property("image-link", image_link.clone())
            .property("messages", messages)
            .property("name-color", random_color)
            .build();

        obj.set_user_id(id);

        if let Some(image_link) = image_link {
            info!("Starting a new channel to update image");
            let (sender, receiver) = MainContext::channel(Priority::default());
            obj.set_user_image(receiver);
            spawn_blocking(move || {
                info!("image link: {:?}", image_link);
                let avatar = get_avatar(image_link);
                sender.send(avatar).unwrap();
            });
        }
        obj.set_user_ws(user_ws);
        obj
    }

    fn set_user_image(&self, receiver: Receiver<Bytes>) {
        receiver.attach(
            None,
            clone!(@weak self as user_object => @default-return ControlFlow::Break,
                move |image_data| {
                    let texture = Texture::from_bytes(&image_data).unwrap();
                    let pixbuf = pixbuf_get_from_texture(&texture).unwrap();
                    let image = Image::from_pixbuf(Some(&pixbuf));
                    image.set_width_request(pixbuf.width());
                    image.set_height_request(pixbuf.height());
                    image.set_pixel_size(pixbuf.width());
                    let paintable = image.paintable().unwrap();
                    user_object.set_image(paintable.clone());
                    ControlFlow::Break
                }
            ),
        );
    }

    pub fn handle_ws(&self, owner_id: u64) -> Receiver<String> {
        let (sender, receiver) = MainContext::channel(Priority::DEFAULT);
        let user_object = self.clone();
        let user_ws = self.user_ws();
        user_ws.connect_closure(
            "ws-success",
            false,
            closure_local!(move |_from: WSObject, success: bool| {
                if success {
                    user_object.start_listening(sender.clone(), owner_id);
                }
            }),
        );

        receiver
    }

    fn start_listening(&self, sender: Sender<String>, owner_id: u64) {
        let user_ws = self.user_ws();

        if self.imp().user_id.get() == 0 {
            let user_data = self.convert_to_json();
            user_ws.create_new_user(user_data);
        } else {
            user_ws.update_ids(self.user_id(), owner_id)
        }

        let id = user_ws.ws_conn().unwrap().connect_message(
            clone!(@weak self as user_object => move |_ws, _s, bytes| {
                let byte_slice = bytes.to_vec();
                let text = String::from_utf8(byte_slice).unwrap();
                info!("{} Received from WS: {text}", user_object.name());

                if text.starts_with("/update-user-id") {
                    let id: u64 = text.split(' ').collect::<Vec<&str>>()[1].parse().unwrap();
                    user_object.set_user_id(id);
                    return;

                } else if text.starts_with("/update-session-id") {
                    let id: u64 = text.split(' ').collect::<Vec<&str>>()[1].parse().unwrap();
                    user_object.user_ws().set_ws_id(id);
                    return;
                }
                sender.send(text).unwrap();
            }),
        );

        self.user_ws().set_signal_id(id);
    }

    fn convert_to_json(&self) -> String {
        let user_id = self.user_id();
        let user_data = FullUserData {
            id: user_id,
            name: self.name(),
            image_link: self.image_link(),
        };

        serde_json::to_string(&user_data).unwrap()
    }
}

#[derive(Default, Clone)]
pub struct UserData {
    pub name: String,
    pub name_color: String,
    pub image: Option<Paintable>,
    pub image_link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullUserData {
    pub id: u64,
    pub name: String,
    pub image_link: Option<String>,
}
