//! `ChatServer` is an actor. It maintains list of connection client session.
//! And manages available rooms. Peers send messages to other peers in same
//! room through `ChatServer`.

use std::collections::HashMap;

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

/// Chat server sends this messages to session
#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

/// Message for chat server communications

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ChattingWithUpdate {
    /// Id of the client session
    pub user_id: usize,
    /// Peer message
    pub chatting_with: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub id: usize,
}

#[derive(Debug)]
pub struct ChatServer {
    sessions: HashMap<usize, (Option<usize>, Recipient<Message>)>,
    rng: ThreadRng,
}

impl ChatServer {
    pub fn new() -> ChatServer {
        // default room
        println!("New Chat Server getting created");
        ChatServer {
            sessions: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl ChatServer {
    /// Send message to all users in the room
    fn send_message(&self, message: &str, sent_from: usize) {
        if let Some((chatting_with, _my_ws)) = self.sessions.get(&sent_from) {
            if let Some(chatting_with) = chatting_with {
                println!("Chatting with {}", chatting_with);
                let (_, receiver_ws) = self.sessions.get(&chatting_with).unwrap();
                receiver_ws.do_send(Message(message.to_owned()));
            }
        }
    }

    fn send_user_id(&self, id: usize) {
        let (_, receiver_ws) = self.sessions.get(&id).unwrap();
        receiver_ws.do_send(Message(format!("/update-user-id {}", id)))
    }

    fn update_chatting_with(&mut self, user_id: usize, update_with: usize) {
        if let Some(chatting_data) = self.sessions.get_mut(&user_id) {
            chatting_data.0 = Some(update_with);
        };
    }
}

/// Make actor from `ChatServer`
impl Actor for ChatServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        self.send_message("Someone joined", 0);

        let id = self.rng.gen::<usize>();

        self.sessions.insert(id, (None, msg.addr));
        self.send_user_id(id);
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnected");
        self.sessions.remove(&msg.id);
    }
}

/// Handler for Message message.
impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        println!(
            "Preparing for sending message {:?}. ClientMessageID: {}",
            msg.msg, msg.id
        );
        self.send_message(msg.msg.as_str(), msg.id);
    }
}

impl Handler<ChattingWithUpdate> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ChattingWithUpdate, _: &mut Context<Self>) {
        println!(
            "Updating user id {} chatting with {}",
            msg.user_id, msg.chatting_with
        );
        self.update_chatting_with(msg.user_id, msg.chatting_with);
    }
}
