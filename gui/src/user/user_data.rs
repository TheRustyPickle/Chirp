mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gdk::Paintable;
    use gio::ListStore;
    use glib::{derived_properties, object_subclass, Properties};
    use gtk::{gdk, glib};
    use std::cell::{OnceCell, RefCell};

    use crate::ws::WSObject;

    use super::UserData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::UserObject)]
    pub struct UserObject {
        #[property(name = "user-id", get, set, type = u64, member = user_id)]
        #[property(name = "big-image", get, set, type = Option<Paintable>, member = big_image)]
        #[property(name = "small-image", get, set, type = Option<Paintable>, member = small_image)]
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "name-color", get, set, type = String, member = name_color)]
        #[property(name = "image-link", get, set, type = Option<String>, member = image_link)]
        pub data: RefCell<UserData>,
        #[property(get, set)]
        pub messages: OnceCell<ListStore>,
        #[property(get, set)]
        pub user_ws: OnceCell<WSObject>,
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
use gdk::{gdk_pixbuf, Paintable, Texture};
use gdk_pixbuf::InterpType;
use gio::{spawn_blocking, ListStore};
use glib::{
    clone, closure_local, Bytes, ControlFlow, MainContext, Object, Priority, Receiver, Sender,
};
use gtk::{gdk, glib, Image};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::utils::{generate_random_avatar_link, get_avatar, get_random_color};
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
        user_id: Option<u64>,
    ) -> Self {
        let random_color = get_random_color(color_to_ignore);

        let id = if let Some(id) = user_id { id } else { 0 };

        let obj: UserObject = Object::builder()
            .property("user-id", id)
            .property("name", name)
            .property("image-link", image_link.clone())
            .property("messages", messages)
            .property("name-color", random_color)
            .build();

        obj.check_image_link();
        obj.set_user_ws(user_ws);
        obj
    }

    // TODO: Pass a result instead of Bytes directly
    fn check_image_link(&self) {
        if let Some(image_link) = self.image_link() {
            info!("Starting a new channel to update image");
            let (sender, receiver) = MainContext::channel(Priority::default());
            self.set_user_image(receiver);
            spawn_blocking(move || {
                info!("Image link: {:?}", image_link);
                let avatar = get_avatar(image_link);
                sender.send(avatar).unwrap();
            });
        }
    }

    // TODO: Verify image link
    #[allow(deprecated)]
    fn set_user_image(&self, receiver: Receiver<Bytes>) {
        receiver.attach(
            None,
            clone!(@weak self as user_object => @default-return ControlFlow::Break,
                move |image_data| {
                    let texture = Texture::from_bytes(&image_data).unwrap();
                    let pixbuf = gdk::pixbuf_get_from_texture(&texture).unwrap();

                    let big_image_buf = pixbuf.scale_simple(150, 150, InterpType::Hyper).unwrap();
                    let small_image_buf = pixbuf.scale_simple(45, 45, InterpType::Hyper).unwrap();

                    let big_image = Image::from_pixbuf(Some(&big_image_buf));
                    let small_image = Image::from_pixbuf(Some(&small_image_buf));

                    let paintable = big_image.paintable().unwrap();
                    user_object.set_big_image(paintable);

                    let paintable = small_image.paintable().unwrap();
                    user_object.set_small_image(paintable);
                    ControlFlow::Break
                }
            ),
        );
    }

    pub fn set_new_name(&self, name: String) {
        self.set_name(name);
    }

    pub fn set_new_image_link(&self, link: String) {
        self.set_image_link(link);
        self.check_image_link()
    }

    pub fn set_random_image(&self) {
        let new_link = generate_random_avatar_link();
        info!("Generated random image link: {}", new_link);
        self.user_ws().image_link_updated(&new_link);
        self.set_new_image_link(new_link);
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

        if self.user_id() == 0 {
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

                if text.starts_with('/') {
                    let splitted_data: Vec<&str> = text.splitn(2, ' ').collect();
                    match splitted_data[0] {
                        "/update-user-id" => {
                            let id: u64 = splitted_data[1].parse().unwrap();
                            user_object.set_user_id(id);
                        }
                        "/update-session-id" => {
                            let id: u64 = splitted_data[1].parse().unwrap();
                            user_object.user_ws().set_ws_id(id);
                        }
                        "/image-updated" => {
                            user_object.set_image_link(splitted_data[1]);
                            user_object.check_image_link();
                        },
                        "/name-updated" => user_object.set_name(splitted_data[1]),
                        "/message" | "/get-user-data"=> sender.send(text).unwrap(),
                        _ => {}
                    }
                }
            }),
        );

        self.user_ws().set_signal_id(id);
    }

    fn convert_to_json(&self) -> String {
        let user_data = FullUserData {
            id: self.user_id(),
            name: self.name(),
            image_link: self.image_link(),
        };

        serde_json::to_string(&user_data).unwrap()
    }
}

#[derive(Default, Clone)]
pub struct UserData {
    pub user_id: u64,
    pub name: String,
    pub name_color: String,
    pub big_image: Option<Paintable>,
    pub small_image: Option<Paintable>,
    pub image_link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullUserData {
    pub id: u64,
    pub name: String,
    pub image_link: Option<String>,
}
