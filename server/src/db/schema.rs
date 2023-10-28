// @generated automatically by Diesel CLI.

diesel::table! {
    messages (message_group, message_number) {
        message_id -> Int4,
        #[max_length = 40]
        message_group -> Varchar,
        message_number -> Int4,
        message_sender -> Int4,
        message_receiver -> Int4,
        created_at -> Timestamptz,
        sender_message -> Nullable<Bytea>,
        receiver_message -> Nullable<Bytea>,
        sender_key -> Nullable<Bytea>,
        receiver_key -> Nullable<Bytea>,
        sender_nonce -> Nullable<Bytea>,
        receiver_nonce -> Nullable<Bytea>,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Int4,
        #[max_length = 250]
        user_name -> Varchar,
        image_link -> Nullable<Text>,
        #[max_length = 70]
        user_token -> Varchar,
        rsa_public_key -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(messages, users,);
