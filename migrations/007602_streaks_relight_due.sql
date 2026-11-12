-- SPEC-076 R27 (section 19): the relight's due list, the study days whose relight grant committed and
-- whose celebration the router has not yet decided (docs/CONTEXT-MAP.md).
--
-- The fold's per-day write inserts a day in the same transaction as the day's grant, so a day is due
-- exactly when its grant committed, and a write that rolls back leaves no day behind. The cycle reads
-- the list after the fold's commit and deletes a day once the router has decided its celebration, so
-- a restart between the commit and the route finds the day still due. Days are epoch days and
-- instants are epoch milliseconds. Exported and erased.
CREATE TABLE relight_due (
    study_day INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL
) STRICT;
