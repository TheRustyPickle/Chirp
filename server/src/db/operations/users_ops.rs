use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

use crate::db::schema::users;
use crate::db::users_model::User;

pub fn create_new_user(conn: &mut PgConnection, user_data: User) {
    diesel::insert_into(users::table)
        .values(user_data)
        .returning(User::as_returning())
        .get_result(conn)
        .unwrap();
}

pub fn get_user_with_id(conn: &mut PgConnection, id: usize) -> Option<User> {
    use crate::db::schema::users::dsl::*;

    let result = users
        .filter(user_id.eq(id as i32))
        .limit(1)
        .select(User::as_select())
        .first(conn);

    match result {
        Ok(user) => Some(user),
        Err(_) => None,
    }
}

pub fn get_user_with_token(conn: &mut PgConnection, token: String) -> Option<User> {
    use crate::db::schema::users::dsl::*;

    let result = users
        .filter(user_token.eq(token))
        .limit(1)
        .select(User::as_select())
        .first(conn);

    match result {
        Ok(user) => Some(user),
        Err(_) => None,
    }
}

pub fn update_user_name(conn: &mut PgConnection, id: usize, new_name: &str) {
    use crate::db::schema::users::dsl::*;

    diesel::update(users.find(id as i32))
        .set(user_name.eq(new_name))
        .execute(conn)
        .unwrap();
}

pub fn update_user_image_link(conn: &mut PgConnection, id: usize, new_image_link: Option<String>) {
    use crate::db::schema::users::dsl::*;

    diesel::update(users.find(id as i32))
        .set(image_link.eq(new_image_link))
        .execute(conn)
        .unwrap();
}
