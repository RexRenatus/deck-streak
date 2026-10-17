-- SPEC-041 R14, R16: a held celebration is claimed, then sent, then settled. A flush moves each row
-- it will send from `held` to `sending` with its own token in `claim`; only that token settles,
-- relatches or abandons it. A `sending` row whose claim lapsed is abandoned by name and never
-- sent again, so a push reaches the owner at most once. SQLite cannot widen a CHECK in place, so
-- the table is rebuilt; no other table references it.
CREATE TABLE notification_queue_new (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    surface TEXT NOT NULL CHECK (surface IN ('bot', 'mini-app')),
    tier_requested TEXT NOT NULL CHECK (tier_requested IN ('T0', 'T1', 'T2', 'T3', 'T4', 'T5')),
    tier_pending TEXT NOT NULL CHECK (tier_pending IN ('T0', 'T1', 'T2', 'T3', 'T4', 'T5')),
    text TEXT NOT NULL,
    hold TEXT NOT NULL CHECK (hold IN ('quiet', 'send')),
    tries INTEGER NOT NULL CHECK (tries >= 0),
    state TEXT NOT NULL CHECK (state IN ('held', 'sending', 'abandoned')),
    claim TEXT,
    deferred_at INTEGER NOT NULL,
    study_day INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    CHECK (state <> 'sending' OR claim IS NOT NULL),
    CHECK (state <> 'held' OR claim IS NULL)
) STRICT;

INSERT INTO notification_queue_new (id, kind, dedupe_key, surface, tier_requested, tier_pending,
    text, hold, tries, state, claim, deferred_at, study_day, created_at)
SELECT id, kind, dedupe_key, surface, tier_requested, tier_pending,
    text, hold, tries, state, NULL, deferred_at, study_day, created_at
FROM notification_queue;

DROP TABLE notification_queue;
ALTER TABLE notification_queue_new RENAME TO notification_queue;
