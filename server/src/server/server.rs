use actix::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::server::ChatServer;

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

#[derive(Message, Deserialize, Serialize)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub from_user: usize,
    pub to_user: usize,
    pub msg: String,
}

impl ClientMessage {
    pub fn new(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
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
    UpdateName,
    UpdateImageLink,
    ReconnectUser,
}

/// Used for sending or relevant data to create an UserObject
/// An optional message field to pass messages along with the user data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserData {
    id: usize,
    pub name: String,
    pub image_link: Option<String>,
    pub message: Option<String>,
}

impl UserData {
    pub fn new(data: String) -> Self {
        serde_json::from_str(&data).unwrap()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn add_message(self, message: &str) -> Self {
        UserData {
            id: self.id,
            name: self.name,
            image_link: self.image_link,
            message: Some(message.to_string()),
        }
    }

    pub fn update_id(self, id: usize) -> Self {
        UserData {
            id,
            name: self.name,
            image_link: self.image_link,
            message: None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct WSData {
    pub user_id: usize,
    pub ws_id: usize,
}

impl WSData {
    pub fn new(user_id: usize, ws_id: usize) -> Self {
        WSData { user_id, ws_id }
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, (0, msg.addr));
        self.send_session_id(id);
        id
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        info!("WS Session {} disconnected", msg.id);
        // TODO remove user details from user_data and sessions
        // Needs to save each sessions owner/client id perhaps?
        self.sessions.remove(&msg.id);
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(&msg.msg, msg.from_user, msg.to_user);
    }
}

impl Handler<CommunicateUser> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: CommunicateUser, _: &mut Context<Self>) {
        match msg.comm_type {
            CommunicationType::SendUserData => {
                self.send_user_data(msg.ws_id, msg.user_data.parse().unwrap())
            }
            CommunicationType::CreateNewUser => self.create_new_user(msg.ws_id, msg.user_data),
            CommunicationType::UpdateUserIDs => {
                let data: Vec<&str> = msg.user_data.split(' ').collect();
                self.update_ids(
                    msg.ws_id,
                    data[0].parse().unwrap(),
                    data[1].parse().unwrap(),
                );
            }
            CommunicationType::UpdateName => self.update_user_name(msg.ws_id, msg.user_data),
            CommunicationType::UpdateImageLink => {
                self.update_user_image_link(msg.ws_id, msg.user_data)
            }
            CommunicationType::ReconnectUser => {
                let data: Vec<&str> = msg.user_data.splitn(2, ' ').collect();
                self.reconnect_user(msg.ws_id, data[0].parse().unwrap(), data[1].to_string());
            }
        }
    }
}
