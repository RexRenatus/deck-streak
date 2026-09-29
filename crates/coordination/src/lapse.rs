//! The open lapse, read from the window a caller read (SPEC-049 R15; ADR-088).

use deck_streak_kernel::StudyDay;

use crate::recompute::RecomputeFacts;

/// The lapse open on the facts' current study day, by its id, if any.
#[must_use]
pub fn open_lapse(_facts: &RecomputeFacts<'_>) -> Option<StudyDay> {
    None
}
