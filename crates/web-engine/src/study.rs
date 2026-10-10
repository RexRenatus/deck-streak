//! The study rule, the same on every target: a wire rating to the grade a press records and to the
//! next state that grade selects (SPEC-338 R1, SPEC-365 R7), and the table of study calls
//! `run_method` admits (ADR-348).
//!
//! It holds no engine type, so the native tests judge exactly the rule the `wasm32` module runs.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

/// The two grades a press records (SPEC-365 R7): the wire's 1 and 3, as Anki's buttons number
/// Again and Good. The core records the same two, and no type here names a third.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// The card was forgotten.
    Again,
    /// The card was recalled.
    Good,
}

impl Grade {
    /// Anki's `Rating` value for this grade: 0 for Again, 2 for Good.
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

    /// The undo offer's word for this grade (SPEC-371 R7), which the page's dialog names.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Again => "again",
            Self::Good => "good",
        }
    }
}

/// The grade a wire rating names: 1 is Again and 3 is Good (SPEC-365 R7).
///
/// # Errors
/// [`StudyError::NotAGrade`] for 2 and 4, Hard and Easy, which no press records, and
/// [`StudyError::RatingOutOfRange`] for any number outside 1 to 4, before anything reaches the
/// engine.
pub fn grade(rating: u32) -> Result<Grade, StudyError> {
    match rating {
        1 => Ok(Grade::Again),
        3 => Ok(Grade::Good),
        2 | 4 => Err(StudyError::NotAGrade(rating)),
        other => Err(StudyError::RatingOutOfRange(other)),
    }
}

/// The backend's services the study calls use, by index of its generated dispatcher at the
/// pinned commit (read from the `wasm32` build's `backend.rs`).
pub mod service {
    /// The sync service.
    pub const SYNC: u32 = 1;
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

/// The sync calls the web client reaches through its own exports, never through `run_method`:
/// service, method, and the method's name (SPEC-364 R1; ADR-375 D11). The parity guard reads them
/// beside [`STUDY_CALLS`] as the web column of the core's table.
pub const SYNC_CALLS: [(u32, u32, &str); 2] = [
    (service::SYNC, 3, "sync_login"),
    (service::SYNC, 5, "sync_collection"),
];

/// The study calls `run_method` admits: service, method, and the method's name. Every pair
/// outside it is refused, the exempt writes of ADR-337 included (#623), Undo among them, which only
/// the owner's gesture runs (SPEC-371 R7), and so is the answer, which only an owner's press
/// records (SPEC-365 R7). The last nine are the review's: the deck list, the card view, its one
/// line of text, its labels and undo label, bury and flag (SPEC-350 R1, SPEC-371 R8).
pub const STUDY_CALLS: [(u32, u32, &str); 15] = [
    (service::COLLECTION, 0, "open_collection"),
    (service::COLLECTION, 1, "close_collection"),
    (service::SCHEDULER, 3, "get_queued_cards"),
    (service::NOTETYPES, 8, "get_notetype_names"),
    (service::NOTES, 0, "new_note"),
    (service::NOTES, 2, "add_notes"),
    (service::DECKS, 4, "deck_tree"),
    (service::DECKS, 22, "set_current_deck"),
    (service::CARD_RENDERING, 6, "render_existing_card"),
    (service::CARD_RENDERING, 9, "strip_av_tags"),
    (service::CARD_RENDERING, 14, "html_to_text_line"),
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

/// The kind of state an undone answer returns its card to (SPEC-371 R3, R7): the state the card
/// was in when it was answered. The core's rule decides it; this mirror keeps the study rule free
/// of engine types, and the `wasm32` module converts one to the other.
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

impl Returns {
    /// The undo offer's word for this kind (SPEC-371 R7), which the page's dialog names.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Learning => "learning",
            Self::Review => "review",
            Self::Relearning => "relearning",
            Self::Preview => "preview",
        }
    }
}

/// Why an undo of the review's own last answer is refused (SPEC-371 R3, R7): the core's rule
/// decides it, and this mirror carries its verdict to the page without an engine type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoRefusal {
    /// The answer is gone: its review row is absent, or the engine has nothing to undo.
    Gone,
    /// The recorded review is of another card.
    NotTheCard,
    /// The recorded review has synced.
    Synced,
    /// Something changed after the answer, or the confirmation names another answer than the
    /// record's.
    Changed,
}

impl UndoRefusal {
    /// The offer's reason for making no offer (SPEC-371 R7): `synced` for a synced answer, so the
    /// page can say so, and `none` for every other refusal.
    #[must_use]
    pub fn why(self) -> &'static str {
        match self {
            Self::Synced => "synced",
            Self::Gone | Self::NotTheCard | Self::Changed => "none",
        }
    }
}

/// What the card view says of an undo (SPEC-371 R7), from the core's verdict on the review's own
/// last answer: `answer` when it may be undone, `synced` when it has synced, and nothing when there
/// is no such answer or any other refusal holds.
#[must_use]
pub fn undo_view(judged: Option<Result<(), UndoRefusal>>) -> Option<&'static str> {
    match judged? {
        Ok(()) => Some("answer"),
        Err(UndoRefusal::Synced) => Some("synced"),
        Err(UndoRefusal::Gone | UndoRefusal::NotTheCard | UndoRefusal::Changed) => None,
    }
}

/// The engine's state the review's own last answer left, as the review records it (SPEC-371 R6):
/// the undo status's last step and undo label right after the answer, and the review-log row the
/// answer wrote, by id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// The engine's last step right after the answer.
    pub step: u32,
    /// The engine's undo label right after the answer.
    pub label: String,
    /// The review-log row the answer wrote, by id.
    pub review: i64,
}

/// The review's own last answer (SPEC-371 R6; ADR-382 D4): the card it answered, the grade it
/// recorded, the kind of state it left, and the record the core judges an undo of it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastAnswer {
    /// The card the answer was of.
    pub card: i64,
    /// The grade the press recorded.
    pub grade: Grade,
    /// The kind of state the card was in when it was answered, which an undo returns it to.
    pub returns: Returns,
    /// The engine's state the answer left.
    pub recorded: Recorded,
}

/// The kept answer, when a confirmation names its card and the step its offer showed (SPEC-371
/// R7): the page confirms only what it was offered.
///
/// # Errors
/// [`StudyError::NotUndoable`] with [`UndoRefusal::Changed`] when no answer is kept, or when
/// `card` or `step` is not the kept answer's.
pub fn last_answer_for(
    last: Option<&LastAnswer>,
    card: i64,
    step: u32,
) -> Result<&LastAnswer, StudyError> {
    last.filter(|kept| kept.card == card && kept.recorded.step == step)
        .ok_or(StudyError::NotUndoable(UndoRefusal::Changed))
}

/// What the review's last bury or flag did (SPEC-383 R2): a bury, with the kind of state its card
/// returns to, or a flag, with the card's flag before the change and the flag the change left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// A bury of the shown card, which an undo returns to the state it was buried from.
    Bury(Returns),
    /// A flag set on the shown card.
    Flag {
        /// The card's flag before the change, which an undo puts back.
        before: u32,
        /// The flag the change left.
        after: u32,
    },
}

/// The review's last bury or flag (SPEC-383 R1, R2): the card it changed, the engine's last step
/// right after it, what it did, and the record the core judges an undo of it by. The record is
/// the core's own type, which only the `wasm32` module names, so the study rule holds it unread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastMark<R> {
    /// The card the change was made on.
    pub card: i64,
    /// The engine's last step right after the change.
    pub step: u32,
    /// What the change did.
    pub change: Change,
    /// The record the core judges an undo of the change by.
    pub record: R,
}

/// The kept change, when a confirmation names its card and the step its offer showed (SPEC-383
/// R9): the page confirms only what it was offered.
///
/// # Errors
/// [`StudyError::NotUndoable`] with [`UndoRefusal::Changed`] when no change is kept, or when
/// `card` or `step` is not the kept change's.
pub fn last_mark_for<R>(
    last: Option<&LastMark<R>>,
    card: i64,
    step: u32,
) -> Result<&LastMark<R>, StudyError> {
    let _ = (last, card, step);
    Err(StudyError::NotUndoable(UndoRefusal::Changed))
}

/// What the card view says of an undo of the review's last bury or flag (SPEC-383 R9), from the
/// core's verdict on it: `bury` or `flag` when it may be undone, `change-synced` when it has
/// synced, and nothing when no change is kept or any other refusal holds.
#[must_use]
pub fn mark_view(judged: Option<(Change, Result<(), UndoRefusal>)>) -> Option<&'static str> {
    let _ = judged;
    None
}

/// What an undo of a flag does to its card's flag (SPEC-383 R9, R11), from the flag before the
/// change and the flag it left: `added` when it set red on a card with none, `removed` when it
/// took red off, and `replaced` when it set red over another flag.
#[must_use]
pub fn flag_change(before: u32, after: u32) -> &'static str {
    let _ = (before, after);
    "added"
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
    /// A wire rating of 2 or 4, Hard or Easy, which no press records (SPEC-365 R7).
    NotAGrade(u32),
    /// A service and method outside the study calls.
    CallRefused {
        /// The service index asked for.
        service: u32,
        /// The method index asked for.
        method: u32,
    },
    /// A rating, bury or flag for a card other than the one shown, or with none shown.
    NotShown,
    /// An undo of the review's own last answer that is not admitted (SPEC-371 R7).
    NotUndoable(UndoRefusal),
}

impl fmt::Display for StudyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RatingOutOfRange(rating) => {
                write!(f, "rating {rating} is outside 1 to 4")
            }
            Self::NotAGrade(rating) => {
                write!(f, "rating {rating} is not a grade: a press records 1 or 3")
            }
            Self::CallRefused { service, method } => {
                write!(
                    f,
                    "run_method refuses service {service} method {method}: not a study call"
                )
            }
            Self::NotShown => write!(f, "not-shown: the card is not the one on screen"),
            Self::NotUndoable(UndoRefusal::Synced) => {
                write!(f, "undo-synced: the answer has synced")
            }
            Self::NotUndoable(UndoRefusal::Gone) => write!(f, "not-undoable: the answer is gone"),
            Self::NotUndoable(UndoRefusal::NotTheCard) => {
                write!(f, "not-undoable: the answer is of another card")
            }
            Self::NotUndoable(UndoRefusal::Changed) => {
                write!(f, "not-undoable: something changed after the answer")
            }
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
