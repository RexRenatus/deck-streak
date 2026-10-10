-- SPEC-381 R1, R10: the decks the learner keeps away from AI, owned by ingest (docs/CONTEXT-MAP.md).
--
-- One row per marked deck, keyed by the collection's deck id. No row means the deck is not marked,
-- so a new install, a new deck and an erased account mark nothing, and every deck starts readable.
-- A deck the server has not ingested yet may be marked (ADR-392 D7). The instant is epoch
-- milliseconds. The table is exported and erased.
CREATE TABLE sensitive_decks (
    deck_id INTEGER PRIMARY KEY CHECK (deck_id > 0),
    created_at INTEGER NOT NULL
) STRICT;
