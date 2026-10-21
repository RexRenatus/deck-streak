//! The hour windows (SPEC-073 R7): the study reviews of one study day whose local clock hour lies
//! from a window's start hour up to, not including, its end hour, equal to
//! `goldens/count_reviews_in_local_hours.json`.

use deck_streak_ingest::reader::{Review, is_study_event};
use deck_streak_kernel::{StudyDay, StudyDayRule, UtcMillis};

/// Milliseconds in a minute.
const MINUTE_MS: i64 = 60_000;
/// Milliseconds in an hour.
const HOUR_MS: i64 = 3_600_000;

/// The local clock hour, 0 to 23, of `instant` at the rule's offset: the hour a wall clock shows,
/// never shifted by the rollover (the predecessor's `analytics.py:_local_hour`).
fn local_hour(rule: StudyDayRule, instant: i64) -> i64 {
    let local =
        i128::from(instant) + i128::from(rule.utc_offset().minutes()) * i128::from(MINUTE_MS);
    // An hour of an i64 of milliseconds is in 0..24, so the fallback is never taken.
    i64::try_from(local.div_euclid(i128::from(HOUR_MS)).rem_euclid(24)).unwrap_or(0)
}

/// The study reviews of `day` under `rule` whose local hour, at the rule's offset, is at least
/// `start_hour` and below `end_hour`.
#[must_use]
pub fn count_in_local_hours(
    reviews: &[Review],
    rule: StudyDayRule,
    day: StudyDay,
    start_hour: u8,
    end_hour: u8,
) -> u64 {
    let window = i64::from(start_hour)..i64::from(end_hour);
    let counted = reviews
        .iter()
        .filter(|review| is_study_event(review.kind, review.ease))
        .filter(|review| rule.study_day(UtcMillis::from_epoch_millis(review.id)) == day)
        .filter(|review| window.contains(&local_hour(rule, review.id)))
        .count();
    u64::try_from(counted).unwrap_or(u64::MAX)
}
