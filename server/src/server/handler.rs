use actix::prelude::*;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashMap;
use tracing::info;

use crate::server::{Message, UserData, WsData};

#[derive(Debug)]
pub struct ChatServer {
    // {session id: {user id this session belongs to, recipient}}
    pub sessions: HashMap<usize, (usize, Recipient<Message>)>,
    user_data: HashMap<usize, UserData>,
    user_session: HashMap<usize, Vec<WsData>>,
    pub rng: ThreadRng,
}

impl ChatServer {
    pub fn new() -> ChatServer {
        info!("New Chat Server getting created");
        ChatServer {
            sessions: HashMap::new(),
            user_data: HashMap::new(),
            user_session: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }

    pub fn send_message(&self, message: &str, from_user: usize, to_user: usize) {
        info!("Sending message from {} to {}", from_user, to_user);
        if let Some(receiver_ws_data) = self.user_session.get(&to_user) {
            for i in receiver_ws_data {
                if i.user_id == from_user {
                    let ws_id = i.ws_id;
                    if let Some(receiver_data) = self.sessions.get(&ws_id) {
                        receiver_data
                            .1
                            .do_send(Message(format!("/message {}", message)))
                    }
                }
            }
        } else {
            info!("No active session id found with the user ID {to_user}");
        }
    }

    /// Sends the WS Connection ID to the client
    pub fn send_session_id(&self, id: usize) {
        if let Some((_, receiver_ws)) = self.sessions.get(&id) {
            receiver_ws.do_send(Message(format!("/update-session-id {}", id)))
        };
    }

    /// Creates a new user and allocates necessary data to communicate with it
    pub fn create_new_user(&mut self, ws_id: usize, other_data: String) {
        let user_id = self.rng.gen::<usize>();
        info!(
            "Creating new user on ws_id {} with user id {user_id}",
            ws_id
        );
        let user_data = UserData::new(other_data).update_id(user_id);
        self.user_data.insert(user_id, user_data);

        if let Some(entry) = self.sessions.get_mut(&ws_id) {
            let (session_user_id, receiver_ws) = entry;
            *session_user_id = user_id;
            receiver_ws.do_send(Message(format!("/update-user-id {}", user_id)))
        }

        let ws_data = WsData::new(user_id, ws_id);

        self.user_session
            .entry(user_id)
            .or_insert(Vec::new())
            .push(ws_data);
    }

    /// Allocates necessary data to communicate with a previously deleted user
    pub fn reconnect_user(&mut self, ws_id: usize, user_id: usize, other_data: String) {
        info!("Reconnecting with user with id {} on ws {ws_id}", user_id);
        let user_data = UserData::new(other_data).update_id(user_id);
        self.user_data.insert(user_id, user_data);

        if let Some(entry) = self.sessions.get_mut(&ws_id) {
            let (session_user_id, _receiver_ws) = entry;
            *session_user_id = user_id;
        }

        let ws_data = WsData::new(user_id, ws_id);

        self.user_session
            .entry(user_id)
            .or_insert(Vec::new())
            .push(ws_data);
    }

    /// Sends user profile data to the client
    pub fn send_user_data(&mut self, ws_id: usize, id: usize) {
        info!("Sending user data of with id {}", id);
        if let Some(data) = self.user_data.get(&id) {
            let user_data = data.to_json();
            if let Some((_, receiver_ws)) = self.sessions.get(&ws_id) {
                receiver_ws.do_send(Message(format!("/get-user-data {}", user_data)))
            };
        }
    }

    /// Used to keep track of active user ws connections
    pub fn update_ids(&mut self, ws_id: usize, user_id: usize, client_id: usize) {
        if let Some(entry) = self.sessions.get_mut(&ws_id) {
            let (session_user_id, _) = entry;
            *session_user_id = user_id;
        }

        let ws_data = WsData::new(user_id, ws_id);

        let session_data = self.user_session.get_mut(&client_id).unwrap();

        if !session_data.contains(&ws_data) {
            session_data.push(ws_data);
        }
    }

    /// Updates user name
    pub fn update_user_name(&mut self, ws_id: usize, new_name: String) {
        let user_id = &self.sessions[&ws_id].0;
        info!("Updating name of user {} to {new_name}", user_id);

        let user_info = self.user_data.get_mut(user_id).unwrap();
        user_info.name = new_name.to_owned();

        for (id, session_data) in self.user_session.iter() {
            if id != user_id {
                for session in session_data {
                    if &session.user_id == user_id {
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
    pub fn update_user_image_link(&mut self, ws_id: usize, new_link: String) {
        let user_id = &self.sessions[&ws_id].0;
        info!("Updating image link of user {} to {new_link}", user_id);

        let user_info = self.user_data.get_mut(user_id).unwrap();
        user_info.image_link = Some(new_link.to_owned());

        for (id, session_data) in self.user_session.iter() {
            if id != user_id {
                for session in session_data {
                    if &session.user_id == user_id {
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
