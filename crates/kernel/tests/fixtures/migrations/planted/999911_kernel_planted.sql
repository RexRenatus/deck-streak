-- SPEC-020 A22: planted defects the schema census must refuse: a table without created_at, and a
-- table that is not STRICT.
CREATE TABLE planted_without_created_at (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL
) STRICT;

CREATE TABLE planted_not_strict (
    id INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL
);
