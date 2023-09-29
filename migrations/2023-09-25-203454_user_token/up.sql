-- Your SQL goes here
ALTER TABLE users
ADD COLUMN user_token VARCHAR(70) NOT NULL UNIQUE;
