use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub id: usize,
    pub msg: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ChattingWithUpdate {
    pub chatting_from: usize,
    pub chatting_with: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CommunicateUser {
    pub user_id: usize,
    pub user_data: String,
    pub is_send: bool,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub id: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserData {
    id: usize,
    name: String,
    image_link: Option<String>,
}

impl UserData {
    fn new(data: String) -> Self {
        serde_json::from_str(&data).unwrap()
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Debug)]
pub struct ChatServer {
    // {session id: {session_user_id, chatting_with_id, recipient}}
    sessions: HashMap<usize, (usize, Option<usize>, Recipient<Message>)>,
    users: HashMap<usize, UserData>,
    rng: ThreadRng,
}

impl ChatServer {
    pub fn new() -> ChatServer {
        info!("New Chat Server getting created");
        ChatServer {
            sessions: HashMap::new(),
            users: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl ChatServer {
    fn send_message(&self, message: &str, sent_from: usize) {
        if let Some((_, chatting_with, _my_ws)) = self.sessions.get(&sent_from) {
            if let Some(chatting_with) = chatting_with {
                info!("Chatting with {}", chatting_with);
                let (_, _, receiver_ws) = self.sessions.get(chatting_with).unwrap();
                receiver_ws.do_send(Message(message.to_owned()));
            }
        }
    }

    fn send_session_id(&self, id: usize) {
        info!("Sending WS session ID {id}");
        if let Some((_, _, receiver_ws)) = self.sessions.get(&id) {
            receiver_ws.do_send(Message(format!("/update-session-id {}", id)))
        };
    }

    // TODO: session_id and user_id won't be the same later. Perhaps session needs to store the ID of the user it belongs to
    fn send_user_id(&self, id: usize) {
        info!("Sending User ID with ws {id}");
        if let Some((_, _, receiver_ws)) = self.sessions.get(&id) {
            receiver_ws.do_send(Message(format!("/update-user-id {}", id)))
        };
    }

    fn send_user_data(&self, id: usize) {
        info!("Sending user data of with id {}", id);
        if let Some(data) = self.users.get(&id) {
            let user_data = data.to_json();
            if let Some((_, _, receiver_ws)) = self.sessions.get(&id) {
                receiver_ws.do_send(Message(format!("/get-user-data {}", user_data)))
            };
        }
    }

    /// Updates the current WS session ID chatting with another WS session ID
    fn update_chatting_with(&mut self, chatting_from: usize, chatting_with: usize) {
        info!(
            "Updating chatting with of {} with {}",
            chatting_from, chatting_with
        );
        if let Some(chatting_data) = self.sessions.get_mut(&chatting_from) {
            chatting_data.1 = Some(chatting_with);
        };
    }

    fn add_new_user(&mut self, id: usize, data: String) {
        info!("Adding new user {}", id);
        let user_data = UserData::new(data);
        self.users.insert(id, user_data);
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        self.send_message("Someone joined", 0);

        let id = self.rng.gen::<usize>();

        self.sessions.insert(id, (0, None, msg.addr));
        self.send_session_id(id);
        self.send_user_id(id);
        id
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        info!("Someone disconnected");
        self.sessions.remove(&msg.id);
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(msg.msg.as_str(), msg.id);
    }
}

impl Handler<ChattingWithUpdate> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ChattingWithUpdate, _: &mut Context<Self>) {
        self.update_chatting_with(msg.chatting_from, msg.chatting_with);
    }
}

impl Handler<CommunicateUser> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: CommunicateUser, _: &mut Context<Self>) {
        if msg.is_send {
            self.send_user_data(msg.user_data.parse().unwrap())
        } else {
            self.add_new_user(msg.user_id, msg.user_data);
        }
    }
}
