//! A review's XP (SPEC-072 R1, R4): the predecessor's `gamification/xp.py:review_xp` at `27ee2bc`.
//!
//! The base times the ease, maturity, type and tier multipliers, left to right in 64-bit floating
//! point, rounded half to even. A row that is not a study event earns nothing. The tier arrives
//! already restricted to a law-track card: this function multiplies by what it is handed.

use deck_streak_ingest::reader::{Review, is_study_event};
use deck_streak_ingest::tier::Tier;

use crate::economy_config::xp;

/// The XP of `review`, whose card carries `tier` (none for a language card or an untagged one).
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the rounded product of a base and multipliers of a few digits is a small \
              non-negative number, as the predecessor's round() returns"
)]
pub fn review_xp(review: &Review, tier: Option<Tier>) -> u32 {
    if !is_study_event(review.kind, review.ease) {
        return 0;
    }
    let economy = xp();
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
