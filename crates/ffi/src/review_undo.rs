//! The review's undo as the app shows it (SPEC-515 R3 to R5; ADR-533 D3): the offer of the
//! review's own last answer, bury or flag, with the words its dialog shows, and the answer to the
//! owner's confirmation. The app lays the words out as text and decides nothing about them. The
//! words are the web's English ones (SPEC-371 R10, SPEC-383 R10 to R13).

use deck_streak_engine_core::answer::Grade;
use deck_streak_engine_core::native_undo::{FlagChange, Kept, Step};
use deck_streak_engine_core::undo_answer::{Returns, UndoRefusal};

/// The dialog's titles.
const ANSWER_TITLE: &str = "Undo this answer?";
const BURY_TITLE: &str = "Undo this bury?";
const FLAG_TITLE: &str = "Undo this flag?";
/// The dialog's two buttons: Keep, which has the focus, and each kind's confirmation.
const KEEP: &str = "Keep it";
const UNDO_ANSWER: &str = "Undo answer";
const UNDO_BURY: &str = "Undo bury";
const UNDO_FLAG: &str = "Undo flag";
/// The labels the dialog's lines open with, and the line for a card with no text.
const CARD: &str = "Card:";
const YOUR_ANSWER: &str = "Your answer:";
const GOES_BACK_TO: &str = "Goes back to:";
const NO_TEXT: &str = "This card has no text to show.";
/// What each undo changes.
const ANSWER_UNSYNCED: &str = "This answer has not synced yet. Undoing it removes it from your \
                               review history; nothing else changes.";
const BURY_UNSYNCED: &str = "This bury has not synced yet. Undoing it puts the card back in \
                             today's queue; nothing else changes.";
const FLAG_ADDED: &str = "You added a red flag. Undoing removes it; nothing else changes.";
const FLAG_REMOVED: &str = "You removed the red flag. Undoing puts it back; nothing else changes.";
const FLAG_REPLACED: &str = "You replaced this card's flag with a red flag. Undoing puts its \
                             earlier flag back; nothing else changes.";
/// Why an answer or a change can no longer be undone.
const ANSWER_SYNCED: &str = "Your last answer has synced, so it can no longer be undone.";
const CHANGE_SYNCED: &str = "Your last change has synced, so it can no longer be undone.";
const ANSWER_GONE: &str = "This answer can no longer be undone: something changed after it.";
const CHANGE_GONE: &str = "This change can no longer be undone: something changed after it.";

/// What the review's last recorded step was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum UndoKind {
    /// The review's own last answer.
    Answer,
    /// The review's last bury of the shown card.
    Bury,
    /// The review's last flag of the shown card.
    Flag,
}

/// The undo the review offers: the card and the step its confirmation names, what the step was,
/// and the dialog's words, each a line the app shows as it is.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UndoOffer {
    /// The card the step was on.
    pub card: i64,
    /// The engine's last step right after the review's step, which the confirmation names.
    pub step: u32,
    /// What the step was.
    pub kind: UndoKind,
    /// The dialog's title.
    pub title: String,
    /// The dialog's lines, in order: the card as one line of text, then the answer and the state
    /// the card goes back to for an answer, the state for a bury, and what the undo changes.
    pub lines: Vec<String>,
    /// The title of the button that keeps the step, which has the focus.
    pub keep: String,
    /// The title of the button that confirms the undo.
    pub confirm: String,
}

/// The adapter's answer to the review's undo press: the offer, or none and the sentence that says
/// why. `said` is empty when an offer is made.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UndoOffered {
    /// The offer, when the review's last step may be undone now.
    pub offer: Option<UndoOffer>,
    /// Why no offer was made, as the app shows it.
    pub said: String,
}

/// The adapter's answer to the owner's confirmation: whether the step was undone, and the sentence
/// that says why not. `said` is empty when it was undone.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct UndoDone {
    /// Whether the undo ran.
    pub undone: bool,
    /// Why it did not, as the app shows it.
    pub said: String,
}

impl UndoOffered {
    /// No offer, and why.
    pub(crate) fn none(said: String) -> Self {
        Self { offer: None, said }
    }
}

impl UndoDone {
    /// The undo ran.
    pub(crate) fn undone() -> Self {
        Self {
            undone: true,
            said: String::new(),
        }
    }

    /// The undo did not run, and why.
    pub(crate) fn refused(said: String) -> Self {
        Self {
            undone: false,
            said,
        }
    }
}

/// What `kept` was.
pub(crate) const fn kind_of(kept: &Kept) -> UndoKind {
    match kept.did() {
        Step::Answer { .. } => UndoKind::Answer,
        Step::Bury { .. } => UndoKind::Bury,
        Step::Flag { .. } => UndoKind::Flag,
    }
}

/// The offer of `kept`, whose card's question reads `line` as one line of text.
pub(crate) fn offer(kept: &Kept, line: &str) -> UndoOffer {
    let text = if line.trim().is_empty() {
        NO_TEXT
    } else {
        line
    };
    let card = format!("{CARD} {text}");
    let (title, lines, confirm) = match kept.did() {
        Step::Answer { grade, returns } => (
            ANSWER_TITLE,
            vec![
                card,
                format!("{YOUR_ANSWER} {}", grade_word(grade)),
                format!("{GOES_BACK_TO} {}", returns_word(returns)),
                ANSWER_UNSYNCED.to_owned(),
            ],
            UNDO_ANSWER,
        ),
        Step::Bury { returns } => (
            BURY_TITLE,
            vec![
                card,
                format!("{GOES_BACK_TO} {}", returns_word(returns)),
                BURY_UNSYNCED.to_owned(),
            ],
            UNDO_BURY,
        ),
        Step::Flag { change } => (
            FLAG_TITLE,
            vec![card, flag_line(change).to_owned()],
            UNDO_FLAG,
        ),
    };
    UndoOffer {
        card: kept.card(),
        step: kept.step(),
        kind: kind_of(kept),
        title: title.to_owned(),
        lines,
        keep: KEEP.to_owned(),
        confirm: confirm.to_owned(),
    }
}

/// Why an undo of a step of `kind` was refused for `why`: it has synced, or something changed
/// after it, each for an answer or for a change.
pub(crate) fn refused(kind: UndoKind, why: UndoRefusal) -> String {
    match (why, kind) {
        (UndoRefusal::Synced, UndoKind::Answer) => ANSWER_SYNCED,
        (UndoRefusal::Synced, UndoKind::Bury | UndoKind::Flag) => CHANGE_SYNCED,
        (UndoRefusal::Gone | UndoRefusal::NotTheCard | UndoRefusal::Changed, UndoKind::Answer) => {
            ANSWER_GONE
        }
        (
            UndoRefusal::Gone | UndoRefusal::NotTheCard | UndoRefusal::Changed,
            UndoKind::Bury | UndoKind::Flag,
        ) => CHANGE_GONE,
    }
    .to_owned()
}

/// Why a step of `kind` can no longer be undone when the review no longer keeps it.
pub(crate) fn gone(kind: UndoKind) -> String {
    refused(kind, UndoRefusal::Changed)
}

/// The dialog's word for the grade an answer recorded.
const fn grade_word(grade: Grade) -> &'static str {
    match grade {
        Grade::Again => "Again",
        Grade::Good => "Good",
    }
}

/// The dialog's words for the state an undone step's card goes back to.
const fn returns_word(returns: Returns) -> &'static str {
    match returns {
        Returns::New => "New, in today's queue",
        Returns::Learning => "Learning, in today's queue",
        Returns::Review => "Review, in today's queue",
        Returns::Relearning => "Relearning, in today's queue",
        Returns::Preview => "Preview, in today's queue",
    }
}

/// The dialog's line for what a flag's undo does to the card's flag.
const fn flag_line(change: FlagChange) -> &'static str {
    match change {
        FlagChange::Added => FLAG_ADDED,
        FlagChange::Removed => FLAG_REMOVED,
        FlagChange::Replaced => FLAG_REPLACED,
    }
}
