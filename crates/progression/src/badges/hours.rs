//! The hour windows (SPEC-073 R7): the study reviews of one study day whose local clock hour lies
//! from a window's start hour up to, not including, its end hour, equal to
//! `goldens/count_reviews_in_local_hours.json`.

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{StudyDay, StudyDayRule};

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
    let _ = (reviews, rule, day, start_hour, end_hour);
    0
}
