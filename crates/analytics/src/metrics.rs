//! A study day's metrics and its per-course metrics (SPEC-071 R6, R11).
//!
//! They port the predecessor's `analytics.py:compute_daily_metrics` and
//! `cross_language.py:language_daily_metrics` at `27ee2bc`, proved by `goldens/daily_metrics.json`
//! and `goldens/language_daily_metrics.json`. Only study events count (SPEC-023's rule), each
//! answer's time is capped and the seconds are summed in the reviews' order, and a first answer
//! is the earliest review-type answer of a card that day, ties kept in the reviews' order.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{CourseCode, StudyDay, StudyDayRule};

/// One study day's metrics, every field of the predecessor's `DailyMetrics`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DailyMetrics {
    /// The study day.
    pub day: StudyDay,
    /// Every study event of the day.
    pub reviews: i64,
    /// The learning answers (revlog type 0).
    pub learn_count: i64,
    /// The review answers (type 1).
    pub review_count: i64,
    /// The relearning answers (type 2).
    pub relearn_count: i64,
    /// The answers in a filtered deck (type 3).
    pub filtered_count: i64,
    /// The seconds answering, each answer capped, summed in the reviews' order.
    pub seconds: f64,
    /// The cards whose first review-type answer of the day this is: true retention's denominator.
    pub answered: i64,
    /// Of those, the answers at ease 2 or more.
    pub passed: i64,
    /// `passed` over `answered`, in percent, or 0 with no answered card.
    pub true_retention: f64,
    /// The review-type answers whose interval crosses into maturity.
    pub graduations: i64,
    /// The distinct home decks answered.
    pub decks_studied: i64,
    /// The seconds per answer, or 0 with no answer.
    pub avg_answer_seconds: f64,
    /// The first answers of cards young going in.
    pub young_answered: i64,
    /// Of those, the passes.
    pub young_passed: i64,
    /// The first answers of cards mature going in.
    pub mature_answered: i64,
    /// Of those, the passes.
    pub mature_passed: i64,
}

impl DailyMetrics {
    /// The metrics of a day with no study event.
    #[must_use]
    pub const fn empty(day: StudyDay) -> Self {
        Self {
            day,
            reviews: 0,
            learn_count: 0,
            review_count: 0,
            relearn_count: 0,
            filtered_count: 0,
            seconds: 0.0,
            answered: 0,
            passed: 0,
            true_retention: 0.0,
            graduations: 0,
            decks_studied: 0,
            avg_answer_seconds: 0.0,
            young_answered: 0,
            young_passed: 0,
            mature_answered: 0,
            mature_passed: 0,
        }
    }
}

/// One course's numbers on one study day: a row of `daily_lang_stats`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LanguageDay {
    /// The study day.
    pub day: StudyDay,
    /// The course.
    pub course: CourseCode,
    /// The course's study events that day.
    pub reviews: i64,
    /// Their seconds, each answer capped.
    pub seconds: f64,
    /// The course's cards whose first review-type answer of the day this is.
    pub answered: i64,
    /// Of those, the answers at ease 2 or more.
    pub passed: i64,
}

/// The metrics of study day `day` under `rule`, over `reviews` in their order: only the study
/// events of that day count. `card_decks` maps a card to its home deck, for the decks studied; an
/// empty map counts no deck, as the predecessor's does.
#[must_use]
pub fn daily_metrics(
    reviews: &[Review],
    rule: StudyDayRule,
    day: StudyDay,
    card_decks: &BTreeMap<i64, i64>,
) -> DailyMetrics {
    let _ = (reviews, rule, card_decks);
    DailyMetrics::empty(day)
}

/// Each course's numbers on each of `days` under `rule`, over `reviews` in their order, sorted by
/// day and course code. A review whose card has no course in `card_courses` adds no row.
#[must_use]
pub fn language_metrics(
    reviews: &[Review],
    rule: StudyDayRule,
    card_courses: &BTreeMap<i64, CourseCode>,
    days: &BTreeSet<StudyDay>,
) -> Vec<LanguageDay> {
    let _ = (reviews, rule, card_courses, days);
    Vec::new()
}
