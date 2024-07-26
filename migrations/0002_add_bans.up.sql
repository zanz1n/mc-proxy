-- Add up migration script here

CREATE TABLE user_bans (
    username text PRIMARY KEY,
    created_at text NOT NULL,
    expiration text,
    reason text
) STRICT;

CREATE TABLE ip_bans (
    ip blob PRIMARY KEY,
    created_at text NOT NULL,
    expiration text,
    reason text
) STRICT;
