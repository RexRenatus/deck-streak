//! The schema-18 adapter (SPEC-512 R6, R7; ADR-530 D8, D9): every statement ingest runs on Anki's
//! schema-18 file, moved here byte for byte from the seven files that ran them, and the read port's
//! implementation over the copy, open read-only and `query_only` through the collection's foreign
//! opener. The live readers import their statements from here and keep their own drivers; the port
//! is dormant until the cut-over switches them to it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use deck_streak_collection::sql::Statement;
use deck_streak_collection::{
    CardRead, CollectionRead, FieldRead, ForeignFile, LogCounts, NoteRead, ReviewRead, StoreError,
    TemplateRead,
};

/// Every statement of this module.
pub const STATEMENTS: &[Statement] = &[];

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
}

impl CollectionRead for Schema18 {
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
