mod imp {
    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gdk::Paintable;
    use gio::ListStore;
    use glib::once_cell::sync::Lazy;
    use glib::subclass::Signal;
    use glib::{derived_properties, object_subclass, Properties, SignalHandlerId};
    use gtk::{gdk, glib, NoSelection, SignalListItemFactory};
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use std::cell::{Cell, OnceCell, RefCell};
    use std::sync::Mutex;

    use crate::window::Window;
    use crate::ws::{RequestType, WSObject};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::UserObject)]
    pub struct UserObject {
        #[property(get, set)]
        pub user_id: Cell<u64>,
        #[property(get, set, nullable)]
        pub big_image: RefCell<Option<Paintable>>,
        #[property(get, set, nullable)]
        pub small_image: RefCell<Option<Paintable>>,
        #[property(get, set)]
        pub name: RefCell<String>,
        #[property(get, set)]
        pub name_color: RefCell<String>,
        #[property(get, set, nullable)]
        pub image_link: RefCell<Option<String>>,
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
        #[property(get, set)]
        pub main_window: OnceCell<Window>,
        #[property(get, set)]
        pub is_syncing: Cell<bool>,
        pub rsa_public: OnceCell<RsaPublicKey>,
        pub rsa_private: OnceCell<RsaPrivateKey>,
        pub receiver_rsa_public: OnceCell<RsaPublicKey>,
        pub aes_key: OnceCell<Vec<u8>>,
        pub receiver_aes_key: RefCell<Option<Vec<u8>>>,
        pub message_factory: OnceCell<SignalListItemFactory>,
        pub selection_model: OnceCell<NoSelection>,
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

use crate::encryption::{
    decrypt_message, decrypt_message_chunk, encrypt_message, generate_new_aes_key,
    generate_new_rsa_keys,
};
use crate::message::{MessageObject, MessageRow};
use crate::utils::{generate_random_avatar_link, get_avatar, get_random_color};
use crate::window::Window;
use crate::ws::{
    DecryptedMessageData, DeleteMessage, FullUserData, ImageUpdate, MessageData, MessageSyncData,
    MessageSyncRequest, NameUpdate, RequestType, UserIDs, WSObject,
};
use adw::prelude::*;
use gdk::{gdk_pixbuf, Paintable, Texture};
use gdk_pixbuf::{InterpType, PixbufLoader};
use gio::subclass::prelude::ObjectSubclassIsExt;
use gio::{spawn_blocking, ListStore};
use glib::{
    clone, closure_local, timeout_add_local_once, Bytes, ControlFlow, MainContext, Object,
    Priority, Receiver,
};
use gtk::{gdk, glib, ListItem, NoSelection, SignalListItemFactory};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::collections::HashSet;
use std::thread;
use std::time::Duration;
use tracing::{debug, info};

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
        window: Window,
        receiving_rsa_public_key: Option<RsaPublicKey>,
        saved_keys: Option<(RsaPublicKey, RsaPrivateKey)>,
    ) -> Self {
        let messages = ListStore::new::<MessageObject>();
        let no_selection = NoSelection::new(Some(messages.clone()));
        let random_color = get_random_color(color_to_ignore);
        let id = if let Some(id) = user_id { id } else { 0 };

        let obj: UserObject = Object::builder()
            .property("user-id", id)
            .property("name", name)
            .property("image-link", image_link.to_owned())
            .property("messages", messages)
            .property("name-color", random_color)
            .property("main-window", window.clone())
            .build();

        // Will only be some in case of owner object and with some data saved
        if let Some(token) = user_token {
            obj.set_user_token(token);
            obj.set_owner_id(id);
        }

        // Will only be some in case of owner object and with some data saved
        if let Some((public_key, private_key)) = saved_keys {
            obj.imp().rsa_public.set(public_key).unwrap();
            obj.imp().rsa_private.set(private_key).unwrap();
        }

        // Will always be when a new user is getting added
        // Will only be Some for owner object when the data is saved
        // In case not saved, creates a new RSA pair in thread
        if let Some(key) = receiving_rsa_public_key {
            obj.imp().receiver_rsa_public.set(key).unwrap();
        } else {
            let (sender, receiver) = MainContext::channel(Priority::default());

            receiver.attach(None, clone!(@weak obj as user_object, @weak window => @default-return ControlFlow::Break, move |(public_key, private_key): (RsaPublicKey, RsaPrivateKey)| {
                user_object.imp().rsa_public.set(public_key.clone()).unwrap();
                user_object.imp().rsa_private.set(private_key).unwrap();
                user_object.imp().receiver_rsa_public.set(public_key).unwrap();
                window.save_rsa_keys();
                user_object.add_queue_to_first(RequestType::CreateNewUser);
                ControlFlow::Break
            }));
            thread::spawn(move || sender.send(generate_new_rsa_keys()));
        }

        // Each object will have its own aes key for encrypting when sending messages
        obj.imp().aes_key.set(generate_new_aes_key()).unwrap();
        obj.set_message_number(0);
        obj.imp().selection_model.set(no_selection).unwrap();
        obj.imp()
            .message_factory
            .set(SignalListItemFactory::new())
            .unwrap();
        obj.start_factory();
        obj.start_signals();

        let ws = WSObject::new();
        obj.set_user_ws(ws);
        obj.check_image_link(image_link, false);
        obj
    }

    fn start_signals(&self) {
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

        self.imp().signal_ids.borrow_mut().push(image_signal_id);
    }

    fn start_factory(&self) {
        let factory = self.imp().message_factory.get().unwrap();
        let window = self.main_window();

        let factory_setup_signal = factory.connect_setup(move |_, list_item| {
            let message_row = MessageRow::new_empty();
            let list_item = list_item.downcast_ref::<ListItem>().unwrap();
            list_item.set_child(Some(&message_row));
            list_item.set_activatable(false);
            list_item.set_selectable(false);
            list_item.set_focusable(false);
        });

        let factory_bind_signal = factory.connect_bind(clone!(@weak window => move |_, item| {
            let list_item = item
            .downcast_ref::<ListItem>()
            .unwrap();

            let message_object = list_item
                .item()
                .and_downcast::<MessageObject>()
                .unwrap();

            let message_row = list_item
                .child()
                .and_downcast::<MessageRow>()
                .unwrap();
            message_row.update(&message_object, &window);
        }));

        let factory_unbind_signal = factory.connect_unbind(move |_, list_item| {
            let message_row = list_item
                .downcast_ref::<ListItem>()
                .unwrap()
                .child()
                .and_downcast::<MessageRow>()
                .unwrap();

            message_row.stop_signals();
        });

        let mut signal_list = self.imp().signal_ids.borrow_mut();

        signal_list.push(factory_setup_signal);
        signal_list.push(factory_bind_signal);
        signal_list.push(factory_unbind_signal);
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
                            if pixbuf_loader.write(&image_data).is_err() {
                                user_object.emit_by_name::<()>("image-modified", &[&String::from("Failed to create the image"), &image_link]);
                                return ControlFlow::Break
                            };

                            if pixbuf_loader.close().is_err() {
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
                        let message_text = msg_obj.message();
                        let new_number = self.message_number() + 1;

                        self.set_message_number(new_number);
                        msg_obj.set_message_number(new_number);

                        let aes_key = self.imp().aes_key.get().unwrap().clone();

                        let rsa_public = self.imp().rsa_public.get().unwrap();
                        let receiver_rsa_public = self.imp().receiver_rsa_public.get().unwrap();

                        // This side's message encrypted with this side's rsa key
                        let (sender_message, sender_key, sender_nonce) =
                            encrypt_message(aes_key.clone(), rsa_public, &message_text);

                        // The receiver side's is encrypted with the rsa key from the receiver's side
                        // The receiver side is where this message is getting sent
                        let (receiver_message, receiver_key, receiver_nonce) =
                            encrypt_message(aes_key, receiver_rsa_public, &message_text);

                        let data = message_data
                            .update_message(
                                sender_message,
                                receiver_message,
                                sender_key,
                                receiver_key,
                                sender_nonce,
                                receiver_nonce,
                            )
                            .update_token(self.user_token())
                            .update_message_number(new_number)
                            .to_json();

                        user_ws.send_text_message(&data);
                        msg_obj.to_process(false);

                        self.main_window()
                            .imp()
                            .message_numbers
                            .borrow_mut()
                            .get_mut(&self.user_id())
                            .unwrap()
                            .insert(new_number);

                        //self.main_window().scroll_to_bottom(self.clone());
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
                        info!(
                            "Sending request to sync message from {} {}",
                            start_at, end_at
                        );
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
                        user_ws.delete_message(data);

                        self.main_window()
                            .imp()
                            .message_numbers
                            .borrow_mut()
                            .get_mut(&self.user_id())
                            .unwrap()
                            .remove(&number);
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

    /// Tries to find the given message ID and remove it from the message list.
    /// If the list changed while the UI was being updated, does a recursive call to remove it
    pub fn remove_message(&self, target_number: u64, is_recursive: bool) {
        let total_len = self.messages().n_items();

        // Try to reduce the number of iterations required. If bigger than total length then the message is likely to be
        // on the right side/other half of the list
        if target_number > total_len as u64 / 2 {
            for (index, message_data) in self.messages().iter().rev().enumerate() {
                let message_content: MessageObject = message_data.unwrap();
                let rev_index = total_len as usize - index - 1;
                let success = self.check_for_removal(
                    message_content,
                    target_number,
                    is_recursive,
                    total_len,
                    rev_index,
                );
                if success {
                    break;
                }
            }
        } else {
            for (index, message_data) in self.messages().iter().enumerate() {
                let message_content: MessageObject = message_data.unwrap();
                let success = self.check_for_removal(
                    message_content,
                    target_number,
                    is_recursive,
                    total_len,
                    index,
                );
                if success {
                    break;
                }
            }
        }
    }

    fn check_for_removal(
        &self,
        message_content: MessageObject,
        target_number: u64,
        is_recursive: bool,
        total_len: u32,
        index: usize,
    ) -> bool {
        if let Some(msg_num) = message_content.imp().message_number.get() {
            if msg_num == &target_number {
                let target_row = message_content.target_row().unwrap();
                if !is_recursive {
                    let revealer = target_row.imp().message_revealer.get();
                    target_row.stop_signals();
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
                return true;
            }
        }
        false
    }

    /// Waits for the websocket connection to be established and calls the function to start listening to messages
    pub fn handle_ws(&self) {
        let user_object = self.clone();
        let user_ws = self.user_ws();

        // Wait for the websocket to emit that a connect was established
        // before starting listening for incoming messages
        // same signal is emitted when the conn is closed then connected again
        let ws_signal_id = user_ws.connect_closure(
            "ws-success",
            false,
            closure_local!(move |_from: WSObject, _success: bool| {
                user_object.start_listening();
            }),
        );
        user_ws.imp().signal_ids.borrow_mut().push(ws_signal_id);
    }

    /// Start listening for incoming messages from the websocket and handle accordingly
    fn start_listening(&self) {
        let window = self.main_window();
        let user_ws = self.user_ws();
        info!(
            "Starting listening for user {} with {}",
            self.name(),
            self.user_id()
        );

        // It will be 0 only in case where owner data is not saved. In this case creating new profile
        // will be called when this object is created at `new` function
        if self.user_id() != 0 {
            self.add_queue_to_first(RequestType::ReconnectUser);
        }

        let id = user_ws.ws_conn().unwrap().connect_message(
            clone!(@weak self as user_object => move |_ws, _s, bytes| {
                let byte_slice = bytes.to_vec();
                let text = String::from_utf8(byte_slice).unwrap();

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
                            user_object.set_is_syncing(false);
                            user_object.add_queue_to_first(RequestType::GetLastMessageNumber(user_object.clone()))
                        }
                        "/update-user-id" => {
                            let id_data = UserIDs::from_json(splitted_data[1]);
                            user_object.set_user_id(id_data.user_id);
                            user_object.set_user_token(id_data.user_token);
                            user_object.set_owner_id(id_data.user_id);
                            window.save_user_data();
                            window.imp()
                                .message_numbers
                                .borrow_mut()
                                .insert(id_data.user_id, HashSet::new());
                            window.save_user_list();
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
                                let sync_target = if message_number > 100 {
                                    message_number - 100
                                } else {
                                    0
                                };
                                user_object.set_message_number(message_number);
                                // Syncing must happen before any pending message sent or deletion is performed
                                user_object.add_queue_to_first(RequestType::SyncMessage(
                                    sync_target,
                                    message_number,
                                ));

                            } else {
                                user_object.process_queue(None);
                            }
                        }
                        "/sync-message" => {
                            if user_object.is_syncing() {
                                return;
                            }
                            user_object.set_is_syncing(true);

                            let owner_id = user_object.owner_id();
                            let chat_data = MessageSyncData::from_json(splitted_data[1]);

                            let rsa_private_key = user_object.imp().rsa_private.get().unwrap().clone();

                            let (sender, receiver) = MainContext::channel(Priority::default());

                            receiver.attach(None, clone!(
                                @weak user_object, @weak window => @default-return ControlFlow::Break,
                                move |(message_data, completed): (Vec<DecryptedMessageData>, bool)| {
                                for message in message_data {
                                    if message.message.is_none() {
                                        let message_exists = window
                                            .imp()
                                            .message_numbers
                                            .borrow_mut()
                                            .get_mut(&user_object.user_id())
                                            .unwrap()
                                            .remove(&message.message_number);

                                        if message_exists {
                                            user_object.remove_message(message.message_number, false);
                                        }
                                        continue;
                                    }
                                    window.receive_message(message, user_object.clone(), false);
                                }

                                if completed {
                                    user_object.set_is_syncing(false);
                                    return ControlFlow::Break
                                }
                                ControlFlow::Continue
                            }));

                            let old_aes_key = user_object.imp().receiver_aes_key.borrow().clone();
                            let existing_numbers = window
                                .imp()
                                .message_numbers
                                .borrow_mut()
                                .get(&user_object.user_id())
                                .unwrap().clone();
                            thread::spawn(move || {
                                decrypt_message_chunk(
                                    sender,
                                    old_aes_key,
                                    chat_data.message_data,
                                    &rsa_private_key,
                                    owner_id,
                                    existing_numbers
                                )
                            });
                            user_object.process_queue(None);
                        }
                        "/delete-message" => {
                            let deletion_data = DeleteMessage::from_json(splitted_data[1]);
                            user_object.remove_message(deletion_data.message_number, false)
                        }
                        "/message" => {
                            let message_data = MessageData::from_json(splitted_data[1]);

                            let rsa_private_key = user_object.imp().rsa_private.get().unwrap();
                            let owner_id = user_object.owner_id();

                            let old_aes_key = user_object.imp().receiver_aes_key.borrow().clone();
                            let decrypted_data =
                                decrypt_message(message_data, &old_aes_key, rsa_private_key, owner_id);

                            user_object.imp().receiver_aes_key.replace(Some(decrypted_data.used_aes_key.clone()));
                            let message_object = window.receive_message(decrypted_data, user_object.clone(), true);
                            if let Some(object) = message_object {
                                object.set_show_initial_message(false)
                            }
                            window.scroll_to_bottom(user_object, true);
                        }
                        "/get-user-data" | "/new-user-message" => {
                            let user_data = FullUserData::from_json(splitted_data[1]);

                            if splitted_data[0] == "/get-user-data" {
                                if user_data.user_id != 0 {
                                    user_object.emit_by_name::<()>("user-exists", &[&true]);
                                } else {
                                    user_object.emit_by_name::<()>("user-exists", &[&false]);
                                    return;
                                }
                            }

                            if window.find_user(user_data.user_id).is_some() {
                                user_object.check_image_link(user_data.image_link, false);
                                info!("User {} has already been added. Dismissing the request", user_data.user_id);
                                return;
                            }

                            let new_user = window.create_user(user_data);
                            if splitted_data[0] == "/new-user-message" {
                                window.add_pending_avatar_css(new_user);
                            }
                        },
                        _ => {}
                    }
                }
            }),
        );
        self.user_ws().set_signal_id(id);
    }
}
