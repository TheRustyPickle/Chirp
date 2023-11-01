use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::db::schema::messages;

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(primary_key(message_group, message_number))]
pub struct Message {
    pub message_id: i32,
    pub message_group: String,
    pub message_number: i32,
    pub sender_message: Option<Vec<u8>>,
    pub receiver_message: Option<Vec<u8>>,
    pub sender_key: Option<Vec<u8>>,
    pub receiver_key: Option<Vec<u8>>,
    pub sender_nonce: Option<Vec<u8>>,
    pub receiver_nonce: Option<Vec<u8>>,
    pub message_sender: i32,
    pub message_receiver: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = messages)]
pub struct NewMessage {
    pub message_group: String,
    pub message_number: i32,
    pub sender_message: Option<Vec<u8>>,
    pub receiver_message: Option<Vec<u8>>,
    pub sender_key: Option<Vec<u8>>,
    pub receiver_key: Option<Vec<u8>>,
    pub sender_nonce: Option<Vec<u8>>,
    pub receiver_nonce: Option<Vec<u8>>,
    pub message_sender: i32,
    pub message_receiver: i32,
    pub created_at: NaiveDateTime,
}

impl NewMessage {
    pub fn new(
        message_group: String,
        message_number: usize,
        sender_message: Vec<u8>,
        receiver_message: Vec<u8>,
        sender_key: Vec<u8>,
        receiver_key: Vec<u8>,
        sender_nonce: Vec<u8>,
        receiver_nonce: Vec<u8>,
        message_sender: usize,
        message_receiver: usize,
        created_at: NaiveDateTime,
    ) -> Self {
        NewMessage {
            message_group,
            message_number: message_number as i32,
            sender_message: Some(sender_message),
            receiver_message: Some(receiver_message),
            sender_key: Some(sender_key),
            receiver_key: Some(receiver_key),
            sender_nonce: Some(sender_nonce),
            receiver_nonce: Some(receiver_nonce),
            message_sender: message_sender as i32,
            message_receiver: message_receiver as i32,
            created_at,
        }
    }
}
