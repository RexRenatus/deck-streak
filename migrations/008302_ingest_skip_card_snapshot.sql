-- SPEC-083 R22: what a skip's write needs to undo it exactly, owned by ingest.
--
-- One row per card a skip moved, keyed to the skip: the card's id and its scheduling as it stood
-- before the reschedule (due, queue, type, interval, ease factor, original deck and original due).
-- After the reschedule the `left_*` columns record the state the write left and the card's
-- modification time, set together, so an undo can tell a card the owner has since reviewed from
-- one still as the skip left it. Card ids and scheduling only: never a field, a note or a deck
-- name. Exported and erased (R17).
CREATE TABLE skip_card_snapshot (
    id INTEGER PRIMARY KEY,
    skip_id INTEGER NOT NULL REFERENCES skip_days (id),
    card_id INTEGER NOT NULL,
    prior_due INTEGER NOT NULL,
    prior_queue INTEGER NOT NULL,
    prior_type INTEGER NOT NULL,
    prior_interval INTEGER NOT NULL,
    prior_ease_factor INTEGER NOT NULL,
    prior_original_deck_id INTEGER NOT NULL,
    prior_original_due INTEGER NOT NULL,
    left_due INTEGER,
    left_queue INTEGER,
    left_type INTEGER,
    left_interval INTEGER,
    left_ease_factor INTEGER,
    left_mtime INTEGER,
    created_at INTEGER NOT NULL,
    UNIQUE (skip_id, card_id),
    CHECK (
        (left_due IS NULL) = (left_mtime IS NULL)
        AND (left_queue IS NULL) = (left_mtime IS NULL)
        AND (left_type IS NULL) = (left_mtime IS NULL)
        AND (left_interval IS NULL) = (left_mtime IS NULL)
        AND (left_ease_factor IS NULL) = (left_mtime IS NULL)
    )
) STRICT;
