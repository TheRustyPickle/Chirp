// @generated automatically by Diesel CLI.

diesel::table! {
    users (user_id) {
        user_id -> Int4,
        #[max_length = 250]
        user_name -> Varchar,
        image_link -> Nullable<Text>,
    }
}
