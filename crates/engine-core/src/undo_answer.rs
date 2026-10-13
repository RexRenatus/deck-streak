//! The rule an undo of the review's own last answer passes (SPEC-371 R3; ADR-382).
//!
//! The review records its answer when the press is recorded: the engine's undo status at that
//! moment and the review-log row the answer wrote. An undo is admitted only while that record is
//! still the front of the engine's undo queue, its review row is still there, for the card the
//! gesture names, and has not synced. The module is pure: the dispatcher reads the engine's state
//! and hands it here, so the rule is judged without an engine.

use anki_proto::collection::UndoStatus;
use anki_proto::scheduler::SchedulingState;
use anki_proto::scheduler::scheduling_state::{self, Normal, filtered, normal};

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
    /// What the record is of, as [`Kind`] numbers it (SPEC-383 R3). A record that carries no kind
    /// decodes as an answer, so every record of an answer reads as it did before kinds.
    #[prost(enumeration = "Kind", tag = "3")]
    pub kind: i32,
    /// The user flag a flag's change left on its card (SPEC-383 R3); 0 for an answer or a bury.
    #[prost(uint32, tag = "4")]
    pub flag: u32,
    /// The card a bury's or a flag's change was made on (SPEC-383 R3); 0 for an answer, whose
    /// review row names its card.
    #[prost(int64, tag = "5")]
    pub card: i64,
}

/// What a [`Recorded`] is of (SPEC-383 R3): the review's own last answer, or its last bury or
/// flag. A number no variant names is refused at the door, never read as one of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum Kind {
    /// An answer, judged by [`judge`] against its review row.
    Answer = 0,
    /// A bury of one card, judged by `undo_change::judge_change` against the card's mark.
    Bury = 1,
    /// A flag of one card, judged by `undo_change::judge_change` against the card's mark.
    Flag = 2,
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

/// Whether the recorded answer may be undone now, for `card`: `now` is the engine's undo status
/// and `review` the recorded review row, both read at the moment of the undo.
///
/// # Errors
///
/// The first refusal that holds, in this order: [`UndoRefusal::Gone`] when the review row is
/// absent, [`UndoRefusal::NotTheCard`] when it is of another card, [`UndoRefusal::Synced`] when a
/// sync has sent it, [`UndoRefusal::Gone`] when the engine has nothing to undo, and
/// [`UndoRefusal::Changed`] when the engine's last step or undo label is not the record's.
pub fn judge(
    recorded: &Recorded,
    now: &UndoStatus,
    review: Option<Review>,
    card: i64,
) -> Result<(), UndoRefusal> {
    let Some(review) = review else {
        return Err(UndoRefusal::Gone);
    };
    if review.cid != card {
        return Err(UndoRefusal::NotTheCard);
    }
    if review.usn != -1 {
        return Err(UndoRefusal::Synced);
    }
    if now.undo.is_empty() {
        return Err(UndoRefusal::Gone);
    }
    let then = recorded.status.as_ref();
    let last_step = then.map_or(0, |then| then.last_step);
    let label = then.map_or("", |then| then.undo.as_str());
    if now.last_step != last_step {
        return Err(UndoRefusal::Changed);
    }
    if now.undo != label {
        return Err(UndoRefusal::Changed);
    }
    Ok(())
}

/// The kind of state `state` is: the state the card was in when it was answered, so the state an
/// undo returns it to. A filtered card's rescheduling state answers its original's kind, and a
/// filtered preview answers [`Returns::Preview`].
#[must_use]
pub fn returns_to(state: &SchedulingState) -> Returns {
    match &state.kind {
        Some(scheduling_state::Kind::Normal(state)) => normal_kind(Some(state)),
        Some(scheduling_state::Kind::Filtered(state)) => match &state.kind {
            Some(filtered::Kind::Rescheduling(rescheduling)) => {
                normal_kind(rescheduling.original_state.as_ref())
            }
            Some(filtered::Kind::Preview(_)) => Returns::Preview,
            None => Returns::New,
        },
        None => Returns::New,
    }
}

/// The kind of a normal state, or new when there is none.
fn normal_kind(state: Option<&Normal>) -> Returns {
    match state.and_then(|state| state.kind.as_ref()) {
        Some(normal::Kind::Learning(_)) => Returns::Learning,
        Some(normal::Kind::Review(_)) => Returns::Review,
        Some(normal::Kind::Relearning(_)) => Returns::Relearning,
        Some(normal::Kind::New(_)) | None => Returns::New,
    }
}
