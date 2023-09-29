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
    SendMessage(String),
    // Ask the WS for a specific user info
    GetUserData(u64),
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
    pub fn new_json(user_object: &UserObject) -> String {
        let id_data = UserIDs {
            user_id: user_object.user_id(),
            user_token: user_object.user_token(),
        };
        serde_json::to_string(&id_data).unwrap()
    }

    pub fn from_json(data: &str) -> Self {
        serde_json::from_str(data).unwrap()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct MessageData {
    pub to_user: u64,
    pub message: String,
    pub user_token: String,
}

impl MessageData {
    pub fn new_json(to_user: u64, message: String, user_token: String) -> String {
        let data = MessageData {
            message,
            to_user,
            user_token,
        };
        serde_json::to_string(&data).unwrap()
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

#[derive(Serialize)]
pub struct GetUserData {
    user_id: u64,
    user_token: String,
}

impl GetUserData {
    pub fn new_json(user_id: u64, user_token: String) -> String {
        let data = GetUserData {
            user_id,
            user_token,
        };
        serde_json::to_string(&data).unwrap()
    }
}
