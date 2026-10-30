-- SPEC-076 R17 to R20: the tables streaks owns (docs/CONTEXT-MAP.md).
--
-- `streak_state` is the derived cache of each track's streak, one row per track. `freeze_events`
-- is the freeze ledger: a signed delta (zero for a break marker) with the reason it was paid for; the three reasons the fold
-- itself writes (consumed, streak_break, streak_earn) are unique per day, so a recompute writes
-- each once. `habit_strength` is the strength the governor folded for a day, and `governor_state`
-- is the governor's one row: the lapse anchor, whether it stood by, and the day it last said so.
-- Instants are epoch milliseconds, days are epoch days. `streak_state`, `freeze_events` and
-- `habit_strength` are exported and erased; `governor_state` is reset in place.
CREATE TABLE streak_state (
    track TEXT PRIMARY KEY CHECK (track IN ('language', 'law')),
    current_days INTEGER NOT NULL CHECK (current_days >= 0),
    longest_days INTEGER NOT NULL CHECK (longest_days >= 0),
    freezes INTEGER NOT NULL CHECK (freezes BETWEEN 0 AND 3),
    last_study_day INTEGER,
    comeback_armed INTEGER NOT NULL CHECK (comeback_armed IN (0, 1)),
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE freeze_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    study_day INTEGER NOT NULL,
    delta INTEGER NOT NULL,
    reason TEXT NOT NULL CHECK (
        reason IN ('consumed', 'streak_break', 'streak_earn', 'chest', 'weekly_quest', 'season', 'shop')
    ),
    created_at INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX freeze_events_fold_once
    ON freeze_events (study_day, reason)
    WHERE reason IN ('consumed', 'streak_break', 'streak_earn');

CREATE TABLE habit_strength (
    study_day INTEGER PRIMARY KEY,
    strength REAL NOT NULL CHECK (strength >= 0.0 AND strength <= 1.0),
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE governor_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    lapse_since INTEGER,
    standby INTEGER NOT NULL CHECK (standby IN (0, 1)),
    notified_day INTEGER,
    created_at INTEGER NOT NULL
) STRICT;

-- The governor's one row exists from the first start, so an erase always has a row to reset.
INSERT INTO governor_state (id, lapse_since, standby, notified_day, created_at)
VALUES (1, NULL, 0, NULL, CAST(strftime('%s', 'now') AS INTEGER) * 1000);
