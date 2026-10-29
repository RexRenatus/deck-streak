-- SPEC-512, ADR-530: the collection store's base schema. One file per user, the same schema on the
-- server and on each device. Every table is STRICT; the review log, its retractions, its client
-- record and the notes' field history are append-only, and leave only while the erase gate is open.
PRAGMA application_id = 1146311747;

-- device-local: the store's own record, written once by the opener
CREATE TABLE store_meta (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  role INTEGER NOT NULL CHECK (role IN (0, 1)),
  origin BLOB NOT NULL CHECK (length(origin) = 16),
  slot INTEGER NOT NULL CHECK (slot BETWEEN 0 AND 63),
  id_floor INTEGER NOT NULL,
  next_seq INTEGER NOT NULL CHECK (next_seq >= 0),
  import_source TEXT
) STRICT;

-- replicated: the study day's rule
CREATE TABLE study_rule (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  rollover_hour INTEGER NOT NULL CHECK (rollover_hour BETWEEN 0 AND 23),
  utc_offset_minutes INTEGER NOT NULL CHECK (utc_offset_minutes BETWEEN -720 AND 840)
) STRICT;

-- device-local: the one switch an erase opens; closed from creation
CREATE TABLE erase_gate (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  open INTEGER NOT NULL CHECK (open IN (0, 1))
) STRICT;
INSERT INTO erase_gate (id, open) VALUES (1, 0);

-- replicated domain tables
CREATE TABLE notetypes (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  name_key TEXT NOT NULL UNIQUE,
  kind INTEGER NOT NULL CHECK (kind IN (0, 1)),
  css TEXT NOT NULL,
  sort_field INTEGER NOT NULL CHECK (sort_field >= 0),
  config TEXT NOT NULL CHECK (json_valid(config)),
  mtime_ms INTEGER NOT NULL
) STRICT;
CREATE TABLE fields (
  notetype_id INTEGER NOT NULL REFERENCES notetypes (id) ON DELETE CASCADE,
  ord INTEGER NOT NULL CHECK (ord >= 0),
  name TEXT NOT NULL,
  config TEXT NOT NULL CHECK (json_valid(config)),
  PRIMARY KEY (notetype_id, ord),
  UNIQUE (notetype_id, name)
) STRICT, WITHOUT ROWID;
CREATE TABLE templates (
  notetype_id INTEGER NOT NULL REFERENCES notetypes (id) ON DELETE CASCADE,
  ord INTEGER NOT NULL CHECK (ord >= 0),
  name TEXT NOT NULL,
  front TEXT NOT NULL,
  back TEXT NOT NULL,
  config TEXT NOT NULL CHECK (json_valid(config)),
  PRIMARY KEY (notetype_id, ord)
) STRICT, WITHOUT ROWID;
CREATE TABLE notes (
  id INTEGER PRIMARY KEY,
  guid TEXT NOT NULL UNIQUE,
  notetype_id INTEGER NOT NULL REFERENCES notetypes (id),
  fields TEXT NOT NULL CHECK (json_valid(fields) AND json_type(fields) = 'array'),
  sort_text TEXT NOT NULL,
  first_digest BLOB NOT NULL CHECK (length(first_digest) = 32),
  mtime_ms INTEGER NOT NULL
) STRICT;
CREATE TABLE note_tags (
  note_id INTEGER NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
  tag_key TEXT NOT NULL,
  tag TEXT NOT NULL,
  PRIMARY KEY (note_id, tag_key)
) STRICT, WITHOUT ROWID;
CREATE TABLE note_field_history (
  note_id INTEGER NOT NULL,
  ord INTEGER NOT NULL,
  version INTEGER NOT NULL CHECK (version >= 1),
  html TEXT NOT NULL,
  origin BLOB NOT NULL CHECK (length(origin) = 16),
  seq INTEGER NOT NULL,
  at_ms INTEGER NOT NULL,
  lost_to_concurrent INTEGER NOT NULL CHECK (lost_to_concurrent IN (0, 1)),
  PRIMARY KEY (note_id, ord, version)
) STRICT, WITHOUT ROWID;
CREATE TABLE presets (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  config TEXT NOT NULL CHECK (json_valid(config)),
  mtime_ms INTEGER NOT NULL
) STRICT;
CREATE TABLE decks (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL CHECK (length(name) > 0),
  name_key TEXT NOT NULL UNIQUE,
  preset_id INTEGER NOT NULL REFERENCES presets (id),
  kind INTEGER NOT NULL CHECK (kind IN (0, 1)),
  config TEXT NOT NULL CHECK (json_valid(config)),
  mtime_ms INTEGER NOT NULL
) STRICT;
CREATE TABLE cards (
  id INTEGER PRIMARY KEY,
  note_id INTEGER NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
  ord INTEGER NOT NULL CHECK (ord >= 0),
  deck_id INTEGER NOT NULL REFERENCES decks (id),
  new_position INTEGER CHECK (new_position IS NULL OR new_position >= 0),
  suspended INTEGER NOT NULL CHECK (suspended IN (0, 1)),
  bury_day INTEGER,
  flag INTEGER NOT NULL CHECK (flag BETWEEN 0 AND 7),
  created_ms INTEGER NOT NULL,
  UNIQUE (note_id, ord)
) STRICT;
CREATE INDEX cards_deck ON cards (deck_id);

-- replicated, append-only: the review log. A row keeps its card's id with no foreign key, so a
-- deleted card keeps its history; the basis key is all three columns or none.
CREATE TABLE review_log (
  id INTEGER PRIMARY KEY,
  origin BLOB NOT NULL CHECK (length(origin) = 16),
  seq INTEGER NOT NULL CHECK (seq >= 0),
  card_id INTEGER NOT NULL,
  note_id INTEGER NOT NULL,
  deck_id INTEGER NOT NULL,
  preset_id INTEGER NOT NULL,
  kind INTEGER NOT NULL CHECK (kind BETWEEN 0 AND 5),
  ease INTEGER NOT NULL CHECK (ease BETWEEN 0 AND 4),
  duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
  study_day INTEGER NOT NULL,
  shown_due_day INTEGER,
  due_day INTEGER,
  interval_days INTEGER CHECK (interval_days IS NULL OR interval_days >= 0),
  scheduler BLOB CHECK (scheduler IS NULL OR length(scheduler) = 32),
  basis_t INTEGER,
  basis_origin BLOB CHECK (basis_origin IS NULL OR length(basis_origin) = 16),
  basis_seq INTEGER,
  server_seq INTEGER UNIQUE,
  UNIQUE (origin, seq),
  CHECK ((basis_t IS NULL) = (basis_origin IS NULL) AND (basis_t IS NULL) = (basis_seq IS NULL))
) STRICT;
CREATE INDEX review_log_card ON review_log (card_id, id);
CREATE INDEX review_log_day ON review_log (study_day);
CREATE TABLE review_retraction (
  review_id INTEGER PRIMARY KEY REFERENCES review_log (id),
  origin BLOB NOT NULL CHECK (length(origin) = 16),
  seq INTEGER NOT NULL,
  at_ms INTEGER NOT NULL,
  UNIQUE (origin, seq)
) STRICT;
-- the client record: 1 web, 2 iOS, 3 stock; no row reads unknown
CREATE TABLE review_client (
  review_id INTEGER PRIMARY KEY REFERENCES review_log (id),
  client INTEGER NOT NULL CHECK (client IN (1, 2, 3))
) STRICT;

-- device-local tables, never replicated
CREATE TABLE outbox (
  seq INTEGER PRIMARY KEY,
  kind INTEGER NOT NULL,
  payload TEXT NOT NULL CHECK (json_valid(payload)),
  made_ms INTEGER NOT NULL,
  server_seq INTEGER,
  outcome INTEGER CHECK (outcome IS NULL OR outcome IN (0, 1))
) STRICT;
CREATE TABLE applied_ops (
  origin BLOB NOT NULL,
  seq INTEGER NOT NULL,
  server_seq INTEGER,
  outcome INTEGER NOT NULL CHECK (outcome IN (0, 1)),
  PRIMARY KEY (origin, seq)
) STRICT, WITHOUT ROWID;
CREATE TABLE undo_journal (
  step INTEGER PRIMARY KEY,
  kind INTEGER NOT NULL,
  card_id INTEGER,
  review_id INTEGER,
  before TEXT NOT NULL CHECK (json_valid(before)),
  after TEXT NOT NULL CHECK (json_valid(after)),
  op_seq INTEGER NOT NULL,
  undone INTEGER NOT NULL CHECK (undone IN (0, 1))
) STRICT;
CREATE TABLE schedule (
  card_id INTEGER PRIMARY KEY,
  scheduler BLOB NOT NULL CHECK (length(scheduler) = 32),
  basis_review INTEGER,
  due_day INTEGER,
  due_ms INTEGER,
  order_key INTEGER,
  CHECK ((due_day IS NULL) <> (due_ms IS NULL))
) STRICT;
CREATE TABLE device_setting (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
) STRICT;

-- append-only: an update always aborts; a delete aborts unless the gate's one row reads open, so a
-- missing gate reads closed
CREATE TRIGGER review_log_no_update BEFORE UPDATE ON review_log
  BEGIN SELECT RAISE(ABORT, 'review_log is append-only'); END;
CREATE TRIGGER review_log_no_delete BEFORE DELETE ON review_log
  WHEN (SELECT open FROM erase_gate WHERE id = 1) IS NOT 1
  BEGIN SELECT RAISE(ABORT, 'review_log is append-only'); END;
CREATE TRIGGER review_retraction_no_update BEFORE UPDATE ON review_retraction
  BEGIN SELECT RAISE(ABORT, 'review_retraction is append-only'); END;
CREATE TRIGGER review_retraction_no_delete BEFORE DELETE ON review_retraction
  WHEN (SELECT open FROM erase_gate WHERE id = 1) IS NOT 1
  BEGIN SELECT RAISE(ABORT, 'review_retraction is append-only'); END;
CREATE TRIGGER review_client_no_update BEFORE UPDATE ON review_client
  BEGIN SELECT RAISE(ABORT, 'review_client is append-only'); END;
CREATE TRIGGER review_client_no_delete BEFORE DELETE ON review_client
  WHEN (SELECT open FROM erase_gate WHERE id = 1) IS NOT 1
  BEGIN SELECT RAISE(ABORT, 'review_client is append-only'); END;
CREATE TRIGGER note_field_history_no_update BEFORE UPDATE ON note_field_history
  BEGIN SELECT RAISE(ABORT, 'note_field_history is append-only'); END;
CREATE TRIGGER note_field_history_no_delete BEFORE DELETE ON note_field_history
  WHEN (SELECT open FROM erase_gate WHERE id = 1) IS NOT 1
  BEGIN SELECT RAISE(ABORT, 'note_field_history is append-only'); END;

-- the server's review row carries its server order; a device's row may hold none
CREATE TRIGGER review_log_server_order BEFORE INSERT ON review_log
  WHEN NEW.server_seq IS NULL AND (SELECT role FROM store_meta WHERE id = 1) IS NOT 1
  BEGIN SELECT RAISE(ABORT, 'a server''s review row needs its server_seq'); END;
