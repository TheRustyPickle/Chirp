-- Your SQL goes here
ALTER TABLE messages DROP COLUMN message_text,
    ADD COLUMN sender_message TEXT,
    ADD COLUMN receiver_message TEXT,
    ADD COLUMN sender_key TEXT,
    ADD COLUMN receiver_key TEXT;
ALTER TABLE users
ADD COLUMN rsa_public_key TEXT NOT NULL;