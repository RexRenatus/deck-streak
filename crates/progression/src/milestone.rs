//! The next milestone (SPEC-073 R14): across the review, streak and mature-card ladders, the nearest
//! unreached rung by the remaining fraction of that rung, ties to reviews, then the streak, then
//! mature cards; a complete ladder contributes nothing, and with all three complete the milestone
//! is the top review rung at 100%. Equal to `goldens/next_milestone.json`, and proved as a rule
//! over exact rationals in `formal/lean/Formal/NextMilestone.lean`.

/// The lifetime-review ladder.
pub const REVIEW_LADDER: [u64; 6] = [0; 6];
/// The streak ladder, in days.
pub const STREAK_LADDER: [u64; 5] = [0; 5];
/// The mature-card ladder.
pub const MATURE_LADDER: [u64; 4] = [0; 4];

/// One of the three ladders, in the tie order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ladder {
    /// Lifetime reviews.
    Reviews,
    /// The day streak.
    Streak,
    /// Mature cards.
    Mature,
}

impl Ladder {
    /// The ladders in the tie order.
    pub const ALL: [Self; 3] = [Self::Reviews, Self::Streak, Self::Mature];

    /// The label a screen shows.
    #[must_use]
    pub const fn label(self) -> &'static str {
        ""
    }

    /// The emoji a screen shows.
    #[must_use]
    pub const fn emoji(self) -> &'static str {
        ""
    }

    /// The rungs, ascending.
    #[must_use]
    pub const fn rungs(self) -> &'static [u64] {
        match self {
            Self::Reviews => &REVIEW_LADDER,
            Self::Streak => &STREAK_LADDER,
            Self::Mature => &MATURE_LADDER,
        }
    }
}

/// The next milestone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Milestone {
    /// The ladder it is on.
    pub ladder: Ladder,
    /// The ladder's current value; the top rung when every ladder is complete.
    pub current: u64,
    /// The rung.
    pub target: u64,
    /// The current value as a percentage of the rung.
    pub pct: f64,
    /// What is left to the rung.
    pub remaining: u64,
}

/// The milestone nearest to `reviews` lifetime reviews, a `streak`-day streak and `mature` mature
/// cards.
#[must_use]
pub fn next_milestone(reviews: u64, streak: u64, mature: u64) -> Milestone {
    let _ = (reviews, streak, mature);
    Milestone {
        ladder: Ladder::Reviews,
        current: 0,
        target: 0,
        pct: 0.0,
        remaining: 0,
    }
}
