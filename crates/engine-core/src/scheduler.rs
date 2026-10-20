//! The scheduling port (SPEC-494 R1, R5 to R8; ADR-512 D2, D4 and D5): the one interface a due
//! day comes through, its review event, the event's key, a probability of recall, the port's two
//! refusals, the event check every implementation shares, and the day queue with its sibling rule.
//!
//! The module names no other crate of this workspace, and reads no clock and no timezone: a
//! producer computes each event's study day and hands it in.

use std::collections::BTreeSet;
use std::fmt;

/// One review, as an implementation of the port folds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewEvent {
    /// The card reviewed.
    pub card: i64,
    /// The card's note, or 0 or less when the producer reads none.
    pub note: i64,
    /// The card's deck.
    pub deck: i64,
    /// The deck's preset.
    pub preset: i64,
    /// The button pressed, 1 to 4.
    pub grade: u8,
    /// The time the answer took, in milliseconds.
    pub answer_ms: u32,
    /// The review's key, its instant.
    pub key: EventKey,
    /// The review's study day, in the producer's frame of days.
    pub study_day: i32,
}

/// A review's instant in milliseconds, as the review log's id stores it: a total order, strictly
/// increasing over one learner's events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventKey(pub i64);

/// A probability of recall, always within 2^-13 and 1 - 2^-13 and never NaN.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recall(f32);

impl Recall {
    /// The least recall, 2^-13.
    pub const MIN: f32 = 1.0 / 8192.0;
    /// The greatest recall, 1 - 2^-13.
    pub const MAX: f32 = 1.0 - 1.0 / 8192.0;

    /// The recall `value`, or `None` when it lies outside [`Self::MIN`] to [`Self::MAX`] or is
    /// NaN.
    #[must_use]
    pub fn new(value: f32) -> Option<Self> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return None;
        }
        Some(Self(value))
    }

    /// The recall `value` held to [`Self::MIN`] to [`Self::MAX`], or `None` for a NaN.
    #[must_use]
    pub fn clamped(value: f32) -> Option<Self> {
        if value.is_nan() {
            return None;
        }
        Some(Self(value.clamp(Self::MIN, Self::MAX)))
    }

    /// The recall's value.
    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }
}

/// The longest answer time a fold admits, in milliseconds: one hour.
pub const ANSWER_MS_CAP: u32 = 3_600_000;

/// The note id a card with no note carries: a card whose note id is this or less takes no part in
/// the sibling rule.
pub const NO_NOTE: i64 = 0;

/// Why a fold, a probe or an interval is refused. No variant carries text a user wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldError {
    /// The event's key is not after the last folded key.
    KeyNotAfter {
        /// The last folded key.
        last: EventKey,
        /// The refused event's key.
        key: EventKey,
    },
    /// The event's grade is outside 1 to 4.
    GradeOutOfRange {
        /// The refused grade.
        grade: u8,
    },
    /// The event's answer time is above [`ANSWER_MS_CAP`].
    AnswerTooLong {
        /// The refused answer time, in milliseconds.
        answer_ms: u32,
    },
    /// The state holds no review of the card.
    UnknownCard {
        /// The card asked for.
        card: i64,
    },
    /// The day asked for is before the card's last review.
    BeforeLastReview {
        /// The card asked for.
        card: i64,
        /// The day asked for.
        at_day: i32,
    },
    /// The implementation's value for the card is not a finite number.
    NotFinite {
        /// The card asked for.
        card: i64,
    },
    /// The state was made under another model's parameters.
    OtherModel,
    /// The model refuses its parameters or a card's history.
    ModelRefused,
}

impl fmt::Display for FoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::KeyNotAfter { .. } => "the review event's key is not after the last folded key",
            Self::GradeOutOfRange { .. } => "the review event's grade is outside 1 to 4",
            Self::AnswerTooLong { .. } => {
                "the review event's answer time is above the cap of 3,600,000 milliseconds"
            }
            Self::UnknownCard { .. } => "the scheduling state holds no review of the card",
            Self::BeforeLastReview { .. } => "the day asked for is before the card's last review",
            Self::NotFinite { .. } => "the scheduler's value for the card is not a finite number",
            Self::OtherModel => "the scheduling state was made under another model's parameters",
            Self::ModelRefused => {
                "the scheduler refuses the model's parameters or a card's history"
            }
        })
    }
}

impl std::error::Error for FoldError {}

/// Why a checkpoint is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The bytes end before the checkpoint does.
    Truncated,
    /// The bytes are no checkpoint of the port.
    NotACheckpoint,
    /// The checkpoint is another implementation's.
    OtherModel,
    /// The checkpoint holds an event the fold refuses.
    Refused(FoldError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Truncated => "the checkpoint's bytes end before its events do",
            Self::NotACheckpoint => "the bytes are not a checkpoint of the scheduling port",
            Self::OtherModel => "the checkpoint is another implementation's",
            Self::Refused(_) => "the checkpoint holds an event the fold refuses",
        })
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Refused(error) => Some(error),
            _ => None,
        }
    }
}

/// The scheduling port: one implementation of a memory model, folding a learner's reviews into a
/// state it can probe, schedule from and checkpoint.
pub trait SchedulerPort {
    /// The implementation's folded state.
    type State;
    /// A fixed identifier per implementation, the same whatever its parameters.
    fn version_id(&self) -> [u8; 32];
    /// A state with no review folded.
    fn new_state(&self) -> Self::State;
    /// Folds one review into `st`. A key not after the last folded key is refused, and the state
    /// is unchanged after any error.
    ///
    /// # Errors
    ///
    /// A [`FoldError`] naming the refused event.
    fn fold(&self, st: &mut Self::State, ev: &ReviewEvent) -> Result<(), FoldError>;
    /// The card's recall on `at_day`: a pure probe that changes nothing.
    ///
    /// # Errors
    ///
    /// A [`FoldError`] for an unknown card, a day before its last review or a value that is not
    /// finite.
    fn predict(&self, st: &Self::State, card: i64, at_day: i32) -> Result<Recall, FoldError>;
    /// The card's whole days at the `target` recall, by the implementation's own whole-day rule;
    /// FSRS-7's is its interval rounded to the nearest whole day (SPEC-494 R1, R2).
    ///
    /// # Errors
    ///
    /// A [`FoldError`] for an unknown card or a state made under another model.
    fn interval_days(&self, st: &Self::State, card: i64, target: Recall) -> Result<u32, FoldError>;
    /// The state's checkpoint: byte-stable, holding the implementation's fixed identifier and the
    /// folded events.
    fn checkpoint(&self, st: &Self::State) -> Vec<u8>;
    /// The state a checkpoint holds.
    ///
    /// # Errors
    ///
    /// A [`LoadError`] for truncated bytes, bytes that are no checkpoint, another
    /// implementation's checkpoint or an event the fold refuses.
    fn restore(&self, bytes: &[u8]) -> Result<Self::State, LoadError>;
}

/// The check every implementation's fold makes first: the event's key after `last`, its grade 1
/// to 4 and its answer time at most [`ANSWER_MS_CAP`] (SPEC-494 R8).
///
/// # Errors
///
/// A [`FoldError`] naming the first check the event fails.
pub fn check_event(last: Option<EventKey>, ev: &ReviewEvent) -> Result<(), FoldError> {
    if let Some(last) = last.filter(|last| ev.key <= *last) {
        return Err(FoldError::KeyNotAfter { last, key: ev.key });
    }
    if !(1..=4).contains(&ev.grade) {
        return Err(FoldError::GradeOutOfRange { grade: ev.grade });
    }
    if ev.answer_ms > ANSWER_MS_CAP {
        return Err(FoldError::AnswerTooLong {
            answer_ms: ev.answer_ms,
        });
    }
    Ok(())
}

/// One card as the day queue reads it: its id, its note and its due day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueCard {
    /// The card.
    pub card: i64,
    /// The card's note, or 0 or less for none.
    pub note: i64,
    /// The card's due day.
    pub due: i32,
}

/// A study day's queue, built from per-card due days.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DayQueue {
    /// The study day.
    pub day: i32,
    /// The cards queued on the day, in the day's order.
    pub cards: Vec<i64>,
    /// The cards moved to the next day by the sibling rule, in the day's order.
    pub moved: Vec<i64>,
    /// How many queued cards carry no note id.
    pub unlinked: u32,
}

impl DayQueue {
    /// The day a moved card moves to: the next day.
    #[must_use]
    pub fn moved_to(&self) -> i32 {
        self.day.saturating_add(1)
    }
}

/// The queue of study day `day` (SPEC-494 R6, R7): every card due on or before it, ordered by due
/// day and then card id; the first card of a note in that order stays and each later one moves to
/// the next day, and a card with no note id enters the queue and is counted.
#[must_use]
pub fn day_queue(cards: &[QueueCard], day: i32) -> DayQueue {
    let mut candidates: Vec<&QueueCard> = cards.iter().filter(|card| card.due <= day).collect();
    candidates.sort_by_key(|card| (card.due, card.card));
    let mut queue = DayQueue {
        day,
        cards: Vec::new(),
        moved: Vec::new(),
        unlinked: 0,
    };
    let mut seen = BTreeSet::new();
    for card in candidates {
        if card.note <= NO_NOTE {
            queue.cards.push(card.card);
            queue.unlinked = queue.unlinked.saturating_add(1);
        } else if seen.insert(card.note) {
            queue.cards.push(card.card);
        } else {
            queue.moved.push(card.card);
        }
    }
    queue
}
