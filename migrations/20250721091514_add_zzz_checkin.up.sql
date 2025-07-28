-- Add up migration script here

CREATE TABLE IF NOT EXISTS "hoyolab_accounts" (
    id INTEGER PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    hoyolab_token TEXT NOT NULL
);
