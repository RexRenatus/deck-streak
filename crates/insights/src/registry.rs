//! The registry (SPEC-094 R6, ADR-094): one row per instrument, live or inert.

use crate::dark_fields;
use crate::instrument::Cadence;

/// Whether a row runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The instrument runs on its cadence and on demand.
    Live,
    /// The instrument never runs; reviving it is this one row.
    Inert,
}

/// One instrument's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// The instrument's id.
    pub id: &'static str,
    /// How often it runs.
    pub cadence: Cadence,
    /// Whether it runs.
    pub state: State,
}

/// Every instrument, one row each.
pub const ROWS: &[Row] = &[Row {
    id: dark_fields::ID,
    cadence: Cadence::Weekly,
    state: State::Live,
}];

/// The rows that may run, in the registry's order.
#[must_use]
pub fn runnable(rows: &[Row]) -> Vec<Row> {
    rows.iter()
        .copied()
        .filter(|row| row.state == State::Live)
        .collect()
}

/// The runnable row for `id`, or none when it is unknown or inert.
#[must_use]
pub fn find(rows: &[Row], id: &str) -> Option<Row> {
    runnable(rows).into_iter().find(|row| row.id == id)
}
