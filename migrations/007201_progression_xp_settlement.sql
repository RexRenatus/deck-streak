-- SPEC-072 R6, R7: the settled XP of a study day, owned by progression (docs/CONTEXT-MAP.md).
--
-- One row per (study day, derived source, track): what a day's review XP and its daily bonuses
-- came to, settled by the fold once per source (ADR-072). `closed` says the day is over: a closed
-- day's amount is only ever raised by a recompute, and only the owner's correction lowers it. The
-- amount is never negative (CHARTER 5). Instants are epoch milliseconds. Exported and erased.
CREATE TABLE xp_settlement (
    id INTEGER PRIMARY KEY,
    study_day INTEGER NOT NULL,
    source TEXT NOT NULL,
    track TEXT NOT NULL CHECK (track IN ('language', 'law')),
    amount INTEGER NOT NULL CHECK (amount >= 0),
    closed INTEGER NOT NULL CHECK (closed IN (0, 1)),
    created_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX xp_settlement_per_day ON xp_settlement (study_day, source, track);
