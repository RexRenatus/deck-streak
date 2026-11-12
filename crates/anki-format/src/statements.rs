//! The 19 statements of ingest's schema-18 adapter (SPEC-602 R9, ADR-622 D4), moved here byte for byte
//! from `crates/ingest/src/schema18.rs`, which re-exports them, so every path that names them there
//! compiles unchanged. Each statement's SHA-256 is pinned in `crates/ingest/tests/collection_read.rs`.
//! The requirements the comments cite (R4, R7) are SPEC-512's.

/// Every deck's id and stored name. The name is only ever read, by id: it is never compared,
/// ordered or aggregated in SQL (R4).
pub const DECK_NAMES: &str = "SELECT id, name FROM decks ORDER BY id";
/// The cards whose home deck (the original deck while a filtered deck borrows the card) is one of
/// the ids in `?1`, a JSON array: `deck_streak_ingest::reader::Card::home_deck_id` in SQL, as the
/// predecessor's recount wrote it.
pub const CARDS: &str = "SELECT id, nid, did, odid, queue, type, due, ivl, factor, reps, lapses, \
     (SELECT n.tags FROM notes n WHERE n.id = cards.nid), cards.data \
     FROM cards WHERE (CASE WHEN odid != 0 THEN odid ELSE did END) IN (SELECT value FROM json_each(?1)) \
     ORDER BY id";
/// The revlog rows newer than the floor `?1` of the cards whose home deck is one of the ids in
/// `?2`, oldest first. The study-event rule is applied to them in Rust, not in SQL.
pub const REVIEWS: &str = "SELECT r.id, r.cid, r.ease, r.ivl, r.lastIvl, r.factor, r.time, r.type \
     FROM revlog r JOIN cards c ON c.id = r.cid \
     WHERE r.id > ?1 AND (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) \
     IN (SELECT value FROM json_each(?2)) ORDER BY r.id";
/// The collection's creation stamp, in epoch seconds (the predecessor's `read_config` reads
/// `col.crt`); no rollover hour is read with it (R7).
pub const CREATED: &str = "SELECT crt FROM col ORDER BY id LIMIT 1";
/// The review ids above the floor `?1` that are study events of the cards whose home deck is one
/// of the ids in `?2`, oldest first, `?3` at a time: the SQL mirror of `reader::is_study_event`
/// and `reader::Card::home_deck_id`, as [`STUDY_EVENTS_BEFORE`] writes its own count.
pub const STUDY_EVENT_IDS: &str = "SELECT r.id FROM revlog r JOIN cards c ON c.id = r.cid \
     WHERE r.id > ?1 AND r.type IN (0, 1, 2, 3) AND r.ease >= 1 \
     AND (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) IN (SELECT value FROM json_each(?2)) \
     ORDER BY r.id LIMIT ?3";
/// The study events at or before the floor `?1` of the cards whose home deck is one of the ids in
/// `?2`: the SQL mirror of `reader::is_study_event` and `reader::Card::home_deck_id`, as the
/// predecessor's `anki_reader.py:count_study_reviews_before` wrote it. A card deleted since takes its
/// reviews out of the count, which is what the self-check sees. `r.id <= ?1` is the exact complement
/// of the read's `r.id > ?1`, so a read above the floor and this count partition the log.
pub const STUDY_EVENTS_BEFORE: &str = "SELECT count(*) FROM revlog r JOIN cards c ON c.id = r.cid \
     WHERE r.id <= ?1 AND r.type IN (0, 1, 2, 3) AND r.ease >= 1 \
     AND (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) IN (SELECT value FROM json_each(?2))";
/// The newest revlog id, by the log's integer primary key.
pub const NEWEST_REVIEW: &str = "SELECT max(id) FROM revlog";
/// The cards' count and fingerprint: one scan of integer columns, each card's weighted term reduced
/// by the modulus before the sum, as the predecessor's `_CARDS_FINGERPRINT_SQL` computes it. The
/// weights are bound as `?1` to `?7`, in `deck_streak_ingest::gate::CARD_FIELD_WEIGHTS`' order, and the
/// modulus as `?8`: bound integers take the same integer arithmetic as the predecessor's literals.
pub const CARD_FINGERPRINT: &str = "SELECT count(id), coalesce(sum((due * ?1 + ivl * ?2 + queue * ?3 \
     + factor * ?4 + lapses * ?5 + type * ?6 + odid * ?7) % ?8), 0) FROM cards";
/// Each template of the note types in scope: note type, ordinal, names and raw config. The names are
/// read, never compared, ordered or joined on (R4).
pub const TEMPLATES: &str = "SELECT t.ntid, t.ord, n.name, t.name, t.config FROM templates t \
     JOIN notetypes n ON n.id = t.ntid \
     WHERE t.ntid IN (SELECT DISTINCT no.mid FROM notes no JOIN cards c ON c.nid = no.id \
     WHERE (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) IN (SELECT value FROM json_each(?1))) \
     ORDER BY t.ntid, t.ord";
/// Each declared field of the note types in scope.
pub const FIELDS: &str = "SELECT f.ntid, f.ord, f.name FROM fields f \
     WHERE f.ntid IN (SELECT DISTINCT no.mid FROM notes no JOIN cards c ON c.nid = no.id \
     WHERE (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) IN (SELECT value FROM json_each(?1))) \
     ORDER BY f.ntid, f.ord";
/// The notes of the cards in scope with a study review, as the predecessor's read wrote it.
pub const REVIEWED_NOTES: &str = "SELECT DISTINCT c.nid FROM cards c JOIN revlog r ON r.cid = c.id \
     WHERE r.type = 1 AND r.ease >= 1 \
     AND (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) IN (SELECT value FROM json_each(?1))";
/// The note type and fields of the notes in `?1`, a JSON array of ids.
pub const NOTE_FIELDS: &str =
    "SELECT id, mid, flds FROM notes WHERE id IN (SELECT value FROM json_each(?1))";
/// The columns a skip reads of each card a search selected, by the ids in `?1`, a JSON array,
/// ascending by id. The engine's card type keeps these fields private, so they are read through the
/// engine's own connection, which its storage hands out for exactly this.
pub const CARD_STATE: &str = "SELECT id, did, odid, due, queue, type, ivl, factor, odue, mod \
     FROM cards WHERE id IN (SELECT value FROM json_each(?1)) ORDER BY id";
/// The card, type and ease of every revlog row above `?1`: the skip's study check after a take.
pub const STUDIED_AFTER: &str = "SELECT cid, type, ease FROM revlog WHERE id > ?1";
/// Every card: one of the skip's counts.
pub const CARD_COUNT: &str = "SELECT count(*) FROM cards";
/// Every note: one of the skip's counts.
pub const NOTE_COUNT: &str = "SELECT count(*) FROM notes";
/// Every revlog row: one of the skip's counts.
pub const REVIEW_ROW_COUNT: &str = "SELECT count(*) FROM revlog";
/// The revlog's rescheduling rows, type 4 with an ease of 0: one of the skip's counts.
pub const RESCHEDULE_ROW_COUNT: &str = "SELECT count(*) FROM revlog WHERE type = 4 AND ease = 0";
/// The cards by queue and type: one of the skip's counts.
pub const CARDS_BY_QUEUE_AND_TYPE: &str =
    "SELECT queue, type, count(*) FROM cards GROUP BY queue, type ORDER BY queue, type";
