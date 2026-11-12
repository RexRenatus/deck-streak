//! The streak's facts the celebration ladder reads (SPEC-084 R5, R11; SPEC-326): the language
//! streak as the streaks context stored it, read through the router's own database before every
//! celebration route and every flush, so the streak-break cap applies on the day the streak broke
//! and each flush re-caps every held celebration at its own study day.
//!
//! The facts are the stored state at the read, and no streak rule is re-derived here (ADR-327). A
//! read that fails is an error: the occasion is not routed and the flush does not run, so nothing
//! renders past the cap on the day the cap is for.

use deck_streak_kernel::{Db, KernelError};
use deck_streak_notifications::{Flushed, Occasion, Router, StreakFacts};

/// The streak the ladder reads: the language track's (SPEC-076).
const TRACK: &str = "language";

/// The language streak's facts as stored in `db`, or none when no streak is stored, which reads
/// no break.
///
/// # Errors
///
/// [`KernelError::Database`] when the stored streak cannot be read.
pub async fn streak_facts(db: &Db) -> Result<Option<StreakFacts>, KernelError> {
    let mut connection = db.reader().acquire().await?;
    let state = deck_streak_streaks::store::state(&mut connection, TRACK).await?;
    Ok(state.map(|state| StreakFacts {
        last_study_day: state.last_study_day,
        current: state.current,
        longest: state.longest,
    }))
}

/// `occasion`, carrying the stored streak's facts for the ladder's cap (SPEC-084 R5).
///
/// # Errors
///
/// [`KernelError::Database`] when the stored streak cannot be read: the occasion is not routed.
pub async fn with_streak_facts(db: &Db, occasion: Occasion) -> Result<Occasion, KernelError> {
    Ok(match streak_facts(db).await? {
        Some(facts) => occasion.with_streak(facts),
        None => occasion,
    })
}

/// `router`'s flush, re-capping each held celebration for the stored streak's facts on the
/// flush's study day (SPEC-084 R11).
///
/// # Errors
///
/// [`KernelError::Database`] when the stored streak cannot be read, and then the flush does not
/// run, or when the flush's own ledger cannot be read or written.
pub async fn flush_re_capped(router: &Router) -> Result<Flushed, KernelError> {
    let facts = streak_facts(router.db()).await?;
    router.flush_with(facts).await
}
