//! The review's undo as the app shows it (SPEC-515 R3 to R5; ADR-533 D3): the offer of the
//! review's own last answer, bury or flag, with the words its dialog shows, and the answer to the
//! owner's confirmation. The app lays the words out as text and decides nothing about them.

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
