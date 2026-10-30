-- SPEC-118 R3, R10, R12: the vault's record of each inbox capture, owned by the vault
-- (docs/CONTEXT-MAP.md). The predecessor kept its captures as files only, so the table starts empty
-- and W8's import maps nothing into it.
--
-- One row per capture, keyed by its stem (`<UTC date>-<kind>-<safe unique>`). The primary key IS the
-- once-only guard: a resend of a recorded stem meets the row and writes nothing. `capture_key` is
-- the capture's safe unique; among the Mini App's captures it is unique as well (the partial index
-- below), so a Mini App retry sent after UTC midnight, whose stem names the next day, meets the
-- first capture's row and answers its name (ADR-118's capture-key amendment). A Telegram capture
-- keeps the claim by stem alone: the same unique resent on a later day is a new capture, as the
-- predecessor writes it.
--
-- `attachment` is the attachment's file name, NULL for a capture without one. `state` is
-- `captured` until the curator files it (SPEC-116), which sets `destination` and `filed_day`.
-- Instants are epoch milliseconds and study days are epoch days. The table is exported and erased;
-- an erase deletes the rows and never a vault file (ADR-118).
CREATE TABLE inbox_captures (
    stem TEXT NOT NULL PRIMARY KEY CHECK (length(stem) > 0),
    capture_key TEXT NOT NULL CHECK (length(capture_key) > 0),
    kind TEXT NOT NULL CHECK (kind IN ('photo', 'voice', 'document', 'text', 'journal')),
    source TEXT NOT NULL CHECK (source IN ('telegram', 'miniapp')),
    attachment TEXT CHECK (attachment IS NULL OR length(attachment) > 0),
    captured_at INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'captured' CHECK (state IN ('captured', 'filed')),
    destination TEXT,
    filed_day INTEGER,
    created_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX inbox_captures_miniapp_key ON inbox_captures (capture_key)
WHERE source = 'miniapp';
