-- Add up migration script here

CREATE TABLE IF NOT EXISTS "manga_subscriptions" (
    id INTEGER PRIMARY KEY,
    manga_dex_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    FOREIGN KEY (manga_dex_id) REFERENCES manga (manga_dex_id)
);
