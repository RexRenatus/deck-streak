-- SPEC-084 R13: the owner's latest message to the bot, which a T1 celebration reacts to. One row,
-- reset in place by an erase (R14): no message is both columns NULL.
CREATE TABLE owner_last_message (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    message_id INTEGER,
    arrived_at INTEGER,
    created_at INTEGER NOT NULL,
    CHECK ((message_id IS NULL) = (arrived_at IS NULL))
) STRICT;

INSERT INTO owner_last_message (id, message_id, arrived_at, created_at)
VALUES (1, NULL, NULL, CAST(unixepoch('subsec') * 1000 AS INTEGER));
