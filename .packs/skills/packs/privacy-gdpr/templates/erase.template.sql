-- Erasure on request (GDPR Art. 17), run in one transaction for one learner (?1).
-- secure_delete zeroes the pages a DELETE frees, so the rows cannot be read back from the file.
PRAGMA secure_delete = ON;
-- An FTS5 index keeps delete-keys until a merge; its own secure-delete removes entries at once.
-- It needs SQLite 3.42.0 or later. Repeat for every personal full-text table.
INSERT INTO notes_fts(notes_fts, rank) VALUES('secure-delete', 1);
DELETE FROM notes_fts WHERE user_id = ?1;
-- One DELETE FROM (or, when anonymising, one UPDATE) per table a category declares.
DELETE FROM learner_notes WHERE user_id = ?1;
DELETE FROM study_sessions WHERE user_id = ?1;
DELETE FROM users WHERE id = ?1;
