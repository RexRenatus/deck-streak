//! A review's XP (SPEC-072 R1, R4): the predecessor's `gamification/xp.py:review_xp` at `27ee2bc`.
//!
//! The base times the ease, maturity, type and tier multipliers, left to right in 64-bit floating
//! point, rounded half to even. A row that is not a study event earns nothing. The tier arrives
//! already restricted to a law-track card: this function multiplies by what it is handed.
//!
//! The rule and its study-event guard moved here from progression (SPEC-360 R1 to R3,
//! ADR-371), so the server and both clients link one copy that reads no file, socket or clock.

use crate::table::table;

/// The three facts of one answer the XP rule reads, as the collection stores them.
///
/// The crate owns its inputs rather than naming ingest's review row, so it depends on no other
/// context and a client can build these from whatever row it holds (SPEC-360 R2, ADR-371).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewFacts {
    /// The answer's ease, the button pressed: 1 to 4, again to easy.
    pub ease: i64,
    /// The card's new interval in days after the answer, which decides its maturity.
    pub interval: i64,
    /// The review type: 0 to 3, learn to filtered; any other value is no study event.
    pub kind: i64,
}

/// A Bloom tier a law card carries, `T1` the shallowest and `T4` the deepest.
///
/// Declared in this order so that `tier as usize` indexes the table's tier multipliers. The crate
/// parses no tag: the caller maps its own tier into this one (SPEC-360 R2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Remember.
    T1,
    /// Understand.
    T2,
    /// Apply.
    T3,
    /// Analyse.
    T4,
}

/// Whether an answer of type `kind` with `ease` is a study event (SPEC-360 R3): a learn, review,
/// relearn or filtered answer of ease 1 or more, and never a manual or rescheduling entry (the
/// predecessor's `types.py:Review.is_study_event`). Ingest keeps its own copy as the reader's
/// filter, and both are held to the same golden (ADR-371 D5).
#[must_use]
pub const fn is_study_event(kind: i64, ease: i64) -> bool {
    matches!(kind, 0..=3) && ease >= 1
}

/// The XP of `review`, whose card carries `tier` (none for a language card or an untagged one).
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the rounded product of a base and multipliers of a few digits is a small \
              non-negative number, as the predecessor's round() returns"
)]
pub fn review_xp(review: &ReviewFacts, tier: Option<Tier>) -> u32 {
    if !is_study_event(review.kind, review.ease) {
        return 0;
    }
    let economy = table();
    let ease = match review.ease {
        1..=4 => economy.ease[usize::try_from(review.ease - 1).unwrap_or(0)],
        _ => 1.0,
    };
    let maturity = if review.interval >= economy.mature_interval_days {
        economy.mature
    } else if review.interval > 0 {
        economy.young
    } else {
        economy.fresh
    };
    let kind = match review.kind {
        0..=3 => economy.types[usize::try_from(review.kind).unwrap_or(0)],
        _ => 1.0,
    };
    let tier = tier.map_or(economy.untagged, |tier| economy.tier[tier as usize]);
    (economy.base * ease * maturity * kind * tier).round_ties_even() as u32
}
