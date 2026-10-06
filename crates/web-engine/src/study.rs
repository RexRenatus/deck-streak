//! The study rule, the same on every target: a wire rating to Anki's answer and to the next state
//! that answer selects, and the table of study calls `run_method` admits (SPEC-338 R1, ADR-348).
//!
//! It holds no engine type, so the native tests judge exactly the rule the `wasm32` module runs.

use std::cell::RefCell;
use std::collections::HashMap;
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

/// The card the review last showed: its id, the scheduling states read to show it, and its flag
/// (SPEC-350 R2, ADR-361 D2). A rating, a bury or a flag reaches this card and no other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown<S> {
    /// The card's id.
    pub card: i64,
    /// The scheduling states the engine gave when it showed the card.
    pub states: S,
    /// The card's flag when it was shown, as the engine numbers flags.
    pub flag: u32,
}

/// The kept card, when `card` is its id.
///
/// # Errors
/// [`StudyError::NotShown`] when no card is kept or `card` is another card's id.
pub fn shown_for<S>(shown: Option<&Shown<S>>, card: i64) -> Result<&Shown<S>, StudyError> {
    shown
        .filter(|kept| kept.card == card)
        .ok_or(StudyError::NotShown)
}

/// The engine's number for the red flag.
pub const RED: u32 = 1;

/// The flag the flag action sets: red to none, and any other flag, none included, to red, as the
/// desktop's red flag key does.
#[must_use]
pub fn toggled_red(flag: u32) -> u32 {
    if flag == RED { 0 } else { RED }
}

/// The engine's bury mode for the user's own bury, which the next day does not undo alone.
pub const BURY_USER: i32 = 2;

/// A bury request's fields, in the engine's order: the cards, the notes and the mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuryOf {
    /// The card ids to bury.
    pub card_ids: Vec<i64>,
    /// The note ids whose cards to bury.
    pub note_ids: Vec<i64>,
    /// The bury mode.
    pub mode: i32,
}

/// The user's bury of one card: that card's id, no note, the user's mode.
#[must_use]
pub fn bury_of(card: i64) -> BuryOf {
    BuryOf {
        card_ids: vec![card],
        note_ids: Vec::new(),
        mode: BURY_USER,
    }
}

/// The languages the engine's `init` receives: the page's list, or English when it sends none
/// (SPEC-350 R4).
#[must_use]
pub fn engine_languages(languages: Vec<String>) -> Vec<String> {
    if languages.is_empty() {
        vec!["en".to_owned()]
    } else {
        languages
    }
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
    /// A rating, bury or flag for a card other than the one shown, or with none shown.
    NotShown,
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
            Self::NotShown => write!(f, "not-shown: the card is not the one on screen"),
        }
    }
}

impl std::error::Error for StudyError {}

/// The media files the Worker read for a face, each under the name the engine stores (SPEC-350
/// R14). The core decides every cap and type; this only hands it the bytes it asked for.
#[derive(Debug, Default)]
pub struct Files {
    by_name: HashMap<String, Vec<u8>>,
}

impl Files {
    /// The files read, by name.
    pub fn new(files: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
        Self {
            by_name: files.into_iter().collect(),
        }
    }

    /// The first `limit` bytes of `name` at most, or `None` when no file of that name was read.
    #[must_use]
    pub fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        let bytes = self.by_name.get(name)?;
        let end = usize::try_from(limit).map_or(bytes.len(), |limit| limit.min(bytes.len()));
        Some(bytes[..end].to_vec())
    }
}

/// The core's reader over [`Files`] for one face call: it answers from the files, and records
/// each name the core asked for that the files lack, with the limit asked, so the Worker can read
/// those names and ask again (SPEC-350 R14, ADR-361 D12).
#[derive(Debug)]
pub struct Wanted<'a> {
    files: &'a Files,
    asked: RefCell<Vec<(String, u64)>>,
}

impl<'a> Wanted<'a> {
    /// A reader over `files` that has recorded nothing yet.
    #[must_use]
    pub fn new(files: &'a Files) -> Self {
        Self {
            files,
            asked: RefCell::new(Vec::new()),
        }
    }

    /// The core's ask for `name`: the file's first `limit` bytes, or `None` and a record of it.
    #[must_use]
    pub fn ask(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        let read = self.files.read(name, limit);
        let mut asked = self.asked.borrow_mut();
        if read.is_none() && !asked.iter().any(|(wanted, _)| wanted == name) {
            asked.push((name.to_owned(), limit));
        }
        read
    }

    /// Each name the core asked for and the files lacked, with its limit, in the order asked.
    #[must_use]
    pub fn into_names(self) -> Vec<(String, u64)> {
        self.asked.into_inner()
    }
}

/// The media type `types` gives `name`'s extension, compared without case. `types` is the core's
/// one table, passed in, so no copy of it lives here (SPEC-350 R15, ADR-361 D12).
#[must_use]
pub fn media_type<'a>(name: &str, types: &[(&str, &'a str)]) -> Option<&'a str> {
    let (_, extension) = name.rsplit_once('.')?;
    types
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(extension))
        .map(|&(_, media_type)| media_type)
}
