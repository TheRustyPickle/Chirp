use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::db::schema::messages;

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(primary_key(message_group, message_number))]
pub struct Message {
    pub message_id: i32,
    pub message_group: String,
    pub message_number: i32,
    pub message_text: String,
    pub message_sender: i32,
    pub message_receiver: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = messages)]
pub struct NewMessage {
    pub message_group: String,
    pub message_number: i32,
    pub message_text: String,
    pub message_sender: i32,
    pub message_receiver: i32,
    pub created_at: NaiveDateTime,
}

impl NewMessage {
    pub fn new(
        message_group: String,
        message_number: i32,
        message_text: String,
        message_sender: usize,
        message_receiver: usize,
        created_at: NaiveDateTime,
    ) -> Self {
        NewMessage {
            message_group,
            message_number,
            message_text,
            message_sender: message_sender as i32,
            message_receiver: message_receiver as i32,
            created_at,
        }
    }
}
