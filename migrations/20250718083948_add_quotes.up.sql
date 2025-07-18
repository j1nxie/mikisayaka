-- Add up migration script here

CREATE TABLE IF NOT EXISTS "quotes" (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS "quote_aliases" (
    id INTEGER PRIMARY KEY,
    quote_id INTEGER NOT NULL,
    alias TEXT NOT NULL UNIQUE,
    FOREIGN KEY (quote_id) REFERENCES quotes (id) ON DELETE CASCADE
);
