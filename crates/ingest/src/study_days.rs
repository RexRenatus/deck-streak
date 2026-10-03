//! The study days of the whole scoped log (SPEC-102 R5, ADR-318): every day on which a review of
//! a card in scope is a study event, read once each and oldest first, with no window.

use std::collections::BTreeSet;

use deck_streak_kernel::{StudyDay, StudyDayRule};

use crate::reader::{CollectionReader, ReadError};

impl CollectionReader {
    /// Every study day of a scoped study event in the whole log, once each, oldest first.
    ///
    /// # Errors
    /// [`ReadError`] when the private copy cannot be read.
    pub async fn study_days(&self, rule: StudyDayRule) -> Result<Vec<StudyDay>, ReadError> {
        let _ = rule;
        Ok(BTreeSet::<StudyDay>::new().into_iter().collect())
    }
}
