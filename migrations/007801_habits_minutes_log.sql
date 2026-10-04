-- SPEC-078 R2 to R4, R21 (part 078a): the minutes log of book reading, owned by habits
-- (docs/CONTEXT-MAP.md; ADR-078).
--
-- `minutes_log` holds one row per reading entry the owner logged: the course's code, the study day
-- it was logged on (the kernel's rule, as an epoch day), the minutes (1 to 600) and an optional note
-- of up to 200 characters, and when it was written. The entry's id is the one the Undo button
-- carries. The habit use cases write it, each inside one `BEGIN IMMEDIATE` write that also settles
-- the entry's reading XP; `/undo` deletes the newest row, and only the owner's erase deletes the
-- rest. Exported and erased.
CREATE TABLE minutes_log (
    id INTEGER PRIMARY KEY,
    code TEXT NOT NULL CHECK (code <> ''),
    study_day INTEGER NOT NULL,
    minutes INTEGER NOT NULL CHECK (minutes BETWEEN 1 AND 600),
    note TEXT NOT NULL CHECK (length(note) <= 200),
    created_at INTEGER NOT NULL
) STRICT;

CREATE INDEX minutes_log_by_day ON minutes_log (study_day, code);
