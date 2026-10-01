//! The badge context of one evaluated day (SPEC-073 R5), equal to `goldens/badge_context.json`
//! (`pipeline.py:GamifyPipeline._evaluate_and_award`).
//!
//! It is a pure function of what the day's evaluation reads: the window's study reviews, the
//! card-to-deck map, the day's card snapshot, the language streak's badge view, the lifetime study
//! reviews, the day's score and the scores of the most recent rollups on or before it. The badge
//! step reads those from the fold's write; the golden builds them from its case.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_analytics::metrics::{DailyMetrics, daily_metrics};
use deck_streak_ingest::reader::{Review, is_study_event};
use deck_streak_kernel::{StudyDay, StudyDayRule, UtcMillis};
use deck_streak_progression::badges::conditions::{
    BACKLOG_SLAYER_CLEARED, BadgeContext, EARLY_BIRD_END_HOUR, IRON_WILL_DAYS, PERFECT_WEEK_DAYS,
    Snapshot,
};
use deck_streak_progression::badges::hours::count_in_local_hours;

/// The days of the week a badge reads, ending on the evaluated day.
const WEEK_DAYS: i64 = 7;

/// The days of the month the mature retention reads, ending on the evaluated day.
const MONTH_DAYS: i64 = 30;

/// What one evaluated day's badge context is built from.
#[derive(Clone, Copy, Debug)]
pub struct DayState<'a> {
    /// The window's study reviews.
    pub reviews: &'a [Review],
    /// The study-day rule.
    pub rule: StudyDayRule,
    /// The day evaluated.
    pub day: StudyDay,
    /// Each card's home deck.
    pub card_decks: &'a BTreeMap<i64, i64>,
    /// The day's card snapshot.
    pub snapshot: Snapshot,
    /// The language streak's current length.
    pub streak_current: u64,
    /// Whether the language streak's comeback shows (SPEC-076's badge view).
    pub comeback_armed: bool,
    /// The lifetime study reviews through the day.
    pub lifetime: u64,
    /// The day's score: the score it closed with for a closing day, the live score for the current
    /// day.
    pub score_total: i64,
    /// The most recent rollups on or before the day, most recent first, each its day and stored
    /// score; the day's own counts with `score_total`.
    pub rollups: &'a [(StudyDay, i64)],
}

/// The badge context of `state`'s day (R5).
#[must_use]
pub fn badge_context(state: &DayState<'_>) -> BadgeContext {
    let rule = state.rule;
    let day = state.day.epoch_day();
    let study_days: BTreeSet<i64> = state
        .reviews
        .iter()
        .filter(|review| is_study_event(review.kind, review.ease))
        .map(|review| {
            rule.study_day(UtcMillis::from_epoch_millis(review.id))
                .epoch_day()
        })
        .collect();
    let today = daily_metrics(state.reviews, rule, state.day, state.card_decks);
    let week: Vec<&Review> = state
        .reviews
        .iter()
        .filter(|review| is_study_event(review.kind, review.ease))
        .filter(|review| {
            let back = day
                - rule
                    .study_day(UtcMillis::from_epoch_millis(review.id))
                    .epoch_day();
            (0..WEEK_DAYS).contains(&back)
        })
        .collect();
    let week_decks: BTreeSet<i64> = week
        .iter()
        .filter_map(|review| state.card_decks.get(&review.card_id).copied())
        .collect();
    let (answered, passed) = summed(state, WEEK_DAYS, &study_days, |m| (m.answered, m.passed));
    let (mature_answered, mature_passed) = summed(state, MONTH_DAYS, &study_days, |m| {
        (m.mature_answered, m.mature_passed)
    });
    let mut week_scores: Vec<i64> = state
        .rollups
        .iter()
        .take(PERFECT_WEEK_DAYS)
        .map(|(rolled, score)| {
            if *rolled == state.day {
                state.score_total
            } else {
                *score
            }
        })
        .collect();
    week_scores.reverse();
    let cleared = if state.snapshot.backlog == 0 {
        count(today.review_count)
    } else {
        0
    };
    let iron_will_days = i64::try_from(IRON_WILL_DAYS).unwrap_or(i64::MAX);
    BadgeContext {
        lifetime: state.lifetime,
        streak_current: state.streak_current,
        comeback_armed: state.comeback_armed,
        day_reviews: count(today.reviews),
        day_decks: count(today.decks_studied),
        day_avg_seconds: today.avg_answer_seconds,
        snapshot: state.snapshot,
        score_total: state.score_total,
        week_scores,
        week_reviews: u64::try_from(week.len()).unwrap_or(u64::MAX),
        week_decks: u64::try_from(week_decks.len()).unwrap_or(u64::MAX),
        week_retention: percent(passed, answered),
        mature30_answered: count(mature_answered),
        mature30_retention: percent(mature_passed, mature_answered),
        night_owl: count_in_local_hours(
            state.reviews,
            rule,
            state.day,
            0,
            rule.rollover_hour().get(),
        ),
        early_bird: count_in_local_hours(
            state.reviews,
            rule,
            state.day,
            rule.rollover_hour().get(),
            EARLY_BIRD_END_HOUR,
        ),
        cleared_backlog: if cleared >= BACKLOG_SLAYER_CLEARED {
            cleared
        } else {
            0
        },
        iron_will_ok: (0..iron_will_days).all(|back| study_days.contains(&(day - back))),
    }
}

/// The sums `pick` reads off each study day's metrics, over the `days` days ending on the
/// evaluated one (`week_days & study_days_set` in the predecessor).
fn summed(
    state: &DayState<'_>,
    days: i64,
    study_days: &BTreeSet<i64>,
    pick: impl Fn(&DailyMetrics) -> (i64, i64),
) -> (i64, i64) {
    let day = state.day.epoch_day();
    (0..days)
        .map(|back| day - back)
        .filter(|day| study_days.contains(day))
        .map(|day| {
            let metrics = daily_metrics(
                state.reviews,
                state.rule,
                StudyDay::from_epoch_day(day),
                state.card_decks,
            );
            pick(&metrics)
        })
        .fold((0, 0), |(a, b), (x, y)| (a + x, b + y))
}

/// A stored count as the badge context holds it; a count is never negative.
fn count(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

/// `part` of `whole` in percent, 0 when `whole` is 0, as the predecessor divides.
#[allow(
    clippy::cast_precision_loss,
    reason = "a count of answers is far below 2^52, so the conversion is exact"
)]
fn percent(part: i64, whole: i64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64 * 100.0
    }
}
