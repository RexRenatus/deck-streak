//! SPEC-386 A7 and A8 (R6): a memory state projected into stock values; SPEC-413 A1 (R1, R2): a
//! stock stability is never below the revision's least, which the store keeps above zero. Every
//! expected value is typed from the pinned revision's own tests and code, never computed by the
//! crate.

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

/// The least stock stability, typed: the pinned revision's least stability (its
/// src/simulation.rs:199), never read from the crate (SPEC-413 R1).
const FLOOR: f32 = 0.0001;

/// The stock stability of each first review, in `FIRST_REVIEWS`' order: its interval at 0.9, but
/// Again's, which is below `FLOOR` and is held at it (SPEC-413 R2, ADR-427 D1).
const FIRST_STOCK_STABILITIES: [f32; 4] = [FLOOR, 0.597_459_8, 4.777_724_7, 53.869_392];

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
    for ((rating, memory, interval), want) in FIRST_REVIEWS.into_iter().zip(FIRST_STOCK_STABILITIES)
    {
        let projected = stock::project(&model, memory, 0.8);
        assert_eq!(
            projected.stability.to_bits(),
            want.to_bits(),
            "{rating}: the stock stability is {}, not {want}, the stock stability of the \
             revision's interval at 0.9, {interval}",
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

    for ((rating, memory, interval), stability) in
        FIRST_REVIEWS.into_iter().zip(FIRST_STOCK_STABILITIES)
    {
        let projected = stock::project(&model, memory, 0.9);
        assert_eq!(
            projected,
            Stock {
                stability,
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

/// The engine's store of a stability: rounded to four decimal places, as its card data's
/// `round_to_places` does, typed here rather than read from the engine.
fn stored(stability: f32) -> f32 {
    (stability * 10_000.0).round() / 10_000.0
}

#[test]
fn a_first_review_rated_again_keeps_a_stock_stability_the_store_holds() {
    let model = FSRS::default();
    let (rating, memory, interval) = FIRST_REVIEWS[0];
    assert_eq!(
        stored(interval).to_bits(),
        0.0_f32.to_bits(),
        "{rating}: the revision's interval at 0.9, {interval}, is stored as 0"
    );

    let projected = stock::project(&model, memory, 0.9);
    assert_eq!(
        projected.stability.to_bits(),
        FLOOR.to_bits(),
        "{rating}: the stock stability is {}, stored as {}, not the floor {FLOOR}",
        projected.stability,
        stored(projected.stability)
    );
    assert_eq!(
        stored(projected.stability).to_bits(),
        FLOOR.to_bits(),
        "{rating}: the store keeps the floor as {}, not {FLOOR}",
        stored(projected.stability)
    );
}
