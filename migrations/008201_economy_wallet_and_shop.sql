-- SPEC-082 R1, R2, R7, R12, R18: the coin wallet's ledger and the shop's one row, owned by economy
-- (docs/CONTEXT-MAP.md; ADR-308).
--
-- `coin_ledger` holds one row per coin movement: the study day it belongs to (the kernel's rule,
-- as an epoch day), its source (what moved the coins: `mint`, a fine, the shop), its reference
-- (which one of that source, empty for a day's mint), a signed whole delta and when it was
-- written. The balance is the sum of every delta; no column stores it. One movement per (study
-- day, source, reference) is a unique index, so the ledger itself refuses a second row and every
-- port's insert writes nothing on that conflict. The wallet's ports alone write it, each inside
-- one `BEGIN IMMEDIATE` write; `settle_mint` alone updates a row (the day's mint) and only the
-- owner's erase deletes one. Exported and erased.
CREATE TABLE coin_ledger (
    id INTEGER PRIMARY KEY,
    study_day INTEGER NOT NULL,
    source TEXT NOT NULL CHECK (source <> ''),
    reference TEXT NOT NULL,
    delta INTEGER NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX coin_ledger_one_movement ON coin_ledger (study_day, source, reference);

-- `economy_state` is one row: the active scroll pass's end and the pass surcharge's end, each as
-- epoch milliseconds, or NULL for none. The shop (E3) writes it; this migration seeds it, and an
-- erase resets it in place to no pass and no surcharge. Instants are epoch milliseconds.
CREATE TABLE economy_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    pass_ends_at INTEGER,
    surcharge_ends_at INTEGER,
    created_at INTEGER NOT NULL
) STRICT;

INSERT INTO economy_state (id, created_at)
VALUES (1, CAST(unixepoch('subsec') * 1000 AS INTEGER));
