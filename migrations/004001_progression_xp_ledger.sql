-- SPEC-040 R1, R3, R4, R6: the XP ledger, owned by progression (docs/CONTEXT-MAP.md).
--
-- One row per grant: the study day it pays for (the kernel's rule, as an epoch day), its source (an
-- opaque token of what it was for), the track it pays on, its amount and its scope. A `per-day`
-- grant pays at most once per (study day, source, track), and a `once` grant at most once per
-- (source, track) across every study day. Nothing updates a grant, and only the owner's erase
-- deletes one. Instants are epoch milliseconds. Exported and erased (R9).
CREATE TABLE xp_ledger (
    id INTEGER PRIMARY KEY,
    study_day INTEGER NOT NULL,
    source TEXT NOT NULL,
    track TEXT NOT NULL CHECK (track IN ('language', 'law')),
    amount INTEGER NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('per-day', 'once')),
    created_at INTEGER NOT NULL
) STRICT;
