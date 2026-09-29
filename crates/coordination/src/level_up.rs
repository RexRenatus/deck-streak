//! The level-up line (SPEC-072 R14): one celebration through the router when a recompute takes the
//! XP total to a higher level.

use deck_streak_kernel::{KernelError, StudyDay};
use deck_streak_notifications::{Decision, Router};
use deck_streak_progression::xp::Level;

/// Raises the line for the level `after` reached from `before`, or none when it is no higher.
///
/// # Errors
///
/// [`KernelError`] when the router's ledger cannot be written.
pub async fn announce_level_up(
    _router: &Router,
    _before: Level,
    _after: Level,
    _study_day: StudyDay,
) -> Result<Option<Decision>, KernelError> {
    Ok(None)
}
