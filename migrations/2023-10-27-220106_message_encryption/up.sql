-- Your SQL goes here
ALTER TABLE messages DROP COLUMN message_text,
    ADD COLUMN sender_message BYTEA,
    ADD COLUMN receiver_message BYTEA,
    ADD COLUMN sender_key BYTEA,
    ADD COLUMN receiver_key BYTEA,
    ADD COLUMN sender_nonce BYTEA,
    ADD COLUMN receiver_nonce BYTEA;
ALTER TABLE users
ADD COLUMN rsa_public_key TEXT NOT NULL;