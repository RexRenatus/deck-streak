-- SPEC-128 R1, R3: the owner's request the job refused, recorded beside the pending flag it clears.
--
-- `refused_reason` is one of the closed set of codes the owner is told (the cycle's own refusal
-- codes and the recompute setup's); `refused_at` is the instant, epoch milliseconds. Both are NULL
-- or both are set, and a new request writes both NULL again (R3). An erase resets them in place
-- (R6). The reason is a code and never an error text: no path, endpoint or credential is stored.
ALTER TABLE ingest_state ADD COLUMN refused_at INTEGER;
ALTER TABLE ingest_state ADD COLUMN refused_reason TEXT CHECK (
    (refused_reason IS NULL AND refused_at IS NULL)
    OR (
        refused_at IS NOT NULL
        AND refused_reason IN (
            'rescore_unrecorded',
            'sync_settings_refused',
            'credentials_directory_refused',
            'scope_settings_refused',
            'recompute_refused',
            'sync_record_failed',
            'obligations_unreadable',
            'recompute_failed'
        )
    )
);
