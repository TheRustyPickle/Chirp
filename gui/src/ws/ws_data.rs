mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use glib::once_cell::sync::Lazy;
    use glib::subclass::Signal;
    use glib::{derived_properties, object_subclass, Properties, Sender, SignalHandlerId};
    use gtk::glib;
    use soup::WebsocketConnection;
    use std::cell::{Cell, OnceCell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::WSObject)]
    pub struct WSObject {
        #[property(get, set, nullable)]
        pub ws_conn: RefCell<Option<WebsocketConnection>>,
        pub ws_signal_id: RefCell<Option<SignalHandlerId>>,
        pub conn_close_signal_id: RefCell<Option<SignalHandlerId>>,
        pub ws_sender: OnceCell<Sender<Option<WebsocketConnection>>>,
        pub notifier: OnceCell<Sender<bool>>,
        #[property(get, set)]
        pub is_reconnecting: Cell<bool>,
        #[property(get, set)]
        pub reconnecting_timer: Cell<u32>,
        #[property(get, set)]
        pub last_timer: Cell<u32>,
        #[property(get, set)]
        pub manually_reloaded: Cell<bool>,
        #[property(get, set)]
        pub stop_processing: Cell<bool>,
        pub signal_ids: RefCell<Vec<SignalHandlerId>>,
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
                vec![
                    Signal::builder("ws-success")
                        .param_types([bool::static_type()])
                        .build(),
                    Signal::builder("ws-reconnect")
                        .param_types([bool::static_type()])
                        .build(),
                    Signal::builder("stop-processing")
                        .param_types([bool::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
}

use adw::subclass::prelude::*;
use gio::Cancellable;
use glib::{
    clone, closure_local, timeout_add_seconds_local, wrapper, ControlFlow, MainContext, Object,
    Priority, SignalHandlerId,
};
use gtk::glib;
use gtk::prelude::*;
use soup::prelude::*;
use soup::{Message, Session, WebsocketConnection};
use std::env;
use tracing::{debug, error, info};

wrapper! {
    pub struct WSObject(ObjectSubclass<imp::WSObject>);
}

impl WSObject {
    pub fn new() -> Self {
        let obj: WSObject = Object::builder().build();
        obj.set_last_timer(10);
        obj.set_ws();
        obj.set_stop_processing(false);
        obj
    }

    pub fn stop_signals(&self) {
        for signal in self.imp().signal_ids.take() {
            self.disconnect(signal);
            debug!("A signal in WSObject was disconnected");
        }

        if let Some(conn) = self.ws_conn() {
            if let Some(signal) = self.imp().conn_close_signal_id.take() {
                conn.disconnect(signal);
                debug!("Connection closed signal in WSObject was disconnected");
            }
            if let Some(signal) = self.imp().ws_signal_id.take() {
                conn.disconnect(signal);
                debug!("Connection message signal in WSObject was disconnected");
            }
            self.set_ws_conn(None::<WebsocketConnection>);
        }
    }

    pub fn connect_to_ws(&self) {
        let session = Session::new();
        let sender = self.imp().ws_sender.get().unwrap().clone();

        let websocket_url = env::var("WEBSOCKET_URL").expect("WEBSOCKET_URL must be set");

        let message = Message::new("GET", &websocket_url).unwrap();

        message.connect_accept_certificate(move |_, _, _| true);

        let cancel = Cancellable::new();

        let is_reconnecting = self.is_reconnecting();
        let notifier = self.imp().notifier.get().unwrap().clone();

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
                    if is_reconnecting {
                        notifier.send(true).unwrap()
                    };
                }
                Err(error) => {
                    sender.send(None).unwrap();
                    debug!("WebSocket connection error: {:?}", error);
                }
            },
        );
    }

    fn set_ws(&self) {
        let (sender, receiver) = MainContext::channel(Priority::DEFAULT);
        let (notifier_send, notifier_receive) = MainContext::channel(Priority::DEFAULT);

        self.imp().ws_sender.set(sender).unwrap();
        self.imp().notifier.set(notifier_send).unwrap();

        self.set_is_reconnecting(false);
        self.connect_to_ws();

        // If ws connection failed, try to reconnect
        // otherwise save the websocket connection and ping it
        receiver.attach(
            None,
            clone!(@weak self as ws_object => @default-return ControlFlow::Break, move |conn| {
                if ws_object.stop_processing() {
                    debug!("Shutting down receiver 1");
                    return ControlFlow::Break
                }
                if conn.is_some() {
                    ws_object.set_ws_conn(Some(conn.unwrap()));
                    info!("WebSocket connection success");
                    ws_object.emit_by_name::<()>("ws-success", &[&true]);
                    ws_object.start_pinging();
                    ws_object.set_last_timer(10);
                } else {
                    let last_timer = ws_object.last_timer();

                    let new_timer = if last_timer > 300 {
                        last_timer
                    } else {
                        (last_timer as f32 * 1.5) as u32
                    };
                    ws_object.set_reconnecting_timer(new_timer);

                    error!("WebSocket connection failed. Starting reconnecting again in {} seconds", new_timer);
                    timeout_add_seconds_local(1, move || {
                        if ws_object.manually_reloaded() {
                            ws_object.set_manually_reloaded(false);
                            return ControlFlow::Break
                        }
                        if ws_object.reconnecting_timer() == 0 {
                            ws_object.set_last_timer(new_timer);
                            ws_object.connect_to_ws();
                            return ControlFlow::Break
                        } else {
                            ws_object.set_reconnecting_timer(ws_object.reconnecting_timer() - 1);
                        }
                        ControlFlow::Continue
                    });
                }
                ControlFlow::Continue
            }),
        );

        notifier_receive.attach(
            None,
            clone!(@weak self as ws => @default-return ControlFlow::Break, move |_| {
                if ws.stop_processing() {
                    debug!("Shutting down receiver 2");
                    return ControlFlow::Break
                }
                ws.emit_by_name::<()>("ws-reconnect", &[&true]);
                ControlFlow::Continue
            }),
        );

        let stop_processing_signal = self.connect_closure(
            "stop-processing",
            false,
            closure_local!(move |from: WSObject, stop: bool| {
                from.set_stop_processing(stop);
                if let Some(conn) = from.ws_conn() {
                    info!("Closing websocket connection");
                    conn.close(1000, None);
                }
                // Send a message via the sender so the receiver can shut down
                if let Some(sender) = from.imp().ws_sender.get() {
                    sender.send(None).unwrap();
                }
                if let Some(sender) = from.imp().notifier.get() {
                    sender.send(true).unwrap();
                }
                from.stop_signals();
            }),
        );
        self.imp()
            .signal_ids
            .borrow_mut()
            .push(stop_processing_signal);
    }

    /// Pings and follows if the connection was closed
    pub fn start_pinging(&self) {
        let conn = self.ws_conn().unwrap();
        conn.set_max_incoming_payload_size(0);
        conn.set_keepalive_interval(5);

        let conn_close_signal =
            conn.connect_closed(clone!(@weak self as ws, @weak conn => move |_| {
                info!("Connection closed. Starting again");
                ws.set_is_reconnecting(true);
                if let Some(id) = ws.imp().ws_signal_id.take() {
                    info!("disconnecting message connection");
                    conn.disconnect(id);
                };
                if let Some(id) = ws.imp().conn_close_signal_id.take() {
                    info!("disconnecting ping connection");
                    conn.disconnect(id);
                };
                ws.connect_to_ws();
                ws.set_ws_conn(None::<WebsocketConnection>);

            }));
        self.imp()
            .conn_close_signal_id
            .replace(Some(conn_close_signal));
    }

    /// Reload the connection without waiting for the timer to end
    pub fn reload_manually(&self) {
        self.set_manually_reloaded(true);
        let new_timer = if self.last_timer() > 300 {
            self.last_timer()
        } else {
            (self.last_timer() as f32 * 1.5) as u32
        };
        self.set_last_timer(new_timer);
        self.connect_to_ws();
    }

    /// Sends a message
    pub fn send_text_message(&self, message: &str) {
        info!("Sending request to WS to process message");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/message {}", message));
    }

    /// Calls the server to create a new user with the given data
    pub fn create_new_user(&self, user_data: String) {
        info!("Sending request to WS to create a new user");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/create-new-user {}", user_data));
    }

    /// Calls the server to get profile data of a user
    pub fn get_user_data(&self, data: &str) {
        info!("Sending request for getting UserObject Data");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/get-user-data {}", data))
    }

    /// Calls the server to update the user image link
    pub fn image_link_updated(&self, link: &str) {
        info!("Sending request to WS to update image link");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/image-updated {}", link))
    }

    /// Calls the server to update the user name
    pub fn name_updated(&self, name: &str) {
        info!("Sending request to WS update name");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/name-updated {}", name))
    }

    /// Connects to the WS to reconnect with previously server deleted user data
    pub fn reconnect_user(&self, id_data: String) {
        info!("Sending request to WS to reconnect");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/reconnect-user {}", id_data))
    }

    /// Calls the server to send last chat message number of a user
    pub fn selection_update(&self, data: String) {
        info!("Sending request to WS get the last message number");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/message-number {}", data))
    }

    pub fn sync_message(&self, data: String) {
        info!("Sending request to WS sync messages");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/sync-message {}", data))
    }

    pub fn delete_message(&self, data: String) {
        info!("Sending request to WS delete a message");
        self.ws_conn()
            .unwrap()
            .send_text(&format!("/delete-message {}", data))
    }

    /// Saves the signal ID of the Websocket Message Signal
    pub fn set_signal_id(&self, id: SignalHandlerId) {
        self.imp().ws_signal_id.replace(Some(id));
    }
}
