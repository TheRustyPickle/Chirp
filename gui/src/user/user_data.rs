mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gio::glib::once_cell::sync::Lazy;
    use gio::glib::subclass::Signal;
    use gio::ListStore;
    use glib::{derived_properties, object_subclass, Properties};
    use gtk::gdk::Paintable;
    use gtk::glib;
    use std::cell::{OnceCell, RefCell};

    use crate::ws::WSObject;

    use super::UserData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::UserObject)]
    pub struct UserObject {
        #[property(name = "image", get, set, type = Option<Paintable>, member = image)]
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "name-color", get, set, type = String, member = name_color)]
        #[property(name = "image-link", get, set, type = String, member = image_link)]
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
    impl ObjectImpl for UserObject {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("updating-image")
                    .param_types([Paintable::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }
    }
}

use adw::prelude::*;
use gio::glib::{clone, closure_local, MainContext, Priority, Receiver, Sender};
use gio::{spawn_blocking, ListStore};
use glib::{Bytes, ControlFlow, Object};
use gtk::gdk::{pixbuf_get_from_texture, Paintable, Texture};
use gtk::{glib, Image};
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

        if image_link.is_some() {
            info!("Starting channel to update image");
            let (sender, receiver) = MainContext::channel(Priority::default());
            obj.set_user_image(receiver);
            spawn_blocking(move || {
                let avatar = get_avatar(image_link.unwrap());
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
                    let status = paintable.to_value().get::<Paintable>().unwrap();
                    user_object.emit_by_name::<()>("updating-image", &[&status]);
                    info!("Emitted image update for {}", user_object.name());
                    ControlFlow::Continue
                }
            ),
        );
    }

    pub fn handle_ws(&self) -> Receiver<String> {
        let (sender, receiver) = MainContext::channel(Priority::DEFAULT);
        let user_object = self.clone();
        let user_ws = self.user_ws();
        user_ws.connect_closure(
            "ws-success",
            false,
            closure_local!(move |_from: WSObject, success: bool| {
                if success {
                    user_object.start_listening(sender.clone());
                }
            }),
        );

        receiver
    }

    fn start_listening(&self, sender: Sender<String>) {
        let user_ws = self.user_ws().ws_conn().unwrap();

        let id =
            user_ws.connect_message(clone!(@weak self as user_object => move |_ws, _s, bytes| {
                let byte_slice = bytes.to_vec();
                let text = String::from_utf8(byte_slice).unwrap();
                info!("Received from WS: {text}");
                sender.send(text).unwrap();

            }));

        self.user_ws().set_id(id);
    }
}

#[derive(Default, Clone)]
pub struct UserData {
    pub name: String,
    pub name_color: String,
    pub image: Option<Paintable>,
    pub image_link: Option<String>,
}
