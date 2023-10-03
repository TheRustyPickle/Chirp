use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

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
