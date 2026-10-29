//! The native review's undo, read and judged by the core (SPEC-515 R2 to R5; ADR-533 D2).
//!
//! The web engine keeps the review's last answer, bury or flag in its `wasm32` module and asks the
//! core's two rules about it (SPEC-371, SPEC-383). The native adapter names no protobuf codec of
//! its own, so this module reads for it: it records the review's last step from the undo status the
//! adapter read through its allow-list and the core's fixed reads, judges a record now with the same
//! two rules, names the card by its question as one line of text, and hands the adapter the
//! record's bytes, which the exempt Undo decodes and the dispatcher judges once more at the write.
//! It records as the web engine's module does: an answer only when the newest review is the
//! answered card's, and a bury or a flag only when the card's mark shows the change.

use anki_proto::collection::UndoStatus;
use anki_proto::scheduler::SchedulingState;
use prost::Message;

use crate::answer::Grade;
use crate::dispatch::{Dispatcher, Read, Refusal, failed};
use crate::undo_answer::{self, Kind, Recorded, Returns, Review, UndoRefusal};
use crate::undo_change::{self, Mark};

/// The engine's number for a card with no user flag.
const NO_FLAG: u32 = 0;

/// What an undo of a flag does to its card's flag (SPEC-383 R9, R11): the review's flag only sets
/// red or takes it off, so a change from no flag added red, a change to no flag removed red, and
/// any other change replaced another flag by setting red over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagChange {
    /// The card had no flag, and the change set red.
    Added,
    /// The card was red, and the change took it off.
    Removed,
    /// The card had another flag, and the change set red over it.
    Replaced,
}

impl FlagChange {
    /// The change from the flag `before` to the flag `after`.
    #[must_use]
    pub const fn of(before: u32, after: u32) -> Self {
        match (before, after) {
            (NO_FLAG, _) => Self::Added,
            (_, NO_FLAG) => Self::Removed,
            _ => Self::Replaced,
        }
    }
}

/// What the review's last recorded step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The review's own last answer.
    Answer {
        /// The grade the press recorded.
        grade: Grade,
        /// The kind of state the card was in when it was answered, which an undo returns it to.
        returns: Returns,
    },
    /// A bury of the shown card.
    Bury {
        /// The kind of state the card was in when it was buried, which an undo returns it to.
        returns: Returns,
    },
    /// A flag set on the shown card.
    Flag {
        /// What an undo does to the card's flag.
        change: FlagChange,
    },
}

/// The review's last recorded step, as the adapter keeps it in its one slot: the card it was on,
/// the engine's last step right after it, what it did, and the record the core judges an undo of
/// it by.
#[derive(Debug, Clone, PartialEq)]
pub struct Kept {
    card: i64,
    step: u32,
    did: Step,
    record: Recorded,
}

impl Kept {
    /// The card the step was on.
    #[must_use]
    pub const fn card(&self) -> i64 {
        self.card
    }

    /// The engine's last step right after the review's step, which a confirmation names.
    #[must_use]
    pub const fn step(&self) -> u32 {
        self.step
    }

    /// What the step did.
    #[must_use]
    pub const fn did(&self) -> Step {
        self.did
    }

    /// Whether a confirmation of `card` at `step` names this step.
    #[must_use]
    pub const fn names(&self, card: i64, step: u32) -> bool {
        self.card == card && self.step == step
    }

    /// The record the exempt Undo decodes, as the owner's gesture hands it to the dispatcher.
    #[must_use]
    pub fn request(&self) -> Vec<u8> {
        self.record.encode_to_vec()
    }
}

/// The review's own last answer of `grade` on `card`, recorded after the press from `status`, the
/// engine's undo status the adapter read through its allow-list, and the newest review; `None`
/// when the newest review is not the answered card's. `current` is the state the card was shown
/// in, which an undo returns it to.
///
/// # Errors
///
/// [`Refusal::Engine`] when `status` is not the engine's undo status, or the engine cannot answer
/// the newest review's read.
pub fn answered(
    dispatcher: &Dispatcher,
    status: &[u8],
    card: i64,
    grade: Grade,
    current: Option<&SchedulingState>,
) -> Result<Option<Kept>, Refusal> {
    let now = undo_status(status)?;
    let Some(newest) = newest_review(dispatcher)? else {
        return Ok(None);
    };
    if newest.card != card {
        return Ok(None);
    }
    let returns = current.map_or(Returns::New, undo_answer::returns_to);
    Ok(Some(Kept {
        card,
        step: now.last_step,
        did: Step::Answer { grade, returns },
        record: Recorded {
            status: Some(now),
            review: newest.id,
            kind: Kind::Answer.into(),
            flag: 0,
            card: 0,
        },
    }))
}

/// The review's bury of `card`, recorded after the write from `status`; `None` when the card's
/// mark does not show it user-buried. `current` is the state the card was shown in.
///
/// # Errors
///
/// [`Refusal::Engine`] when `status` is not the engine's undo status, or the engine cannot answer
/// the card's mark.
pub fn buried(
    dispatcher: &Dispatcher,
    status: &[u8],
    card: i64,
    current: Option<&SchedulingState>,
) -> Result<Option<Kept>, Refusal> {
    let now = undo_status(status)?;
    let returns = current.map_or(Returns::New, undo_answer::returns_to);
    let kept = Kept {
        card,
        step: now.last_step,
        did: Step::Bury { returns },
        record: Recorded {
            status: Some(now),
            review: 0,
            kind: Kind::Bury.into(),
            flag: 0,
            card,
        },
    };
    shown(dispatcher, kept)
}

/// The review's flag of `card` from `before` to `after`, recorded after the write from `status`;
/// `None` when the card's mark does not show the flag the change left.
///
/// # Errors
///
/// [`Refusal::Engine`] when `status` is not the engine's undo status, or the engine cannot answer
/// the card's mark.
pub fn flagged(
    dispatcher: &Dispatcher,
    status: &[u8],
    card: i64,
    before: u32,
    after: u32,
) -> Result<Option<Kept>, Refusal> {
    let now = undo_status(status)?;
    let kept = Kept {
        card,
        step: now.last_step,
        did: Step::Flag {
            change: FlagChange::of(before, after),
        },
        record: Recorded {
            status: Some(now),
            review: 0,
            kind: Kind::Flag.into(),
            flag: after,
            card,
        },
    };
    shown(dispatcher, kept)
}

/// Whether `kept` may be undone now, by the core's rule for its kind: [`undo_answer::judge`] for an
/// answer against its review row, [`undo_change::judge_change`] for a bury or a flag against its
/// card's mark, each against `status`, the engine's undo status the adapter read at this moment.
///
/// # Errors
///
/// [`Refusal::Engine`] when `status` is not the engine's undo status, or the engine cannot answer
/// a read.
pub fn judged(
    dispatcher: &Dispatcher,
    status: &[u8],
    kept: &Kept,
) -> Result<Result<(), UndoRefusal>, Refusal> {
    let now = undo_status(status)?;
    Ok(match kept.did {
        Step::Answer { .. } => undo_answer::judge(
            &kept.record,
            &now,
            review(dispatcher, kept.record.review)?,
            kept.card,
        ),
        Step::Bury { .. } | Step::Flag { .. } => {
            undo_change::judge_change(&kept.record, &now, mark(dispatcher, kept.card)?, kept.card)
        }
    })
}

/// `card`'s question as one line of text, as the web engine's offer names a card (SPEC-371 R8):
/// the question's nodes joined, its AV tags stripped, and its HTML made one line with media file
/// names kept.
///
/// # Errors
///
/// [`Refusal::Engine`] when the engine cannot render the card.
pub fn question_line(dispatcher: &Dispatcher, card: i64) -> Result<String, Refusal> {
    dispatcher.question_line(card)
}

/// `kept` when its card's mark shows its change, and `None` otherwise.
fn shown(dispatcher: &Dispatcher, kept: Kept) -> Result<Option<Kept>, Refusal> {
    let mark = mark(dispatcher, kept.card)?;
    Ok(mark
        .filter(|mark| undo_change::shows(&kept.record, mark))
        .map(|_| kept))
}

/// The engine's undo status, from the reply of `CollectionService.GetUndoStatus` (3,7).
fn undo_status(status: &[u8]) -> Result<UndoStatus, Refusal> {
    UndoStatus::decode(status).map_err(|_| failed("the engine's undo status is not its message"))
}

/// One fixed read's rows, as the engine's database door answers them.
fn rows(dispatcher: &Dispatcher, read: Read) -> Result<serde_json::Value, Refusal> {
    let reply = dispatcher.read(read)?;
    serde_json::from_slice(&reply).map_err(|_| failed("a fixed read's reply is not its rows"))
}

/// The newest review-log row: its id and the card it reviewed.
struct Newest {
    id: i64,
    card: i64,
}

/// The newest review-log row, by the core's fixed read, or `None` when the collection holds no
/// review.
fn newest_review(dispatcher: &Dispatcher) -> Result<Option<Newest>, Refusal> {
    let rows = rows(dispatcher, Read::NewestReview)?;
    let Some(row) = rows.pointer("/0") else {
        return Ok(None);
    };
    let id = row.pointer("/0").and_then(serde_json::Value::as_i64);
    let card = row.pointer("/1").and_then(serde_json::Value::as_i64);
    match (id, card) {
        (Some(id), Some(card)) => Ok(Some(Newest { id, card })),
        _ => Err(failed(
            "the newest review read answered other than a row and a card",
        )),
    }
}

/// The review-log row `id`, by the core's fixed read, or `None` when the collection lacks it.
fn review(dispatcher: &Dispatcher, id: i64) -> Result<Option<Review>, Refusal> {
    let rows = rows(dispatcher, Read::Review(id))?;
    let Some(row) = rows.pointer("/0") else {
        return Ok(None);
    };
    let cid = row.pointer("/0").and_then(serde_json::Value::as_i64);
    let usn = row
        .pointer("/1")
        .and_then(serde_json::Value::as_i64)
        .and_then(|usn| i32::try_from(usn).ok());
    match (cid, usn) {
        (Some(cid), Some(usn)) => Ok(Some(Review { cid, usn })),
        _ => Err(failed(
            "the review read answered other than a card and a sync mark",
        )),
    }
}

/// `card`'s mark, by the core's fixed read, or `None` when the collection lacks the card.
fn mark(dispatcher: &Dispatcher, card: i64) -> Result<Option<Mark>, Refusal> {
    let rows = rows(dispatcher, Read::CardMark(card))?;
    Mark::from_rows(&rows)
        .map_err(|_| failed("the mark read answered other than a queue, flags and a sync mark"))
}
