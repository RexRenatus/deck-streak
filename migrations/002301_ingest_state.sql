-- SPEC-023 R6, R8, R11: the change gate's anchor, the owner's pending rescore and the ingest
-- window's base, owned by ingest (docs/CONTEXT-MAP.md).
--
-- One row. The anchor is what the last full recompute saw: the probe's newest review id, card count
-- and card fingerprint, the study day (the kernel's rule, as an epoch day), the instant the
-- recompute ran and the settings generation it read. It is NULL until the first full recompute and
-- after an erase, and the gate runs whenever any part of it is missing. A full recompute writes a
-- fresh anchor and clears the rescore flag in the same write; a skip leaves the row as it was. The
-- window's base is the ingest floor (a review id, which is epoch milliseconds) and the count of
-- study events at or before it, NULL until the first recount. An erase resets the row in place
-- (R13). Instants are epoch milliseconds.
CREATE TABLE ingest_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    anchor_newest_review_id INTEGER,
    anchor_card_count INTEGER,
    anchor_card_fingerprint INTEGER,
    anchor_study_day INTEGER,
    anchor_recomputed_at INTEGER,
    anchor_settings_generation INTEGER,
    rescore_pending INTEGER NOT NULL CHECK (rescore_pending IN (0, 1)),
    window_floor INTEGER,
    window_count INTEGER CHECK (window_count >= 0),
    created_at INTEGER NOT NULL,
    CHECK ((window_floor IS NULL) = (window_count IS NULL))
) STRICT;

INSERT INTO ingest_state (id, rescore_pending, created_at)
VALUES (1, 0, CAST(unixepoch('subsec') * 1000 AS INTEGER));
