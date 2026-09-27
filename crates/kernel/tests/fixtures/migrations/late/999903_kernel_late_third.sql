-- SPEC-020 A20: a fixture migration, the third of three. The test applies the first and the third,
-- then adds this set's second, a lower number that arrives after a higher one was applied.
CREATE TABLE late_third (
    id INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL
) STRICT;
