//! The owner's answer: one press, its one card and its one grade (SPEC-365 R1 to R3; ADR-376).
//!
//! A grade is recorded only through [`crate::dispatch::Dispatcher::run_answer`], which consumes an
//! [`OwnerAnswer`]. The answer's fields are private, it is neither `Clone`, `Copy` nor `Default`,
//! and [`OwnerAnswer::from_press`] is its one constructor, so one press records at most one grade
//! for one card. Which crates may name the constructor is the crate graph's to decide, and the
//! containment census holds it to the two UI adapters' entry files, as it holds the gesture's.
//!
//! The native adapter names no protobuf codec of its own (ADR-376 D15), so the core also carries
//! the two codec steps of a native press: [`shown_states`] reads the states a card was shown with,
//! and [`answer_request`] writes the engine's `CardAnswer` from the next state the adapter picked.

use std::fmt;

use anki_proto::scheduler::{CardAnswer, SchedulingState, SchedulingStates};
use prost::Message;

/// The two grades a press records. The engine numbers its ratings 0 to 3; a press names Again or
/// Good, and no type here names a third.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// The card was forgotten.
    Again,
    /// The card was recalled.
    Good,
}

impl Grade {
    /// The engine's rating number for this grade: 0 for Again, 2 for Good.
    #[must_use]
    pub fn rating(self) -> i32 {
        match self {
            Self::Again => 0,
            Self::Good => 2,
        }
    }

    /// The next state this grade selects, out of the states the card was shown with.
    #[must_use]
    pub fn pick<T>(self, again: T, good: T) -> T {
        match self {
            Self::Again => again,
            Self::Good => good,
        }
    }
}

/// Why an answer was not recorded: refused before the engine saw it, or by the engine itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerRefusal {
    /// The request is not the engine's `CardAnswer`.
    Undecodable,
    /// The request names another card than the one pressed.
    NotTheCard {
        /// The card the press named.
        pressed: i64,
        /// The card the request names.
        named: i64,
    },
    /// The request's rating is not the pressed grade's.
    NotTheGrade {
        /// The grade the press named.
        pressed: Grade,
        /// The rating the request names, as the engine numbers it.
        named: i32,
    },
    /// The engine refused the checked answer.
    Engine {
        /// The engine's `BackendError`, as the engine encoded it.
        error: Vec<u8>,
    },
}

impl fmt::Display for AnswerRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Undecodable => f.write_str("the answer is not the engine's card answer"),
            Self::NotTheCard { pressed, named } => {
                write!(
                    f,
                    "the press named card {pressed}; the answer names card {named}"
                )
            }
            Self::NotTheGrade { pressed, named } => {
                write!(
                    f,
                    "the press named {pressed:?}; the answer names rating {named}"
                )
            }
            Self::Engine { error } => {
                write!(f, "the engine refused the answer ({} bytes)", error.len())
            }
        }
    }
}

impl std::error::Error for AnswerRefusal {}

/// One owner's press on one card with one grade. Only the UI adapters build one, from the press
/// itself, and recording its grade consumes it.
#[derive(Debug)]
pub struct OwnerAnswer {
    card: i64,
    grade: Grade,
}

impl OwnerAnswer {
    /// The answer of a press of `grade` on the card `card`.
    #[must_use]
    pub fn from_press(card: i64, grade: Grade) -> Self {
        Self { card, grade }
    }

    /// The request the engine runs for this answer: the caller's `CardAnswer`, decoded, checked
    /// against the press and encoded again, so the engine never runs the caller's own bytes.
    pub(crate) fn checked(self, input: &[u8]) -> Result<Vec<u8>, AnswerRefusal> {
        let Self { card, grade } = self;
        let request = CardAnswer::decode(input).map_err(|_| AnswerRefusal::Undecodable)?;
        if request.card_id != card {
            return Err(AnswerRefusal::NotTheCard {
                pressed: card,
                named: request.card_id,
            });
        }
        if request.rating != grade.rating() {
            return Err(AnswerRefusal::NotTheGrade {
                pressed: grade,
                named: request.rating,
            });
        }
        Ok(request.encode_to_vec())
    }
}

/// The states the queue gave a card when it was shown, from the encoded `SchedulingStates` a native
/// caller passes back with its press (SPEC-365 R6).
///
/// # Errors
///
/// [`AnswerRefusal::Undecodable`] when the bytes are not the engine's `SchedulingStates`.
pub fn shown_states(states: &[u8]) -> Result<SchedulingStates, AnswerRefusal> {
    SchedulingStates::decode(states).map_err(|_| AnswerRefusal::Undecodable)
}

/// The engine's `CardAnswer` for a press of `grade` on `card`, encoded: the state the card was
/// shown in, the next state the adapter picked for the grade, the grade's own rating, the time the
/// adapter's clock read and how long the learner took (SPEC-365 R6).
#[must_use]
pub fn answer_request(
    card: i64,
    current: Option<SchedulingState>,
    next: Option<SchedulingState>,
    grade: Grade,
    answered_at_millis: i64,
    milliseconds_taken: u32,
) -> Vec<u8> {
    CardAnswer {
        card_id: card,
        current_state: current,
        new_state: next,
        rating: grade.rating(),
        answered_at_millis,
        milliseconds_taken,
    }
    .encode_to_vec()
}
