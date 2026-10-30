-- SPEC-041 R6 to R8, R11: the notification router's five tables, owned by notifications
-- (docs/CONTEXT-MAP.md). Each is the owner's data, exported and erased. Instants are epoch
-- milliseconds and study days epoch day numbers.

-- The owner's notification settings, by key: a kind's switch (off at '0') and the owner's quiet
-- window, `quiet_start_min` and `quiet_end_min` in minutes of the day. An absent key is the policy's
-- default.
CREATE TABLE notification_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

-- One row claims one key in its dedupe scope: '' for once-ever and per-incident, the study day, or
-- the lapse and the study day. The unique index is what holds a key to one delivery across both
-- surfaces: the router claims by inserting, and a conflict writes nothing (R5).
CREATE TABLE notification_deliveries (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    scope TEXT NOT NULL,
    surface TEXT NOT NULL CHECK (surface IN ('bot', 'mini-app')),
    study_day INTEGER NOT NULL,
    lapse_id INTEGER,
    created_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX notification_deliveries_scoped_key
    ON notification_deliveries (kind, dedupe_key, scope);

-- Every decision, in the pack's phx.notifications.decision.v1 shape. A withheld occasion is recorded
-- under its kind with ':withheld' appended, and nothing else is, so a withhold never stands for a
-- delivery of its key; a send has no reason, and a deferral or a withhold always has one (R6).
CREATE TABLE notification_decisions (
    id INTEGER PRIMARY KEY,
    dedupe_key TEXT NOT NULL,
    kind TEXT NOT NULL,
    surface TEXT NOT NULL CHECK (surface IN ('bot', 'mini-app')),
    arm TEXT NOT NULL CHECK (arm IN ('send', 'defer', 'withhold')),
    reason TEXT,
    tier_requested TEXT NOT NULL CHECK (tier_requested IN ('T0', 'T1', 'T2', 'T3', 'T4', 'T5')),
    tier_rendered TEXT NOT NULL CHECK (tier_rendered IN ('T0', 'T1', 'T2', 'T3', 'T4', 'T5')),
    study_day INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    CHECK ((arm = 'withhold') = (kind LIKE '%:withheld')),
    CHECK ((arm = 'send') = (reason IS NULL))
) STRICT;

-- The held celebrations, deferred by quiet hours or held after a failed send, with the first
-- deferral time kept; and the abandoned ones, until a recap line names them (R7, R8).
CREATE TABLE notification_queue (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    surface TEXT NOT NULL CHECK (surface IN ('bot', 'mini-app')),
    tier_requested TEXT NOT NULL CHECK (tier_requested IN ('T0', 'T1', 'T2', 'T3', 'T4', 'T5')),
    tier_pending TEXT NOT NULL CHECK (tier_pending IN ('T0', 'T1', 'T2', 'T3', 'T4', 'T5')),
    text TEXT NOT NULL,
    hold TEXT NOT NULL CHECK (hold IN ('quiet', 'send')),
    tries INTEGER NOT NULL CHECK (tries >= 0),
    state TEXT NOT NULL CHECK (state IN ('held', 'abandoned')),
    deferred_at INTEGER NOT NULL,
    study_day INTEGER NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

-- The in-app feed: what the router delivered to the Mini App, served once to the owner (R12).
CREATE TABLE in_app_feed (
    id INTEGER PRIMARY KEY,
    dedupe_key TEXT NOT NULL,
    kind TEXT NOT NULL,
    tier TEXT NOT NULL CHECK (tier IN ('T0', 'T1', 'T2', 'T3', 'T4', 'T5')),
    text TEXT NOT NULL,
    seen_at INTEGER,
    created_at INTEGER NOT NULL
) STRICT;
