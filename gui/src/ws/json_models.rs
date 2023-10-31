use gio::subclass::prelude::ObjectSubclassIsExt;
use serde::{Deserialize, Serialize};

use crate::encryption::stringify_rsa_public;
use crate::message::MessageObject;
use crate::user::UserObject;

/// Types of request that are processed by the GUI to WS currently
#[derive(Debug, Clone)]
pub enum RequestType {
    // Ask the WS to create a new user
    CreateNewUser,
    // Broadcast name update to the WS
    NameUpdated(String),
    // Broadcast image update to the WS
    ImageUpdated(Option<String>),
    // Try to reconnect with the WS again
    ReconnectUser,
    // Send a message to another user
    SendMessage(MessageData, MessageObject),
    // Ask the WS for a specific user info
    GetUserData(u64),
    // Broadcast new user selection to the WS
    GetLastMessageNumber(UserObject),
    // Ask the WS to send un-synced messages
    SyncMessage(u64, u64),
    // Ask the WS to delete a message
    DeleteMessage(u64, u64),
}

/// Used for sending or receiving relevant data to create an UserObject
/// An optional message field to receive messages along with the user data
#[derive(Serialize, Deserialize)]
pub struct FullUserData {
    pub user_id: u64,
    pub user_name: String,
    pub image_link: Option<String>,
    pub user_token: String,
    pub rsa_public_key: String,
}

impl FullUserData {
    pub fn new(user_object: &UserObject) -> Self {
        let user_token = if user_object.imp().user_token.get().is_some() {
            user_object.user_token()
        } else {
            String::new()
        };

        let rsa_public_key = user_object.imp().rsa_public.get().unwrap();

        FullUserData {
            user_id: user_object.user_id(),
            user_name: user_object.name(),
            image_link: user_object.image_link(),
            user_token,
            rsa_public_key: stringify_rsa_public(rsa_public_key),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn empty_token(self) -> Self {
        FullUserData {
            user_id: self.user_id,
            user_name: self.user_name,
            image_link: self.image_link,
            user_token: String::new(),
            rsa_public_key: self.rsa_public_key,
        }
    }

    pub fn from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserIDs {
    pub user_id: u64,
    pub user_token: String,
}

impl UserIDs {
    pub fn new_json(user_id: u64, user_token: String) -> String {
        let id_data = UserIDs {
            user_id,
            user_token,
        };
        serde_json::to_string(&id_data).unwrap()
    }

    pub fn from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MessageData {
    pub created_at: String,
    pub from_user: u64,
    pub to_user: u64,
    pub sender_message: Option<Vec<u8>>,
    pub receiver_message: Option<Vec<u8>>,
    pub sender_key: Option<Vec<u8>>,
    pub receiver_key: Option<Vec<u8>>,
    pub sender_nonce: Option<Vec<u8>>,
    pub receiver_nonce: Option<Vec<u8>>,
    pub message_number: u64,
    #[serde(skip_deserializing)]
    pub user_token: String,
}

impl MessageData {
    pub fn new_incomplete(created_at: String, from_user: u64, to_user: u64) -> Self {
        MessageData {
            created_at,
            from_user,
            to_user,
            sender_message: None,
            receiver_message: None,
            sender_key: None,
            receiver_key: None,
            sender_nonce: None,
            receiver_nonce: None,
            message_number: 0,
            user_token: String::new(),
        }
    }

    pub fn update_message(
        self,
        sender_message: Vec<u8>,
        receiver_message: Vec<u8>,
        sender_key: Vec<u8>,
        receiver_key: Vec<u8>,
        sender_nonce: Vec<u8>,
        receiver_nonce: Vec<u8>,
    ) -> Self {
        MessageData {
            created_at: self.created_at,
            from_user: self.from_user,
            to_user: self.to_user,
            sender_message: Some(sender_message),
            receiver_message: Some(receiver_message),
            sender_key: Some(sender_key),
            receiver_key: Some(receiver_key),
            sender_nonce: Some(sender_nonce),
            receiver_nonce: Some(receiver_nonce),
            message_number: self.message_number,
            user_token: self.user_token,
        }
    }

    pub fn update_token(self, user_token: String) -> Self {
        MessageData {
            created_at: self.created_at,
            from_user: self.from_user,
            to_user: self.to_user,
            sender_message: self.sender_message,
            receiver_message: self.receiver_message,
            sender_key: self.sender_key,
            receiver_key: self.receiver_key,
            sender_nonce: self.sender_nonce,
            receiver_nonce: self.receiver_nonce,
            message_number: self.message_number,
            user_token,
        }
    }

    pub fn update_message_number(self, message_number: u64) -> Self {
        MessageData {
            created_at: self.created_at,
            from_user: self.from_user,
            to_user: self.to_user,
            sender_message: self.sender_message,
            receiver_message: self.receiver_message,
            sender_key: self.sender_key,
            receiver_key: self.receiver_key,
            sender_nonce: self.sender_nonce,
            receiver_nonce: self.receiver_nonce,
            message_number,
            user_token: self.user_token,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ImageUpdate {
    pub image_link: Option<String>,
    #[serde(skip_deserializing)]
    user_token: String,
}

impl ImageUpdate {
    pub fn new_json(image_link: Option<String>, user_token: String) -> String {
        let data = ImageUpdate {
            image_link,
            user_token,
        };
        serde_json::to_string(&data).unwrap()
    }

    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Serialize)]
pub struct NameUpdate {
    new_name: String,
    user_token: String,
}

impl NameUpdate {
    pub fn new_json(new_name: String, user_token: String) -> String {
        let data = NameUpdate {
            new_name,
            user_token,
        };
        serde_json::to_string(&data).unwrap()
    }
}

#[derive(Serialize)]
pub struct MessageSyncRequest {
    user_id: u64,
    start_at: u64,
    end_at: u64,
    user_token: String,
}

impl MessageSyncRequest {
    pub fn new_json(user_id: u64, start_at: u64, end_at: u64, user_token: String) -> String {
        let data = MessageSyncRequest {
            user_id,
            start_at,
            end_at,
            user_token,
        };
        serde_json::to_string(&data).unwrap()
    }
}

#[derive(Deserialize)]
pub struct MessageSyncData {
    pub message_data: Vec<MessageData>,
    pub last_message_number: u64,
    pub start_at: u64,
    pub ends_at: u64,
}

impl MessageSyncData {
    pub fn from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Deserialize, Serialize)]
pub struct DeleteMessage {
    user_id: u64,
    pub message_number: u64,
    #[serde(skip_deserializing)]
    user_token: String,
}

impl DeleteMessage {
    pub fn new_json(user_id: u64, message_number: u64, user_token: String) -> String {
        let data = DeleteMessage {
            user_id,
            message_number,
            user_token,
        };
        serde_json::to_string(&data).unwrap()
    }
    pub fn from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

pub struct DecryptedMessageData {
    pub created_at: String,
    pub from_user: u64,
    pub to_user: u64,
    pub message: Option<String>,
    pub message_number: u64,
    pub used_aes_key: Vec<u8>,
}

impl DecryptedMessageData {
    pub fn new(
        created_at: String,
        from_user: u64,
        to_user: u64,
        message: String,
        message_number: u64,
        used_aes_key: Vec<u8>,
    ) -> Self {
        DecryptedMessageData {
            created_at,
            from_user,
            to_user,
            message: Some(message),
            message_number,
            used_aes_key,
        }
    }

    pub fn new_empty_message(
        created_at: String,
        from_user: u64,
        to_user: u64,
        message_number: u64,
    ) -> Self {
        DecryptedMessageData {
            created_at,
            from_user,
            to_user,
            message: None,
            message_number,
            used_aes_key: Vec::new(),
        }
    }
}
