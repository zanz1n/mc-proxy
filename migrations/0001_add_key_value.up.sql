-- Add up migration script here

CREATE TABLE key_value (
    key text PRIMARY KEY,
    created_at integer NOT NULL,
    expiration integer,
    value text NOT NULL
) STRICT;
