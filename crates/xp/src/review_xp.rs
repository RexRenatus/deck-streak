//! A review's XP and the study-event guard it applies (SPEC-360 R1 to R3).

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

/// Whether an answer of type `kind` with `ease` is a study event.
#[must_use]
pub const fn is_study_event(kind: i64, ease: i64) -> bool {
    let _ = (kind, ease);
    false
}

/// The XP of `review`, whose card carries `tier` (none for a language card or an untagged one).
#[must_use]
pub fn review_xp(review: &ReviewFacts, tier: Option<Tier>) -> u32 {
    let _ = (review, tier);
    0
}
