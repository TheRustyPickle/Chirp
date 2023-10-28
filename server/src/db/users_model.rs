use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::users;

#[derive(Queryable, Selectable, Insertable, Identifiable, Clone, Serialize, Deserialize)]
#[diesel(primary_key(user_id))]
pub struct User {
    pub user_id: i32,
    pub user_name: String,
    pub image_link: Option<String>,
    pub user_token: String,
    pub rsa_public_key: String,
}

impl User {
    pub fn new() -> Self {
        User {
            user_id: 0,
            user_name: String::new(),
            image_link: None,
            user_token: String::new(),
            rsa_public_key: String::new(),
        }
    }

    pub fn from_json(data: String) -> Self {
        serde_json::from_str(&data).unwrap()
    }

    pub fn update_id(self, id: usize) -> Self {
        User {
            user_id: id as i32,
            user_name: self.user_name,
            image_link: self.image_link,
            user_token: self.user_token,
            rsa_public_key: self.rsa_public_key,
        }
    }

    pub fn update_token(self, token: String) -> Self {
        User {
            user_id: self.user_id,
            user_name: self.user_name,
            image_link: self.image_link,
            user_token: token,
            rsa_public_key: self.rsa_public_key,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
