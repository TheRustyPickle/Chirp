use chrono::DateTime;
use serde::{Deserialize, Serialize};

/// The types of requests that the WS can process currently
pub enum CommunicationType {
    // Send a message to another user
    SendMessage,
    // Sends user data of a specific user to another user
    SendUserData,
    // Create a new user
    CreateNewUser,
    // Broadcast name updates to relevant sessions
    UpdateName,
    // Broadcast image updates to relevant sessions
    UpdateImageLink,
    // Reconnect with an existing user
    ReconnectUser,
    // Send the last message of this user group
    SendMessageNumber,
    // Send message data to sync messages
    SyncMessage,
    // Broadcast message deletion
    DeleteMessage,
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

#[derive(Deserialize, Serialize, Clone)]
pub struct IDInfo {
    #[serde(skip_deserializing)]
    pub owner_id: usize,
    pub user_id: usize,
    pub user_token: String,
}

impl IDInfo {
    pub fn new() -> Self {
        IDInfo {
            owner_id: 0,
            user_id: 0,
            user_token: String::new(),
        }
    }

    pub fn new_from_json(data: String) -> Self {
        serde_json::from_str(&data).unwrap()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn update_owner_id(&mut self, id: usize) {
        self.owner_id = id;
    }
}

#[derive(Deserialize, Serialize)]
pub struct MessageData {
    pub created_at: String,
    pub from_user: usize,
    pub to_user: usize,
    pub message: String,
    pub message_number: usize,
    #[serde(skip_serializing)]
    pub user_token: String,
}

impl MessageData {
    pub fn new_from_json(data: &str) -> Self {
        let mut data: MessageData = serde_json::from_str(data).unwrap();
        let new_created_at = DateTime::parse_from_str(&data.created_at, "%Y-%m-%d %H:%M:%S%.3f %z")
            .unwrap()
            .naive_utc()
            .to_string();

        data.created_at = new_created_at;
        data
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Deserialize)]
pub struct SendUserData {
    pub user_id: usize,
    pub user_token: String,
}

impl SendUserData {
    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Deserialize)]
pub struct NameUpdate {
    pub new_name: String,
    pub user_token: String,
}

impl NameUpdate {
    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Deserialize, Serialize)]
pub struct ImageUpdate {
    pub image_link: Option<String>,
    #[serde(skip_serializing)]
    pub user_token: String,
}

impl ImageUpdate {
    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Deserialize)]
pub struct SyncMessage {
    pub user_id: usize,
    pub start_at: usize,
    pub end_at: usize,
    pub user_token: String,
}

impl SyncMessage {
    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Serialize)]
pub struct SyncMessageData {
    message_data: Vec<MessageData>,
}

impl SyncMessageData {
    pub fn new_json(message_data: Vec<MessageData>) -> String {
        let data = SyncMessageData { message_data };
        serde_json::to_string(&data).unwrap()
    }
}

#[derive(Deserialize, Serialize)]
pub struct DeleteMessage {
    pub user_id: usize,
    pub message_number: usize,
    #[serde(skip_serializing)]
    pub user_token: String,
}

impl DeleteMessage {
    pub fn from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
