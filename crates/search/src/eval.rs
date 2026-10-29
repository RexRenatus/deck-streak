//! The stub of the evaluator: it selects nothing.

use crate::collection::Collection;
use crate::day::EngineDay;
use crate::query::Query;

/// The engine's default learn-ahead limit, in seconds.
pub const LEARN_AHEAD_SECS: i64 = 1_200;

/// The stub's evaluation: no card.
#[must_use]
pub fn eval(query: &Query, collection: &Collection, day: EngineDay) -> Vec<usize> {
    let _ = (query, collection, day);
    Vec::new()
}

/// The stub's selection: never.
#[must_use]
pub fn selects(query: &Query, collection: &Collection, day: EngineDay, index: usize) -> bool {
    let _ = (query, collection, day, index);
    false
}
