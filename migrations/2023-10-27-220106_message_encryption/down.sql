-- This file should undo anything in `up.sql`
ALTER TABLE messages
ADD COLUMN message_text TEXT,
    DROP COLUMN sender_message,
    DROP COLUMN receiver_message,
    DROP COLUMN sender_key,
    DROP COLUMN receiver_key;
ALTER TABLE users DROP COLUMN rsa_public_key;