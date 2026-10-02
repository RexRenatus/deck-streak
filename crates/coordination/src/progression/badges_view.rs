//! The badges view (SPEC-073 R16, R18): the earned badges, most recently awarded first, and the
//! catalog badges not yet earned, each with its criteria and, where its input is stored, that
//! input's value against its threshold. The API's `GET /api/badges` and the bot's `/badges` both
//! read it, so the two surfaces cannot drift.
//!
//! It only reads. A locked badge's progress is shown only where the input its condition reads is
//! stored: the language streak, today's study reviews, decks and score, and the day's recorded
//! mature cards. The lifetime reviews, the hour, week and backlog counts and the two-input
//! conditions are never stored, so those badges carry no progress (#560), and a habit or focus
//! badge carries none until its context supplies it (#93, #94).

use deck_streak_kernel::{Courses, Db, KernelError, StudyDay, UtcMillis};

/// One earned badge as the surfaces show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EarnedLine {
    /// The key.
    pub key: String,
    /// The tier.
    pub tier: u32,
    /// The name.
    pub name: String,
    /// The emoji.
    pub emoji: String,
    /// The study day that earned it.
    pub study_day: StudyDay,
    /// When it was awarded.
    pub awarded_at: UtcMillis,
}

/// A locked badge's input against the threshold that earns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// The input's stored value.
    pub value: i64,
    /// The value that earns the badge.
    pub threshold: i64,
}

/// One catalog badge not yet earned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockedLine {
    /// The key.
    pub key: String,
    /// The name.
    pub name: String,
    /// The emoji.
    pub emoji: String,
    /// The criterion in words.
    pub criteria: String,
    /// The family that decides it: `study`, `habit` or `focus`.
    pub family: &'static str,
    /// Its input against its threshold, when the input is stored.
    pub progress: Option<Progress>,
}

/// The badges view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BadgesView {
    /// Every earned badge, most recently awarded first.
    pub earned: Vec<EarnedLine>,
    /// Every catalog badge with no earned tier, in the catalog's order.
    pub locked: Vec<LockedLine>,
}

/// The stored input a locked study badge's progress reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// The language streak's current run.
    Streak,
    /// Today's study reviews.
    DayReviews,
    /// The decks studied today.
    DayDecks,
    /// Today's score.
    DayScore,
    /// The mature cards of today's recorded card state.
    MatureCards,
}

/// The study badges whose input is stored, each with the input its progress reads.
pub const PROGRESS_INPUTS: &[(&str, Input)] = &[];

/// The catalog badges that carry no progress: the study badges whose input is not stored, and the
/// habit and focus badges, whose contexts supply none yet.
pub const WITHOUT_PROGRESS: &[&str] = &[];

/// What a locked badge's progress is read from, as stored for the day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProgressInputs {
    /// The language streak's current run.
    pub streak: i64,
    /// Today's study reviews.
    pub day_reviews: i64,
    /// The decks studied today.
    pub day_decks: i64,
    /// Today's score.
    pub day_score: i64,
    /// The mature cards, when today recorded a card state.
    pub mature_cards: Option<i64>,
}

/// `earned`, most recently awarded first: newest award first, then by key and tier.
#[must_use]
pub fn most_recent_first(earned: Vec<EarnedLine>) -> Vec<EarnedLine> {
    let _ = earned;
    Vec::new()
}

/// The progress of the locked badge `key` over `inputs`, or `None` when its input is not stored.
#[must_use]
pub fn progress(key: &str, inputs: &ProgressInputs) -> Option<Progress> {
    let _ = (key, inputs);
    None
}

/// The catalog badges under `courses` that `earned` holds no tier of, in the catalog's order, each
/// with its criteria and its progress over `inputs`.
#[must_use]
pub fn locked(
    courses: &Courses,
    earned: &[EarnedLine],
    inputs: &ProgressInputs,
) -> Vec<LockedLine> {
    let _ = (courses, earned, inputs);
    Vec::new()
}

/// Every earned badge, most recently awarded first.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn earned_badges(db: &Db) -> Result<Vec<EarnedLine>, KernelError> {
    let _ = db;
    Ok(Vec::new())
}

/// The badges view for `today` under `courses` (R16).
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn badges_view(
    db: &Db,
    today: StudyDay,
    courses: &Courses,
) -> Result<BadgesView, KernelError> {
    let _ = (db, today, courses);
    Ok(BadgesView::default())
}
