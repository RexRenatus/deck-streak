//! The read port (SPEC-512 R6, R7; ADR-530 D9): `CollectionRead`, what the server's contexts read
//! of a collection, in domain values both a store and Anki's schema-18 file hold, and the store's
//! own adapter over a [`Snapshot`].

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, Params, Row};

use crate::error::StoreError;
use crate::sql;
use crate::store::Snapshot;

/// One card of a deck scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardRead {
    /// The card's id.
    pub id: i64,
    /// The card's note.
    pub note_id: i64,
    /// The card's home deck: the deck a filtered deck borrowed it from, else the deck it sits in.
    pub home_deck_id: i64,
    /// Whether the card is suspended.
    pub suspended: bool,
    /// The note's tags.
    pub tags: BTreeSet<String>,
}

/// One study review: a review of kind 0 to 3 with an ease of 1 or more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewRead {
    /// The review's id, the instant it was answered in epoch milliseconds.
    pub id: i64,
    /// The card answered.
    pub card_id: i64,
    /// The answer button, 1 to 4.
    pub ease: i64,
    /// How long the answer took, in milliseconds.
    pub duration_ms: i64,
    /// The review's kind: 0 learn, 1 review, 2 relearn, 3 filtered.
    pub kind: i64,
}

/// One template of a notetype.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateRead {
    /// The template's notetype.
    pub notetype_id: i64,
    /// The template's ordinal.
    pub ord: i64,
    /// The notetype's name.
    pub notetype_name: String,
    /// The template's name.
    pub name: String,
}

/// One declared field of a notetype.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRead {
    /// The field's notetype.
    pub notetype_id: i64,
    /// The field's ordinal.
    pub ord: i64,
    /// The field's name.
    pub name: String,
}

/// One note's fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteRead {
    /// The note's id.
    pub id: i64,
    /// The note's notetype.
    pub notetype_id: i64,
    /// The note's fields, in ordinal order, as written.
    pub fields: Vec<String>,
}

/// The collection's counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogCounts {
    /// Every card.
    pub cards: i64,
    /// Every note.
    pub notes: i64,
    /// Every row of the review log.
    pub review_rows: i64,
    /// The review log's rescheduling rows: kind 4 with an ease of 0.
    pub reschedule_rows: i64,
}

/// What the server's contexts read of a collection, from the store or from Anki's schema-18 file.
/// A deck scope is a set of home deck ids; a review's deck is its card's current home deck.
pub trait CollectionRead {
    /// Every deck's name by its id, its parts separated by U+001F.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn deck_names(&self) -> Result<BTreeMap<i64, String>, StoreError>;

    /// The cards whose home deck is in `scope`, by id.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn cards_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<CardRead>, StoreError>;

    /// The study reviews above `floor` of the cards in `scope`, oldest first.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn study_reviews_after(
        &self,
        floor: i64,
        scope: &BTreeSet<i64>,
    ) -> Result<Vec<ReviewRead>, StoreError>;

    /// The ids of the study reviews above `floor` of the cards in `scope`, oldest first, at most
    /// `limit` of them.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn study_review_ids_after(
        &self,
        floor: i64,
        scope: &BTreeSet<i64>,
        limit: i64,
    ) -> Result<Vec<i64>, StoreError>;

    /// How many study reviews at or below `floor` the cards in `scope` have.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn study_reviews_through(&self, floor: i64, scope: &BTreeSet<i64>) -> Result<i64, StoreError>;

    /// The newest review's id, or none for an empty log.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn newest_review(&self) -> Result<Option<i64>, StoreError>;

    /// Every template of the notetypes the cards in `scope` use, by notetype and ordinal.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn templates_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<TemplateRead>, StoreError>;

    /// Every declared field of the notetypes the cards in `scope` use, by notetype and ordinal.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn fields_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<FieldRead>, StoreError>;

    /// The notes of the cards in `scope` with a study review of kind 1.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn reviewed_notes(&self, scope: &BTreeSet<i64>) -> Result<BTreeSet<i64>, StoreError>;

    /// The fields of the notes in `notes`, by id.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn note_fields(&self, notes: &BTreeSet<i64>) -> Result<Vec<NoteRead>, StoreError>;

    /// The cards with a study review above `after`.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn studied_cards_after(&self, after: i64) -> Result<BTreeSet<i64>, StoreError>;

    /// The collection's counts.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the file cannot be read.
    fn counts(&self) -> Result<LogCounts, StoreError>;
}

/// `ids` as the JSON array a statement's `json_each` expands: the ids are bound, never written into
/// the SQL.
#[must_use]
pub fn scope_json(ids: &BTreeSet<i64>) -> String {
    let ids: Vec<String> = ids.iter().map(i64::to_string).collect();
    format!("[{}]", ids.join(","))
}
/// Runs `statement` with `params` on `conn` and maps every row.
fn rows<P, T>(
    conn: &Connection,
    statement: &str,
    params: P,
    map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, StoreError>
where
    P: Params,
{
    let mut prepared = conn.prepare_cached(statement)?;
    let mapped = prepared.query_map(params, map)?;
    Ok(mapped.collect::<Result<Vec<T>, _>>()?)
}

/// Runs `statement`, which answers one integer, with `params` on `conn`.
fn count<P: Params>(conn: &Connection, statement: &str, params: P) -> Result<i64, StoreError> {
    Ok(conn
        .prepare_cached(statement)?
        .query_row(params, |row| row.get(0))?)
}

impl CollectionRead for Snapshot<'_> {
    fn deck_names(&self) -> Result<BTreeMap<i64, String>, StoreError> {
        Ok(rows(self.conn(), sql::STORE_DECK_NAMES, [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .into_iter()
        .collect())
    }

    fn cards_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<CardRead>, StoreError> {
        let scope = scope_json(scope);
        let mut tags: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
        for (note, tag) in rows(self.conn(), sql::STORE_CARD_TAGS, [&scope], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })? {
            tags.entry(note).or_default().insert(tag);
        }
        rows(self.conn(), sql::STORE_CARDS, [&scope], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map(|cards| {
            cards
                .into_iter()
                .map(|(id, note_id, home_deck_id, suspended)| CardRead {
                    id,
                    note_id,
                    home_deck_id,
                    suspended: suspended == 1,
                    tags: tags.get(&note_id).cloned().unwrap_or_default(),
                })
                .collect()
        })
    }

    fn study_reviews_after(
        &self,
        floor: i64,
        scope: &BTreeSet<i64>,
    ) -> Result<Vec<ReviewRead>, StoreError> {
        rows(
            self.conn(),
            sql::STORE_STUDY_REVIEWS,
            (floor, scope_json(scope)),
            |row| {
                Ok(ReviewRead {
                    id: row.get(0)?,
                    card_id: row.get(1)?,
                    ease: row.get(2)?,
                    duration_ms: row.get(3)?,
                    kind: row.get(4)?,
                })
            },
        )
    }

    fn study_review_ids_after(
        &self,
        floor: i64,
        scope: &BTreeSet<i64>,
        limit: i64,
    ) -> Result<Vec<i64>, StoreError> {
        rows(
            self.conn(),
            sql::STORE_STUDY_REVIEW_IDS,
            (floor, scope_json(scope), limit),
            |row| row.get(0),
        )
    }

    fn study_reviews_through(&self, floor: i64, scope: &BTreeSet<i64>) -> Result<i64, StoreError> {
        count(
            self.conn(),
            sql::STORE_STUDY_REVIEWS_THROUGH,
            (floor, scope_json(scope)),
        )
    }

    fn newest_review(&self) -> Result<Option<i64>, StoreError> {
        Ok(self
            .conn()
            .prepare_cached(sql::STORE_NEWEST_REVIEW)?
            .query_row([], |row| row.get(0))?)
    }

    fn templates_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<TemplateRead>, StoreError> {
        rows(
            self.conn(),
            sql::STORE_TEMPLATES,
            [scope_json(scope)],
            |row| {
                Ok(TemplateRead {
                    notetype_id: row.get(0)?,
                    ord: row.get(1)?,
                    notetype_name: row.get(2)?,
                    name: row.get(3)?,
                })
            },
        )
    }

    fn fields_in(&self, scope: &BTreeSet<i64>) -> Result<Vec<FieldRead>, StoreError> {
        rows(self.conn(), sql::STORE_FIELDS, [scope_json(scope)], |row| {
            Ok(FieldRead {
                notetype_id: row.get(0)?,
                ord: row.get(1)?,
                name: row.get(2)?,
            })
        })
    }

    fn reviewed_notes(&self, scope: &BTreeSet<i64>) -> Result<BTreeSet<i64>, StoreError> {
        Ok(rows(
            self.conn(),
            sql::STORE_REVIEWED_NOTES,
            [scope_json(scope)],
            |row| row.get(0),
        )?
        .into_iter()
        .collect())
    }

    fn note_fields(&self, notes: &BTreeSet<i64>) -> Result<Vec<NoteRead>, StoreError> {
        let mut read: Vec<NoteRead> = Vec::new();
        for (id, notetype_id, field) in rows(
            self.conn(),
            sql::STORE_NOTE_FIELDS,
            [scope_json(notes)],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )? {
            match read.last_mut() {
                Some(note) if note.id == id => note.fields.push(field),
                _ => read.push(NoteRead {
                    id,
                    notetype_id,
                    fields: vec![field],
                }),
            }
        }
        Ok(read)
    }

    fn studied_cards_after(&self, after: i64) -> Result<BTreeSet<i64>, StoreError> {
        Ok(rows(self.conn(), sql::STORE_STUDIED_CARDS, [after], |row| {
            row.get(0)
        })?
        .into_iter()
        .collect())
    }

    fn counts(&self) -> Result<LogCounts, StoreError> {
        Ok(LogCounts {
            cards: count(self.conn(), sql::STORE_CARD_COUNT, [])?,
            notes: count(self.conn(), sql::STORE_NOTE_COUNT, [])?,
            review_rows: count(self.conn(), sql::STORE_REVIEW_ROW_COUNT, [])?,
            reschedule_rows: count(self.conn(), sql::STORE_RESCHEDULE_ROW_COUNT, [])?,
        })
    }
}
