mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gio::glib::once_cell::sync::Lazy;
    use gio::glib::subclass::Signal;
    use gio::glib::SignalHandlerId;
    use glib::{derived_properties, object_subclass, Properties};
    use gtk::glib;
    use soup::WebsocketConnection;
    use std::cell::{OnceCell, RefCell};
    use std::rc::Rc;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::WSObject)]
    pub struct WSObject {
        #[property(get, set)]
        pub ws_conn: Rc<RefCell<Option<WebsocketConnection>>>,
        #[property(get, set)]
        pub ws_id: OnceCell<u64>,
        pub ws_signal_id: RefCell<Option<SignalHandlerId>>,
    }

    #[object_subclass]
    impl ObjectSubclass for WSObject {
        const NAME: &'static str = "WSObject";
        type Type = super::WSObject;
    }

    #[derived_properties]
    impl ObjectImpl for WSObject {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("ws-success")
                    .param_types([bool::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }
    }
}

use adw::subclass::prelude::*;
use gio::glib::{clone, MainContext, Priority, Receiver, SignalHandlerId};
use gio::Cancellable;
use glib::wrapper;
use glib::{ControlFlow, Object};
use gtk::glib;
use gtk::prelude::*;
use soup::prelude::*;
use soup::{Message, Session, WebsocketConnection};
use tracing::{error, info};

wrapper! {
    pub struct WSObject(ObjectSubclass<imp::WSObject>);
}

impl WSObject {
    pub fn new() -> Self {
        let obj: WSObject = Object::builder().build();
        obj.set_ws();
        obj
    }

    pub fn get_ws_receiver(&self) -> Receiver<Option<WebsocketConnection>> {
        let session = Session::new();

        let websocket_url = "ws://127.0.0.1:8080/ws/";

        let (sender, receiver) = MainContext::channel(Priority::DEFAULT);
        let message = Message::new("GET", websocket_url).unwrap();
        let cancel = Cancellable::new();

        info!("Starting websocket connection with {}", websocket_url);
        session.websocket_connect_async(
            &message,
            None,
            &[],
            Priority::default(),
            Some(&cancel),
            move |result| match result {
                Ok(connection) => {
                    sender.send(Some(connection)).unwrap();
                }
                Err(error) => {
                    sender.send(None).unwrap();
                    error!("WebSocket connection error: {:?}", error);
                }
            },
        );
        receiver
    }

    fn set_ws(&self) {
        let receiver = self.get_ws_receiver();

        receiver.attach(
            None,
            clone!(@weak self as ws_object => @default-return ControlFlow::Break, move |conn| {
                if conn.is_some() {
                    ws_object.set_ws_conn(conn.unwrap());
                    info!("WebSocket connection success");
                    ws_object.emit_by_name::<()>("ws-success", &[&true]);
                } else {
                    info!("WebSocket connection failed");
                    ws_object.emit_by_name::<()>("ws-success", &[&false]);
                }
                ControlFlow::Continue
            }),
        );
    }

    pub fn update_chatting_with(&self, id: u64) {
        if let Some(conn) = self.ws_conn() {
            info!("Sending request for updating chatting with id {}", id);
            conn.send_text(&format!("/update-chatting-with {}", id))
        }
    }

    pub fn get_user_data(&self, id: u64) {
        if let Some(conn) = self.ws_conn() {
            info!(
                "Sending request for getting UserObject Data with id: {}",
                id
            );
            conn.send_text(&format!("/get-user-data {}", id))
        }
    }

    pub fn update_user_data(&self, data: String) {
        if let Some(conn) = self.ws_conn() {
            info!("Updating ws with UserObject Data: {}", data);
            conn.send_text(&format!("/update-user-data {}", data))
        }
    }

    pub fn set_signal_id(&self, id: SignalHandlerId) {
        self.imp().ws_signal_id.replace(Some(id));
    }
}
