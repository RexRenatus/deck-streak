//! The next milestone (SPEC-073 R14): across the review, streak and mature-card ladders, the nearest
//! unreached rung by the remaining fraction of that rung, ties to reviews, then the streak, then
//! mature cards; a complete ladder contributes nothing, and with all three complete the milestone
//! is the top review rung at 100%. Equal to `goldens/next_milestone.json`, and proved as a rule
//! over exact rationals in `formal/lean/Formal/NextMilestone.lean`.

/// The lifetime-review ladder.
pub const REVIEW_LADDER: [u64; 6] = [100, 500, 1_000, 5_000, 10_000, 50_000];
/// The streak ladder, in days.
pub const STREAK_LADDER: [u64; 5] = [7, 30, 100, 365, 1_000];
/// The mature-card ladder.
pub const MATURE_LADDER: [u64; 4] = [100, 500, 1_000, 5_000];

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
        match self {
            Self::Reviews => "Lifetime reviews",
            Self::Streak => "Day streak",
            Self::Mature => "Mature cards",
        }
    }

    /// The emoji a screen shows.
    #[must_use]
    pub const fn emoji(self) -> &'static str {
        match self {
            Self::Reviews => "📚",
            Self::Streak => "🔥",
            Self::Mature => "🌳",
        }
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
    let top = REVIEW_LADDER[REVIEW_LADDER.len() - 1];
    // Three complete ladders report the top review rung, fully reached.
    let complete = Milestone {
        ladder: Ladder::Reviews,
        current: top,
        target: top,
        pct: 100.0,
        remaining: 0,
    };
    let reviews = candidate(Ladder::Reviews, reviews);
    let streak = candidate(Ladder::Streak, streak);
    let mature = candidate(Ladder::Mature, mature);
    pick(pick(reviews, streak), mature).unwrap_or(complete)
}

/// `value`'s nearest unreached rung on `ladder`, the smallest rung strictly above it; `None` when
/// the ladder is complete.
fn candidate(ladder: Ladder, value: u64) -> Option<Milestone> {
    let target = *ladder.rungs().iter().find(|rung| **rung > value)?;
    Some(Milestone {
        ladder,
        current: value,
        target,
        pct: float(value) / float(target) * 100.0,
        remaining: target - value,
    })
}

/// Whether `a` has strictly the smaller remaining fraction of its rung. The fractions are compared
/// by cross-multiplication, so the comparison is exact; the predecessor's float quotients agree
/// with it, since two distinct fractions over rungs of at most 50,000 differ by far more than a
/// float's rounding.
fn smaller(a: &Milestone, b: &Milestone) -> bool {
    (a.target - a.current) * b.target < (b.target - b.current) * a.target
}

/// The pick over two ladders in the tie order: the earlier keeps a tie, and the later wins only
/// when its remaining fraction is strictly smaller.
fn pick(earlier: Option<Milestone>, later: Option<Milestone>) -> Option<Milestone> {
    match (earlier, later) {
        (None, only) | (only, None) => only,
        (Some(earlier), Some(later)) => Some(if smaller(&later, &earlier) {
            later
        } else {
            earlier
        }),
    }
}

/// `value` as a float, as the predecessor's int-to-float division takes it.
#[allow(
    clippy::cast_precision_loss,
    reason = "a rung is at most 50,000 and a candidate's value is below its rung, so both convert exactly"
)]
const fn float(value: u64) -> f64 {
    value as f64
}
