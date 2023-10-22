mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gdk::Paintable;
    use gio::ListStore;
    use glib::once_cell::sync::Lazy;
    use glib::subclass::Signal;
    use glib::{derived_properties, object_subclass, Properties, SignalHandlerId};
    use gtk::{gdk, glib};
    use std::cell::{Cell, OnceCell, RefCell};
    use std::sync::Mutex;

    use super::UserData;
    use crate::ws::RequestType;
    use crate::ws::WSObject;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::UserObject)]
    pub struct UserObject {
        #[property(name = "user-id", get, set, type = u64, member = user_id)]
        #[property(name = "big-image", get, set, nullable, type = Option<Paintable>, member = big_image)]
        #[property(name = "small-image", get, set, nullable, type = Option<Paintable>, member = small_image)]
        #[property(name = "name", get, set, type = String, member = name)]
        #[property(name = "name-color", get, set, type = String, member = name_color)]
        #[property(name = "image-link", get, set, nullable, type = Option<String>, member = image_link)]
        pub data: RefCell<UserData>,
        #[property(get, set)]
        pub messages: OnceCell<ListStore>,
        #[property(get, set)]
        pub user_ws: OnceCell<WSObject>,
        pub request_queue: Mutex<RefCell<Vec<RequestType>>>,
        #[property(get, set)]
        pub request_processing: Cell<bool>,
        #[property(get, set)]
        pub owner_id: Cell<u64>,
        #[property(get, set)]
        pub user_token: OnceCell<String>,
        #[property(get, set)]
        pub message_number: Cell<u64>,
        pub signal_ids: RefCell<Vec<SignalHandlerId>>,
    }

    #[object_subclass]
    impl ObjectSubclass for UserObject {
        const NAME: &'static str = "UserObject";
        type Type = super::UserObject;
    }

    #[derived_properties]
    impl ObjectImpl for UserObject {
        fn signals() -> &'static [Signal] {
            // Gets emitted when updating image to open a Toast on profile page
            // Empty string => Success
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("image-modified")
                        .param_types([String::static_type(), String::static_type()])
                        .build(),
                    Signal::builder("user-exists")
                        .param_types([bool::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
}

use adw::prelude::*;
use gdk::{gdk_pixbuf, Paintable, Texture};
use gdk_pixbuf::{InterpType, PixbufLoader};
use gio::subclass::prelude::ObjectSubclassIsExt;
use gio::{spawn_blocking, ListStore};
use glib::{
    clone, closure_local, timeout_add_local_once, Bytes, ControlFlow, MainContext, Object,
    Priority, Receiver, Sender,
};
use gtk::{gdk, glib};
use std::time::Duration;
use tracing::{debug, info};

use crate::message::MessageObject;
use crate::utils::{generate_random_avatar_link, get_avatar, get_random_color};
use crate::window::Window;
use crate::ws::{
    DeleteMessage, FullUserData, ImageUpdate, MessageSyncData, MessageSyncRequest, NameUpdate,
    RequestType, UserIDs, WSObject,
};

glib::wrapper! {
    pub struct UserObject(ObjectSubclass<imp::UserObject>);
}

impl UserObject {
    pub fn new(
        name: &str,
        image_link: Option<String>,
        color_to_ignore: Option<&str>,
        user_id: Option<u64>,
        user_token: Option<String>,
    ) -> Self {
        let messages = ListStore::new::<MessageObject>();
        let random_color = get_random_color(color_to_ignore);
        let id = if let Some(id) = user_id { id } else { 0 };

        let obj: UserObject = Object::builder()
            .property("user-id", id)
            .property("name", name)
            .property("image-link", image_link.to_owned())
            .property("messages", messages)
            .property("name-color", random_color)
            .build();

        // Will only be some in case of owner object and with some data saved
        if let Some(token) = user_token {
            obj.set_user_token(token);
            obj.set_owner_id(id);
        }

        let ws = WSObject::new();
        obj.check_image_link(image_link, false);

        obj.set_user_ws(ws);
        obj.set_message_number(0);
        obj.start_signals();

        obj
    }

    fn start_signals(&self) {
        let user_object = self.clone();
        // This signal gets emitted when the connection is once lost but reconnected again
        let websocket_signal_id = self.user_ws().connect_closure(
            "ws-reconnect",
            false,
            closure_local!(move |_from: WSObject, _success: bool| {
                // Until reconnection success is received, all queue process is stopped
                user_object.add_queue_to_first(RequestType::ReconnectUser);
            }),
        );

        // Is only processed when the image update needs to be processed by the websocket
        // Not processed when image link is received from the websocket
        let image_signal_id = self.connect_closure(
            "image-modified",
            false,
            closure_local!(
                move |from: UserObject, error_message: String, image_link: String| {
                    if error_message.is_empty() && !image_link.is_empty() {
                        let image_data = ImageUpdate::new_json(Some(image_link), from.user_token());
                        from.user_ws().image_link_updated(&image_data);
                    }
                }
            ),
        );

        let mut signal_ids = self.imp().signal_ids.borrow_mut();
        self.user_ws()
            .imp()
            .signal_ids
            .borrow_mut()
            .push(websocket_signal_id);
        signal_ids.push(image_signal_id);
    }

    pub fn stop_signals(&self) {
        for signal in self.imp().signal_ids.take() {
            self.disconnect(signal);
            debug!("A signal in UserObject was disconnected");
        }
    }

    pub fn check_image_link(&self, new_link: Option<String>, from_queue: bool) {
        info!("Starting checking image link {:?}", new_link);
        if let Some(link) = new_link {
            let (sender, receiver) = MainContext::channel(Priority::default());
            self.set_user_image(receiver, from_queue);
            spawn_blocking(move || {
                let avatar = get_avatar(link);
                sender.send(avatar).unwrap();
            });
        } else {
            self.remove_image()
        }
    }

    fn set_user_image(
        &self,
        receiver: Receiver<Result<(String, Bytes), String>>,
        from_queue: bool,
    ) {
        let working_link = self.image_link();
        receiver.attach(
            None,
            clone!(@weak self as user_object => @default-return ControlFlow::Break,
                move |image_result| {
                    match image_result {
                        Ok((image_link, image_data)) => {
                            let pixbuf_loader = PixbufLoader::new();
                            if let Err(_) = pixbuf_loader.write(&image_data) {
                                user_object.emit_by_name::<()>("image-modified", &[&String::from("Failed to create the image"), &image_link]);
                                return ControlFlow::Break
                            };

                            if let Err(_) = pixbuf_loader.close() {
                                user_object.emit_by_name::<()>("image-modified", &[&String::from("Failed to create the image"), &image_link]);
                                return ControlFlow::Break
                            };

                            let pixbuf = if let Some(data) = pixbuf_loader.pixbuf() {
                                data
                            } else {
                                user_object.emit_by_name::<()>("image-modified", &[&String::from("Failed to create the image"), &image_link]);
                                return ControlFlow::Break
                            };
                            // Gtk handles some scaling by itself but the quality is terrible for smaller size.
                            // Manually scaling it retains some quality
                            let big_image_buf = pixbuf.scale_simple(150, 150, InterpType::Hyper).unwrap();
                            let small_image_buf = pixbuf.scale_simple(45, 45, InterpType::Hyper).unwrap();

                            let big_paintable = Paintable::from(Texture::for_pixbuf(&big_image_buf));
                            let small_paintable = Paintable::from(Texture::for_pixbuf(&small_image_buf));

                            // If not same means while the image was being fetched
                            // it was updated again so changing them would show inaccurate
                            // current image than what's on the server
                            if user_object.image_link() == working_link {
                                user_object.set_big_image(Some(big_paintable));
                                user_object.set_small_image(Some(small_paintable));
                                user_object.set_image_link(Some(image_link.to_owned()));
                            } else {
                                info!("Image link was updated. Abandoning {}", image_link)
                            }
                            if !from_queue {
                                user_object.emit_by_name::<()>("image-modified", &[&String::new(), &String::new()]);
                            } else {
                                user_object.emit_by_name::<()>("image-modified", &[&String::new(), &image_link]);
                            }
                        }
                        Err(msg) => {
                            // Emit the signal to the user profile to show a toast with the error
                            user_object.emit_by_name::<()>("image-modified", &[&msg, &String::new()]);
                        }
                    }
                    ControlFlow::Break
                }
            ),
        );
    }

    /// Adds stuff to queue and start the process to process them
    pub fn add_to_queue(&self, request_type: RequestType) -> &Self {
        debug!("Adding to queue: {:#?}", request_type);
        {
            let locked_queue = self.imp().request_queue.lock().unwrap();
            let mut queue = locked_queue.borrow_mut();
            queue.push(request_type);
        }

        // The process must not start twice otherwise the same
        // request can get processed twice, creating disaster
        if !self.request_processing() {
            self.process_queue(None);
        };
        self
    }

    /// Adds stuff to queue at the index 0 and call to process only the first queue request
    pub fn add_queue_to_first(&self, request_type: RequestType) {
        debug!("Adding to queue: {:#?}", request_type);
        {
            let locked_queue = self.imp().request_queue.lock().unwrap();
            let mut queue = locked_queue.borrow_mut();
            queue.insert(0, request_type);
        }

        self.process_queue(Some(1));
    }

    /// Processes queued stuff if websocket is available
    fn process_queue(&self, process_limit: Option<u64>) {
        self.set_request_processing(true);

        let user_ws = self.user_ws();

        let queue_list = { self.imp().request_queue.lock().unwrap().borrow().clone() };

        let mut highest_index: u64 = 0;
        let mut connection_lost = false;

        for task in queue_list {
            if user_ws.ws_conn().is_some() {
                debug!("starting processing {task:#?}");
                match task {
                    RequestType::ReconnectUser => {
                        let id_data = UserIDs::new_json(self.user_id(), self.user_token());
                        user_ws.reconnect_user(id_data);
                    }
                    RequestType::CreateNewUser => {
                        let user_data = FullUserData::new(self).to_json();
                        user_ws.create_new_user(user_data);
                    }
                    RequestType::SendMessage(message_data, msg_obj) => {
                        self.set_message_number(self.message_number() + 1);
                        msg_obj.set_message_number(self.message_number());

                        let data = message_data
                            .update_token(self.user_token())
                            .update_message_number(self.message_number())
                            .to_json();
                        user_ws.send_text_message(&data);
                        msg_obj.to_process(false);
                    }
                    RequestType::ImageUpdated(link) => {
                        self.check_image_link(link.to_owned(), true);
                        if link.is_none() {
                            let image_data = ImageUpdate::new_json(link, self.user_token());
                            user_ws.image_link_updated(&image_data);
                        }
                    }
                    RequestType::NameUpdated(name) => {
                        self.set_name(name.to_owned());
                        let name_data = NameUpdate::new_json(name.to_string(), self.user_token());
                        user_ws.name_updated(&name_data)
                    }
                    RequestType::GetUserData(id) => {
                        let user_data = UserIDs::new_json(id.to_owned(), self.user_token());
                        user_ws.get_user_data(&user_data)
                    }
                    RequestType::GetLastMessageNumber(user) => {
                        let data = UserIDs::new_json(user.user_id(), self.user_token());
                        user_ws.selection_update(data)
                    }
                    RequestType::SyncMessage(start_at, end_at) => {
                        let data = MessageSyncRequest::new_json(
                            self.user_id(),
                            start_at,
                            end_at,
                            self.user_token(),
                        );
                        user_ws.sync_message(data)
                    }
                    RequestType::DeleteMessage(user_id, number) => {
                        self.remove_message(number, false);
                        let data = DeleteMessage::new_json(user_id, number, self.user_token());
                        user_ws.delete_message(data)
                    }
                }
                highest_index += 1;

                if let Some(limit) = process_limit {
                    if limit == highest_index {
                        break;
                    }
                }
            } else {
                info!("Connection lost. Stopping processing request");
                connection_lost = true;
                break;
            }
        }

        // Remove the processed requests
        {
            let locked_queue = self.imp().request_queue.lock().unwrap();
            let mut queue_list = locked_queue.borrow_mut();
            for _x in 0..highest_index {
                queue_list.remove(0);
            }
        }

        // In case connection lost, this will prevent further queue processing
        // If there is a process limit it means certain request/s must be processed
        // right away. Such requests means after the processing is done, something else
        // will call the non-blocking processing request later
        if !connection_lost && process_limit.is_none() {
            self.set_request_processing(false);
        }
    }

    pub fn set_random_image(&self) {
        let new_link = generate_random_avatar_link();
        info!("Generated random image link: {}", new_link);
        self.add_to_queue(RequestType::ImageUpdated(Some(new_link)));
    }

    pub fn remove_image(&self) {
        self.set_image_link(None::<String>);
        self.set_big_image(None::<Paintable>);
        self.set_small_image(None::<Paintable>);
    }

    pub fn remove_message(&self, target_number: u64, is_recursive: bool) {
        let total_len = self.messages().n_items();
        for (index, message_data) in self.messages().iter().enumerate() {
            let message_content: MessageObject = message_data.unwrap();

            if let Some(msg_num) = message_content.imp().message_number.get() {
                if msg_num == &target_number {
                    debug!("Target number exists for deletion");
                    let target_row = message_content.target_row().unwrap();
                    if !is_recursive {
                        let revealer = target_row.imp().message_revealer.get();
                        target_row.stop_signals();
                        // Remove the transition time before it gets remove for smoother animation
                        revealer.set_transition_duration(4000);
                        revealer.set_reveal_child(false);
                    }

                    let user_object = self.clone();
                    if is_recursive {
                        user_object.messages().remove(index as u32);
                        debug!("Removal happened inside recursion");
                    } else {
                        timeout_add_local_once(Duration::from_millis(500), move || {
                            // Ideally it would remove the object in first attempt however if the
                            // length is changed it would mean the index 500 millis ago is potentially no longer valid
                            // call the function again and the next time it would skip any timeout
                            if user_object.messages().n_items() == total_len {
                                user_object.messages().remove(index as u32);
                            } else {
                                debug!("Length changed. Starting recursion for deleting");
                                user_object.remove_message(target_number, true);
                            }
                        });
                    }
                    break;
                }
            }
        }
    }

    pub fn handle_ws(&self, window: Window) {
        let user_object = self.clone();
        let user_ws = self.user_ws();

        // Wait for the websocket to emit that a connect was established
        // before starting listening for incoming messages
        let ws_signal_id = user_ws.connect_closure(
            "ws-success",
            false,
            closure_local!(move |_from: WSObject, _success: bool| {
                let (sender, receiver) = MainContext::channel(Priority::DEFAULT);
                user_object.start_listening(sender.clone(), window.clone());
                window.handle_ws_message(&user_object, receiver);
            }),
        );
        user_ws.imp().signal_ids.borrow_mut().push(ws_signal_id);
    }

    fn start_listening(&self, sender: Sender<String>, window: Window) {
        let user_ws = self.user_ws();
        info!(
            "Starting listening for user {} with {}",
            self.name(),
            self.user_id()
        );
        if !user_ws.is_reconnecting() {
            if self.user_id() == 0 {
                self.add_queue_to_first(RequestType::CreateNewUser);
            } else {
                self.add_queue_to_first(RequestType::ReconnectUser);
            }
        }

        let id = user_ws.ws_conn().unwrap().connect_message(
            clone!(@weak self as user_object => move |_ws, _s, bytes| {
                let byte_slice = bytes.to_vec();
                let text = String::from_utf8(byte_slice).unwrap();
                debug!("{} {} Received from WS: {text}", user_object.name(), user_object.user_id());

                if text.starts_with('/') {
                    let splitted_data: Vec<&str> = text.splitn(2, ' ').collect();
                    match splitted_data[0] {
                        "/reconnect-success" => {
                            let user_data = FullUserData::from_json(splitted_data[1]);
                            user_object.set_name(user_data.user_name);
                            user_object.check_image_link(user_data.image_link, false);
                            window.save_user_list();
                            // It must be set to zero to ensure the server sends every single message from the server
                            // If from the other side a message gets deleted
                            // while this client is on but not connected it would mean this client would not receive
                            // the deletion event. So we have to check every single message the server has to ensure
                            // nothing is missed
                            user_object.set_message_number(0);
                            user_object.add_queue_to_first(RequestType::GetLastMessageNumber(user_object.clone()))
                        }
                        "/update-user-id" => {
                            let id_data = UserIDs::from_json(splitted_data[1]);
                            user_object.set_user_id(id_data.user_id);
                            user_object.set_user_token(id_data.user_token);
                            sender.send(text).unwrap();
                            user_object.process_queue(None);
                        }
                        "/image-updated" => {
                            let image_data = ImageUpdate::new_from_json(splitted_data[1]);
                            user_object.check_image_link(image_data.image_link, false);
                        }
                        "/name-updated" => user_object.set_name(splitted_data[1]),
                        "/message-number" => {
                            let message_number = splitted_data[1].parse::<u64>().unwrap();
                            info!(
                                "Current message_number number {}, gotten number {}",
                                user_object.message_number(),
                                message_number
                            );
                            if message_number > user_object.message_number() {
                                // Syncing must happen before any pending message sent or deletion is performed
                                user_object.add_queue_to_first(RequestType::SyncMessage(
                                    user_object.message_number(),
                                    message_number,
                                ));
                                user_object.set_message_number(message_number);
                            } else {
                                user_object.process_queue(None);
                            }
                        }
                        "/sync-message" => {
                            let chat_data = MessageSyncData::from_json(splitted_data[1]);

                            for message in chat_data.message_data.into_iter() {
                                // if content is None, this message was deleted.
                                // We will check if this message exists in the UI. If yes, remove it
                                if message.message.is_none() {
                                    user_object.remove_message(message.message_number, false);
                                    continue;
                                }
                                window.receive_message(message, user_object.clone(), false)
                            }
                            user_object.process_queue(None);
                        }
                        "/delete-message" => {
                            let deletion_data = DeleteMessage::from_json(splitted_data[1]);
                            user_object.remove_message(deletion_data.message_number, false)
                        }
                        "/message" | "/get-user-data" | "/new-user-message" => sender.send(text).unwrap(),
                        _ => {}
                    }
                }
            }),
        );
        self.user_ws().set_signal_id(id);
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
