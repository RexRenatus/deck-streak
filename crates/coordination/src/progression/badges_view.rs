//! The badges view (SPEC-073 R16, R18): the earned badges, most recently awarded first, and the
//! catalog badges not yet earned, each with its criteria and, where its input is stored, that
//! input's value against its threshold. The API's `GET /api/badges` and the bot's `/badges` both
//! read it, so the two surfaces cannot drift.
//!
//! It only reads. A locked badge's progress is shown only where the input its condition reads is
//! stored: the language streak, today's study reviews, decks and score, and the day's recorded
//! mature cards. The lifetime reviews, the hour, week and backlog counts and the two-input
//! conditions are never stored, so those badges carry no progress (#560), and a habit or focus
//! badge carries none until its context supplies it (#93, #94, #98).

use std::cmp::Reverse;
use std::collections::BTreeSet;

use deck_streak_kernel::{Courses, Db, KernelError, StudyDay, UtcMillis};
use deck_streak_progression::badges::catalog::{Family, catalog};
use deck_streak_progression::badges::conditions::{
    CENTURION_DAY_REVIEWS, FOREST_GUARDIAN_COUNT, LEGENDARY_DAY_SCORE, MATURITY_MILESTONE_COUNT,
    POLYGLOT_DECKS_DAY,
};

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
pub const PROGRESS_INPUTS: &[(&str, Input)] = &[
    ("week_warrior", Input::Streak),
    ("monthly_monk", Input::Streak),
    ("century_flame", Input::Streak),
    ("year_of_iron", Input::Streak),
    ("centurion_day", Input::DayReviews),
    ("maturity_milestone", Input::MatureCards),
    ("forest_guardian", Input::MatureCards),
    ("polyglot", Input::DayDecks),
    ("legendary_day", Input::DayScore),
];

/// The catalog badges that carry no progress: the study badges whose input is not stored, and the
/// habit and focus badges, whose contexts supply none yet.
pub const WITHOUT_PROGRESS: &[&str] = &[
    // The study badges whose input is never stored: lifetime reviews, the hour, the week, the
    // backlog and the two-input conditions (#560).
    "first_steps",
    "grinder",
    "marathoner",
    "sharpshooter",
    "sniper_elite",
    "inbox_zero",
    "backlog_slayer",
    "night_owl",
    "early_bird",
    "comeback_kid",
    "leech_tamer",
    "globetrotter",
    "perfect_week",
    "speed_demon",
    "iron_will",
    // The habit badges, decided by the reading and writing habit contexts (#93, #94).
    "first_page",
    "quill_initiate",
    "ink_week",
    "ink_month",
    "ink_century",
    "bookworm_week",
    "polyglot_reader",
    "marathon_reader",
    // The focus badges, decided by the focus context (#98).
    "focus_initiate",
    "deep_work_day",
    "deep_diver",
    "focus_week",
    "focus_month",
    "monk_mode",
    "deep_work_centurion",
    "subject_devotee",
];

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
pub fn most_recent_first(mut earned: Vec<EarnedLine>) -> Vec<EarnedLine> {
    earned.sort_by(|left, right| {
        (Reverse(left.awarded_at), &left.key, left.tier).cmp(&(
            Reverse(right.awarded_at),
            &right.key,
            right.tier,
        ))
    });
    earned
}

/// The progress of the locked badge `key` over `inputs`, or `None` when its input is not stored.
#[must_use]
pub fn progress(key: &str, inputs: &ProgressInputs) -> Option<Progress> {
    let (_, input) = PROGRESS_INPUTS.iter().find(|(listed, _)| *listed == key)?;
    let (value, threshold) = match input {
        // Progression writes the four streak thresholds inline in its conditions, so they are
        // written here as literals and a test pins each to the boundary its condition holds at.
        Input::Streak => (inputs.streak, streak_threshold(key)?),
        Input::DayReviews => (inputs.day_reviews, wide(CENTURION_DAY_REVIEWS)?),
        Input::DayDecks => (inputs.day_decks, wide(POLYGLOT_DECKS_DAY)?),
        Input::DayScore => (inputs.day_score, LEGENDARY_DAY_SCORE),
        Input::MatureCards => (inputs.mature_cards?, mature_threshold(key)?),
    };
    Some(Progress { value, threshold })
}

/// The streak a streak badge's condition holds at.
fn streak_threshold(key: &str) -> Option<i64> {
    match key {
        "week_warrior" => Some(7),
        "monthly_monk" => Some(30),
        "century_flame" => Some(100),
        "year_of_iron" => Some(365),
        _ => None,
    }
}

/// The mature cards a mature-card badge's condition holds at.
fn mature_threshold(key: &str) -> Option<i64> {
    match key {
        "maturity_milestone" => Some(MATURITY_MILESTONE_COUNT),
        "forest_guardian" => Some(FOREST_GUARDIAN_COUNT),
        _ => None,
    }
}

/// `threshold` as the view's integer; every progression threshold fits.
fn wide(threshold: u64) -> Option<i64> {
    i64::try_from(threshold).ok()
}

/// The family's name as the surfaces show it.
const fn family_name(family: Family) -> &'static str {
    match family {
        Family::Study => "study",
        Family::Habit => "habit",
        Family::Focus => "focus",
    }
}

/// The catalog badges under `courses` that `earned` holds no tier of, in the catalog's order, each
/// with its criteria and its progress over `inputs`.
#[must_use]
pub fn locked(
    courses: &Courses,
    earned: &[EarnedLine],
    inputs: &ProgressInputs,
) -> Vec<LockedLine> {
    let held: BTreeSet<&str> = earned.iter().map(|line| line.key.as_str()).collect();
    catalog(courses)
        .into_iter()
        .filter(|badge| !held.contains(badge.key.as_str()))
        .map(|badge| LockedLine {
            progress: progress(&badge.key, inputs),
            family: family_name(badge.family),
            key: badge.key,
            name: badge.name,
            emoji: badge.emoji,
            criteria: badge.description,
        })
        .collect()
}

/// Every earned badge, most recently awarded first.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn earned_badges(db: &Db) -> Result<Vec<EarnedLine>, KernelError> {
    let mut connection = db.reader().acquire().await?;
    // The order is the pure `most_recent_first`'s, not the query's, so a test holds it (Q1).
    let rows = sqlx::query!(
        "SELECT badge_key, tier, name, emoji, study_day, created_at FROM badges_earned"
    )
    .fetch_all(&mut *connection)
    .await?;
    let earned = rows
        .into_iter()
        .map(|row| EarnedLine {
            key: row.badge_key,
            // The column's check admits no negative tier.
            tier: u32::try_from(row.tier).unwrap_or(0),
            name: row.name,
            emoji: row.emoji,
            study_day: StudyDay::from_epoch_day(row.study_day),
            awarded_at: UtcMillis::from_epoch_millis(row.created_at),
        })
        .collect();
    Ok(most_recent_first(earned))
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
    let earned = earned_badges(db).await?;
    let mut connection = db.reader().acquire().await?;
    let day = deck_streak_analytics::rollup::stored(&mut connection, today).await?;
    let streak = deck_streak_streaks::store::state(&mut connection, "language").await?;
    let inputs = ProgressInputs {
        streak: streak.map_or(0, |state| i64::from(state.current)),
        day_reviews: day.as_ref().map_or(0, |day| day.metrics.reviews),
        day_decks: day.as_ref().map_or(0, |day| day.metrics.decks_studied),
        day_score: day.as_ref().map_or(0, |day| day.score.total),
        mature_cards: day
            .as_ref()
            .and_then(|day| day.card_state.as_ref())
            .map(|state| state.mature_count),
    };
    let locked = locked(courses, &earned, &inputs);
    Ok(BadgesView { earned, locked })
}
