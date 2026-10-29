//! The schema-18 adapter (SPEC-512 R6, R7; ADR-530 D8, D9): every statement ingest runs on Anki's
//! schema-18 file, moved here byte for byte from the seven files that ran them, and the read port's
//! implementation over the copy, open read-only and `query_only` through the collection's foreign
//! opener. The live readers import their statements from here and keep their own drivers; the port
//! is dormant until the cut-over switches them to it. Four statements read columns only Anki keeps
//! (the creation stamp, the cards' fingerprint, a card's scheduling state and the cards by queue and
//! type), so they serve the live readers alone and stay outside the port.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use deck_streak_collection::sql::Statement;
use deck_streak_collection::{
    CardRead, CollectionRead, FieldRead, ForeignFile, LogCounts, NoteRead, ReviewRead, StoreError,
    TemplateRead, scope_json,
};
use rusqlite::{Params, Row};

use crate::reader::is_study_event;

/// Every deck's id and stored name. The name is only ever read, by id: it is never compared,
/// ordered or aggregated in SQL (R4).
pub const DECK_NAMES: &str = "SELECT id, name FROM decks ORDER BY id";
/// The cards whose home deck (the original deck while a filtered deck borrows the card) is one of
/// the ids in `?1`, a JSON array: [`crate::reader::Card::home_deck_id`] in SQL, as the
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
/// weights are bound as `?1` to `?7`, in [`crate::gate::CARD_FIELD_WEIGHTS`]' order, and the
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

/// A register entry for the constant `$name`.
macro_rules! registered {
    ($name:ident) => {
        Statement {
            name: stringify!($name),
            sql: $name,
        }
    };
}

/// Every statement of this module: the 13 ingest held as constants and the six it wrote inline.
pub const STATEMENTS: &[Statement] = &[
    registered!(DECK_NAMES),
    registered!(CARDS),
    registered!(REVIEWS),
    registered!(CREATED),
    registered!(STUDY_EVENT_IDS),
    registered!(STUDY_EVENTS_BEFORE),
    registered!(NEWEST_REVIEW),
    registered!(CARD_FINGERPRINT),
    registered!(TEMPLATES),
    registered!(FIELDS),
    registered!(REVIEWED_NOTES),
    registered!(NOTE_FIELDS),
    registered!(CARD_STATE),
    registered!(STUDIED_AFTER),
    registered!(CARD_COUNT),
    registered!(NOTE_COUNT),
    registered!(REVIEW_ROW_COUNT),
    registered!(RESCHEDULE_ROW_COUNT),
    registered!(CARDS_BY_QUEUE_AND_TYPE),
];

/// The character that separates a note's fields in `notes.flds`.
const FIELD_SEPARATOR: char = '\u{1f}';

/// The read port over a schema-18 file.
#[derive(Debug)]
pub struct Schema18 {
    file: ForeignFile,
}

impl Schema18 {
    /// Opens the schema-18 file at `path` read-only and `query_only`.
    ///
    /// # Errors
    ///
    /// [`StoreError::StoreFile`] for a collection store, and [`StoreError::Sqlite`] when the file
    /// cannot be opened.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Ok(Self {
            file: ForeignFile::open_read_only(path)?,
        })
    }

    /// Prepares `statement` against the file without running it: the statement census's check.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when `SQLite` cannot prepare it.
    pub fn check_statement(&self, statement: &str) -> Result<(), StoreError> {
        self.file.check_statement(statement)
    }

    /// Runs `statement` with `params` and maps every row.
    fn rows<P: Params, T>(
        &self,
        statement: &str,
        params: P,
        map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, StoreError> {
        let mut prepared = self.file.connection().prepare_cached(statement)?;
        let mapped = prepared.query_map(params, map)?;
        Ok(mapped.collect::<Result<Vec<T>, _>>()?)
    }

    /// Runs `statement`, which answers one integer, with `params`.
    fn count<P: Params>(&self, statement: &str, params: P) -> Result<i64, StoreError> {
        Ok(self
            .file
            .connection()
            .prepare_cached(statement)?
            .query_row(params, |row| row.get(0))?)
    }
}

impl CollectionRead for Schema18 {
    fn deck_names(&self) -> Result<BTreeMap<i64, String>, StoreError> {
        Ok(self
            .rows(DECK_NAMES, [], |row| Ok((row.get(0)?, row.get(1)?)))?
            .into_iter()
            .collect())
    }

    fn cards_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<CardRead>, StoreError> {
        self.rows(CARDS, [scope_json(scope)], |row| {
            let deck_id: i64 = row.get(2)?;
            let original_deck_id: i64 = row.get(3)?;
            let tags: Option<String> = row.get(11)?;
            Ok(CardRead {
                id: row.get(0)?,
                note_id: row.get(1)?,
                home_deck_id: if original_deck_id != 0 {
                    original_deck_id
                } else {
                    deck_id
                },
                suspended: row.get::<_, i64>(4)? == -1,
                tags: tags
                    .as_deref()
                    .unwrap_or("")
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect(),
            })
        })
    }

    fn study_reviews_after(
        &self,
        floor: i64,
        scope: &BTreeSet<i64>,
    ) -> Result<Vec<ReviewRead>, StoreError> {
        Ok(self
            .rows(REVIEWS, (floor, scope_json(scope)), |row| {
                Ok(ReviewRead {
                    id: row.get(0)?,
                    card_id: row.get(1)?,
                    ease: row.get(2)?,
                    duration_ms: row.get(6)?,
                    kind: row.get(7)?,
                })
            })?
            .into_iter()
            .filter(|review| is_study_event(review.kind, review.ease))
            .collect())
    }

    fn study_review_ids_after(
        &self,
        floor: i64,
        scope: &BTreeSet<i64>,
        limit: i64,
    ) -> Result<Vec<i64>, StoreError> {
        self.rows(STUDY_EVENT_IDS, (floor, scope_json(scope), limit), |row| {
            row.get(0)
        })
    }

    fn study_reviews_through(&self, floor: i64, scope: &BTreeSet<i64>) -> Result<i64, StoreError> {
        self.count(STUDY_EVENTS_BEFORE, (floor, scope_json(scope)))
    }

    fn newest_review(&self) -> Result<Option<i64>, StoreError> {
        Ok(self
            .file
            .connection()
            .prepare_cached(NEWEST_REVIEW)?
            .query_row([], |row| row.get(0))?)
    }

    fn templates_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<TemplateRead>, StoreError> {
        self.rows(TEMPLATES, [scope_json(scope)], |row| {
            Ok(TemplateRead {
                notetype_id: row.get(0)?,
                ord: row.get(1)?,
                notetype_name: row.get(2)?,
                name: row.get(3)?,
            })
        })
    }

    fn fields_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<FieldRead>, StoreError> {
        self.rows(FIELDS, [scope_json(scope)], |row| {
            Ok(FieldRead {
                notetype_id: row.get(0)?,
                ord: row.get(1)?,
                name: row.get(2)?,
            })
        })
    }

    fn reviewed_notes(&self, scope: &BTreeSet<i64>) -> Result<BTreeSet<i64>, StoreError> {
        Ok(self
            .rows(REVIEWED_NOTES, [scope_json(scope)], |row| row.get(0))?
            .into_iter()
            .collect())
    }

    fn note_fields(&self, notes: &BTreeSet<i64>) -> Result<Vec<NoteRead>, StoreError> {
        let mut read = self.rows(NOTE_FIELDS, [scope_json(notes)], |row| {
            let fields: String = row.get(2)?;
            Ok(NoteRead {
                id: row.get(0)?,
                notetype_id: row.get(1)?,
                fields: fields.split(FIELD_SEPARATOR).map(str::to_owned).collect(),
            })
        })?;
        read.sort_by_key(|note| note.id);
        Ok(read)
    }

    fn studied_cards_after(&self, after: i64) -> Result<BTreeSet<i64>, StoreError> {
        Ok(self
            .rows(STUDIED_AFTER, [after], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .into_iter()
            .filter(|(_, kind, ease)| is_study_event(*kind, *ease))
            .map(|(card, _, _)| card)
            .collect())
    }

    fn counts(&self) -> Result<LogCounts, StoreError> {
        Ok(LogCounts {
            cards: self.count(CARD_COUNT, [])?,
            notes: self.count(NOTE_COUNT, [])?,
            review_rows: self.count(REVIEW_ROW_COUNT, [])?,
            reschedule_rows: self.count(RESCHEDULE_ROW_COUNT, [])?,
        })
    }
}
