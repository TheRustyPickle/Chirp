use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{db::schema::users, server::MessageData};

#[derive(Queryable, Selectable, Insertable, Identifiable, Clone, Serialize, Deserialize)]
#[diesel(primary_key(user_id))]
pub struct User {
    pub user_id: i32,
    pub user_name: String,
    pub image_link: Option<String>,
    pub user_token: String,
}

impl User {
    pub fn new(data: String) -> Self {
        serde_json::from_str(&data).unwrap()
    }

    pub fn update_id(self, id: usize) -> Self {
        User {
            user_id: id as i32,
            user_name: self.user_name,
            image_link: self.image_link,
            user_token: self.user_token,
        }
    }

    pub fn update_token(self, token: String) -> Self {
        User {
            user_id: self.user_id,
            user_name: self.user_name,
            image_link: self.image_link,
            user_token: token,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn to_json_with_message(&self, message: MessageData) -> String {
        let mut user_json: Value = serde_json::to_value(self).unwrap();
        user_json["message"] = json!(message);

        serde_json::to_string(&user_json).unwrap()
    }
}
