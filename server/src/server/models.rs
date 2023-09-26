use serde::{Deserialize, Serialize};

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

#[derive(Deserialize, Serialize, Clone)]
pub struct IDInfo {
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
}

#[derive(Deserialize)]
pub struct MessageData {
    pub from_user: usize,
    pub to_user: usize,
    pub message: String,
    pub user_token: String,
}

impl MessageData {
    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(&data).unwrap()
    }
}

#[derive(Deserialize)]
pub struct SendUserData {
    pub user_id: usize,
    pub user_token: String,
}

impl SendUserData {
    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(&data).unwrap()
    }
}

#[derive(Deserialize)]
pub struct NameUpdate {
    pub new_name: String,
    pub user_token: String,
}

impl NameUpdate {
    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(&data).unwrap()
    }
}

#[derive(Deserialize)]
pub struct ImageUpdate {
    pub image_link: String,
    pub user_token: String,
}

impl ImageUpdate {
    pub fn new_from_json(data: &str) -> Self {
        serde_json::from_str(&data).unwrap()
    }
}
