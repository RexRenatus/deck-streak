-- SPEC-022 R10, R15, R16: the record of every sync, owned by ingest (docs/CONTEXT-MAP.md).
--
-- One row per run: its trigger, the study day it started in (the kernel's rule, as an epoch day),
-- when it started and finished, `ok` or `error` with one reason code from R9's closed set, the
-- attempts it used and whether it was a full download. The change gate's `skipped` rows arrive
-- with SPEC-023. No column holds error text, a path, the endpoint or a credential. Instants are
-- epoch milliseconds. Exported and erased (R12).
CREATE TABLE sync_runs (
    id INTEGER PRIMARY KEY,
    trigger TEXT NOT NULL CHECK (trigger IN ('scheduled', 'owner')),
    study_day INTEGER NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER NOT NULL CHECK (finished_at >= started_at),
    status TEXT NOT NULL CHECK (status IN ('ok', 'error', 'skipped')),
    reason TEXT CHECK (
        reason IN (
            'missing_credentials',
            'auth_rejected',
            'network_unreachable',
            'server_error',
            'sync_timeout',
            'full_upload_required',
            'collection_locked',
            'open_failed',
            'engine_failed'
        )
    ),
    attempts INTEGER NOT NULL CHECK (attempts >= 0),
    full_download INTEGER NOT NULL CHECK (full_download IN (0, 1)),
    created_at INTEGER NOT NULL,
    CHECK ((status = 'error') = (reason IS NOT NULL))
) STRICT;

CREATE INDEX sync_runs_by_study_day ON sync_runs (study_day, trigger);
