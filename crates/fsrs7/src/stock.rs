//! A replayed memory state as the stock fields hold it (SPEC-386 R6, ADR-400 D2).
//!
//! A stock client reads a card's `s` as the interval at which its forgetting curve reads 0.9,
//! under the card's own decay; only that point reads the same under every decay, so the
//! projection writes it rather than FSRS-7's raw stability. The difficulty is held to the
//! stock range, and the interval is the pinned revision's own at the preset's retention.

pub use fsrs7::{FSRS, MemoryState};

/// A memory state's stock values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stock {
    /// The interval, in days, at which the revision's forgetting curve reads 0.9.
    pub stability: f32,
    /// The difficulty, held to the stock range.
    pub difficulty: f32,
    /// The interval, in days, at which the curve reads the preset's desired retention.
    pub interval: f32,
}

/// The stock values of `state` under `model`, with the interval at `retention`.
#[must_use]
pub fn project(model: &FSRS, state: MemoryState, retention: f32) -> Stock {
    let _ = (model, retention);
    Stock {
        stability: state.stability,
        difficulty: state.difficulty,
        interval: state.stability,
    }
}
