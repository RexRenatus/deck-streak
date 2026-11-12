//! The CEFR band a unit falls in, read from the course's configured bands (SPEC-077 R4).

use deck_streak_kernel::courses::Course;

/// The band of `course` that holds `unit`, or none when no configured band does.
#[must_use]
pub fn band_for_unit(course: &Course, unit: u32) -> Option<&'static str> {
    course
        .unit_bands
        .iter()
        .find(|band| band.first <= unit && unit <= band.last)
        .map(|band| band.band)
}
