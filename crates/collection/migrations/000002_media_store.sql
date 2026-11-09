-- SPEC-601, ADR-621: the media store's index. Each media file lives in the user's blob folder
-- under its SHA-256, never in this file; these tables say which files the store holds, which
-- names the notes bind to them and which files a reason keeps. A media_file row is the commit
-- point of a put: a placed file without its row is an orphan the next open deletes.

-- one row a media file: its hash, its length, its kind, whether it is present (0), quarantined
-- or missing (1) or tombstoned (2), and whether the sync has carried it
CREATE TABLE media_file (
  hash BLOB PRIMARY KEY CHECK (length(hash) = 32),
  size INTEGER NOT NULL CHECK (size >= 0),
  kind INTEGER NOT NULL CHECK (kind BETWEEN 0 AND 4),
  state INTEGER NOT NULL CHECK (state IN (0, 1, 2)),
  synced INTEGER NOT NULL CHECK (synced IN (0, 1)),
  added_ms INTEGER NOT NULL,
  last_seen_ms INTEGER NOT NULL
) WITHOUT ROWID, STRICT;

-- one row a name the notes reference, bound to the file it reads; a name of 1 to 255 bytes
CREATE TABLE media_name (
  name TEXT PRIMARY KEY CHECK (length(CAST(name AS BLOB)) BETWEEN 1 AND 255),
  hash BLOB NOT NULL REFERENCES media_file (hash),
  origin INTEGER NOT NULL CHECK (origin BETWEEN 1 AND 3),
  bound_ms INTEGER NOT NULL,
  unbound INTEGER NOT NULL DEFAULT 0 CHECK (unbound IN (0, 1))
) WITHOUT ROWID, STRICT;

CREATE INDEX media_name_by_hash ON media_name (hash);

-- one row a reason that keeps a file, until its time
CREATE TABLE media_pin (
  hash BLOB NOT NULL REFERENCES media_file (hash),
  reason INTEGER NOT NULL CHECK (reason BETWEEN 1 AND 3),
  until_ms INTEGER NOT NULL,
  PRIMARY KEY (hash, reason)
) WITHOUT ROWID, STRICT;
