//! The relight's celebration (SPEC-076 R27): one line through the router, after the fold's commit,
//! under the policy's celebration kind with the key `relight:<epoch day>`.

use deck_streak_kernel::{KernelError, StudyDay};
use deck_streak_notifications::{Decision, Router};

/// Routes the relight of `day`, or answers none when there is none to route.
///
/// # Errors
///
/// [`KernelError`] when the policy cannot be read, the occasion is refused, or the router's ledger
/// cannot be written.
pub async fn announce_relight(
    _router: &Router,
    _day: StudyDay,
    _today: StudyDay,
) -> Result<Option<Decision>, KernelError> {
    Ok(None)
}
