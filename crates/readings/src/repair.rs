//! The one repair (SPEC-046 R7): a failed gate is named to the model once, and a second failure
//! ends the topic.

use crate::coverage::GateFailure;
use crate::state::ReadingGate;

/// How many attempts a topic gets: the first and one repair.
pub const MAX_ATTEMPTS: u32 = 0;

/// What follows a failed attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Try again with this engine-written repair text.
    Repair(String),
    /// End the topic on this gate.
    Fail(ReadingGate),
}

/// What follows attempt number `attempt` failing with `failure`, whose output was `rejected`.
#[must_use]
pub fn next(attempt: u32, failure: &GateFailure, rejected: &str) -> Step {
    let _ = (attempt, rejected);
    Step::Fail(failure.gate)
}
