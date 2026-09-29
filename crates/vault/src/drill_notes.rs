//! The drill notes' folder (SPEC-110 R1 to R7, R9), as its inert shape: listing and viewing the Active drills, reading
//! the Graded ones, and the one write the vault contract lets this context make, an answer's append
//! (ADR-011).
//!
//! The folder is the layout's `drill-coach` folder. A missing `Active` or `Graded` subfolder is an
//! empty list. A note that cannot be read is left out and logged by its error's type, never by its
//! path. A note is written only through the atomic writer and only after the rails pass it.

use std::path::PathBuf;

use crate::VaultError;
use crate::config::VaultSettings;
use crate::drill_store::Surface;
use crate::drills::{DrillMeta, DrillView, GradedDrill};
use crate::fs::VaultFs;
use crate::rails::Rails;
use deck_streak_kernel::{Db, StudyDay, UtcMillis};

/// What an answer came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// The answer was appended and its marker ticked; the drill's title, or its id.
    Appended {
        /// The drill's title.
        title: String,
    },
    /// The answer was empty or all space: nothing was read or written.
    EmptyAnswer,
    /// There is no such note in the Active folder.
    NotActive,
    /// The drill has an answer already, by its row or by its ticked marker.
    AlreadyAnswered,
    /// The rails refused the note as it would have been written: nothing was written.
    RailRefused,
}

/// The drill notes of one vault.
#[derive(Debug)]
pub struct DrillNotes<F: VaultFs> {
    fs: F,
    rails: Rails,
    folder: PathBuf,
}

impl<F: VaultFs> DrillNotes<F> {
    /// Opens the drill notes of the vault `settings` names.
    ///
    /// # Errors
    ///
    /// [`VaultError::Start`] when the vault root is not a folder.
    pub fn open(_settings: &VaultSettings, fs: F, rails: Rails) -> Result<Self, VaultError> {
        Ok(Self {
            fs,
            rails,
            folder: PathBuf::new(),
        })
    }

    /// Every Active drill.
    ///
    /// # Errors
    ///
    /// [`VaultError::Io`] when the folder cannot be listed.
    pub fn list_active(&self, _today: StudyDay) -> Result<Vec<DrillMeta>, VaultError> {
        let _ = (&self.fs, &self.rails, &self.folder);
        Ok(Vec::new())
    }

    /// The single view of the Active drill `id`.
    #[must_use]
    pub fn view(&self, _id: &str, _today: StudyDay) -> Option<DrillView> {
        None
    }

    /// The graded drills in the Graded folder.
    ///
    /// # Errors
    ///
    /// [`VaultError::Io`] when the folder cannot be listed.
    pub fn graded(&self) -> Result<Vec<GradedDrill>, VaultError> {
        Ok(Vec::new())
    }

    /// Answers the Active drill `id`.
    ///
    /// # Errors
    ///
    /// [`VaultError::Database`] when the row cannot be written.
    #[allow(clippy::too_many_arguments)]
    pub async fn answer(
        &self,
        _db: &Db,
        _id: &str,
        _answer: &str,
        _surface: Surface,
        _day: StudyDay,
        _when: &str,
        _at: UtcMillis,
    ) -> Result<AnswerOutcome, VaultError> {
        Ok(AnswerOutcome::NotActive)
    }
}
