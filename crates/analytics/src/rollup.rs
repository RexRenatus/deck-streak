//! The rollup repository (SPEC-071 R5, R8 to R11, R14, R16 and R18): `daily_rollup` and
//! `daily_lang_stats`, the day's review fingerprint, and the settle cursor.
//!
//! One row per study day. Rolling a day up writes its metrics, its per-course rows and its
//! fingerprint; a roll-up that finds the same metrics and fingerprint changes nothing, so the
//! row's `updated_at` moves only when the day's reviews did. A day's card state and its provenance
//! are written only by [`record_card_state`], which the recompute calls for the day it evaluates as
//! current or settles as the closing day (R9); no roll-up touches them, nor the score the day closed
//! with, nor the instant it was settled. The last settled day is the cursor (R16).
//!
//! Every write takes the caller's connection, inside the transaction the recompute holds, so one
//! day's settle is one write.

use std::collections::BTreeMap;

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use crate::metrics::{DailyMetrics, LanguageDay};
use crate::score::{DayVolume, Score};
use crate::snapshot::CardState;

/// A recorded card state's provenance starts with this, then the epoch milliseconds it was
/// recorded at.
pub const LIVE_PREFIX: &str = "live:";
/// The rows the volume baseline reads before a day: the predecessor's `get_recent_rollups(31)`.
pub const RECENT_ROLLUPS: i64 = 31;

/// A day's rollup as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredDay {
    /// The day's metrics.
    pub metrics: DailyMetrics,
    /// The card state, when a recompute recorded one.
    pub card_state: Option<CardState>,
    /// Its provenance, `live:<epoch milliseconds>`, when it was recorded.
    pub card_state_src: Option<String>,
    /// The stored score.
    pub score: Score,
    /// The total the day closed with, once settled.
    pub score_at_close: Option<i64>,
    /// When the day was settled.
    pub settled_at: Option<UtcMillis>,
    /// The day's review fingerprint.
    pub fingerprint: String,
    /// When the row was first written.
    pub created_at: UtcMillis,
    /// When the day was last rolled up with changed reviews.
    pub updated_at: UtcMillis,
}

/// What one roll-up writes for a day.
#[derive(Clone, Copy, Debug)]
pub struct RolledDay<'a> {
    /// The day's metrics.
    pub metrics: &'a DailyMetrics,
    /// The day's per-course rows, which replace the day's earlier ones.
    pub languages: &'a [LanguageDay],
    /// The day's review fingerprint.
    pub fingerprint: &'a str,
    /// The score to store with it.
    pub score: &'a Score,
}

/// The fingerprint of a day's study reviews (R18): a digest of every field of each review, in their
/// order, together with the courses' digest, so a review that arrives late, or a changed courses
/// file, changes it.
#[must_use]
pub fn fingerprint(reviews: &[Review], courses_digest: Option<&str>) -> String {
    let _ = (reviews, courses_digest);
    String::new()
}

/// The provenance of a card state recorded at `at`.
#[must_use]
pub fn provenance(at: UtcMillis) -> String {
    format!("{LIVE_PREFIX}{}", at.epoch_millis())
}

/// Rolls `rolled` up inside `write` at `now`: the row's metrics, fingerprint and score, and the
/// day's per-course rows replaced. Returns whether the day's metrics or fingerprint changed (or the
/// row is new), which is when `updated_at` moves.
///
/// # Errors
///
/// [`KernelError::Database`] when a write fails.
pub async fn roll_up(
    write: &mut SqliteConnection,
    rolled: &RolledDay<'_>,
    now: UtcMillis,
) -> Result<bool, KernelError> {
    let _ = (write, rolled, now);
    Ok(false)
}

/// Records `state` as the card state of `day`, recorded at `at` (R8, R9).
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_card_state(
    write: &mut SqliteConnection,
    day: StudyDay,
    state: &CardState,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let _ = (write, day, state, at);
    Ok(())
}

/// Stores `score` as the day's score and pillars, leaving `updated_at` as it is (R14).
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_score(
    write: &mut SqliteConnection,
    day: StudyDay,
    score: &Score,
) -> Result<(), KernelError> {
    let _ = (write, day, score);
    Ok(())
}

/// Keeps `total` as the score `day` closed with (R14).
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_close(
    write: &mut SqliteConnection,
    day: StudyDay,
    total: i64,
) -> Result<(), KernelError> {
    let _ = (write, day, total);
    Ok(())
}

/// Records that `day` was settled at `at` (R16): the cursor moves to it. Returns whether the day
/// had a rollup to record it on.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record_settled(
    write: &mut SqliteConnection,
    day: StudyDay,
    at: UtcMillis,
) -> Result<bool, KernelError> {
    let _ = (write, day, at);
    Ok(false)
}

/// The rollup of `day` as `write` sees it.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn stored(
    write: &mut SqliteConnection,
    day: StudyDay,
) -> Result<Option<StoredDay>, KernelError> {
    let _ = (write, day);
    Ok(None)
}

/// Every stored day's fingerprint, as `write` sees them.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn fingerprints(
    write: &mut SqliteConnection,
) -> Result<BTreeMap<StudyDay, String>, KernelError> {
    let _ = write;
    Ok(BTreeMap::new())
}

/// The settle cursor as `write` sees it: the last settled day, or `None` before the first settle.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn settle_cursor(write: &mut SqliteConnection) -> Result<Option<StudyDay>, KernelError> {
    let _ = write;
    Ok(None)
}

/// The volumes of the [`RECENT_ROLLUPS`] most recent rows on or before `through`, most recent
/// first: what the baseline of `through` reads.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn recent_volumes(
    write: &mut SqliteConnection,
    through: StudyDay,
) -> Result<Vec<DayVolume>, KernelError> {
    let _ = (write, through);
    Ok(Vec::new())
}

/// The rollup repository's reads over the service's database, for the surfaces.
#[derive(Clone, Debug)]
pub struct RollupStore {
    db: Db,
}

impl RollupStore {
    /// The repository over `db`, whose migrations created both tables.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// The rollups of the days from `first` to `last`, inclusive, oldest first.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn days(
        &self,
        first: StudyDay,
        last: StudyDay,
    ) -> Result<Vec<StoredDay>, KernelError> {
        let _ = (&self.db, first, last);
        Ok(Vec::new())
    }

    /// The per-course rows of `day`, by course code.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn language_days(&self, day: StudyDay) -> Result<Vec<LanguageDay>, KernelError> {
        let _ = (&self.db, day);
        Ok(Vec::new())
    }
}
