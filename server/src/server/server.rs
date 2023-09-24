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

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let mut id = self.rng.gen::<u32>() as usize;

        while self.sessions.contains_key(&id) {
            id = self.rng.gen::<u32>() as usize;
        }
        let id_data = IDInfo::new();
        self.sessions.insert(id, (id_data, msg.addr));
        id
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        let id_data = &self.sessions.get(&msg.id).unwrap().0;
        info!(
            "WS Session {} disconnected. Removing session data related to user {}",
            msg.id, id_data.user_id
        );

        if let Some(sessions) = self.user_session.get_mut(&id_data.owner_id) {
            sessions.retain(|i| i.ws_id != msg.id);
            if self.user_session[&id_data.owner_id].is_empty() {
                self.user_session.remove(&id_data.owner_id);
            }
        }
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
                let id_data = IDInfo::new_from_json(msg.user_data);
                self.update_ids(msg.ws_id, id_data);
            }
            CommunicationType::UpdateName => self.user_name_update(msg.ws_id, &msg.user_data),
            CommunicationType::UpdateImageLink => self.image_link_update(msg.ws_id, &msg.user_data),
            CommunicationType::ReconnectUser => {
                let id_data = IDInfo::new_from_json(msg.user_data);
                self.reconnect_user(msg.ws_id, id_data);
            }
        }
    }
}

pub enum CommunicationType {
    SendUserData,
    CreateNewUser,
    UpdateUserIDs,
    UpdateName,
    UpdateImageLink,
    ReconnectUser,
}

#[derive(PartialEq)]
pub struct WSData {
    pub user_id: usize,
    pub ws_id: usize,
}

impl WSData {
    pub fn new(user_id: usize, ws_id: usize) -> Self {
        WSData { user_id, ws_id }
    }
}

#[derive(Deserialize)]
pub struct IDInfo {
    pub owner_id: usize,
    pub user_id: usize,
}

impl IDInfo {
    pub fn new() -> Self {
        IDInfo {
            owner_id: 0,
            user_id: 0,
        }
    }

    pub fn new_from_json(data: String) -> Self {
        serde_json::from_str(&data).unwrap()
    }
}
