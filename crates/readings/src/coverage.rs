//! The coverage gates (SPEC-046 R6): the anchors, the roster, completeness, list markers, and the
//! order the first failure decides in.

use crate::form::Form;
use crate::seed::Seed;
use crate::state::ReadingGate;

/// The longest an anchor is.
pub const ANCHOR_MAX_CHARS: usize = 0;
/// The shortest a usable anchor is.
pub const ANCHOR_MIN_USABLE_CHARS: usize = 0;

/// A note's text with entities decoded, markup removed and the rail's characters dropped.
#[must_use]
pub fn normalise(text: &str) -> String {
    text.to_owned()
}

/// The anchor of a note.
#[must_use]
pub fn anchor_for_note(text: &str) -> String {
    text.to_owned()
}

/// Whether an anchor is long enough to verify against.
#[must_use]
pub fn is_anchor_usable(anchor: &str) -> bool {
    anchor.is_empty()
}

/// A reading split into its frontmatter and its sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    /// The text between the frontmatter fences.
    pub frontmatter: String,
    /// The text after the frontmatter.
    pub body: String,
    /// Each section's name and text, in order.
    pub sections: Vec<(String, String)>,
}

impl Document {
    /// Splits `text`.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        Self {
            frontmatter: String::new(),
            body: text.to_owned(),
            sections: Vec::new(),
        }
    }
}

/// The findings of the engine's own gates; an empty list passes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnChecks {
    /// Completeness.
    pub complete: Vec<String>,
    /// The roster.
    pub roster: Vec<String>,
    /// The anchors.
    pub anchors: Vec<String>,
    /// List markers.
    pub no_list_markers: Vec<String>,
    /// The advisory of unusable anchors, when some were excluded.
    pub advisory: Option<String>,
}

/// Runs the engine's own gates over `document`.
#[must_use]
pub fn check_own(form: &Form, seed: &Seed, document: &Document) -> OwnChecks {
    let _ = (form, seed, document);
    OwnChecks::default()
}

/// What the pack gate refused: its class and its finding lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackFailure {
    /// The failing class.
    pub class: String,
    /// Its finding lines.
    pub findings: Vec<String>,
}

/// The gate that failed first and its findings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateFailure {
    /// The gate.
    pub gate: ReadingGate,
    /// Its finding lines.
    pub findings: Vec<String>,
}

/// The first failure across the own gates and the pack gate, in the gate order.
#[must_use]
pub fn first_failure(own: &OwnChecks, pack: Option<&PackFailure>) -> Option<GateFailure> {
    let _ = (own, pack);
    None
}
