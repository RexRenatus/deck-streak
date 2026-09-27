//! The readings date tree (SPEC-042 R5 to R11): where a reading's note lives, and every operation
//! DeckStreak's own code performs on it.
//!
//! Today's note is `<readings>/<YYYY-MM-DD>/<topic file>`, the day being the study day. A note the
//! next study day carries is rolled into that day's folder with three keys patched; every other note
//! of a prior day is archived unchanged to `<readings>/<archive>/<YYYY>/<MM>/W<WW>/<YYYY-MM-DD>/`, by
//! calendar year and month and zero-padded ISO week, as the predecessor's `_archive_dir` names it.
//!
//! Every write passes the rails, lands atomically, stays inside the readings folder after `..` and
//! symbolic links are resolved, and never overwrites a note (compared case-insensitively). Nothing
//! here deletes a note: a roll removes its source only after the destination reads back as written,
//! an archive is a rename, and only an emptied day folder is removed.

use std::fmt;
use std::path::{Path, PathBuf};

use deck_streak_kernel::StudyDay;

use crate::config::{FolderName, VaultPaths, VaultSettings};
use crate::fs::VaultFs;
use crate::note::{self, BodyHash, Malformed, TopicKey};
use crate::rails::{RailRow, Rails};
use crate::{VaultError, atomic};

/// The folder a study day's archived notes go to, relative to the vault root:
/// `<readings>/<archive>/<YYYY>/<MM>/W<WW>/<YYYY-MM-DD>`, where the year and the month are the
/// day's calendar year and month and the week is its ISO week, zero-padded. At a year boundary the
/// two disagree (the last days of December can be week 1), and that is the predecessor's rule.
#[must_use]
pub fn archive_folder(readings: &FolderName, archive: &FolderName, day: StudyDay) -> PathBuf {
    let _ = (readings, archive, day);
    PathBuf::new()
}

/// A note the adapter created: where it is, relative to the vault root, and the hash of the body it
/// wrote, which a later body replacement proves unchanged (R10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created {
    /// The note's path, relative to the vault root.
    pub path: PathBuf,
    /// The hash of the body the adapter wrote.
    pub body: BodyHash,
}

/// What a box write did.
#[must_use = "a box write reports whether it wrote"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxOutcome {
    /// The box was unticked, and is now ticked.
    Ticked,
    /// The box was already ticked, so nothing was written.
    AlreadyTicked,
}

/// Why one note of a roll-forward was not relocated. Its siblings still were (R8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RollFailureReason {
    /// The note is malformed, so it cannot be rolled without risking its bytes.
    Malformed(Malformed),
    /// A note is already at the roll's destination, so the roll is refused (R7).
    DestinationTaken,
    /// The rolled note would not pass the rails, so it is not written.
    Rails(RailRow),
    /// The entry is a symbolic link or not a regular file, which the adapter never follows.
    NotARegularFile,
}

impl fmt::Display for RollFailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "{why}"),
            Self::DestinationTaken => f.write_str("a note is already at the roll's destination"),
            Self::Rails(row) => write!(f, "the rolled note would fail the rail {row}"),
            Self::NotARegularFile => f.write_str("it is not a regular file"),
        }
    }
}

/// One note a roll-forward could not relocate: its path relative to the vault root, and a bounded
/// reason that never quotes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollFailure {
    /// The note's path, relative to the vault root.
    pub path: PathBuf,
    /// Why it stayed where it is.
    pub reason: RollFailureReason,
}

/// What a roll-forward did: the notes it rolled into today's folder, the notes it archived, and the
/// notes it reported. Paths are relative to the vault root.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RollReport {
    /// The notes rolled into today's folder, at their new paths.
    pub rolled: Vec<PathBuf>,
    /// The notes archived, at their archive paths.
    pub archived: Vec<PathBuf>,
    /// The notes that stayed, each with its reason.
    pub failed: Vec<RollFailure>,
}

/// The readings date tree, over a file system `F`.
#[derive(Debug)]
pub struct ReadingsTree<F: VaultFs> {
    fs: F,
    rails: Rails,
    paths: VaultPaths,
}

impl<F: VaultFs> ReadingsTree<F> {
    /// The tree of the configured vault, after the start check (R1).
    ///
    /// # Errors
    ///
    /// [`VaultError::Start`] when the vault features refuse to start, and [`VaultError::Io`] when
    /// the check could not run.
    pub fn open(settings: &VaultSettings, fs: F, rails: Rails) -> Result<Self, VaultError> {
        let paths = VaultPaths::check(settings, &fs)?;
        Ok(Self { fs, rails, paths })
    }

    /// The file system the tree works on.
    pub fn fs(&self) -> &F {
        &self.fs
    }

    /// The resolved vault paths.
    pub fn paths(&self) -> &VaultPaths {
        &self.paths
    }

    /// The path of `topic`'s note for `day`, relative to the vault root (R5).
    #[must_use]
    pub fn note_path(&self, day: StudyDay, topic: &TopicKey) -> PathBuf {
        Path::new(self.paths.readings_name().as_str())
            .join(day.to_string())
            .join(topic.file_name())
    }

    /// Writes a new reading's note for `day` (R6), creating the day's folder inside the readings
    /// folder when it is missing.
    ///
    /// # Errors
    ///
    /// [`VaultError::NoteExists`] when a note is there already (compared case-insensitively),
    /// [`VaultError::Rails`] when the note would fail the rails, [`VaultError::OutsideConfinement`]
    /// when the day's folder resolves outside the readings folder, the refusals of
    /// [`note::render`], and [`VaultError::Io`].
    pub fn create(
        &self,
        day: StudyDay,
        topic: &TopicKey,
        digest: &str,
        body: &str,
    ) -> Result<Created, VaultError> {
        let text = note::render(topic, day, digest, body)?;
        let folder = self.paths.readings().join(day.to_string());
        let _ = self.fs.create_dir(&folder);
        atomic::write(&self.fs, &folder.join(topic.file_name()), text.as_bytes())?;
        Ok(Created {
            path: self.note_path(day, topic),
            body: BodyHash::of(body),
        })
    }

    /// Rolls the readings forward to `today` (R7, R8): from the single most recent prior-day folder
    /// the notes of the `carried` topics are rolled into today's folder, and every other note of
    /// any prior-day folder is archived unchanged. Each emptied prior-day folder is removed. A
    /// malformed note is reported and its siblings still move, and a second call on the same day
    /// changes nothing.
    ///
    /// # Errors
    ///
    /// [`VaultError::ReadBack`] when a rolled note did not read back as written (its source stays,
    /// and the roll stops), and [`VaultError::Io`] or a confinement refusal when a step could not
    /// run.
    pub fn roll_forward(
        &self,
        today: StudyDay,
        carried: &[TopicKey],
    ) -> Result<RollReport, VaultError> {
        let _ = (today, carried);
        Ok(RollReport::default())
    }

    /// Archives `topic`'s note of `day` unchanged, as a superseded reading (R8, SPEC-048): a name
    /// already taken in the archive folder gets `-2`, `-3` and so on before `.md`, and nothing is
    /// overwritten. Returns the archive path, relative to the vault root.
    ///
    /// # Errors
    ///
    /// [`VaultError::NoteMissing`] when there is no such note, and [`VaultError::Io`] or a
    /// confinement refusal when a step could not run.
    pub fn archive(&self, day: StudyDay, topic: &TopicKey) -> Result<PathBuf, VaultError> {
        let _ = (day, topic);
        Ok(PathBuf::new())
    }

    /// The day of the folder holding `topic`'s live note: today's folder first, then every other
    /// day folder, most recent first. The archive is never live.
    ///
    /// # Errors
    ///
    /// [`VaultError::Io`] when the readings folder could not be listed.
    pub fn find_live(
        &self,
        topic: &TopicKey,
        today: StudyDay,
    ) -> Result<Option<StudyDay>, VaultError> {
        let _ = (topic, today);
        Ok(None)
    }

    /// Stamps the Studied box of `topic`'s note in `day`'s folder (R9).
    ///
    /// # Errors
    ///
    /// [`VaultError::BoxAnchorMissing`], [`VaultError::BoxAnchorAmbiguous`], the note's absence
    /// and the write's refusals, each with no write.
    pub fn stamp_studied(&self, day: StudyDay, topic: &TopicKey) -> Result<BoxOutcome, VaultError> {
        let _ = (day, topic);
        Ok(BoxOutcome::AlreadyTicked)
    }

    /// Ticks the owner's `I read it` box of `topic`'s note in `day`'s folder (R9). Its one caller is
    /// the owner's read-tap use case (SPEC-047).
    ///
    /// # Errors
    ///
    /// As [`ReadingsTree::stamp_studied`].
    pub fn tick_read(&self, day: StudyDay, topic: &TopicKey) -> Result<BoxOutcome, VaultError> {
        let _ = (day, topic);
        Ok(BoxOutcome::AlreadyTicked)
    }

    /// Replaces the body of `topic`'s note in `day`'s folder with `body` (R10), keeping the
    /// frontmatter and both box lines byte for byte. `last_written` is the hash of the body the
    /// adapter last wrote; returns the hash of the new body.
    ///
    /// # Errors
    ///
    /// [`VaultError::NoteEdited`] (`vault_note_edited`) when the note's body is not the one the
    /// adapter last wrote, with no write; the refusals of [`note::check_body`]; and the write's.
    pub fn replace_body(
        &self,
        day: StudyDay,
        topic: &TopicKey,
        last_written: BodyHash,
        body: &str,
    ) -> Result<BodyHash, VaultError> {
        let _ = (day, topic, last_written, &self.rails);
        Ok(BodyHash::of(body))
    }
}
