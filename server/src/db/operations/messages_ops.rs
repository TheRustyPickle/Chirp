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
