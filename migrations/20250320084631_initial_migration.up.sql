-- Add up migration script here

CREATE TABLE IF NOT EXISTS "roles" (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    role_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS "manga" (
    id INTEGER PRIMARY KEY,
    manga_dex_id TEXT NOT NULL UNIQUE,
    last_updated DATETIME NOT NULL,
    last_chapter_date DATETIME
);
