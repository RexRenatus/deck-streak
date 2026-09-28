-- SPEC-027 R3: the cron-fire ledger, owned by coordination (docs/CONTEXT-MAP.md).
--
-- One row per (job, fire date): the predecessor's `cron_fires` (its migration 18, `27ee2bc`), with
-- its columns and its constraint. `fire_date` is the LOCAL calendar date of the scheduled fire
-- under the configured offset, as an epoch day number; the predecessor called it `fire_day`. It is
-- a calendar date, deliberately not a study day. A claim writes a `catchup` attempt before the job
-- acts, and succeeds only while the row counts no attempt (ok, error and catchup all zero; a
-- `missed` is not an attempt). `last_fire_at` advances only for an outcome that ran. Instants are
-- epoch milliseconds, and `created_at` is the first-seen instant. Exempt from export and erase
-- (CHARTER 13, R10): an erase must never re-arm the catch-up double-send guard.
CREATE TABLE cron_fires (
    job_id TEXT NOT NULL,
    fire_date INTEGER NOT NULL,
    first_seen_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_fire_at INTEGER,
    ok_count INTEGER NOT NULL DEFAULT 0 CHECK (ok_count >= 0),
    error_count INTEGER NOT NULL DEFAULT 0 CHECK (error_count >= 0),
    catchup_count INTEGER NOT NULL DEFAULT 0 CHECK (catchup_count >= 0),
    missed_count INTEGER NOT NULL DEFAULT 0 CHECK (missed_count >= 0),
    last_outcome TEXT NOT NULL CHECK (last_outcome IN ('ok', 'error', 'catchup', 'missed')),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (job_id, fire_date)
) STRICT;
