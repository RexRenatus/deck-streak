//! The one freeze use case other contexts' use cases call (SPEC-076 R7, R8): a shop purchase, an
//! Epic chest's choice, the weekly quest's reward and a season node each pay a freeze through
//! [`grant_freeze`], and nothing else writes the streak rows.
//!
//! The caps are the streaks context's [`admit`]; this module reads what they judge, writes the
//! freeze and its event in one write, and answers what happened.

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};
use deck_streak_streaks::freeze::{FreezeReason, Refusal, admit};
use deck_streak_streaks::store;
use deck_streak_streaks::streak::StreakState;
use sqlx::SqliteConnection;

/// What a freeze grant came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreezeGrant {
    /// One freeze was added to the language row.
    Granted,
    /// A cap refused it, and nothing was written.
    Refused(Refusal),
}

/// Grants one freeze for `reason` on `day`, in one write that judges the caps first.
///
/// # Errors
///
/// [`KernelError`] when a read or the write fails.
pub async fn grant_freeze(
    db: &Db,
    day: StudyDay,
    reason: FreezeReason,
    at: UtcMillis,
) -> Result<FreezeGrant, KernelError> {
    let mut write = db.write().await?;
    let grant = grant_freeze_on(&mut write, day, reason, at).await?;
    match grant {
        FreezeGrant::Granted => write.commit().await?,
        FreezeGrant::Refused(_) => write.rollback().await?,
    }
    Ok(grant)
}

/// Grants one freeze for `reason` on `day` inside the caller's `write` (SPEC-411 R9), judging the
/// caps first, so a use case that pays a freeze writes it in the write that earned it.
///
/// # Errors
///
/// [`KernelError`] when a read or the write fails.
pub async fn grant_freeze_on(
    write: &mut SqliteConnection,
    day: StudyDay,
    reason: FreezeReason,
    at: UtcMillis,
) -> Result<FreezeGrant, KernelError> {
    if let Err(refusal) = admission(write, reason, day).await? {
        return Ok(FreezeGrant::Refused(refusal));
    }
    store::add_freeze(write, day, reason, at).await?;
    Ok(FreezeGrant::Granted)
}

/// The streaks' own judgement of one more freeze for `reason` on `day`, read inside the caller's
/// `write` (SPEC-411 R8): the freezes held and the freeze events, judged by [`admit`]. An Epic's
/// pick reads its cap here, so the cap it settles with is the one the grant would meet.
///
/// # Errors
///
/// [`KernelError`] when a read fails.
pub(crate) async fn admission(
    write: &mut SqliteConnection,
    reason: FreezeReason,
    day: StudyDay,
) -> Result<Result<(), Refusal>, KernelError> {
    let held = store::state(write, "language")
        .await?
        .unwrap_or_else(StreakState::start)
        .freezes;
    let events = store::events(write).await?;
    Ok(admit(held, reason, day, &events))
}
