//! The five-pillar daily score and its grade (SPEC-071 R12 to R14), with the raw streak the
//! consistency pillar reads and the volume baseline.
//!
//! They port the predecessor's `scoring.py:compute_score`, `_consistency`, `_retention`,
//! `_workload`, `_volume`, `_mastery` and `grade_band`, and `analytics.py:raw_streak` and
//! `volume_baseline` over the rows `pipeline.py:GamifyPipeline._baseline` selects, at `27ee2bc`;
//! each is proved by its own golden. Every float operation is done in the predecessor's order, and
//! the total is rounded half to even, as Python's `round` does.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

use crate::constants::{
    GRADE_BANDS, MASTERY_GRADUATION_TARGET, MASTERY_LEECH_PENALTY, MASTERY_LEECH_PENALTY_CAP,
    RETENTION_BLEND_TARGET, RETENTION_CEIL_PCT, RETENTION_FLOOR_PCT, RETENTION_MIN_SAMPLE,
    VOLUME_CAP_RATIO, VOLUME_REVIEW_WEIGHT, VOLUME_TIME_WEIGHT, WEIGHT_CONSISTENCY, WEIGHT_MASTERY,
    WEIGHT_RETENTION, WEIGHT_VOLUME, WEIGHT_WORKLOAD, WORKLOAD_BACKLOG_DIVISOR,
    WORKLOAD_BACKLOG_PENALTY_CAP,
};
use crate::metrics::DailyMetrics;
use crate::snapshot::{CardSnapshot, CardState};

/// The rows the baseline's store read returns: the predecessor's `get_recent_rollups(31)`, a
/// literal inside `pipeline.py:GamifyPipeline._baseline`, proved by `goldens/baseline_window.json`.
pub const BASELINE_ROWS_READ: usize = 31;
/// The rows the baseline keeps once today's is dropped: the same function's `[:30]`.
pub const BASELINE_ROWS: usize = 30;
/// The volume baseline's floor on reviews: a literal inside `analytics.py:volume_baseline`, proved
/// by `goldens/volume_baseline.json`.
const BASE_REVIEWS_FLOOR: f64 = 10.0;
/// The volume baseline's floor on minutes, from the same function.
const BASE_MINUTES_FLOOR: f64 = 5.0;
/// The grade of a total below every band: `scoring.py:grade_band`'s own fallback.
const NO_BAND: (&str, &str) = ("NO STUDY", "\u{1f4a4}");

/// The card state the score reads: the workload's due cards and backlog, the mastery's leeches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScoreState {
    /// Review cards due on the day.
    pub due_today: i64,
    /// Review cards overdue.
    pub backlog: i64,
    /// Active leeches.
    pub leech_active: i64,
}

impl From<&CardSnapshot> for ScoreState {
    fn from(snapshot: &CardSnapshot) -> Self {
        Self {
            due_today: snapshot.due_today,
            backlog: snapshot.backlog,
            leech_active: snapshot.leech_active,
        }
    }
}

impl From<&CardState> for ScoreState {
    fn from(state: &CardState) -> Self {
        Self {
            due_today: state.due_today,
            backlog: state.backlog,
            leech_active: state.leech_active,
        }
    }
}

/// The volume pillar's baseline: the median reviews and minutes of the active days before.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Baseline {
    /// Reviews a day.
    pub reviews: f64,
    /// Minutes a day.
    pub minutes: f64,
}

/// One rollup row's volume, as the baseline reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DayVolume {
    /// The row's study day.
    pub day: StudyDay,
    /// Its reviews.
    pub reviews: i64,
    /// Its seconds.
    pub seconds: f64,
}

/// A day's score: the five pillars, the total and its grade band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Score {
    /// Showing up, and the streak.
    pub consistency: f64,
    /// True retention against its floor and ceiling.
    pub retention: f64,
    /// The due cards cleared, less the backlog; 0 without a card state.
    pub workload: f64,
    /// Reviews and minutes against the baseline.
    pub volume: f64,
    /// Graduations, less the leeches; 0 without a card state.
    pub mastery: f64,
    /// The weighted total, 0 to 100.
    pub total: i64,
    /// The grade band's label.
    pub grade_label: &'static str,
    /// The grade band's emoji.
    pub grade_emoji: &'static str,
}

/// The consistency pillar of a day with `reviews` and a streak of `streak_days`.
#[must_use]
pub fn consistency(reviews: i64, streak_days: i64) -> f64 {
    // `scoring.py:_consistency`'s own literals: showing up is 100, the streak bonus 2 a day up to
    // 50, weighted 0.7 and 0.6.
    let showed_up = if reviews >= 1 { 100.0 } else { 0.0 };
    let streak_bonus = py_min(50.0, float(streak_days) * 2.0);
    clamp(0.7 * showed_up + 0.6 * streak_bonus)
}

/// The retention pillar of a day whose `answered` cards had `true_retention` percent.
#[must_use]
pub fn retention(true_retention: f64, answered: i64) -> f64 {
    if answered == 0 {
        return 0.0;
    }
    let span = RETENTION_CEIL_PCT - RETENTION_FLOOR_PCT;
    let mut pillar = clamp((true_retention - RETENTION_FLOOR_PCT) / span * 100.0);
    if answered < RETENTION_MIN_SAMPLE {
        pillar = 0.5 * pillar + 0.5 * RETENTION_BLEND_TARGET;
    }
    clamp(pillar)
}

/// The workload pillar: `review_count` answers against `due_today`, less the backlog's penalty; 0
/// on a day not studied.
#[must_use]
pub fn workload(review_count: i64, due_today: i64, backlog: i64, studied: bool) -> f64 {
    if !studied {
        return 0.0;
    }
    let denominator = review_count + due_today;
    let cleared_ratio = if denominator == 0 {
        1.0
    } else {
        float(review_count) / float(denominator)
    };
    let penalty = py_min(
        WORKLOAD_BACKLOG_PENALTY_CAP,
        float(backlog) / WORKLOAD_BACKLOG_DIVISOR,
    );
    clamp(100.0 * py_min(1.0, cleared_ratio) - penalty)
}

/// The volume pillar: `reviews` and `minutes` against the baseline.
#[must_use]
pub fn volume(reviews: i64, minutes: f64, base_reviews: f64, base_minutes: f64) -> f64 {
    let base_reviews = py_max(base_reviews, 1.0);
    let base_minutes = py_max(base_minutes, 1.0);
    let by_reviews = float(reviews) / base_reviews;
    let by_minutes = minutes / base_minutes;
    let combined = py_min(
        VOLUME_CAP_RATIO,
        VOLUME_REVIEW_WEIGHT * by_reviews + VOLUME_TIME_WEIGHT * by_minutes,
    );
    clamp(100.0 * combined / VOLUME_CAP_RATIO)
}

/// The mastery pillar: `graduations` rewarded, `leech_active` penalised.
#[must_use]
pub fn mastery(graduations: i64, leech_active: i64) -> f64 {
    let reward = 100.0 * py_min(1.0, float(graduations) / float(MASTERY_GRADUATION_TARGET));
    let penalty = py_min(
        MASTERY_LEECH_PENALTY_CAP,
        float(leech_active) * MASTERY_LEECH_PENALTY,
    );
    clamp(reward - penalty)
}

/// The grade band of a total: its label and emoji.
#[must_use]
pub fn grade_band(score: i64) -> (&'static str, &'static str) {
    GRADE_BANDS
        .iter()
        .find(|band| score >= band.threshold)
        .map_or(NO_BAND, |band| (band.label, band.emoji))
}

/// The score of a day with `metrics`, its card state when it has one, a streak of `streak_days`
/// and `baseline`. Without a card state it is the historical form: the three time-accurate pillars
/// renormalised by their weights.
#[must_use]
pub fn compute_score(
    metrics: &DailyMetrics,
    state: Option<ScoreState>,
    streak_days: i64,
    baseline: Baseline,
) -> Score {
    let studied = metrics.reviews > 0;
    let consistency = consistency(metrics.reviews, streak_days);
    let retention = retention(metrics.true_retention, metrics.answered);
    let volume = volume(
        metrics.reviews,
        metrics.seconds / 60.0,
        baseline.reviews,
        baseline.minutes,
    );
    let (workload, mastery, weighted) = match state {
        None => {
            // A day with no card state: the time-accurate pillars, renormalised by their weights.
            let weights = WEIGHT_CONSISTENCY + WEIGHT_RETENTION + WEIGHT_VOLUME;
            let weighted = (WEIGHT_CONSISTENCY * consistency
                + WEIGHT_RETENTION * retention
                + WEIGHT_VOLUME * volume)
                / weights;
            (0.0, 0.0, weighted)
        }
        Some(state) => {
            let workload = workload(
                metrics.review_count,
                state.due_today,
                state.backlog,
                studied,
            );
            let mastery = mastery(metrics.graduations, state.leech_active);
            let weighted = WEIGHT_CONSISTENCY * consistency
                + WEIGHT_RETENTION * retention
                + WEIGHT_WORKLOAD * workload
                + WEIGHT_VOLUME * volume
                + WEIGHT_MASTERY * mastery;
            (workload, mastery, weighted)
        }
    };
    let total = rounded(clamp(weighted));
    let (grade_label, grade_emoji) = grade_band(total);
    Score {
        consistency,
        retention,
        workload,
        volume,
        mastery,
        total,
        grade_label,
        grade_emoji,
    }
}

/// The raw streak ending on `today`: the run of consecutive study days in `days` ending on it, or
/// on the day before while `today` has no study.
#[must_use]
pub fn raw_streak(days: &BTreeSet<StudyDay>, today: StudyDay) -> i64 {
    let before = |day: StudyDay| StudyDay::from_epoch_day(day.epoch_day() - 1);
    let anchor = if days.contains(&today) {
        today
    } else {
        before(today)
    };
    let mut streak = 0;
    let mut day = anchor;
    while days.contains(&day) {
        streak += 1;
        day = before(day);
    }
    streak
}

/// The baseline of `rows`: the median reviews and minutes of the rows with a review, each with its
/// floor.
#[must_use]
pub fn volume_baseline(rows: &[DayVolume]) -> Baseline {
    let active: Vec<&DayVolume> = rows.iter().filter(|row| row.reviews > 0).collect();
    let mut reviews: Vec<i64> = active.iter().map(|row| row.reviews).collect();
    let mut minutes: Vec<f64> = active.iter().map(|row| row.seconds / 60.0).collect();
    reviews.sort_unstable();
    minutes.sort_by(f64::total_cmp);
    // `statistics.median`: the middle value, or the mean of the two middle values.
    let middle = reviews.len() / 2;
    let base_reviews = match reviews.len() {
        0 => 0.0,
        count if count % 2 == 1 => float(reviews[middle]),
        _ => float(reviews[middle - 1] + reviews[middle]) / 2.0,
    };
    let base_minutes = match minutes.len() {
        0 => 0.0,
        count if count % 2 == 1 => minutes[middle],
        _ => (minutes[middle - 1] + minutes[middle]) / 2.0,
    };
    Baseline {
        reviews: py_max(base_reviews, BASE_REVIEWS_FLOOR),
        minutes: py_max(base_minutes, BASE_MINUTES_FLOOR),
    }
}

/// The baseline of `today` over `recent`, the rollup rows most recent first: the rows the
/// predecessor's `_baseline` selects (its 31 most recent, less today's, cut to 30), then
/// [`volume_baseline`].
#[must_use]
pub fn baseline_window(recent: &[DayVolume], today: StudyDay) -> Baseline {
    let prior: Vec<DayVolume> = recent
        .iter()
        .take(BASELINE_ROWS_READ)
        .filter(|row| row.day != today)
        .take(BASELINE_ROWS)
        .copied()
        .collect();
    volume_baseline(&prior)
}

/// Python's `max(0.0, min(100.0, value))`.
fn clamp(value: f64) -> f64 {
    py_max(0.0, py_min(100.0, value))
}

/// Python's two-argument `min`: `a`, unless `b` is smaller.
fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// Python's two-argument `max`: `a`, unless `b` is larger.
fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// Python's `int(round(value))` of a clamped total: half to even, as `round` sends it.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a clamped total lies in 0 to 100, which an i64 holds exactly"
)]
fn rounded(value: f64) -> i64 {
    value.round_ties_even() as i64
}

/// `value` as a float, as Python's int-to-float arithmetic takes it.
#[allow(
    clippy::cast_precision_loss,
    reason = "a count or a streak is far below 2^53, so it converts exactly"
)]
const fn float(value: i64) -> f64 {
    value as f64
}
