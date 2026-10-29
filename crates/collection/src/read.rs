//! The read port (SPEC-512 R6, R7; ADR-530 D9): `CollectionRead`, what the server's contexts read
//! of a collection, in domain values both a store and Anki's schema-18 file hold, and the store's
//! own adapter over a [`Snapshot`].

use std::collections::{BTreeMap, BTreeSet};

use crate::error::StoreError;
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

impl CollectionRead for Snapshot<'_> {
    fn deck_names(&self) -> Result<BTreeMap<i64, String>, StoreError> {
        Ok(BTreeMap::new())
    }

    fn cards_in(&self, _scope: &BTreeSet<i64>) -> Result<Vec<CardRead>, StoreError> {
        Ok(Vec::new())
    }

    fn study_reviews_after(
        &self,
        _floor: i64,
        _scope: &BTreeSet<i64>,
    ) -> Result<Vec<ReviewRead>, StoreError> {
        Ok(Vec::new())
    }

    fn study_review_ids_after(
        &self,
        _floor: i64,
        _scope: &BTreeSet<i64>,
        _limit: i64,
    ) -> Result<Vec<i64>, StoreError> {
        Ok(Vec::new())
    }

    fn study_reviews_through(
        &self,
        _floor: i64,
        _scope: &BTreeSet<i64>,
    ) -> Result<i64, StoreError> {
        Ok(0)
    }

    fn newest_review(&self) -> Result<Option<i64>, StoreError> {
        Ok(None)
    }

    fn templates_in(&self, _scope: &BTreeSet<i64>) -> Result<Vec<TemplateRead>, StoreError> {
        Ok(Vec::new())
    }

    fn fields_in(&self, _scope: &BTreeSet<i64>) -> Result<Vec<FieldRead>, StoreError> {
        Ok(Vec::new())
    }

    fn reviewed_notes(&self, _scope: &BTreeSet<i64>) -> Result<BTreeSet<i64>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn note_fields(&self, _notes: &BTreeSet<i64>) -> Result<Vec<NoteRead>, StoreError> {
        Ok(Vec::new())
    }

    fn studied_cards_after(&self, _after: i64) -> Result<BTreeSet<i64>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn counts(&self) -> Result<LogCounts, StoreError> {
        Ok(LogCounts {
            cards: 0,
            notes: 0,
            review_rows: 0,
            reschedule_rows: 0,
        })
    }
}
