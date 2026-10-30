-- SPEC-083 R36 (ADR-301 (c), ADR-321 D16): the declared write class's stop, owned by ingest.
--
-- One row, `id = 1`, seeded not stopped. While `stopped` is 1 nothing writes to the collection: the
-- preview says so, naming who set it, why and since when, and the take and the undo refuse before
-- any request or write. A count that moved sets it (`set_by = 'counts'`, the count's name as
-- `reason`); only the owner's command clears it (E4c), so nothing here clears it. The setter, the
-- reason and the time are set together. A missing row reads as stopped. Exempt from export and
-- erase: an erase that cleared it would be a second clear path. It names who set the stop, why and
-- when, and holds none of the owner's data.
CREATE TABLE write_class_stop (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    stopped INTEGER NOT NULL CHECK (stopped IN (0, 1)),
    set_by TEXT CHECK (set_by IN ('counts', 'owner')),
    reason TEXT CHECK (length(reason) BETWEEN 1 AND 40 AND reason NOT GLOB '*[^a-z_]*'),
    set_at INTEGER,
    created_at INTEGER NOT NULL,
    CHECK ((set_by IS NULL) = (reason IS NULL) AND (set_by IS NULL) = (set_at IS NULL)),
    CHECK (stopped = 0 OR set_by IS NOT NULL)
) STRICT;

INSERT INTO write_class_stop (id, stopped, created_at)
VALUES (1, 0, CAST(unixepoch('subsec') * 1000 AS INTEGER));
