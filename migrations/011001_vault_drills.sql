-- SPEC-110 R6, R11, R17: the vault's record of the law drills, owned by the vault
-- (docs/CONTEXT-MAP.md). The predecessor kept an answer only in the note.
--
-- `drill_answers` holds one row per drill, keyed by the drill's id (its note's stem): the study day
-- the answer was made and the surface it came through. The primary key IS the once-only guard: a
-- second answer meets the row, whatever the note's marker reads. The row and the note's write land
-- in one transaction, so a note write that fails leaves no row.
--
-- `drill_grades` holds one row per graded drill: its type, its subject, the accepted XP (the
-- clamp's band, 10 to 25, held here as well) and the study day of the poll that recorded it. The
-- row is inserted once and a re-poll finds it, so a poll that stops between the row and the grant
-- is completed by the next one.
--
-- Both tables are exported and erased; an erase deletes no note (ADR-118). Instants are epoch
-- milliseconds and study days are epoch days.
CREATE TABLE drill_answers (
    drill_id TEXT NOT NULL PRIMARY KEY CHECK (length(drill_id) > 0),
    study_day INTEGER NOT NULL,
    surface TEXT NOT NULL CHECK (surface IN ('bot', 'mini_app')),
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE drill_grades (
    drill_id TEXT NOT NULL PRIMARY KEY CHECK (length(drill_id) > 0),
    drill_type TEXT NOT NULL,
    subject TEXT NOT NULL,
    xp INTEGER NOT NULL CHECK (xp BETWEEN 10 AND 25),
    study_day INTEGER NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;
