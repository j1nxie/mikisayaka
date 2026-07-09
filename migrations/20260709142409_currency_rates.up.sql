-- Add up migration script here

CREATE TABLE currency_rates (
    id TEXT NOT NULL PRIMARY KEY,
    date TEXT NOT NULL,
    vnd REAL NOT NULL,
    usd REAL NOT NULL,
    eur REAL NOT NULL,
    gbp REAL NOT NULL
);
