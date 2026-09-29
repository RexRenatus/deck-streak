-- SPEC-083 R1, R2, R4, R5: the record of every skip day, owned by ingest (docs/CONTEXT-MAP.md).
--
-- One row per take: the study day it covers (an epoch day), the due review count the preview
-- showed (absent when the day had no rollup), the state of its write, the cards it moved, whether
-- its tariff went unfunded, and whether and when it was undone. A `failed` row holds one bounded
-- reason code and nothing else: no error text, no path, no card. The closed set of codes is the
-- Rust `FailReason`; the column holds its shape, so a new code is not a migration. Instants are
-- epoch milliseconds. Exported and erased (R17).
CREATE TABLE skip_days (
    id INTEGER PRIMARY KEY,
    study_day INTEGER NOT NULL,
    due_count INTEGER CHECK (due_count IS NULL OR due_count >= 0),
    state TEXT NOT NULL CHECK (state IN ('pending', 'applied', 'failed')),
    reason TEXT CHECK (
        reason IS NULL
        OR (length(reason) BETWEEN 1 AND 40 AND reason NOT GLOB '*[^a-z_]*')
    ),
    cards_moved INTEGER NOT NULL DEFAULT 0 CHECK (cards_moved >= 0),
    tariff_unfunded INTEGER NOT NULL DEFAULT 0 CHECK (tariff_unfunded IN (0, 1)),
    undone INTEGER NOT NULL DEFAULT 0 CHECK (undone IN (0, 1)),
    undone_at INTEGER,
    created_at INTEGER NOT NULL,
    CHECK ((state = 'failed') = (reason IS NOT NULL)),
    CHECK ((undone = 1) = (undone_at IS NOT NULL)),
    CHECK (undone = 0 OR state = 'applied')
) STRICT;

CREATE INDEX skip_days_by_day ON skip_days (study_day);
