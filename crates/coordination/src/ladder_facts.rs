//! The streak's facts the celebration ladder reads (SPEC-084 R5, R11): the facts an occasion carries
//! and the router's flush re-caps each held celebration for.
//!
//! The streaks context supplies them (SPEC-076). Until it lands this module supplies none, so the
//! streak-break cap never applies and no streak source is invented here: the flush runs on no facts,
//! exactly as the router's own `flush` does.

use deck_streak_notifications::StreakFacts;

/// The streak's facts on the flush's study day: none until SPEC-076 supplies them.
#[must_use]
pub const fn streak_facts() -> Option<StreakFacts> {
    None
}
