-- Your SQL goes here
CREATE TABLE messages (
    message_id SERIAL UNIQUE,
    message_group VARCHAR(40) UNIQUE,
    message_number INT NOT NULL,
    message_text TEXT NOT NULL,
    message_sender INT NOT NULL,
    message_receiver INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (message_sender) REFERENCES users (user_id),
    FOREIGN KEY (message_receiver) REFERENCES users (user_id),
    PRIMARY KEY (message_group, message_number)
)