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
    pub ws_id: usize,
    pub user_data: String,
    pub comm_type: CommunicationType,
}

pub enum CommunicationType {
    SendUserData,
    CreateNewUser,
    UpdateUserIDs,
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

    fn update_id(self, id: usize) -> Self {
        UserData {
            id,
            name: self.name,
            image_link: self.image_link,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct WsData {
    user_id: usize,
    ws_id: usize,
}

impl WsData {
    fn new(user_id: usize, ws_id: usize) -> Self {
        WsData { user_id, ws_id }
    }
}

#[derive(Debug)]
pub struct ChatServer {
    // {session id: {user id this session belongs to, chatting_with_ws_id, recipient}}
    sessions: HashMap<usize, (usize, Option<usize>, Recipient<Message>)>,
    user_data: HashMap<usize, UserData>,
    user_session: HashMap<usize, Vec<WsData>>,
    rng: ThreadRng,
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
}

impl ChatServer {
    fn send_message(&self, message: &str, sent_from: usize) {
        if let Some((sent_from, chatting_with, _)) = self.sessions.get(&sent_from) {
            if let Some(chatting_with_id) = chatting_with {
                for i in self.user_session[chatting_with_id].iter() {
                    if sent_from == &i.user_id {
                        info!("Sending message from {} to {}", sent_from, i.user_id);
                        let receiver_ws = &self.sessions.get(&i.ws_id).unwrap().2;
                        receiver_ws.do_send(Message(message.to_string()))
                    }
                }
            }
        }
    }

    fn send_session_id(&self, id: usize) {
        if let Some((_, _, receiver_ws)) = self.sessions.get(&id) {
            receiver_ws.do_send(Message(format!("/update-session-id {}", id)))
        };
    }

    fn create_new_user(&mut self, ws_id: usize, other_data: String) {
        let user_id = self.rng.gen::<usize>();
        info!(
            "Creating new user on ws_id {} with user id {user_id}",
            ws_id
        );
        let user_data = UserData::new(other_data).update_id(user_id);
        self.user_data.insert(user_id, user_data);

        if let Some(entry) = self.sessions.get_mut(&ws_id) {
            let (session_user_id, _, receiver_ws) = entry;
            *session_user_id = user_id;
            receiver_ws.do_send(Message(format!("/update-user-id {}", user_id)))
        }

        let ws_data = WsData::new(user_id, ws_id);

        self.user_session
            .entry(user_id)
            .or_insert(Vec::new())
            .push(ws_data);
    }

    fn send_user_data(&mut self, ws_id: usize, id: usize) {
        info!("Sending user data of with id {}", id);
        if let Some(data) = self.user_data.get(&id) {
            let user_data = data.to_json();
            if let Some((_, _, receiver_ws)) = self.sessions.get(&ws_id) {
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

    fn update_ids(&mut self, ws_id: usize, user_id: usize, client_id: usize) {
        if let Some(entry) = self.sessions.get_mut(&ws_id) {
            let (session_user_id, _, _) = entry;
            *session_user_id = user_id;
        }

        let ws_data = WsData::new(user_id, ws_id);

        let session_data = self.user_session.get_mut(&client_id).unwrap();

        if !session_data.contains(&ws_data) {
            session_data.push(ws_data);
        }
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.rng.gen::<usize>();

        self.sessions.insert(id, (0, None, msg.addr));
        self.send_session_id(id);
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
        match msg.comm_type {
            CommunicationType::SendUserData => {
                self.send_user_data(msg.ws_id, msg.user_data.parse().unwrap())
            }
            CommunicationType::CreateNewUser => {
                self.create_new_user(msg.ws_id, msg.user_data);
            }
            CommunicationType::UpdateUserIDs => {
                let data: Vec<&str> = msg.user_data.split(' ').collect();
                self.update_ids(
                    msg.ws_id,
                    data[0].parse().unwrap(),
                    data[1].parse().unwrap(),
                );
            }
        }
    }
}
