-- SPEC-077 R6, R7, R10, R18: Road to C2's stored course progress, its band milestones and the law
-- dues, owned by curriculum (docs/CONTEXT-MAP.md; ADR-077).
--
-- `language_progress` holds one row per course, keyed by the course code: its name and flag, its
-- mastery in percent, its current band, its mature and total card counts, its current unit (NULL
-- when no counted card is mature) and its bands as one JSON array of objects (band, total, mature,
-- mastery percent, achieved). The progress step rewrites a course's row at each recompute of the
-- current study day, and the next recompute reads its current band as the stored one. Instants are
-- epoch milliseconds. Exported and erased.
CREATE TABLE language_progress (
    course TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    flag TEXT NOT NULL,
    mastery_pct REAL NOT NULL CHECK (mastery_pct >= 0.0 AND mastery_pct <= 100.0),
    current_band TEXT NOT NULL CHECK (current_band IN ('A1', 'A2', 'B1', 'B2', 'C1', 'C2')),
    mature_cards INTEGER NOT NULL CHECK (mature_cards >= 0),
    total_cards INTEGER NOT NULL CHECK (total_cards >= mature_cards),
    current_unit INTEGER CHECK (current_unit >= 0),
    bands TEXT NOT NULL CHECK (json_valid(bands) AND json_type(bands) = 'array'),
    updated_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

-- `band_milestones` holds one row per course and band reached, written once: the key on the course
-- and the band is the existence check, so a band reached again writes nothing and pays nothing. A
-- course's first sighting writes its current band as a silent baseline (`baseline` 1), which owes no
-- celebration and so is written marked; a band-up writes `baseline` 0 with its celebration mark
-- unset, and the fold's offers raise it until the router answers (ADR-303). `study_day` is the
-- study day it was reached on, as an epoch day. Exported and erased.
CREATE TABLE band_milestones (
    course TEXT NOT NULL,
    band TEXT NOT NULL CHECK (band IN ('A1', 'A2', 'B1', 'B2', 'C1', 'C2')),
    study_day INTEGER NOT NULL,
    baseline INTEGER NOT NULL CHECK (baseline IN (0, 1)),
    celebrated_at INTEGER,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (course, band),
    CHECK (baseline = 0 OR celebrated_at IS NOT NULL)
) STRICT;

-- `law_dues` is at most one row: the law track's backlog plus the cards due today, counted at the
-- last recompute over the law cards at that study day's collection day number. No row before the
-- first recompute, so every surface reads the dues as pending, never 0; an erase deletes the row,
-- so the dues read pending again. Exported and erased.
CREATE TABLE law_dues (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    study_day INTEGER NOT NULL,
    backlog INTEGER NOT NULL CHECK (backlog >= 0),
    due_today INTEGER NOT NULL CHECK (due_today >= 0),
    updated_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;
