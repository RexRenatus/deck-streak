//! A review's XP (SPEC-072 R1, R4), translated at progression's edge into the XP crate's rule
//! (SPEC-360 R7, ADR-371 D2 and D3).
//!
//! The rule lives in `deck-streak-xp`, which owns its inputs and depends on no context, so a client
//! can link it. This function keeps its path and signature for coordination's callers: it builds
//! the crate's facts from an ingest review (its ease, its new interval and its review type) and maps
//! ingest's tier arm by arm, so a reordered enum fails to compile rather than remapping a tier.

use deck_streak_ingest::reader::Review;
use deck_streak_ingest::tier::Tier;
use deck_streak_xp::review_xp::{ReviewFacts, Tier as XpTier};

/// The XP of `review`, whose card carries `tier` (none for a language card or an untagged one).
#[must_use]
pub fn review_xp(review: &Review, tier: Option<Tier>) -> u32 {
    deck_streak_xp::review_xp::review_xp(&facts(review), tier.map(xp_tier))
}

/// The three facts of `review` the XP rule reads; maturity is decided by the new interval.
const fn facts(review: &Review) -> ReviewFacts {
    ReviewFacts {
        ease: review.ease,
        interval: review.interval,
        kind: review.kind,
    }
}

/// Ingest's tier as the XP crate's, one arm per tier and no wildcard.
const fn xp_tier(tier: Tier) -> XpTier {
    match tier {
        Tier::T1 => XpTier::T1,
        Tier::T2 => XpTier::T2,
        Tier::T3 => XpTier::T3,
        Tier::T4 => XpTier::T4,
    }
}
