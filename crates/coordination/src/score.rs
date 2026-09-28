//! The owner's score reads (SPEC-071 R10, R20, R21): a study day's score, and the rollups of a
//! range of study days, read once here for the API's analytics routes and the bot's `/score`, so
//! the numbers the two surfaces show cannot drift.
//!
//! A day whose card state no recompute recorded reads as no card state, and a day with no answered
//! review as no retention, never as 0: the rollup stores the predecessor's 0 for both pillars'
//! inputs (R10), and a surface that showed it would claim a measurement nobody made. Every rule is
//! analytics' own; this module only reads and says what is absent.

use deck_streak_analytics::metrics::DailyMetrics;
use deck_streak_analytics::rollup::{RollupStore, StoredDay};
use deck_streak_analytics::snapshot::CardState;
use deck_streak_ingest::window::INGEST_WINDOW_DAYS;
use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};

/// The most study days one read of the rollups spans: the window's length (SPEC-023).
pub const MAX_RANGE_DAYS: i64 = INGEST_WINDOW_DAYS;

/// A day's five pillars as the surfaces show them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pillars {
    /// Showing up, and the streak.
    pub consistency: f64,
    /// True retention against its floor and ceiling; absent on a day with no answered review.
    pub retention: Option<f64>,
    /// The due cards cleared, less the backlog.
    pub workload: f64,
    /// Reviews and minutes against the baseline.
    pub volume: f64,
    /// Graduations, less the leeches.
    pub mastery: f64,
}

/// A study day's score as the surfaces show it (R20, R21): the total, its grade band, the five
/// pillars, and the day's reviews and true retention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DayScore {
    /// The day scored.
    pub day: StudyDay,
    /// The weighted total, 0 to 100.
    pub total: i64,
    /// The grade band's label.
    pub grade_label: &'static str,
    /// The grade band's emoji.
    pub grade_emoji: &'static str,
    /// The five pillars.
    pub pillars: Pillars,
    /// The day's study reviews.
    pub reviews: i64,
    /// The day's true retention, in percent; absent on a day with no answered review.
    pub retention: Option<f64>,
}

/// A study day's rollup as the surfaces show it (R20): its metrics, its card state and that
/// state's provenance when a recompute recorded one, its score, and its settle.
#[derive(Clone, Debug, PartialEq)]
pub struct DayRollup {
    /// The day's metrics as stored; their true retention is shown through [`DayScore::retention`].
    pub metrics: DailyMetrics,
    /// The card state, when a recompute recorded one.
    pub card_state: Option<CardState>,
    /// Its provenance, `live:<epoch milliseconds>`, when it was recorded.
    pub card_state_src: Option<String>,
    /// The day's score.
    pub score: DayScore,
    /// The total the day closed with, once settled.
    pub score_at_close: Option<i64>,
    /// When the day was settled.
    pub settled_at: Option<UtcMillis>,
}

/// Why a range of rollups cannot be read.
#[derive(Debug, thiserror::Error)]
pub enum RangeError {
    /// The range ends before it starts.
    #[error("the range ends before it starts")]
    Backwards,
    /// The range spans more study days than the window holds.
    #[error("the range spans more than {MAX_RANGE_DAYS} study days")]
    TooLong,
    /// The rollups could not be read.
    #[error(transparent)]
    Read(#[from] KernelError),
}

/// The score of `day`, or `None` while no recompute has rolled it up.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn day_score(db: &Db, day: StudyDay) -> Result<Option<DayScore>, KernelError> {
    let days = RollupStore::new(db.clone()).days(day, day).await?;
    Ok(days.first().map(score_of))
}

/// The rollups of the study days from `first` to `last`, inclusive, oldest first: a day no
/// recompute rolled up has none.
///
/// # Errors
///
/// [`RangeError::Backwards`] and [`RangeError::TooLong`] for a range the window cannot hold, and
/// [`RangeError::Read`] when the read fails.
pub async fn day_rollups(
    db: &Db,
    first: StudyDay,
    last: StudyDay,
) -> Result<Vec<DayRollup>, RangeError> {
    if last < first {
        return Err(RangeError::Backwards);
    }
    if last.epoch_day() - first.epoch_day() >= MAX_RANGE_DAYS {
        return Err(RangeError::TooLong);
    }
    let days = RollupStore::new(db.clone()).days(first, last).await?;
    Ok(days.into_iter().map(rollup_of).collect())
}

/// `stored`'s score, with its retention absent when the day had no answered review (R10).
fn score_of(stored: &StoredDay) -> DayScore {
    let answered = stored.metrics.answered > 0;
    DayScore {
        day: stored.metrics.day,
        total: stored.score.total,
        grade_label: stored.score.grade_label,
        grade_emoji: stored.score.grade_emoji,
        pillars: Pillars {
            consistency: stored.score.consistency,
            retention: answered.then_some(stored.score.retention),
            workload: stored.score.workload,
            volume: stored.score.volume,
            mastery: stored.score.mastery,
        },
        reviews: stored.metrics.reviews,
        retention: answered.then_some(stored.metrics.true_retention),
    }
}

/// `stored` as the surfaces show it: a card state only when a recompute recorded one (R9, R10).
fn rollup_of(stored: StoredDay) -> DayRollup {
    let score = score_of(&stored);
    DayRollup {
        metrics: stored.metrics,
        card_state: stored.card_state,
        card_state_src: stored.card_state_src,
        score,
        score_at_close: stored.score_at_close,
        settled_at: stored.settled_at,
    }
}
