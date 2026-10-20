-- SPEC-387 R4, R7, R9: the record of each preset proposal, owned by ingest (docs/CONTEXT-MAP.md).
--
-- One row per proposal: the preset it names, the vector and field it would replace (the undo
-- values), the vector it proposes, the preset's desired retention and the count of non-new cards
-- whose memory state the change recomputes. A vector is a JSON array of numbers. A proposal is
-- `open` until `verify` settles it once, `moved` or `diverged`; `settled_at` is when that was
-- observed, the preset's change point, and `retention_kept` whether the desired retention was
-- unchanged then, which only a `moved` row records. Instants are epoch milliseconds. Exported and
-- erased (R9).
CREATE TABLE preset_proposals (
    id INTEGER PRIMARY KEY,
    preset_id INTEGER NOT NULL,
    preset_name TEXT NOT NULL,
    prior_vector TEXT NOT NULL CHECK (json_valid(prior_vector) AND json_type(prior_vector) = 'array'),
    prior_field TEXT NOT NULL CHECK (prior_field IN ('fsrs6', 'fsrs5', 'fsrs4', 'empty')),
    proposed_vector TEXT NOT NULL CHECK (
        json_valid(proposed_vector) AND json_type(proposed_vector) = 'array'
    ),
    desired_retention REAL NOT NULL CHECK (desired_retention > 0 AND desired_retention < 1),
    non_new_cards INTEGER NOT NULL CHECK (non_new_cards >= 0),
    state TEXT NOT NULL CHECK (state IN ('open', 'moved', 'diverged')),
    settled_at INTEGER,
    retention_kept INTEGER CHECK (retention_kept IN (0, 1)),
    created_at INTEGER NOT NULL,
    CHECK ((state = 'open') = (settled_at IS NULL)),
    CHECK ((state = 'moved') = (retention_kept IS NOT NULL))
) STRICT;

-- At most one open proposal per preset (R4): a settled one leaves room for a new one beside it.
CREATE UNIQUE INDEX preset_proposals_one_open ON preset_proposals (preset_id) WHERE state = 'open';
