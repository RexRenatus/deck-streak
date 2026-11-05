//! # deck-streak-render
//!
//! What this crate owns: how a card is drawn from its note type, its templates and its fields
//! (SPEC-603, ADR-624). A template is tokenized and parsed ([`template`]), and [`render`] fills
//! its field replacements, sections, inverted sections, special fields, `{{FrontSide}}` and
//! filters into the card's two faces. [`check_note_type`] refuses a note type whose templates
//! cannot render. Every problem is a [`RenderIssue`] class whose text is `DeckStreak`'s own.
//!
//! What it does not own: any I/O. No source here reads a file, the clock, the environment or the
//! network, and the crate has no dependency, so the server and both clients link one rule. The
//! cloze filters, sound and speech tags, media references and math are later deliveries.

#![forbid(unsafe_code)]

mod filters;
mod render;
pub mod template;

/// The largest template, in bytes, the renderer reads; a longer one is [`RenderIssue::TooLarge`].
pub const MAX_TEMPLATE_BYTES: usize = 1 << 20;

/// The deepest nesting of sections a template may hold; a deeper one is [`RenderIssue::TooDeep`].
pub const MAX_DEPTH: usize = 32;

/// Which kind of note type: one card per template, or one card per cloze number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteTypeKind {
    /// Each template makes one card.
    Standard,
    /// The one template makes a card per cloze number (its filters are a later delivery).
    Cloze,
}

/// One card template: its name and its two sides' source, as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardTemplate {
    /// The template's name, which the `Card` special field shows.
    pub name: String,
    /// The question side's template source.
    pub front: String,
    /// The answer side's template source.
    pub back: String,
}

/// A note type: its name, its kind, its field names in order and its card templates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteType {
    /// The note type's name, which the `Type` special field shows.
    pub name: String,
    /// Standard or cloze.
    pub kind: NoteTypeKind,
    /// The field names; a field's index is its ordinal.
    pub fields: Vec<String>,
    /// The card templates; a template's index is its card ordinal.
    pub templates: Vec<CardTemplate>,
}

/// A note's content: its field values in the note type's field order, and its tags.
#[derive(Debug, Clone, Copy)]
pub struct NoteContent<'a> {
    /// The field values; a missing value reads as empty.
    pub fields: &'a [String],
    /// The note's tags, which the `Tags` special field shows joined by spaces.
    pub tags: &'a [String],
}

/// The card being drawn: its ordinal and what its special fields show.
#[derive(Debug, Clone, Copy)]
pub struct CardContext<'a> {
    /// The card's template ordinal.
    pub ordinal: u16,
    /// The card's deck, `::` between levels; `Deck` shows it whole and `Subdeck` its last level.
    pub deck_path: &'a str,
    /// The card's flag number, which `CardFlag` shows as `flag<n>`.
    pub flag: u8,
    /// The card's id, which `CardID` shows.
    pub card_id: i64,
}

/// A rendered card: the question's and the answer's HTML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The question side.
    pub question: String,
    /// The answer side, where `{{FrontSide}}` is the question.
    pub answer: String,
}

/// Why a template or a note type cannot render: a class, whose text is `DeckStreak`'s own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderIssue {
    /// A replacement or a section names a field the note type does not have.
    UnknownField {
        /// The name as the template wrote it.
        name: String,
    },
    /// A section is opened and never closed.
    UnclosedSection {
        /// The section's name.
        name: String,
    },
    /// A section is closed that is not the one open.
    StrayClose {
        /// The closing tag's name.
        name: String,
    },
    /// The card's front shows none of the note's filled fields.
    EmptyFront,
    /// Sections nest deeper than [`MAX_DEPTH`].
    TooDeep,
    /// A template is longer than [`MAX_TEMPLATE_BYTES`].
    TooLarge,
    /// The card's ordinal names no template of a standard note type.
    NoTemplate {
        /// The card's ordinal.
        ordinal: u16,
    },
    /// A template's front names no field at all.
    FrontNamesNoField {
        /// The template's ordinal.
        ordinal: usize,
    },
    /// A template's front repeats an earlier template's front.
    DuplicateFront {
        /// The repeating template's ordinal.
        ordinal: usize,
    },
    /// A cloze note type's template lacks a cloze filter on its front or its back.
    NoClozeFilter,
}

impl RenderIssue {
    /// `DeckStreak`'s English text for this issue.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::UnknownField { name } => format!(
                "The template asks for a field named \"{name}\", and this note type has no such \
                 field."
            ),
            Self::UnclosedSection { name } => {
                format!("The section \"{name}\" is opened in the template but never closed.")
            }
            Self::StrayClose { name } => {
                format!("The template closes a section \"{name}\" that is not open.")
            }
            Self::EmptyFront => {
                "Nothing to ask: this card's front uses no field that holds text.".to_owned()
            }
            Self::TooDeep => format!("The template nests sections more than {MAX_DEPTH} deep."),
            Self::TooLarge => format!("The template is longer than {MAX_TEMPLATE_BYTES} bytes."),
            Self::NoTemplate { ordinal } => format!(
                "This note type has no card template number {}.",
                u32::from(*ordinal) + 1
            ),
            Self::FrontNamesNoField { ordinal } => {
                format!("Card template {} shows no field on its front.", ordinal + 1)
            }
            Self::DuplicateFront { ordinal } => format!(
                "Card template {} has the same front as an earlier template.",
                ordinal + 1
            ),
            Self::NoClozeFilter => "A cloze note type needs a cloze field on both the front and \
                                    the back of its template."
                .to_owned(),
        }
    }

    /// The face shown in place of a card that cannot render: the message, escaped, in a `div`.
    #[must_use]
    pub fn face(&self) -> String {
        format!(
            "<div class=\"ds-render-issue\">{}</div>",
            filters::escape(&self.message())
        )
    }
}

/// Renders the card `card` of a note of `note_type`.
///
/// # Errors
///
/// The [`RenderIssue`] class of the first problem met: a template that does not parse, a field
/// the note type lacks, a front that shows no filled field, or an ordinal with no template.
pub fn render(
    note_type: &NoteType,
    note: &NoteContent<'_>,
    card: &CardContext<'_>,
) -> Result<Rendered, RenderIssue> {
    render::card(note_type, note, card)
}

/// Checks that every template of `note_type` can render.
///
/// # Errors
///
/// The [`RenderIssue`] class of the first problem met, template by template.
pub fn check_note_type(note_type: &NoteType) -> Result<(), RenderIssue> {
    render::check(note_type)
}
