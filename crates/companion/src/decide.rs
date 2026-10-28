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

/// The handshake: the model id when the companion is present, speaks [`MAJOR`] and holds a
/// trained model. A minor difference is admitted.
///
/// # Errors
///
/// [`Refusal::Absent`] for an absent companion, [`Refusal::Status`] for any other status but
/// `Ok`, [`Refusal::Mismatch`] for another major and [`Refusal::Fixture`] for a fixture model,
/// checked in that order.
pub fn negotiate(hello: &HelloReply) -> Result<[u64; 4], Refusal> {
    match hello.status {
        Status::Ok => {}
        Status::Absent => return Err(Refusal::Absent),
        status => return Err(Refusal::Status(status)),
    }
    if hello.major != MAJOR {
        return Err(Refusal::Mismatch);
    }
    if hello.model != Model::Trained {
        return Err(Refusal::Fixture);
    }
    Ok(hello.model_id)
}

/// The decision: `Ok` only when the session negotiated a model, the reply is a `Due` with
/// status `Ok`, its epoch is the session's, its card and its previewed review's id are the
/// press's, its model is the negotiated one, its interval is 1 to 36,500 days, and its due day
/// is the press's study day plus the interval by checked arithmetic.
///
/// # Errors
///
/// The first condition that fails, as a named [`Refusal`], in the order above. A reply that is
/// not a `Due` refuses by its status: [`Refusal::Absent`], [`Refusal::Status`], or
/// [`Refusal::WrongStage`] when its status is `Ok`.
pub fn decide(
    session: &Session,
    asked: &Preview,
    replied_epoch: u32,
    reply: &Reply,
) -> Result<NewDue, Refusal> {
    let Some(model) = session.model else {
        return Err(Refusal::NotNegotiated);
    };
    let Reply::Due {
        status,
        card,
        staged_t_ms,
        interval_days,
        due_day,
        model_id,
    } = *reply
    else {
        return Err(match reply.status() {
            Status::Absent => Refusal::Absent,
            Status::Ok => Refusal::WrongStage,
            status => Refusal::Status(status),
        });
    };
    if status == Status::Absent {
        return Err(Refusal::Absent);
    }
    if status != Status::Ok {
        return Err(Refusal::Status(status));
    }
    if replied_epoch != session.epoch {
        return Err(Refusal::StaleEpoch);
    }
    if card != asked.event.card {
        return Err(Refusal::WrongCard);
    }
    if staged_t_ms != asked.event.t_ms {
        return Err(Refusal::WrongStage);
    }
    if model_id != model {
        return Err(Refusal::ModelChanged);
    }
    if !(1..=INTERVAL_DAYS_MAX).contains(&interval_days) {
        return Err(Refusal::IntervalOutOfRange);
    }
    if asked.event.study_day.checked_add_unsigned(interval_days) != Some(due_day) {
        return Err(Refusal::DueDayInconsistent);
    }
    Ok(NewDue {
        card,
        interval_days,
        due_day,
    })
}

/// Decodes a reply frame, then decides: [`Refusal::Malformed`] for a frame that does not decode,
/// and otherwise [`decide`] over the frame's epoch and reply.
///
/// # Errors
///
/// [`Refusal::Malformed`], or any refusal of [`decide`].
pub fn decide_frame(session: &Session, asked: &Preview, frame: &[u8]) -> Result<NewDue, Refusal> {
    let decoded = decode_reply(frame).map_err(|_| Refusal::Malformed)?;
    decide(session, asked, decoded.epoch, &decoded.message)
}
