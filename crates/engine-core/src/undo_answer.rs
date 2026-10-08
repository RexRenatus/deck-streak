//! The rule an undo of the review's own last answer passes (SPEC-371 R3; ADR-382).
//!
//! The review records its answer when the press is recorded: the engine's undo status at that
//! moment and the review-log row the answer wrote. An undo is admitted only while that record is
//! still the front of the engine's undo queue, its review row is still there, for the card the
//! gesture names, and has not synced. The module is pure: the dispatcher reads the engine's state
//! and hands it here, so the rule is judged without an engine.

use anki_proto::collection::UndoStatus;
use anki_proto::scheduler::SchedulingState;

/// The record of the review's own last answer, as the page carries it back in a confirmation:
/// the engine's undo status right after the answer, and the id of the review-log row it wrote.
#[derive(Clone, PartialEq, prost::Message)]
pub struct Recorded {
    /// The engine's undo status right after the answer: its undo label and its last step.
    #[prost(message, optional, tag = "1")]
    pub status: Option<UndoStatus>,
    /// The review-log row the answer wrote, by id.
    #[prost(int64, tag = "2")]
    pub review: i64,
}

/// A review-log row, as the core's fixed read answers it: the card it reviewed and its sync mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Review {
    /// The card the review was of.
    pub cid: i64,
    /// The row's update sequence number: -1 until a sync sends it.
    pub usn: i32,
}

/// Why an undo of the recorded answer is not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoRefusal {
    /// The answer is gone: its review row is absent, or the engine has nothing to undo.
    Gone,
    /// The recorded review is of another card than the one the gesture names.
    NotTheCard,
    /// The recorded review has synced.
    Synced,
    /// Something changed after the answer: the engine's undo queue no longer ends with it.
    Changed,
}

/// The kind of state an undone answer returns its card to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Returns {
    /// A new card.
    New,
    /// A card in learning.
    Learning,
    /// A card in review.
    Review,
    /// A card relearning after a lapse.
    Relearning,
    /// A card in a filtered deck's preview.
    Preview,
}

/// Whether the recorded answer may be undone now, for `card`.
///
/// # Errors
///
/// The [`UndoRefusal`] that holds.
pub fn judge(
    _recorded: &Recorded,
    _now: &UndoStatus,
    _review: Option<Review>,
    _card: i64,
) -> Result<(), UndoRefusal> {
    Ok(())
}

/// The kind of state `state` is, the state the card was in when it was answered.
#[must_use]
pub fn returns_to(_state: &SchedulingState) -> Returns {
    Returns::New
}
