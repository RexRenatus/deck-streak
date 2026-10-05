//! The replay: each card's item through the pinned revision's FSRS-7 defaults, card by card or in
//! one batch (SPEC-342 R6).

use fsrs7::{FSRS, FSRSError, FSRSItem, MemoryState};

/// How the items are replayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    /// One memory-state call per card.
    Single,
    /// The upstream batch call, once for every card.
    Batch,
}

impl Method {
    /// Both methods, in the order the grid runs them.
    pub const ALL: [Self; 2] = [Self::Single, Self::Batch];

    /// The method's name in the fixed line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Batch => "batch",
        }
    }

    /// The method a fixed line names, if it names one.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|method| method.name() == name)
    }
}

/// Each item's memory state, from no starting state, by `method`.
///
/// # Errors
///
/// The scheduler's error, when an item is not a review history it can replay.
pub fn replay(
    method: Method,
    model: &FSRS,
    items: Vec<FSRSItem>,
) -> Result<Vec<MemoryState>, FSRSError> {
    match method {
        Method::Single => items
            .into_iter()
            .map(|item| model.memory_state(item, None))
            .collect(),
        Method::Batch => {
            let starting_states = vec![None; items.len()];
            model.memory_state_batch(items, starting_states)
        }
    }
}

/// The sum of the states' stabilities: the same work replayed twice sums to the same number, so
/// two targets' replays are compared by it (SPEC-342 R6, R8).
#[must_use]
pub fn checksum(states: &[MemoryState]) -> f64 {
    states.iter().map(|state| f64::from(state.stability)).sum()
}
