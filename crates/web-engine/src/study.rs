//! The study rule, the same on every target: a wire rating to Anki's answer and to the next state
//! that answer selects, and the table of study calls `run_method` admits (SPEC-338 R1, ADR-348).
//!
//! It holds no engine type, so the native tests judge exactly the rule the `wasm32` module runs.

use std::fmt;

/// Anki's four answers, in the order of its `Rating` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Rating 0: the card is forgotten.
    Again,
    /// Rating 1: recalled with difficulty.
    Hard,
    /// Rating 2: recalled.
    Good,
    /// Rating 3: recalled easily.
    Easy,
}

impl Answer {
    /// The answer a wire rating names. The page sends 1 to 4, as Anki's buttons number them.
    ///
    /// # Errors
    /// [`StudyError::RatingOutOfRange`] for any other number, before anything reaches the engine.
    pub fn from_wire(rating: u32) -> Result<Self, StudyError> {
        match rating {
            1 => Ok(Self::Again),
            2 => Ok(Self::Hard),
            3 => Ok(Self::Good),
            4 => Ok(Self::Easy),
            other => Err(StudyError::RatingOutOfRange(other)),
        }
    }

    /// Anki's `Rating` value for this answer, 0 to 3.
    #[must_use]
    pub fn rating(self) -> i32 {
        match self {
            Self::Again => 0,
            Self::Hard => 1,
            Self::Good => 2,
            Self::Easy => 3,
        }
    }

    /// The next state this answer selects out of the scheduler's four, in their field order.
    #[must_use]
    pub fn pick<T>(self, again: T, hard: T, good: T, easy: T) -> T {
        match self {
            Self::Again => again,
            Self::Hard => hard,
            Self::Good => good,
            Self::Easy => easy,
        }
    }
}

/// The backend's services the study calls use, by index of its generated dispatcher at the
/// pinned commit (read from the `wasm32` build's `backend.rs`).
pub mod service {
    /// The collection service.
    pub const COLLECTION: u32 = 3;
    /// The cards service.
    pub const CARDS: u32 = 5;
    /// The decks service.
    pub const DECKS: u32 = 7;
    /// The scheduler service.
    pub const SCHEDULER: u32 = 13;
    /// The notetypes service.
    pub const NOTETYPES: u32 = 23;
    /// The notes service.
    pub const NOTES: u32 = 25;
    /// The card rendering service.
    pub const CARD_RENDERING: u32 = 27;
}

/// The study calls `run_method` admits: service, method, and the method's name. Every pair
/// outside it is refused, the exempt writes of ADR-337 included (#623). The last eight are the
/// review's: the deck list, the card view, its labels and undo label, bury and flag (SPEC-350 R1).
pub const STUDY_CALLS: [(u32, u32, &str); 16] = [
    (service::COLLECTION, 0, "open_collection"),
    (service::COLLECTION, 1, "close_collection"),
    (service::COLLECTION, 8, "undo"),
    (service::SCHEDULER, 3, "get_queued_cards"),
    (service::SCHEDULER, 4, "answer_card"),
    (service::NOTETYPES, 8, "get_notetype_names"),
    (service::NOTES, 0, "new_note"),
    (service::NOTES, 2, "add_notes"),
    (service::DECKS, 4, "deck_tree"),
    (service::DECKS, 22, "set_current_deck"),
    (service::CARD_RENDERING, 6, "render_existing_card"),
    (service::CARD_RENDERING, 9, "strip_av_tags"),
    (service::SCHEDULER, 24, "describe_next_states"),
    (service::COLLECTION, 7, "get_undo_status"),
    (service::SCHEDULER, 14, "bury_or_suspend_cards"),
    (service::CARDS, 4, "set_flag"),
];

/// The name of the study call at `service` and `method`.
///
/// # Errors
/// [`StudyError::CallRefused`] for a pair outside [`STUDY_CALLS`].
pub fn admit(service: u32, method: u32) -> Result<&'static str, StudyError> {
    STUDY_CALLS
        .iter()
        .find(|&&(s, m, _)| (s, m) == (service, method))
        .map(|&(_, _, name)| name)
        .ok_or(StudyError::CallRefused { service, method })
}

/// Why the study rule refused a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudyError {
    /// A wire rating outside 1 to 4.
    RatingOutOfRange(u32),
    /// A service and method outside the study calls.
    CallRefused {
        /// The service index asked for.
        service: u32,
        /// The method index asked for.
        method: u32,
    },
}

impl fmt::Display for StudyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RatingOutOfRange(rating) => {
                write!(f, "rating {rating} is outside 1 to 4")
            }
            Self::CallRefused { service, method } => {
                write!(
                    f,
                    "run_method refuses service {service} method {method}: not a study call"
                )
            }
        }
    }
}

impl std::error::Error for StudyError {}
