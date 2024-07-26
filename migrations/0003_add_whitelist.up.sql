-- Add up migration script here

CREATE TABLE whitelist (
    username text PRIMARY KEY,
    created_at integer NOT NULL
) STRICT;
