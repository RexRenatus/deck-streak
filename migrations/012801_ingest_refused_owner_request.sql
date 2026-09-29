-- SPEC-128 R1, R3: the owner's request the job refused, recorded beside the pending flag it clears.
ALTER TABLE ingest_state ADD COLUMN refused_at INTEGER;
ALTER TABLE ingest_state ADD COLUMN refused_reason TEXT;
