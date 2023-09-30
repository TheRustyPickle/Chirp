use actix::prelude::*;
use rand::Rng;
use tracing::info;

use crate::server::{
    ChatServer, CommunicationType, IDInfo, ImageUpdate, MessageData, NameUpdate, SendUserData,
};

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
pub struct CommunicateUser {
    pub ws_id: usize,
    pub data: String,
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

impl Handler<CommunicateUser> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: CommunicateUser, _: &mut Context<Self>) {
        match msg.comm_type {
            CommunicationType::SendMessage => {
                let message_data = MessageData::new_from_json(&msg.data);
                self.send_message(message_data);
            }
            CommunicationType::SendUserData => {
                let user_data = SendUserData::new_from_json(&msg.data);
                self.send_user_data(msg.ws_id, user_data)
            }
            CommunicationType::CreateNewUser => self.create_new_user(msg.ws_id, msg.data),
            CommunicationType::UpdateUserIDs => {
                let id_data = IDInfo::new_from_json(msg.data);
                self.update_ids(msg.ws_id, id_data);
            }
            CommunicationType::UpdateName => {
                let update_data = NameUpdate::new_from_json(&msg.data);
                self.user_name_update(update_data)
            }
            CommunicationType::UpdateImageLink => {
                let update_data = ImageUpdate::new_from_json(&msg.data);
                self.image_link_update(update_data)
            }
            CommunicationType::ReconnectUser => {
                let id_data = IDInfo::new_from_json(msg.data);
                self.reconnect_user(msg.ws_id, id_data);
            }
        }
    }
}
