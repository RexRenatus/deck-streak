-- SPEC-072 R22: the buffs a study day holds, owned by progression (docs/CONTEXT-MAP.md).
--
-- The Ascendant buff, armed for a day by the day before it. One row per (study day, kind), so a
-- recompute arms it once. Instants are epoch milliseconds. Exported and erased.
CREATE TABLE buffs (
    study_day INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('ascendant')),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (study_day, kind)
) STRICT;
