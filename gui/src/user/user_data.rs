mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gdk::Paintable;
    use gio::ListStore;
    use glib::{derived_properties, object_subclass, Properties};
    use gtk::{gdk, glib};
    use std::cell::{Cell, OnceCell, RefCell};

    use crate::ws::WSObject;

    use super::{RequestType, UserData};

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
        pub request_queue: RefCell<Vec<RequestType>>,
        #[property(get, set)]
        pub request_processing: Cell<bool>,
        #[property(get, set)]
        pub owner_id: Cell<u64>,
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
use gio::subclass::prelude::ObjectSubclassIsExt;
use gio::{spawn_blocking, ListStore};
use glib::{
    clone, closure_local, Bytes, ControlFlow, MainContext, Object, Priority, Receiver, Sender,
};
use gtk::{gdk, glib, Image};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::message::MessageObject;
use crate::utils::{generate_random_avatar_link, get_avatar, get_random_color};
use crate::window::Window;
use crate::ws::WSObject;

glib::wrapper! {
    pub struct UserObject(ObjectSubclass<imp::UserObject>);
}

impl UserObject {
    pub fn new(
        name: &str,
        image_link: Option<String>,
        color_to_ignore: Option<&str>,
        user_id: Option<u64>,
        owner_id: u64,
    ) -> Self {
        let ws = WSObject::new();
        let messages = ListStore::new::<MessageObject>();
        let random_color = get_random_color(color_to_ignore);

        let id = if let Some(id) = user_id { id } else { 0 };

        let obj: UserObject = Object::builder()
            .property("user-id", id)
            .property("name", name)
            .property("image-link", image_link.clone())
            .property("messages", messages)
            .property("name-color", random_color)
            .property("owner-id", owner_id)
            .build();

        obj.check_image_link();
        obj.set_user_ws(ws);
        let user_object = obj.clone();
        obj.user_ws().connect_closure(
            "ws-reconnect",
            false,
            closure_local!(move |_from: WSObject, _success: bool| {
                info!("Reconnected");
                let old_queue = user_object.imp().request_queue.borrow().clone();
                user_object.imp().request_queue.replace(Vec::new());
                user_object
                    .add_to_queue(RequestType::ReconnectUser)
                    .add_to_queue(RequestType::UpdateIDs);

                for old in old_queue {
                    user_object.add_to_queue(old);
                }
            }),
        );
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

    pub fn add_to_queue(&self, request_type: RequestType) -> &Self {
        {
            let mut queue = self.imp().request_queue.borrow_mut();
            queue.push(request_type);
        }

        if !self.request_processing() {
            self.process_queue();
        };
        self
    }

    fn process_queue(&self) {
        self.set_request_processing(true);

        let user_ws = self.user_ws();

        let mut queue_list = self.imp().request_queue.borrow_mut();

        let mut highest_index = 0;
        for task in queue_list.iter() {
            if user_ws.ws_conn().is_some() {
                info!("starting processing {task:?}");
                match task {
                    RequestType::ReconnectUser => {
                        let owner_id = self.owner_id();
                        let user_data = self.convert_to_json();
                        user_ws.reconnect_user(owner_id, user_data);
                    }
                    RequestType::UpdateIDs => user_ws.update_ids(self.user_id(), self.owner_id()),
                    RequestType::CreateNewUser => {
                        let user_data = self.convert_to_json();
                        user_ws.create_new_user(user_data);
                    }
                    RequestType::SendMessage(msg) => {
                        user_ws.send_text_message(&msg.convert_to_json())
                    }
                    RequestType::ImageUpdated(link) => user_ws.image_link_updated(link),
                    RequestType::NameUpdated(name) => user_ws.name_updated(name),
                    RequestType::GetUserData(id) => user_ws.get_user_data(id),
                    RequestType::UpdateChattingWith(id) => user_ws.update_chatting_with(id),
                }
                highest_index += 1;
            } else {
                info!("Connection lost. Stopping processing request");
                break;
            }
        }

        for _x in 0..highest_index {
            info!("Removing {:?}", queue_list[0]);
            queue_list.remove(0);
        }

        self.set_request_processing(false);
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
        self.add_to_queue(RequestType::ImageUpdated(new_link.to_owned()));
        self.set_new_image_link(new_link);
    }

    pub fn handle_ws(&self, window: Window) {
        let user_object = self.clone();

        let user_ws = self.user_ws();
        user_ws.connect_closure(
            "ws-success",
            false,
            closure_local!(move |_from: WSObject, _success: bool| {
                let (sender, receiver) = MainContext::channel(Priority::DEFAULT);
                user_object.start_listening(sender.clone());
                window.handle_ws_message(&user_object, receiver);
            }),
        );
    }

    fn start_listening(&self, sender: Sender<String>) {
        let user_ws = self.user_ws();

        if !user_ws.reconnecting() {
            if self.user_id() == 0 {
                self.add_to_queue(RequestType::CreateNewUser);
            } else {
                self.add_to_queue(RequestType::UpdateIDs);
            }
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

                            if user_object.owner_id() == 0 {
                                user_object.set_owner_id(id);
                            }
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

#[derive(Debug, Clone)]
pub enum RequestType {
    CreateNewUser,
    NameUpdated(String),
    ImageUpdated(String),
    ReconnectUser,
    UpdateIDs,
    SendMessage(MessageData),
    UpdateChattingWith(u64),
    GetUserData(u64),
}

#[derive(Debug, Serialize, Clone)]
pub struct MessageData {
    pub from_user: u64,
    pub to_user: u64,
    pub msg: String,
}

impl MessageData {
    pub fn new(from_user: u64, to_user: u64, msg: String) -> Self {
        MessageData {
            msg,
            from_user,
            to_user,
        }
    }
    fn convert_to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
