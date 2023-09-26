use actix::prelude::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashMap;
use std::env;
use tracing::{error, info};

use crate::db::{
    create_new_user, get_user_with_id, update_user_image_link, update_user_name, User,
};
use crate::server::{IDInfo, ImageUpdate, Message, MessageData, NameUpdate, SendUserData, WSData};
use crate::utils::generate_user_token;

pub struct ChatServer {
    pub sessions: HashMap<usize, (IDInfo, Recipient<Message>)>,
    pub user_session: HashMap<usize, Vec<WSData>>,
    pub rng: ThreadRng,
    conn: PgConnection,
}

impl ChatServer {
    pub fn new() -> ChatServer {
        info!("New Chat Server getting created");

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let conn = PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

        ChatServer {
            sessions: HashMap::new(),
            user_session: HashMap::new(),
            rng: rand::thread_rng(),
            conn,
        }
    }

    /// Send a message to another WS session
    pub fn send_message(&mut self, message_data: MessageData) {
        let message = message_data.message;
        let from_user = message_data.from_user;
        let to_user = message_data.to_user;

        info!("Sending message from {} to {}", from_user, to_user);
        if let Some(receiver_ws_data) = self.user_session.get(&to_user) {
            let mut conn_found = false;
            for i in receiver_ws_data {
                if i.user_id == from_user {
                    conn_found = true;
                    let ws_id = i.ws_id;
                    if let Some(receiver_data) = self.sessions.get(&ws_id) {
                        receiver_data
                            .1
                            .do_send(Message(format!("/message {}", message)))
                    }
                    break;
                }
            }

            if !conn_found {
                info!("Client session exists but the user was not added. Sending request to add the user");
                for i in receiver_ws_data {
                    if i.user_id == to_user {
                        let ws_id = i.ws_id;
                        if let Some(receiver_data) = self.sessions.get(&ws_id) {
                            let user_data = get_user_with_id(&mut self.conn, from_user)
                                .unwrap()
                                .update_token(String::new())
                                .to_json_with_message(message.to_string());

                            receiver_data
                                .1
                                .do_send(Message(format!("/new-user-message {}", user_data)))
                        }
                        break;
                    }
                }
            }
        } else {
            info!("No active session id found with the User ID {to_user}");
        }
    }

    /// Creates, saves and broadcasts the new user to the relevant session
    pub fn create_new_user(&mut self, ws_id: usize, other_data: String) {
        let mut user_id = self.rng.gen_range(1..=2_147_483_647) as usize;
        let user_token = generate_user_token();

        while get_user_with_id(&mut self.conn, user_id).is_some() {
            info!("Generated user ID already exist. Creating a new ID");
            user_id = self.rng.gen_range(1..=2_147_483_647) as usize;
        }

        info!(
            "Creating new user on WS ID {} with User ID {user_id}.",
            ws_id
        );

        let user_data = User::new(other_data)
            .update_id(user_id)
            .update_token(user_token.to_owned());

        create_new_user(&mut self.conn, user_data);

        let id_data = IDInfo {
            user_id: user_id,
            owner_id: user_id,
            user_token: user_token,
        };

        if let Some(entry) = self.sessions.get_mut(&ws_id) {
            let (id_info, receiver_ws) = entry;
            *id_info = id_data.clone();
            receiver_ws.do_send(Message(format!("/update-user-id {}", id_data.to_json())))
        }

        let ws_data = WSData::new(user_id, ws_id);

        self.user_session
            .entry(user_id)
            .or_insert(Vec::new())
            .push(ws_data);
    }

    /// Reconnect with an existing user and save necessary session information
    // TODO further verification here to ensure it's the correct user
    pub fn reconnect_user(&mut self, ws_id: usize, id_data: IDInfo) {
        let user_id = id_data.user_id;
        let owner_id = id_data.owner_id;

        info!("Reconnecting with User ID {} on WS ID {ws_id}.", user_id);

        if get_user_with_id(&mut self.conn, user_id).is_some() {
            if let Some(entry) = self.sessions.get_mut(&ws_id) {
                let (id_info, _receiver_ws) = entry;
                *id_info = id_data;
            }

            let ws_data = WSData::new(user_id, ws_id);

            self.user_session
                .entry(owner_id)
                .or_insert(Vec::new())
                .push(ws_data);
        } else {
            error!("Unable to reconnect with a non-existing user")
        }
    }

    /// Sends a user profile data to a client
    pub fn send_user_data(&mut self, ws_id: usize, user_data: SendUserData) {
        let id = user_data.user_id;
        info!("Sending user data of with id {}", id);
        if let Some(user_data) = get_user_with_id(&mut self.conn, id) {
            let user_data = user_data.update_token(String::new()).to_json();
            if let Some((_, receiver_ws)) = self.sessions.get(&ws_id) {
                receiver_ws.do_send(Message(format!("/get-user-data {}", user_data)))
            };
        }
    }

    /// Used to keep track of active user ws connections
    pub fn update_ids(&mut self, ws_id: usize, id_data: IDInfo) {
        let user_id = id_data.user_id;
        let owner_id = id_data.owner_id;

        if let Some(entry) = self.sessions.get_mut(&ws_id) {
            let (id_info, _) = entry;
            *id_info = id_data;
        }

        let ws_data = WSData::new(user_id, ws_id);

        let session_data = self.user_session.get_mut(&owner_id).unwrap();

        if !session_data.contains(&ws_data) {
            session_data.push(ws_data);
        }
    }

    /// Updates user name of a user
    pub fn user_name_update(&mut self, ws_id: usize, update_data: NameUpdate) {
        let new_name = update_data.new_name;
        let user_id = self.sessions[&ws_id].0.user_id;
        info!("Updating name of user {} to {new_name}", user_id);

        update_user_name(&mut self.conn, user_id, &new_name);

        // broadcast the name update to every active session that has added this user id
        for (id, session_data) in self.user_session.iter() {
            if id != &user_id {
                for session in session_data {
                    if &session.user_id == &user_id {
                        if let Some(data) = self.sessions.get(&session.ws_id) {
                            let receiver = &data.1;
                            receiver.do_send(Message(format!("/name-updated {new_name}")));
                        }
                    }
                }
            }
        }
    }

    /// Updates image link of a user
    pub fn image_link_update(&mut self, ws_id: usize, update_data: ImageUpdate) {
        let new_link = update_data.image_link;
        let user_id = self.sessions[&ws_id].0.user_id;
        info!("Updating image link of user {} to {new_link}", user_id);

        update_user_image_link(&mut self.conn, user_id, &new_link);

        // broadcast the image update update to every active session that has added this user id
        for (id, session_data) in self.user_session.iter() {
            if id != &user_id {
                for session in session_data {
                    if &session.user_id == &user_id {
                        if let Some(data) = self.sessions.get(&session.ws_id) {
                            let receiver = &data.1;
                            receiver.do_send(Message(format!("/image-updated {new_link}")));
                        }
                    }
                }
            }
        }
    }
}
