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

pub use deck_streak_anki_format::statements::{
    CARD_COUNT, CARD_FINGERPRINT, CARD_STATE, CARDS, CARDS_BY_QUEUE_AND_TYPE, CREATED, DECK_NAMES,
    FIELDS, NEWEST_REVIEW, NOTE_COUNT, NOTE_FIELDS, RESCHEDULE_ROW_COUNT, REVIEW_ROW_COUNT,
    REVIEWED_NOTES, REVIEWS, STUDIED_AFTER, STUDY_EVENT_IDS, STUDY_EVENTS_BEFORE, TEMPLATES,
};

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
