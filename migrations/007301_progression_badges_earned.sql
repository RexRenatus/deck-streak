-- SPEC-073 R1: the badges earned, owned by progression (docs/CONTEXT-MAP.md).
--
-- One row per badge key and tier, written once and never updated except for the celebration mark.
-- `celebrated_at` is set from the services clock after the router answers, and at insert for a band
-- badge, which the band-up celebrates. Instants are epoch milliseconds. Exported and erased.
CREATE TABLE badges_earned (
    badge_key TEXT NOT NULL,
    tier INTEGER NOT NULL CHECK (tier >= 0),
    name TEXT NOT NULL,
    emoji TEXT NOT NULL,
    study_day INTEGER NOT NULL,
    celebrated_at INTEGER,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (badge_key, tier)
) STRICT;
