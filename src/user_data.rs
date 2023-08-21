mod imp {
    use std::cell::RefCell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gio::{
        glib::{
            once_cell::sync::{Lazy, OnceCell},
            subclass::Signal,
        },
        ListStore,
    };
    use glib::Properties;
    use gtk::gdk::Paintable;
    use gtk::glib;

    use super::UserData;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::UserObject)]
    pub struct UserObject {
        #[property(name = "image", get, set, type = Option<Paintable>, member = image)]
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "image-link", get, set, type = String, member = image_link)]
        pub data: RefCell<UserData>,
        #[property(get, set)]
        pub messages: OnceCell<ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UserObject {
        const NAME: &'static str = "UserObject";
        type Type = super::UserObject;
    }

    #[glib::derived_properties]
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
use gio::glib::clone;
use gio::glib::MainContext;
use gio::glib::Priority;
use gio::glib::Receiver;
use gio::ListStore;
use glib::Bytes;
use glib::Object;
use gtk::gdk::Paintable;
use gtk::{gdk, glib, Image};

use crate::utils::get_avatar;

glib::wrapper! {
    pub struct UserObject(ObjectSubclass<imp::UserObject>);
}

impl UserObject {
    pub fn new(name: &str, image_link: Option<String>, messages: ListStore) -> Self {
        let obj: UserObject = Object::builder()
            .property("name", name)
            .property("image-link", image_link.clone())
            .property("messages", messages)
            .build();

        /*if image_link.is_some() {
            let (sender, receiver) = MainContext::channel(Priority::default());
            obj.set_user_image(receiver);
            gio::spawn_blocking(move || {
                let avatar = get_avatar(image_link.unwrap());
                sender.send(avatar).unwrap();
            });
        }*/
        obj
    }

    fn set_user_image(&self, receiver: Receiver<Vec<u8>>) {
        receiver.attach(
            None,
            clone!(@weak self as user_object => @default-return glib::ControlFlow::Break,
                move |image_data| {
                    let pixbuf = gdk::Texture::from_bytes(&Bytes::from(&image_data)).unwrap();

                    let buf = gdk::pixbuf_get_from_texture(&pixbuf).unwrap();
                    let image = Image::from_pixbuf(Some(&buf));
                    image.set_width_request(buf.width());
                    image.set_height_request(buf.height());
                    image.set_pixel_size(buf.width());
                    let paintable = image.paintable().unwrap();
                    println!("{:?}", user_object.image());
                    user_object.set_image(paintable.clone());
                    let status = paintable.to_value().get::<Paintable>().unwrap();
                    user_object.emit_by_name::<()>("updating-image", &[&status]);
                    println!("{:?}", user_object.image());
                    println!("Emitted");
                    glib::ControlFlow::Continue
                }
            ),
        );
    }
}

#[derive(Default, Clone)]
pub struct UserData {
    pub name: String,
    pub image: Option<Paintable>,
    pub image_link: Option<String>,
}
