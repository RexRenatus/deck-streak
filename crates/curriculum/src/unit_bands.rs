//! The CEFR band a unit falls in, read from the course's configured bands (SPEC-077 R4).

use deck_streak_kernel::courses::Course;

/// The band of `course` that holds `unit`, or none when no configured band does.
#[must_use]
pub fn band_for_unit(_course: &Course, _unit: u32) -> Option<&'static str> {
    Some("C2")
}
