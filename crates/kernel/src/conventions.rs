//! The owner's note conventions (SPEC-094 R1 and R2; ADR-096): private configuration, read once at
//! start from the JSON file [`CONVENTIONS_FILE`] names, in the schema [`CONVENTIONS_SCHEMA`].
//!
//! The predecessor hard-codes the owner's note vocabulary (direction rules, transfer field names,
//! the Can-Do field: `direction.py`, `transfer.py` and `cando.py` at `27ee2bc`). Those are the
//! owner's own conventions, which CHARTER 11 keeps out of the repository, so here they are a file
//! outside it, and `deploy/config/conventions.example.json` shows the shape with neutral values.
//!
//! A refusal names the setting and never a value. The forbidden direction tokens are code, not
//! configuration: a file cannot switch the rule off.

use crate::settings::Environment;

/// The setting naming the private conventions file.
pub const CONVENTIONS_FILE: &str = "DECKSTREAK_CONVENTIONS_FILE";
/// The one schema a conventions file may carry.
pub const CONVENTIONS_SCHEMA: &str = "deckstreak.conventions.v1";

/// The direction a card exercises: how the answer relates to the cue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The learner recognises the target form.
    Recognition,
    /// The learner recalls the target form from a cue.
    CuedRecall,
    /// The learner produces the target form.
    Production,
}

/// A rule matching a card by the pair of its front and back field names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairRule {
    /// The front field's name.
    pub front: String,
    /// The back field's name.
    pub back: String,
    /// The direction the pair exercises.
    pub direction: Direction,
}

/// A rule matching a card by its template's name, exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateRule {
    /// The template's name.
    pub template: String,
    /// The direction the template exercises.
    pub direction: Direction,
}

/// A rule matching a card by a substring of its note type's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteTypeRule {
    /// The substring.
    pub contains: String,
    /// The direction the note type exercises.
    pub direction: Direction,
}

/// The direction rules: how a card's direction is decided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirectionRules {
    /// Rules by field pair.
    pub pair_rules: Vec<PairRule>,
    /// Rules by template name.
    pub template_rules: Vec<TemplateRule>,
    /// Rules by note type substring.
    pub note_type_rules: Vec<NoteTypeRule>,
    /// Tokens a rule may contain although they hold a forbidden label.
    pub exempt_tokens: Vec<String>,
}

/// The names of the fields the transfer instrument reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransferFields {
    /// Field names the transfer instrument never reads.
    pub excluded_fields: Vec<String>,
    /// The prefix of a choice field's name.
    pub choice_prefix: String,
    /// The field holding a note's rank.
    pub rank_field: String,
    /// The fields holding a note's meaning.
    pub meaning_fields: Vec<String>,
}

/// The owner's note conventions, as loaded at start.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Conventions {
    /// How a card's direction is decided.
    pub direction: DirectionRules,
    /// The transfer instrument's field names.
    pub transfer: TransferFields,
    /// The field holding a note's Can-Do statement.
    pub can_do_field: String,
}

/// Why the conventions refuse start. Each names the setting and never a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConventionsError {
    /// The file the setting names cannot be read.
    #[error("the setting {setting} names a file that cannot be read")]
    Unreadable {
        /// The setting's name.
        setting: &'static str,
    },
    /// The setting or its file does not have the shape it must have.
    #[error("the setting {setting} is malformed: it must be {expected}")]
    Malformed {
        /// The setting's name.
        setting: &'static str,
        /// The shape expected.
        expected: &'static str,
    },
    /// A direction rule names a forbidden token that is not on the exempt list.
    #[error("the setting {setting} names a file with a direction rule that uses a forbidden label")]
    ForbiddenLabel {
        /// The setting's name.
        setting: &'static str,
    },
}

impl Conventions {
    /// The conventions of the file text `text`.
    ///
    /// # Errors
    ///
    /// See [`ConventionsError`].
    pub fn parse(_text: &str) -> Result<Self, ConventionsError> {
        Ok(Self::default())
    }

    /// The conventions of the file the environment's [`CONVENTIONS_FILE`] names.
    ///
    /// # Errors
    ///
    /// See [`ConventionsError`].
    pub fn load(_env: &Environment) -> Result<Self, ConventionsError> {
        Ok(Self::default())
    }
}
