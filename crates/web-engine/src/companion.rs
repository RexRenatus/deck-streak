//! The scheduling companion on the web (SPEC-511 R9, R10; ADR-529 D3, D4).
//!
//! The owner's companion module runs in the Worker beside this engine, and the Worker moves bytes
//! only: every frame it sends the module is built here, and every reply is judged here, with the
//! interface crate's own codec and decision, so the web client decides as the native build does.
//! This module is compiled on every target, so the native tests run it over the same vectors as the
//! interface crate's decision; `wasm.rs` exports each function as one call.
//!
//! - [`decide`]: one press's answer over the hello, the `Preview` request and the reply frames,
//!   17 bytes for an admitted due date and 3 for a refusal.
//! - [`restore`] and [`purge`]: the `Restore` frame for a stored checkpoint, built only through
//!   `Restore::from(CheckpointReply)`, and the `Purge` frame.
//! - [`load_check`]: the web load check, which keeps a stored checkpoint only when every condition
//!   holds, and otherwise names one [`Verdict`] discard.

use deck_streak_companion::codec::{decode_reply, decode_request, encode_request};
use deck_streak_companion::decide::{NewDue, Refusal, Session, decide_frame, negotiate};
use deck_streak_companion::wire::{
    CheckpointReply, HelloReply, Preview, Reply, Request, Restore, Status,
};

/// The first byte of an admitted answer.
pub const ADMITTED: u8 = 0;
/// The first byte of a refused answer.
pub const REFUSED: u8 = 1;
/// The status byte of a refusal that carries no status.
pub const NO_STATUS: u8 = 0xff;

/// What the web load check says of a stored checkpoint: keep it, or one named discard, in the
/// order the check asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// Every condition holds: the checkpoint is kept.
    Keep,
    /// The hello does not negotiate.
    NotNegotiated,
    /// The stored frame is not an `Ok` checkpoint.
    Unreadable,
    /// The checkpoint belongs to another model.
    ModelChanged,
    /// The checkpoint was folded under another adapter mode.
    AdapterChanged,
    /// The companion did not take the checkpoint back.
    LoadError,
    /// The checkpoint's head names no review in the store.
    HeadNamesNoReview,
    /// Reviews were folded and no fold index is held.
    IndexMissing,
    /// The fold index's count or head is not the checkpoint's.
    IndexMismatch,
}

impl Verdict {
    /// The verdict's code: 0 to keep, else the discard's place in the declared order, from 1.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::Keep => 0,
            Self::NotNegotiated => 1,
            Self::Unreadable => 2,
            Self::ModelChanged => 3,
            Self::AdapterChanged => 4,
            Self::LoadError => 5,
            Self::HeadNamesNoReview => 6,
            Self::IndexMissing => 7,
            Self::IndexMismatch => 8,
        }
    }
}

/// The decision over one press's frames: `hello` is the companion's hello frame, `epoch` the
/// session's, `preview` the `Preview` request frame the press sent and `reply` the companion's
/// reply frame. The session is opened with the interface crate's `Session::new` and the reply is
/// judged by its `decide_frame`.
///
/// The answer is 17 bytes for an admitted due date: [`ADMITTED`], then `card` (i64),
/// `interval_days` (u32) and `due_day` (i32), little-endian. It is 3 bytes for a refusal:
/// [`REFUSED`], the refusal's place in `Refusal`'s declared order from 1, and the status for
/// `Refusal::Status`, else [`NO_STATUS`]. A hello frame that is not a hello negotiates nothing, and
/// a `preview` frame that is not a `Preview` request refuses as malformed: nothing is admitted from
/// a frame the engine cannot read.
#[must_use]
pub fn decide(hello: &[u8], epoch: u32, preview: &[u8], reply: &[u8]) -> Vec<u8> {
    let Some(asked) = preview_of(preview) else {
        return answer(Err(Refusal::Malformed));
    };
    let outcome = match hello_of(hello) {
        Some(hello) => decide_frame(&Session::new(epoch, &hello), &asked, reply),
        // The interface's order: a reply that does not decode is malformed before the session is
        // asked; any other reply meets a session with no negotiated model.
        None if decode_reply(reply).is_err() => Err(Refusal::Malformed),
        None => Err(Refusal::NotNegotiated),
    };
    answer(outcome)
}

/// The `Restore` frame at `epoch` for a stored checkpoint frame, built through
/// `Restore::from(CheckpointReply)`; empty when the stored frame is not an `Ok` checkpoint, or the
/// frame does not encode.
#[must_use]
pub fn restore(epoch: u32, checkpoint: &[u8]) -> Vec<u8> {
    match checkpoint_of(checkpoint) {
        Some(stored) => {
            encode_request(epoch, &Request::Restore(Restore::from(stored))).unwrap_or_default()
        }
        None => Vec::new(),
    }
}

/// The `Purge` frame at `epoch`.
#[must_use]
pub fn purge(epoch: u32) -> Vec<u8> {
    encode_request(epoch, &Request::Purge).unwrap_or_default()
}

/// The web load check over the companion's hello frame, the stored checkpoint frame and the
/// companion's reply frame to the `Restore` built from it; `head_named` says whether the reply's
/// head names a review in the store, and the fold index held, if any, is its count and its head.
///
/// The checkpoint is kept only when, in this order: the hello negotiates; the stored frame is a
/// `Checkpoint` with status `Ok`; its model id is the negotiated one and its adapter mode the
/// hello's; the reply is an `Ack` with status `Ok`; unless the `Ack` folded nothing, its head names
/// a review; and a fold index is held whose count and head are the `Ack`'s, or nothing was folded.
/// The first condition that fails is the discard.
#[must_use]
pub fn load_check(
    hello: &[u8],
    checkpoint: &[u8],
    ack: &[u8],
    head_named: bool,
    index_count: Option<u64>,
    index_head: Option<i64>,
) -> Verdict {
    let Some(hello) = hello_of(hello) else {
        return Verdict::NotNegotiated;
    };
    let Ok(model) = negotiate(&hello) else {
        return Verdict::NotNegotiated;
    };
    let Some(stored) = checkpoint_of(checkpoint) else {
        return Verdict::Unreadable;
    };
    if stored.model_id != model {
        return Verdict::ModelChanged;
    }
    if stored.adapter != hello.adapter {
        return Verdict::AdapterChanged;
    }
    let Ok(decoded) = decode_reply(ack) else {
        return Verdict::LoadError;
    };
    let Reply::Ack {
        status: Status::Ok,
        folded,
        head_t_ms,
    } = decoded.message
    else {
        return Verdict::LoadError;
    };
    if folded == 0 {
        return Verdict::Keep;
    }
    if !head_named {
        return Verdict::HeadNamesNoReview;
    }
    match (index_count, index_head) {
        (Some(count), Some(head)) if count == folded && head == head_t_ms => Verdict::Keep,
        (Some(_), Some(_)) => Verdict::IndexMismatch,
        _ => Verdict::IndexMissing,
    }
}

/// The hello a frame carries, or `None` when it is not a hello reply.
fn hello_of(frame: &[u8]) -> Option<HelloReply> {
    match decode_reply(frame).ok()?.message {
        Reply::Hello(hello) => Some(hello),
        _ => None,
    }
}

/// The press a `Preview` request frame carries, or `None` when it is not one.
fn preview_of(frame: &[u8]) -> Option<Preview> {
    match decode_request(frame).ok()?.message {
        Request::Preview(asked) => Some(asked),
        _ => None,
    }
}

/// The checkpoint a stored frame carries when it is a `Checkpoint` with status `Ok`.
fn checkpoint_of(frame: &[u8]) -> Option<CheckpointReply> {
    match decode_reply(frame).ok()?.message {
        Reply::Checkpoint(stored) if stored.status == Status::Ok => Some(stored),
        _ => None,
    }
}

/// The answer's bytes for a decision's outcome.
fn answer(outcome: Result<NewDue, Refusal>) -> Vec<u8> {
    match outcome {
        Ok(due) => {
            let mut bytes = Vec::with_capacity(17);
            bytes.push(ADMITTED);
            bytes.extend_from_slice(&due.card.to_le_bytes());
            bytes.extend_from_slice(&due.interval_days.to_le_bytes());
            bytes.extend_from_slice(&due.due_day.to_le_bytes());
            bytes
        }
        Err(refusal) => {
            let status = match refusal {
                Refusal::Status(status) => status as u8,
                _ => NO_STATUS,
            };
            vec![REFUSED, code(refusal), status]
        }
    }
}

/// A refusal's place in `Refusal`'s declared order, from 1.
fn code(refusal: Refusal) -> u8 {
    match refusal {
        Refusal::Malformed => 1,
        Refusal::NotNegotiated => 2,
        Refusal::Absent => 3,
        Refusal::Mismatch => 4,
        Refusal::Fixture => 5,
        Refusal::Status(_) => 6,
        Refusal::StaleEpoch => 7,
        Refusal::WrongCard => 8,
        Refusal::WrongStage => 9,
        Refusal::ModelChanged => 10,
        Refusal::IntervalOutOfRange => 11,
        Refusal::DueDayInconsistent => 12,
    }
}
