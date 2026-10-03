-- SPEC-081 R5, R6, R11, R12, R21 (as amended by T7): the session chests, their pity counters, the
-- double-XP tokens and the chest settings, owned by quests (docs/CONTEXT-MAP.md; ADR-081). Study
-- days are epoch day numbers and instants epoch milliseconds. Every column is NOT NULL: where a
-- value may be absent, the column's default names the absence.
--
-- `chests` holds one row per chest: its study day, its origin (`session`, `challenge` or
-- `weekly`), a session chest's session start (0 for the two quest origins, which hold one chest a
-- study day each), its rarity and payout as rolled once when it was earned, its state, an Epic's
-- choice ('' until it is made), and whether its announcement was handed to the router (T3). One
-- chest per (study day, origin, session start) is a unique index, so the table itself refuses a
-- second chest for a key and the grant's insert writes nothing on that conflict. The grant writes a
-- chest and the pity counters after it in one write, the caller's (ADR-081). Exported and erased.
CREATE TABLE chests (
    id INTEGER PRIMARY KEY,
    study_day INTEGER NOT NULL,
    origin TEXT NOT NULL CHECK (origin IN ('session', 'challenge', 'weekly')),
    session_start INTEGER NOT NULL CHECK (session_start >= 0),
    rarity TEXT NOT NULL CHECK (rarity IN ('common', 'rare', 'epic', 'legendary')),
    payout_xp INTEGER NOT NULL CHECK (payout_xp >= 0),
    state TEXT NOT NULL CHECK (state IN ('sealed', 'vaulted', 'opened', 'resolved')),
    choice TEXT NOT NULL DEFAULT '' CHECK (choice IN ('', 'token', 'freeze')),
    announced INTEGER NOT NULL DEFAULT 0 CHECK (announced IN (0, 1)),
    created_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX chests_one_per_key ON chests (study_day, origin, session_start);

-- `pity` is one row: the chests since the last Epic and since the last Legendary (R6). This
-- migration seeds it at 0 and 0; the grant moves it in the write that stores the chest which moved
-- it, and an erase resets it in place to 0 and 0.
CREATE TABLE pity (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    since_epic INTEGER NOT NULL DEFAULT 0 CHECK (since_epic >= 0),
    since_legendary INTEGER NOT NULL DEFAULT 0 CHECK (since_legendary >= 0),
    created_at INTEGER NOT NULL
) STRICT;

INSERT INTO pity (id, created_at)
VALUES (1, CAST(unixepoch('subsec') * 1000 AS INTEGER));

-- `xp_tokens` holds one row per double-XP token (R12): the Epic chest whose choice granted it (0
-- for a token carried over from the predecessor, which kept no chest for it), when it was granted,
-- when it was activated and when its window ends (0 and 0 while it is held), and whether its
-- window is over and its bonus settled for good. One token per chest is a unique index over the
-- chests that name one. Exported and erased.
CREATE TABLE xp_tokens (
    id INTEGER PRIMARY KEY,
    chest_id INTEGER NOT NULL CHECK (chest_id >= 0),
    granted_at INTEGER NOT NULL,
    activated_at INTEGER NOT NULL DEFAULT 0 CHECK (activated_at >= 0),
    window_ends_at INTEGER NOT NULL DEFAULT 0 CHECK (window_ends_at >= activated_at),
    consumed INTEGER NOT NULL DEFAULT 0 CHECK (consumed IN (0, 1)),
    created_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX xp_tokens_one_per_chest ON xp_tokens (chest_id) WHERE chest_id > 0;

-- `chest_settings` is one row: the most chests a study day holds and the local hour from which a
-- chest earned that day is vaulted (R2, R7). This migration seeds the predecessor's defaults, 3
-- and 21; an erase resets it in place to them.
CREATE TABLE chest_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    per_day_max INTEGER NOT NULL DEFAULT 3 CHECK (per_day_max >= 0),
    vault_hour INTEGER NOT NULL DEFAULT 21 CHECK (vault_hour BETWEEN 0 AND 23),
    created_at INTEGER NOT NULL
) STRICT;

INSERT INTO chest_settings (id, created_at)
VALUES (1, CAST(unixepoch('subsec') * 1000 AS INTEGER));
