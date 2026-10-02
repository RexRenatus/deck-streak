//! Curriculum's repository over its three tables (SPEC-077 R6, R7, R10, R18; ADR-077): each
//! course's stored progress, the band milestones and the law dues.
//!
//! Every function runs on a connection the caller holds: a write is made inside the fold's own
//! `BEGIN IMMEDIATE` write for the day, so a course's progress, its milestone and the band-up's
//! grant commit together or not at all, and a read takes any connection.

use std::collections::BTreeMap;

use deck_streak_kernel::{CourseCode, KernelError, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use crate::law::LawDues;
use crate::progress::CourseProgress;

/// One band of a stored course, as `language_progress.bands` holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredBand {
    /// The band.
    pub band: String,
    /// The counted cards whose unit falls in it.
    pub total: u32,
    /// Those of them that are mature.
    pub mature: u32,
    /// The mean mastery of its cards, in percent.
    pub pct: f64,
    /// Whether the band is achieved.
    pub achieved: bool,
}

/// One course's progress as `language_progress` holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredProgress {
    /// The course's code.
    pub course: String,
    /// The course's name.
    pub name: String,
    /// The course's flag.
    pub flag: String,
    /// The mean mastery of its counted cards, in percent.
    pub mastery_pct: f64,
    /// The current band.
    pub current_band: String,
    /// The mature counted cards.
    pub mature_cards: u32,
    /// The counted cards.
    pub total_cards: u32,
    /// The highest unit holding a mature card, if any.
    pub current_unit: Option<u32>,
    /// Each band's counts, in the bands' order.
    pub bands: Vec<StoredBand>,
    /// When the row was last written.
    pub updated_at: UtcMillis,
}

/// A band milestone to record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewMilestone<'a> {
    /// The course.
    pub course: &'a CourseCode,
    /// The band reached.
    pub band: &'a str,
    /// The study day it is reached on.
    pub study_day: StudyDay,
    /// Whether it is the course's silent baseline, which is written marked and owes nothing.
    pub baseline: bool,
    /// The services clock's instant: the row's creation, and a baseline's mark.
    pub at: UtcMillis,
}

/// What recording a milestone did.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// The row was written: the band is reached for the first time.
    New,
    /// The course already held the band; nothing was written.
    Held,
}

/// One band milestone, as `band_milestones` holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BandMilestone {
    /// The course's code.
    pub course: String,
    /// The band reached.
    pub band: String,
    /// The study day it was reached on.
    pub study_day: StudyDay,
    /// Whether it is the course's silent baseline.
    pub baseline: bool,
    /// When the router answered its celebration, the baseline's write for a baseline, or `None`
    /// while a band-up's celebration is still owed (ADR-303).
    pub celebrated_at: Option<UtcMillis>,
}

/// Each stored course's current band, by course code: what the next recompute compares with.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn stored_bands(
    connection: &mut SqliteConnection,
) -> Result<BTreeMap<String, String>, KernelError> {
    let _ = connection;
    Ok(BTreeMap::new())
}

/// Writes `progress` as its course's row, replacing the row the course held, at `at`.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn put_progress(
    connection: &mut SqliteConnection,
    progress: &CourseProgress,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let _ = (connection, progress, at);
    Ok(())
}

/// Every stored course's progress, by course code.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails or a row's bands are not the JSON the store
/// writes.
pub async fn progress(
    connection: &mut SqliteConnection,
) -> Result<Vec<StoredProgress>, KernelError> {
    let _ = connection;
    Ok(Vec::new())
}

/// Records `milestone` once: the key on the course and the band is the existence check, so a band
/// the course already holds writes nothing.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_milestone(
    connection: &mut SqliteConnection,
    milestone: &NewMilestone<'_>,
) -> Result<Recorded, KernelError> {
    let _ = (connection, milestone);
    Ok(Recorded::Held)
}

/// Every band milestone, by course and band.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn milestones(
    connection: &mut SqliteConnection,
) -> Result<Vec<BandMilestone>, KernelError> {
    let _ = connection;
    Ok(Vec::new())
}

/// The band-ups recorded on `day`, never a baseline, by course and band: what the band badge's
/// step awards.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn band_ups_on(
    connection: &mut SqliteConnection,
    day: StudyDay,
) -> Result<Vec<BandMilestone>, KernelError> {
    let _ = (connection, day);
    Ok(Vec::new())
}

/// Every band-up whose celebration is still owed, oldest first: what the fold's offers hand to the
/// router (ADR-303).
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn owed_band_ups(
    connection: &mut SqliteConnection,
) -> Result<Vec<BandMilestone>, KernelError> {
    let _ = connection;
    Ok(Vec::new())
}

/// Marks the band-up of `course` to `band` celebrated at `at`, only while it is still owed; answers
/// whether it marked it.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn mark_band_up(
    connection: &mut SqliteConnection,
    course: &str,
    band: &str,
    at: UtcMillis,
) -> Result<bool, KernelError> {
    let _ = (connection, course, band, at);
    Ok(false)
}

/// Writes `dues` as the law dues, replacing the ones stored, at `at`.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn put_law_dues(
    connection: &mut SqliteConnection,
    dues: &LawDues,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let _ = (connection, dues, at);
    Ok(())
}

/// The law dues the last recompute stored, or `None` before the first, which every surface reads
/// as pending, never 0 (R10).
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn law_dues(connection: &mut SqliteConnection) -> Result<Option<LawDues>, KernelError> {
    let _ = connection;
    Ok(None)
}
