//! A study day's metrics and its per-course metrics (SPEC-071 R6, R11).
//!
//! They port the predecessor's `analytics.py:compute_daily_metrics` and
//! `cross_language.py:language_daily_metrics` at `27ee2bc`, proved by `goldens/daily_metrics.json`
//! and `goldens/language_daily_metrics.json`. Only study events count (SPEC-023's rule), each
//! answer's time is capped and the seconds are summed in the reviews' order, and a first answer
//! is the earliest review-type answer of a card that day, ties kept in the reviews' order.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_ingest::reader::{Review, is_study_event};
use deck_streak_kernel::{CourseCode, StudyDay, StudyDayRule, UtcMillis};

use crate::constants::{ANSWER_TIME_CAP_SECONDS, MATURE_IVL_DAYS};

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
    let day_reviews: Vec<&Review> = reviews
        .iter()
        .filter(|review| is_study_event(review.kind, review.ease) && day_of(review, rule) == day)
        .collect();
    let of_kind = |kind: i64| count(day_reviews.iter().filter(|review| review.kind == kind));
    let reviews_n = count(day_reviews.iter());
    let seconds = seconds_of(&day_reviews);
    let first = first_answers(&day_reviews);
    let answered = count(first.values());
    let passed = count(first.values().filter(|review| passes(review)));
    let (young, mature): (Vec<&Review>, Vec<&Review>) = first
        .values()
        .copied()
        .partition(|review| review.last_interval < MATURE_IVL_DAYS);
    let graduations = count(day_reviews.iter().filter(|review| {
        review.kind == 1
            && review.last_interval < MATURE_IVL_DAYS
            && MATURE_IVL_DAYS <= review.interval
    }));
    let decks_studied = if card_decks.is_empty() {
        0
    } else {
        let decks: BTreeSet<i64> = day_reviews
            .iter()
            .filter_map(|review| card_decks.get(&review.card_id).copied())
            .collect();
        count(decks.iter())
    };
    DailyMetrics {
        day,
        reviews: reviews_n,
        learn_count: of_kind(0),
        review_count: of_kind(1),
        relearn_count: of_kind(2),
        filtered_count: of_kind(3),
        seconds,
        answered,
        passed,
        true_retention: if answered > 0 {
            float(passed) / float(answered) * 100.0
        } else {
            0.0
        },
        graduations,
        decks_studied,
        avg_answer_seconds: if reviews_n > 0 {
            seconds / float(reviews_n)
        } else {
            0.0
        },
        young_answered: count(young.iter()),
        young_passed: count(young.iter().filter(|review| passes(review))),
        mature_answered: count(mature.iter()),
        mature_passed: count(mature.iter().filter(|review| passes(review))),
    }
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
    let mut buckets: BTreeMap<(StudyDay, CourseCode), Vec<&Review>> = BTreeMap::new();
    for review in reviews {
        if !is_study_event(review.kind, review.ease) {
            continue;
        }
        let day = day_of(review, rule);
        if !days.contains(&day) {
            continue;
        }
        let Some(course) = card_courses.get(&review.card_id) else {
            continue;
        };
        buckets.entry((day, *course)).or_default().push(review);
    }
    buckets
        .into_iter()
        .map(|((day, course), bucket)| {
            let first = first_answers(&bucket);
            LanguageDay {
                day,
                course,
                reviews: count(bucket.iter()),
                seconds: seconds_of(&bucket),
                answered: count(first.values()),
                passed: count(first.values().filter(|review| passes(review))),
            }
        })
        .collect()
}

/// The study day `review` was answered in, under `rule`: its id is its instant.
fn day_of(review: &Review, rule: StudyDayRule) -> StudyDay {
    rule.study_day(UtcMillis::from_epoch_millis(review.id))
}

/// The seconds of `reviews`, each answer capped at [`ANSWER_TIME_CAP_SECONDS`] and summed in their
/// order, as the predecessor's `sum(min(r.time_ms / 1000, ANSWER_TIME_CAP_SECONDS) ...)` does.
fn seconds_of(reviews: &[&Review]) -> f64 {
    python_sum(reviews.iter().map(|review| {
        let taken = float(review.taken_ms) / 1000.0;
        // Python's `min(a, b)` keeps `a` unless `b` is smaller.
        if ANSWER_TIME_CAP_SECONDS < taken {
            ANSWER_TIME_CAP_SECONDS
        } else {
            taken
        }
    }))
}

/// Python's built-in `sum` of floats from its integer start of 0, as `CPython` 3.12 and later
/// compute it: Neumaier's compensated summation, with the compensation added once at the end when
/// it is finite and not zero (`Python/bltinmodule.c`, `builtin_sum_impl`). A plain running sum
/// differs from it in the last bit of a long sum, which a golden compared bit for bit refuses.
fn python_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut values = values.into_iter();
    let Some(first) = values.next() else {
        return 0.0;
    };
    // The integer start plus the first float is that float, exactly.
    let mut total = 0.0 + first;
    let mut compensation = 0.0;
    for value in values {
        let sum = total + value;
        if total.abs() >= value.abs() {
            compensation += (total - sum) + value;
        } else {
            compensation += (value - sum) + total;
        }
        total = sum;
    }
    if compensation != 0.0 && compensation.is_finite() {
        total += compensation;
    }
    total
}

/// Each card's first review-type answer among `reviews`: the earliest by id, the first of equal ids
/// in their order (a stable sort, as the predecessor's `sorted` is).
fn first_answers<'a>(reviews: &[&'a Review]) -> BTreeMap<i64, &'a Review> {
    let mut answers: Vec<&Review> = reviews
        .iter()
        .copied()
        .filter(|review| review.kind == 1)
        .collect();
    answers.sort_by_key(|review| review.id);
    let mut first = BTreeMap::new();
    for review in answers {
        first.entry(review.card_id).or_insert(review);
    }
    first
}

/// Whether an answer passed: ease 2 or more.
const fn passes(review: &Review) -> bool {
    review.ease >= 2
}

/// How many items `items` yields.
fn count<T>(items: impl Iterator<Item = T>) -> i64 {
    i64::try_from(items.count()).unwrap_or(i64::MAX)
}

/// `value` as a float, as Python's int-to-float arithmetic takes it.
#[allow(
    clippy::cast_precision_loss,
    reason = "a count or a millisecond duration is far below 2^53, so it converts exactly"
)]
const fn float(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::python_sum;

    /// `CPython` adds the compensation only while it is finite, so a sum that overflows stays
    /// infinite rather than turning into NaN (`builtin_sum_impl`'s `Py_IS_FINITE(c)`).
    #[test]
    fn an_overflowing_python_sum_stays_infinite() {
        let sum = python_sum([f64::MAX, f64::MAX]);
        assert!(sum.is_infinite() && sum.is_sign_positive(), "{sum}");
    }
}
