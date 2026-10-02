//! A card's memory state: what the scheduler stored in `cards.data` about how durable a memory is.
//!
//! The predecessor's `parse_fsrs` is the oracle (SPEC-077 R1): the parse is total, so a card the
//! scheduler never gave a state to, or gave a state we cannot read, is simply a card with none.

/// The decay a card gets when its stored one is missing or not positive.
pub const DEFAULT_DECAY: f64 = 0.2;

/// How durable one card's memory is, as the scheduler stored it.
///
/// Equality compares each float's bits, so it is an equivalence relation and `Card` keeps `Eq`.
#[derive(Clone, Copy, Debug)]
pub struct MemoryState {
    /// Days for recall to fall to the target retention: finite and positive.
    pub stability: f64,
    /// How hard the card is; 0.0 when the scheduler stored none.
    pub difficulty: f64,
    /// The forgetting curve's decay: positive, `DEFAULT_DECAY` when none was stored.
    pub decay: f64,
    /// The retention the card was scheduled for, when stored.
    pub desired_retention: Option<f64>,
    /// The last review's time in epoch seconds, when stored.
    pub last_review_sec: Option<i64>,
}

impl PartialEq for MemoryState {
    fn eq(&self, other: &Self) -> bool {
        self.stability.to_bits() == other.stability.to_bits()
            && self.difficulty.to_bits() == other.difficulty.to_bits()
            && self.decay.to_bits() == other.decay.to_bits()
            && self.desired_retention.map(f64::to_bits) == other.desired_retention.map(f64::to_bits)
            && self.last_review_sec == other.last_review_sec
    }
}

impl Eq for MemoryState {}

/// The memory state in a card's `data` text, or none.
#[must_use]
pub fn parse(_data: Option<&str>) -> Option<MemoryState> {
    None
}
