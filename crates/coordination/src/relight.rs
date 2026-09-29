//! The relight's celebration (SPEC-076 R27): one line through the router, after the fold's commit,
//! under the policy's celebration kind with the key `relight:<epoch day>`.

use deck_streak_kernel::{KernelError, StudyDay};
use deck_streak_notifications::{
    Decision, DedupeKey, LapseContext, Occasion, Policy, Router, Surface, Tier,
};
use deck_streak_streaks::constants::RELIGHT_XP;

/// The event the line is raised for: the policy's celebration kind, which has no `record` of its
/// own.
pub const RELIGHT_KIND: &str = "celebration";

/// Routes the relight of `day`, raised on `today`.
///
/// # Errors
///
/// [`KernelError`] when the policy cannot be read, the occasion is refused, or the router's ledger
/// cannot be written.
pub async fn announce_relight(
    router: &Router,
    day: StudyDay,
    today: StudyDay,
) -> Result<Option<Decision>, KernelError> {
    let policy = Policy::compiled().map_err(refused)?;
    let kind = policy
        .kind(RELIGHT_KIND)
        .ok_or_else(|| refused("the policy has no celebration kind"))?;
    let key = DedupeKey::new(&format!("relight:{}", day.epoch_day())).map_err(refused)?;
    let occasion = Occasion::new(
        kind,
        key,
        Surface::Bot,
        Tier::T2,
        format!("Streak relit: +{RELIGHT_XP} XP for coming back"),
        today,
        LapseContext::NoLapse,
    )
    .map_err(refused)?;
    router.route(&occasion).await.map(Some)
}

/// A refusal the line cannot recover from, as the database error kind the cycle already reports.
fn refused(reason: impl std::fmt::Display) -> KernelError {
    KernelError::Database(sqlx::Error::Protocol(format!("{reason}")))
}
