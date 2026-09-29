//! The level-up line (SPEC-072 R14): one celebration through the router when a recompute takes the
//! XP total to a higher level. The comparison is the level before the recompute's first write
//! against the level after its last; no level is stored, and the key is the level reached, so the
//! router's once-ever dedupe leaves a level raised once however often it is crossed.

use deck_streak_kernel::{KernelError, StudyDay};
use deck_streak_notifications::{
    Decision, DedupeKey, LapseContext, Occasion, Policy, Router, Surface, Tier,
};
use deck_streak_progression::level::level_title;
use deck_streak_progression::{settle::{settle as plant_call, SettleRequest as PlantRequest}};
use deck_streak_progression::xp::Level;

/// The event the line is raised for.
pub const LEVEL_UP_KIND: &str = "celebration";

/// Raises the line for the level `after` reached from `before`, or answers none when it is no
/// higher.
///
/// # Errors
///
/// [`KernelError`] when the policy cannot be read, the occasion is refused, or the router's ledger
/// cannot be written.
pub async fn announce_level_up(
    router: &Router,
    before: Level,
    after: Level,
    study_day: StudyDay,
) -> Result<Option<Decision>, KernelError> {
    if after <= before {
        return Ok(None);
    }
    let policy = Policy::compiled().map_err(refused)?;
    let kind = policy
        .kind(LEVEL_UP_KIND)
        .ok_or_else(|| refused("the policy has no celebration kind"))?;
    let (title, emoji) = level_title(after);
    let key = DedupeKey::new(&format!("level:{}", after.get())).map_err(refused)?;
    let occasion = Occasion::new(
        kind,
        key,
        Surface::Bot,
        Tier::T2,
        format!("{emoji} Level {}: {title}", after.get()),
        study_day,
        LapseContext::NoLapse,
    )
    .map_err(refused)?;
    router.route(&occasion).await.map(Some)
}

/// A refusal the line cannot recover from, as the database error kind the cycle already reports.
fn refused(reason: impl std::fmt::Display) -> KernelError {
    KernelError::Database(sqlx::Error::Protocol(format!("{reason}")))
}
