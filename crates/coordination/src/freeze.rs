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
    let held = store::state(&mut write, "language")
        .await?
        .unwrap_or_else(StreakState::start)
        .freezes;
    let events = store::events(&mut write).await?;
    if let Err(refusal) = admit(held, reason, day, &events) {
        write.rollback().await?;
        return Ok(FreezeGrant::Refused(refusal));
    }
    store::add_freeze(&mut write, day, reason, at).await?;
    write.commit().await?;
    Ok(FreezeGrant::Granted)
}
