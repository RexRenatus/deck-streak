//! The handshake, and the one decision whose `Ok` lets a due date be written.
//!
//! **The caller's rule.** A [`Refusal`] means nothing is written and no other scheduler is asked:
//! no FSRS, no engine next state and no cached interval stands in for a refused due date.

use crate::codec::decode_reply;
use crate::wire::{HelloReply, INTERVAL_DAYS_MAX, MAJOR, Model, Preview, Reply, Status};

/// Why no due date is written. Each reason is named; none falls back to another scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// The reply frame did not decode.
    Malformed,
    /// The session has no negotiated model: the handshake failed or never ran.
    NotNegotiated,
    /// No companion is present.
    Absent,
    /// The companion speaks another schema major.
    Mismatch,
    /// The companion holds a fixture model.
    Fixture,
    /// The companion answered with a status other than `Ok`.
    Status(Status),
    /// The reply belongs to an earlier session.
    StaleEpoch,
    /// The reply is for another card.
    WrongCard,
    /// The reply answers another press, or is not a due date at all.
    WrongStage,
    /// The reply was computed by a model other than the negotiated one.
    ModelChanged,
    /// The interval is outside 1 to 36,500 days.
    IntervalOutOfRange,
    /// The due day is not the press's study day plus the interval.
    DueDayInconsistent,
}

/// The due date a press may write: the one output of [`decide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NewDue {
    /// The card.
    pub card: i64,
    /// The interval, in days.
    pub interval_days: u32,
    /// The study day the card falls due on.
    pub due_day: i32,
}

/// One companion session: its epoch, and the model its handshake negotiated, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Session {
    epoch: u32,
    model: Option<[u64; 4]>,
}

impl Session {
    /// The session `epoch` opened with the companion's `hello`. Its model is the negotiated one
    /// when [`negotiate`] is `Ok`, and none otherwise, so every later [`decide`] refuses with
    /// [`Refusal::NotNegotiated`].
    #[must_use]
    pub fn new(epoch: u32, hello: &HelloReply) -> Self {
        Self {
            epoch,
            model: negotiate(hello).ok(),
        }
    }

    /// The session's epoch.
    #[must_use]
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// The negotiated model, if the handshake was `Ok`.
    #[must_use]
    pub fn model(&self) -> Option<[u64; 4]> {
        self.model
    }
}

/// The handshake: this stub admits every hello.
///
/// # Errors
///
/// None in this stub.
pub fn negotiate(hello: &HelloReply) -> Result<[u64; 4], Refusal> {
    Ok(hello.model_id)
}

/// The decision: this stub admits every reply, due the next study day.
///
/// # Errors
///
/// None in this stub.
pub fn decide(
    _session: &Session,
    asked: &Preview,
    _replied_epoch: u32,
    _reply: &Reply,
) -> Result<NewDue, Refusal> {
    Ok(NewDue {
        card: asked.event.card,
        interval_days: 1,
        due_day: asked.event.study_day.wrapping_add(1),
    })
}

/// Decodes a reply frame, then decides: this stub admits every frame.
///
/// # Errors
///
/// None in this stub.
pub fn decide_frame(session: &Session, asked: &Preview, frame: &[u8]) -> Result<NewDue, Refusal> {
    decide(
        session,
        asked,
        0,
        &Reply::Refused {
            status: Status::Absent,
        },
    )
    .map(|due| {
        let _ = frame;
        due
    })
}
