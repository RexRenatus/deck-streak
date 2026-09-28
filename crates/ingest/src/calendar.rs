//! The collection day number (SPEC-071 R7): the day Anki's scheduler counts due dates in.
//!
//! A review card's `due` is a whole number of days since the collection was created, counted in
//! study days, so whether a card is due on a study day is a comparison with that day's number. The
//! number is the study day minus the study day of the collection's creation instant, both under the
//! kernel's configured rule: the predecessor's `analytics.py:today_day_number` at `27ee2bc`, proved
//! by `goldens/today_day_number.json`. It belongs to ingest because the creation instant is Anki's.

use deck_streak_kernel::{StudyDay, StudyDayRule, UtcMillis};

/// The collection day number of `day`, for a collection created at `created_at`: `day` minus the
/// study day of `created_at`, under `rule`.
#[must_use]
pub fn collection_day_number(rule: StudyDayRule, created_at: UtcMillis, day: StudyDay) -> i64 {
    let _ = (rule, created_at, day);
    0
}
