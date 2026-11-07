//! The replay's bench (SPEC-618 R9, R13): the pinned revision's FSRS-7 over the
//! items `convert` makes from a 20,000-row review log, card by card and as one batch, both of the
//! replay's modes (SPEC-342 R6).
//!
//! The log is the `measure` module's synthetic history, fixed and with no randomness, built before
//! the measured function runs; no bench asserts a value the replay computes. The measured body is
//! the named `#[inline(never)]` function `replay_measured`, which the Divan closure calls with a
//! fresh copy of the items, made outside the measured section.

#![allow(
    clippy::expect_used,
    reason = "a bench whose fixed input cannot be built should stop, as a failed test does"
)]

use deck_streak_fsrs7::convert;
use deck_streak_fsrs7::measure::history;
use deck_streak_fsrs7::replay::Method;
use deck_streak_fsrs7::stock::{FSRS, MemoryState};
use divan::Bencher;
use fsrs7::FSRSItem;

/// The review log's rows, and the mean reviews of each of its cards.
const ROWS: usize = 20_000;
const MEAN: usize = 8;

fn main() {
    divan::main();
}

#[divan::bench(args = ["single", "batch"])]
fn replay(bencher: Bencher, method: &str) {
    let method = Method::named(method).expect("each argument names a replay method");
    let model = FSRS::new(&[]).expect("the pinned revision's defaults make a model");
    let items: Vec<FSRSItem> = convert::histories(&history::rows(ROWS, MEAN))
        .into_iter()
        .map(|card| card.item)
        .collect();
    bencher
        .with_inputs(|| items.clone())
        .bench_local_values(|items| replay_measured(method, &model, items));
}

/// Each item's memory state from no starting state, by `method`.
#[inline(never)]
fn replay_measured(method: Method, model: &FSRS, items: Vec<FSRSItem>) -> Option<Vec<MemoryState>> {
    deck_streak_fsrs7::replay::replay(method, model, items).ok()
}
