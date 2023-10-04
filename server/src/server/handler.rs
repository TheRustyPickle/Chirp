use actix::prelude::*;
use chrono::DateTime;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashMap;
use std::env;
use tracing::{error, info};

use crate::db::{
    create_new_message, create_new_user, get_last_message_number, get_user_with_id,
    get_user_with_token, update_user_image_link, update_user_name, NewMessage, User,
};
use crate::server::{IDInfo, ImageUpdate, Message, MessageData, NameUpdate, SendUserData, WSData};
use crate::utils::{create_message_group, generate_user_token};

pub struct ChatServer {
    // {WS session ID: (IDInfo, WS Receiver)}
    pub sessions: HashMap<usize, (IDInfo, Recipient<Message>)>,
    // The gui side has 1 WS session per user added
    // 1 owner session + 1 for every single users added for chatting
    // If there are 2 users chatting with each other, there will be total 4 sessions
    // user 1: [user 1/owner session, a WS session containing user 2 ID]
    // user 2: [user 2/owner session, a WS session containing user 1 ID]
    // {User ID: [All the sessions this user added including owner session]}
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
        let from_user_id;

        if let Some(from_user) =
            get_user_with_token(&mut self.conn, message_data.user_token.to_owned())
        {
            from_user_id = from_user.user_id as usize;
        } else {
            error!("Invalid user token received. Discarding request");
            return;
        }

        let send_message_data = message_data.to_json();
        let user_message = message_data.message.to_owned();
        let to_user_id = message_data.to_user;
        let mut conn_found = false;
        let message_group = create_message_group(from_user_id, to_user_id);
        let message_number = message_data.message_number;

        let created_at =
            DateTime::parse_from_str(&message_data.created_at, "%Y-%m-%d %H:%M:%S%.6f %:z")
                .unwrap()
                .naive_utc();

        let new_message_data = NewMessage::new(
            message_group,
            message_number,
            user_message,
            from_user_id,
            to_user_id,
            created_at,
        );

        create_new_message(&mut self.conn, new_message_data);

        if from_user_id == to_user_id {
            info!("From and to users are the same. Stopping sending.");
            return;
        }

        // If a Gui Client adds 10 users for chatting, there will be 10 + owner = 11 WS sessions
        // We store every single session of an owner in a vec. So it goes like this to find the proper
        // session and the receiver
        // Get all the sessions of the to_user => Out of all of them find the sessions
        // that has the user id of the from_user. Here from_user = a WS session with from_user id that
        // was added on the to_user's side => Get the receiver => Send message
        //
        // If there is no from_user session inside the list of to_user, to_user hasn't
        // added from_user as a new chat yet. So in this case, get all the sessions of the to_user
        // => Find the session that has the user id to_user which will always be the owner session
        // => send a request to the owner

        info!("Sending message from {} to {}", from_user_id, to_user_id);
        if let Some(receiver_ws_data) = self.user_session.get(&to_user_id) {
            for i in receiver_ws_data {
                if i.user_id == from_user_id {
                    conn_found = true;
                    let ws_id = i.ws_id;
                    if let Some(receiver_data) = self.sessions.get(&ws_id) {
                        receiver_data
                            .1
                            .do_send(Message(format!("/message {}", send_message_data)))
                    }
                    break;
                }
            }

            if !conn_found {
                info!("Client session exists but the user was not added. Sending request to add the user");
                for i in receiver_ws_data {
                    if i.user_id == to_user_id {
                        let ws_id = i.ws_id;
                        if let Some(receiver_data) = self.sessions.get(&ws_id) {
                            let user_data = get_user_with_id(&mut self.conn, from_user_id)
                                .unwrap()
                                .update_token(String::new())
                                .to_json_with_message(message_data);

                            receiver_data
                                .1
                                .do_send(Message(format!("/new-user-message {}", user_data)))
                        }
                        break;
                    }
                }
            }
        } else {
            info!("No active session id found with the User ID {to_user_id}");
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
            user_id,
            owner_id: user_id,
            user_token,
        };

        let ws_data = WSData::new(user_id, ws_id);

        self.user_session
            .entry(user_id)
            .or_insert(Vec::new())
            .push(ws_data);

        if let Some(entry) = self.sessions.get_mut(&ws_id) {
            let (id_info, receiver_ws) = entry;
            *id_info = id_data.clone();
            receiver_ws.do_send(Message(format!("/update-user-id {}", id_data.to_json())))
        }
    }

    /// Reconnect with an existing user and save necessary session information
    pub fn reconnect_user(&mut self, ws_id: usize, mut id_data: IDInfo) {
        let owner_id;

        let user_data = if let Some(user_data) =
            get_user_with_token(&mut self.conn, id_data.user_token.clone())
        {
            owner_id = user_data.user_id as usize;
            user_data
        } else {
            error!("Invalid user token received. Discarding request");
            return;
        }
        .update_token(String::new())
        .to_json();

        let user_id = id_data.user_id;
        id_data.update_owner_id(owner_id);

        info!("Reconnecting with User ID {} on WS ID {ws_id}.", user_id);

        if get_user_with_id(&mut self.conn, user_id).is_some() {
            let ws_data = WSData::new(user_id, ws_id);

            self.user_session
                .entry(owner_id)
                .or_insert(Vec::new())
                .push(ws_data);

            if let Some(entry) = self.sessions.get_mut(&ws_id) {
                let (id_info, receiver_ws) = entry;
                *id_info = id_data;
                receiver_ws.do_send(Message(format!("/reconnect-success {}", user_data)));
            }
        } else {
            error!("Unable to reconnect with a non-existing user")
        }
    }

    /// Sends a user profile data to a client
    pub fn send_user_data(&mut self, ws_id: usize, user_data: SendUserData) {
        if get_user_with_token(&mut self.conn, user_data.user_token).is_none() {
            error!("Invalid user token received. Discarding request");
            return;
        }

        let id = user_data.user_id;

        info!("Sending User ID {} profile data", id);
        if let Some(user_data) = get_user_with_id(&mut self.conn, id) {
            let user_data = user_data.update_token(String::new()).to_json();
            if let Some((_, receiver_ws)) = self.sessions.get(&ws_id) {
                receiver_ws.do_send(Message(format!("/get-user-data {}", user_data)))
            };
        }
    }

    /// Used to keep track of active user ws connections
    pub fn update_ids(&mut self, ws_id: usize, mut id_data: IDInfo) {
        let owner_id;

        if let Some(user_data) = get_user_with_token(&mut self.conn, id_data.user_token.clone()) {
            owner_id = user_data.user_id as usize;
        } else {
            error!("Invalid user token received. Discarding request");
            return;
        }

        let user_id = id_data.user_id;
        id_data.update_owner_id(owner_id);

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
    pub fn user_name_update(&mut self, update_data: NameUpdate) {
        let user_id;

        if let Some(user_data) = get_user_with_token(&mut self.conn, update_data.user_token) {
            user_id = user_data.user_id as usize;
        } else {
            error!("Invalid user token received. Discarding request");
            return;
        }

        let new_name = update_data.new_name;

        info!("Updating name of user {} to {new_name}", user_id);

        update_user_name(&mut self.conn, user_id, &new_name);

        // broadcast the name update to every active session that has added this user id
        for (id, session_data) in self.user_session.iter() {
            if id != &user_id {
                for session in session_data {
                    if session.user_id == user_id {
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
    pub fn image_link_update(&mut self, update_data: ImageUpdate) {
        let user_id;

        if let Some(user_data) = get_user_with_token(&mut self.conn, update_data.user_token) {
            user_id = user_data.user_id as usize;
        } else {
            error!("Invalid user token received. Discarding request");
            return;
        }

        let new_link = update_data.image_link;

        info!("Updating image link of user {} to {new_link}", user_id);

        update_user_image_link(&mut self.conn, user_id, &new_link);

        // broadcast the image update update to every active session that has added this user id
        for (id, session_data) in self.user_session.iter() {
            if id != &user_id {
                for session in session_data {
                    if session.user_id == user_id {
                        if let Some(data) = self.sessions.get(&session.ws_id) {
                            let receiver = &data.1;
                            receiver.do_send(Message(format!("/image-updated {new_link}")));
                        }
                    }
                }
            }
        }
    }

    pub fn send_message_number(&mut self, id_data: IDInfo) {
        let owner_id;

        if let Some(user_data) = get_user_with_token(&mut self.conn, id_data.user_token) {
            owner_id = user_data.user_id as usize;
        } else {
            error!("Invalid user token received. Discarding request");
            return;
        }

        let message_group = create_message_group(owner_id, id_data.user_id);
        let last_message_number = get_last_message_number(&mut self.conn, message_group);

        if self.user_session.contains_key(&owner_id) {
            for session in self.user_session[&owner_id].iter() {
                if session.user_id == id_data.user_id {
                    if let Some(data) = self.sessions.get(&session.ws_id) {
                        let receiver = &data.1;
                        receiver.do_send(Message(format!("/message-number {last_message_number}")));
                    }
                }
            }
        }
    }
}
