//! SPEC-386 A7 and A8 (R6): a memory state projected into stock values. Every expected value is
//! typed from the pinned revision's own tests, never computed by the crate.

use deck_streak_fsrs7::stock::{self, Stock};
use fsrs7::{FSRS, MemoryState};

/// The four first-review states and their intervals at a desired retention of 0.9, as the
/// pinned revision's `next_states` example asserts them (src/inference.rs:346-349): default
/// parameters, no previous state, 0 days elapsed, compared exactly (`assert_eq!`).
const FIRST_REVIEWS: [(&str, MemoryState, f32); 4] = [
    ("Again", state(0.1104, 6.1686, 0.08832), 3.827_562_6e-5),
    ("Hard", state(2.2395, 5.261_278, 1.791_600_1), 0.597_459_8),
    ("Good", state(3.9221, 3.530_724_3, 3.13768), 4.777_724_7),
    ("Easy", state(11.7841, 1.0, 9.427_279), 53.869_392),
];

/// The state the pinned revision's S90 test reads (src/inference_v7.rs:62-74), and that test's
/// tolerance on the retrievability at the interval.
const S90_STATE: MemoryState = state(10.0, 5.0, 8.0);
const S90_TOLERANCE: f32 = 1e-3;

const fn state(stability: f32, difficulty: f32, stability_fast: f32) -> MemoryState {
    MemoryState {
        stability,
        difficulty,
        stability_fast,
    }
}

#[test]
fn the_stock_stability_is_the_ninety_percent_interval() {
    let model = FSRS::default();
    for (rating, memory, interval) in FIRST_REVIEWS {
        let projected = stock::project(&model, memory, 0.8);
        assert_eq!(
            projected.stability.to_bits(),
            interval.to_bits(),
            "{rating}: the stock stability is {}, not the revision's interval at 0.9, {interval}",
            projected.stability
        );
    }
}

#[test]
fn the_difficulty_is_clamped_and_the_interval_meets_the_retention() {
    let model = FSRS::default();
    for (difficulty, stock) in [(11.0, 10.0), (0.5, 1.0), (5.0, 5.0)] {
        let projected = stock::project(&model, state(10.0, difficulty, 8.0), 0.9);
        assert_eq!(
            projected.difficulty.to_bits(),
            f32::to_bits(stock),
            "difficulty {difficulty} projects to {}, not {stock}",
            projected.difficulty
        );
    }

    for (rating, memory, interval) in FIRST_REVIEWS {
        let projected = stock::project(&model, memory, 0.9);
        assert_eq!(
            projected,
            Stock {
                stability: interval,
                difficulty: memory.difficulty,
                interval,
            },
            "{rating}: at a desired retention of 0.9 the interval is the revision's own"
        );
    }

    for retention in [0.8, 0.9, 0.95] {
        let projected = stock::project(&model, S90_STATE, retention);
        let read = model.current_retrievability(S90_STATE, projected.interval);
        assert!(
            (read - retention).abs() <= S90_TOLERANCE,
            "at the interval {} the curve reads {read}, not {retention} within {S90_TOLERANCE}",
            projected.interval
        );
    }
}
