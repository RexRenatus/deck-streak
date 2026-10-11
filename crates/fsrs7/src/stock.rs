//! A replayed memory state as the stock fields hold it (SPEC-386 R6, ADR-400 D2).
//!
//! A stock client reads a card's `s` as the interval at which its forgetting curve reads 0.9,
//! under the card's own decay; only that point reads the same under every decay, so the
//! projection writes it rather than FSRS-7's raw stability. The difficulty is held to the
//! stock range, and the interval is the pinned revision's own at the preset's retention. A
//! 90-percent interval below the revision's least stability is held at it, which the engine's
//! store keeps above zero (SPEC-413 R1, ADR-427 D1).

pub use fsrs7::{FSRS, MemoryState};

/// The retrievability at which a stock client reads a card's stability: its `s` is the interval
/// at which the forgetting curve reads this.
pub const STOCK_RETENTION: f32 = 0.9;

/// The least difficulty a stock client holds.
pub const DIFFICULTY_MIN: f32 = 1.0;

/// The greatest difficulty a stock client holds.
pub const DIFFICULTY_MAX: f32 = 10.0;

/// The least stock stability, in days: the pinned revision's own least stability, which it keeps
/// private to its simulation, and the least value the engine's four-place store keeps above zero
/// (SPEC-413 R1, ADR-427 D1).
pub const STABILITY_MIN: f32 = 0.0001;

/// A memory state's stock values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stock {
    /// The interval, in days, at which the revision's forgetting curve reads 0.9, held at
    /// `STABILITY_MIN` when it is shorter.
    pub stability: f32,
    /// The difficulty, held to the stock range.
    pub difficulty: f32,
    /// The interval, in days, at which the curve reads the preset's desired retention.
    pub interval: f32,
}

/// The stock stability of a 90-percent interval: the interval, or `STABILITY_MIN` when it is
/// shorter or NaN. A shorter stability is stored as 0, which the engine's curve divides by
/// (SPEC-413 R1, ADR-427 D1).
#[must_use]
pub fn stock_stability(interval: f32) -> f32 {
    interval
}

/// The stock values of `state` under `model`, with the interval at `retention`. Both intervals
/// are the pinned revision's own, by its interval function at a retrievability; the stability is
/// the 90-percent one's stock stability.
#[must_use]
pub fn project(model: &FSRS, state: MemoryState, retention: f32) -> Stock {
    Stock {
        stability: stock_stability(model.interval_at_retrievability(state, STOCK_RETENTION)),
        difficulty: state.difficulty.clamp(DIFFICULTY_MIN, DIFFICULTY_MAX),
        interval: model.interval_at_retrievability(state, retention),
    }
}
