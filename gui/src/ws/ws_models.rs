use gio::subclass::prelude::ObjectSubclassIsExt;
use serde::{Deserialize, Serialize};

use crate::user::UserObject;

/// Types of request that are processed by the GUI to WS currently
#[derive(Debug, Clone)]
pub enum RequestType {
    // Ask the WS to create a new user
    CreateNewUser,
    // Broadcast name update to the WS
    NameUpdated(String),
    // Broadcast image update to the WS
    ImageUpdated(String),
    // Try to reconnect with the WS again
    ReconnectUser,
    // Send my IDs to the WS
    UpdateIDs,
    // Send a message to another user
    SendMessage(SendMessageData),
    // Ask the WS for a specific user info
    GetUserData(u64),
    // Broadcast new user selection to the WS
    NewUserSelection(UserObject),
}

/// Used for sending or receiving relevant data to create an UserObject
/// An optional message field to receive messages along with the user data
#[derive(Serialize, Deserialize)]
pub struct FullUserData {
    pub user_id: u64,
    pub user_name: String,
    pub image_link: Option<String>,
    pub user_token: String,
    // Don't serialize message because as of now message is only received from ws
    #[serde(skip_serializing)]
    pub message: Option<String>,
}

impl FullUserData {
    pub fn new_json(user_object: &UserObject) -> String {
        let user_token = if user_object.imp().user_token.get().is_some() {
            user_object.user_token()
        } else {
            String::new()
        };

        let user_data = FullUserData {
            user_id: user_object.user_id(),
            user_name: user_object.name(),
            image_link: user_object.image_link(),
            user_token,
            message: None,
        };
        serde_json::to_string(&user_data).unwrap()
    }

    pub fn from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Clone)]
pub struct SendMessageData {
    pub created_at: String,
    pub to_user: u64,
    pub message: String,
    pub message_number: u64,
    pub user_token: String,
}

impl SendMessageData {
    pub fn new_incomplete(created_at: String, to_user: u64, message: String) -> Self {
        SendMessageData {
            created_at,
            to_user,
            message,
            message_number: 0,
            user_token: String::new(),
        }
    }

    pub fn update_token(self, user_token: String) -> Self {
        SendMessageData {
            created_at: self.created_at,
            to_user: self.to_user,
            message: self.message,
            message_number: self.message_number,
            user_token,
        }
    }

    pub fn update_message_number(self, message_number: u64) -> Self {
        SendMessageData {
            created_at: self.created_at,
            to_user: self.to_user,
            message: self.message,
            message_number,
            user_token: self.user_token,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize)]
pub struct ImageUpdate {
    image_link: String,
    user_token: String,
}

impl ImageUpdate {
    pub fn new_json(image_link: String, user_token: String) -> String {
        let data = ImageUpdate {
            image_link,
            user_token,
        };
        serde_json::to_string(&data).unwrap()
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
