use diesel::{update, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

use crate::db::messages_model::Message;
use crate::db::schema::messages;
use crate::db::NewMessage;

pub fn create_new_message(conn: &mut PgConnection, message_data: NewMessage) {
    diesel::insert_into(messages::table)
        .values(message_data)
        .returning(Message::as_returning())
        .get_result(conn)
        .unwrap();
}

pub fn get_last_message_number(conn: &mut PgConnection, group: String) -> usize {
    use crate::db::schema::messages::dsl::*;

    let result: Result<Message, diesel::result::Error> = messages
        .filter(message_group.eq(group))
        .order(message_number.desc())
        .limit(1)
        .select(Message::as_select())
        .first(conn);

    match result {
        Ok(data) => data.message_number as usize,
        Err(_) => 0,
    }
}

pub fn get_messages_from_number(
    conn: &mut PgConnection,
    group: String,
    start_at: usize,
    end_at: usize,
) -> Vec<Message> {
    use crate::db::schema::messages::dsl::*;

    messages
        .filter(message_group.eq(group))
        .filter(message_number.gt(start_at as i32))
        .filter(message_number.le(end_at as i32))
        .filter(sender_message.is_not_null())
        .order(message_number.desc())
        .select(Message::as_select())
        .load(conn)
        .unwrap()
}

pub fn get_deleted_messages_from_number(
    conn: &mut PgConnection,
    group: String,
    start_at: usize,
    end_at: usize,
) -> Vec<Message> {
    use crate::db::schema::messages::dsl::*;

    messages
        .filter(message_group.eq(group))
        .filter(message_number.gt(start_at as i32))
        .filter(message_number.le(end_at as i32))
        .filter(sender_message.is_null())
        .order(message_number.desc())
        .select(Message::as_select())
        .load(conn)
        .unwrap()
}

pub fn delete_message_with_number(conn: &mut PgConnection, group: String, number: usize) {
    use crate::db::schema::messages::dsl::*;

    update(messages)
        .filter(message_group.eq(group))
        .filter(message_number.eq(number as i32))
        .set((
            sender_message.eq(None::<Vec<u8>>),
            receiver_message.eq(None::<Vec<u8>>),
            sender_key.eq(None::<Vec<u8>>),
            receiver_key.eq(None::<Vec<u8>>),
            sender_nonce.eq(None::<Vec<u8>>),
            receiver_nonce.eq(None::<Vec<u8>>),
        ))
        .execute(conn)
        .unwrap();
}
