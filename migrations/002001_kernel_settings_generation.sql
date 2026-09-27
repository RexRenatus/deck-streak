-- SPEC-020 R19: the owner-config generation, owned by the kernel (docs/CONTEXT-MAP.md).
--
-- One row. Every write that changes a runtime setting's value bumps it inside the same
-- transaction (Db::bump_settings_generation), and the change gate (SPEC-023) recomputes when it
-- moved. An erase resets it in place (CHARTER 13). Instants are epoch milliseconds.
CREATE TABLE settings_generation (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    generation INTEGER NOT NULL CHECK (generation >= 0),
    created_at INTEGER NOT NULL
) STRICT;

INSERT INTO settings_generation (id, generation, created_at)
VALUES (1, 0, CAST(unixepoch('subsec') * 1000 AS INTEGER));
