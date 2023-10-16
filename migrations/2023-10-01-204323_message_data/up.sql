-- Your SQL goes here
CREATE TABLE messages (
    message_id SERIAL UNIQUE,
    message_group VARCHAR(40) NOT NULL,
    message_number INT NOT NULL,
    message_text TEXT,
    message_sender INT NOT NULL,
    message_receiver INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (message_sender) REFERENCES users (user_id),
    FOREIGN KEY (message_receiver) REFERENCES users (user_id),
    PRIMARY KEY (message_group, message_number)
);
CREATE INDEX messages_message_group_idx ON messages (message_group);
