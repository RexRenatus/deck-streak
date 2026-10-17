-- SPEC-078 R6 to R8, R21 (part 078b): the writing confirmations, owned by habits
-- (docs/CONTEXT-MAP.md; ADR-078).
--
-- `writing_log` holds one row per writing course and study day the owner confirmed writing in: the
-- course's code, the study day (the kernel's rule, as an epoch day), and when it was confirmed. A
-- confirmation is a row, and clearing it deletes the row, so a course is confirmed at most once a
-- study day. The writing use cases write it, each inside one `BEGIN IMMEDIATE` write that also
-- settles the day's writing XP; only the owner's erase deletes the rest. Exported and erased.
CREATE TABLE writing_log (
    code TEXT NOT NULL,
    study_day INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (code, study_day)
) STRICT;
