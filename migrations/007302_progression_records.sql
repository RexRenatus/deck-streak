-- SPEC-073 R9: the personal records, owned by progression (docs/CONTEXT-MAP.md).
--
-- One row per record kind: the best so far, the day that set it, the value it beat, and the
-- celebration mark (set from the services clock after the router answers). Instants are epoch
-- milliseconds. Exported and erased.
CREATE TABLE records (
    kind TEXT NOT NULL PRIMARY KEY CHECK (kind IN ('best_score', 'most_reviews', 'most_minutes')),
    value INTEGER NOT NULL CHECK (value >= 0),
    study_day INTEGER NOT NULL,
    previous INTEGER NOT NULL CHECK (previous >= 0),
    celebrated_at INTEGER,
    created_at INTEGER NOT NULL
) STRICT;
