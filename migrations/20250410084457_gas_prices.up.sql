-- Add up migration script here

CREATE TABLE IF NOT EXISTS "gas_prices" (
    id STRING PRIMARY KEY,
    gas_name TEXT NOT NULL,
    zone1_price INTEGER DEFAULT 0 NOT NULL,
    zone2_price INTEGER DEFAULT 0 NOT NULL,
    last_modified DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
);
