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

use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::settings::{Environment, Setting};

/// The setting naming the private conventions file.
pub const CONVENTIONS_FILE: &str = "DECKSTREAK_CONVENTIONS_FILE";
/// The one schema a conventions file may carry.
pub const CONVENTIONS_SCHEMA: &str = "deckstreak.conventions.v1";

/// The shape a conventions file must have, named in a refusal in place of any value.
const FILE_SHAPE: &str = "a JSON object of the schema deckstreak.conventions.v1 with direction \
    rules, transfer fields and a Can-Do field";
/// The shape of the direction rules.
const DIRECTION_SHAPE: &str = "direction rules of pair, template and note type rules, each with \
    a name and one of the three direction buckets, and a list of exempt tokens";
/// The shape of the transfer fields.
const TRANSFER_SHAPE: &str = "transfer fields with excluded fields, a choice prefix, a rank \
    field and meaning fields, each a non-blank name";
/// The tokens a direction rule may not carry unless the exempt list names its text. They are code:
/// the file cannot switch the rule off (the predecessor's `direction.py:_FORBIDDEN_TOKENS`).
pub const FORBIDDEN_TOKENS: [&str; 3] = ["recall", "recognition", "recognize"];

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
    /// [`ConventionsError::Malformed`] when the text is not the schema's shape, and
    /// [`ConventionsError::ForbiddenLabel`] when a direction rule holds a forbidden token that
    /// the exempt list does not name.
    pub fn parse(text: &str) -> Result<Self, ConventionsError> {
        let file: Value = serde_json::from_str(text).map_err(|_| malformed(FILE_SHAPE))?;
        let file = file.as_object().ok_or_else(|| malformed(FILE_SHAPE))?;
        if file.get("schema").and_then(Value::as_str) != Some(CONVENTIONS_SCHEMA) {
            return Err(malformed(FILE_SHAPE));
        }
        let direction = direction_rules(section(file, "direction", DIRECTION_SHAPE)?)?;
        let transfer = transfer_fields(section(file, "transfer", TRANSFER_SHAPE)?)?;
        let can_do_field = name(file, "can_do_field", FILE_SHAPE)?;
        let conventions = Self {
            direction,
            transfer,
            can_do_field,
        };
        conventions.direction.refuse_forbidden()?;
        Ok(conventions)
    }

    /// The conventions of the file the environment's [`CONVENTIONS_FILE`] names, read once at
    /// start; empty, said once in the log, when the setting is unset.
    ///
    /// # Errors
    ///
    /// [`ConventionsError::Unreadable`] when the file cannot be read, and every refusal of
    /// [`Conventions::parse`].
    pub fn load(env: &Environment) -> Result<Self, ConventionsError> {
        let path = env
            .optional::<ConventionsPath>(CONVENTIONS_FILE)
            .map_err(|_| malformed(ConventionsPath::SHAPE))?;
        let Some(ConventionsPath(path)) = path else {
            tracing::info!(
                setting = CONVENTIONS_FILE,
                "no conventions file is configured: the instruments that read note conventions have none"
            );
            return Ok(Self::default());
        };
        let text = std::fs::read_to_string(path).map_err(|_| ConventionsError::Unreadable {
            setting: CONVENTIONS_FILE,
        })?;
        Self::parse(&text)
    }
}

impl DirectionRules {
    /// Refuses a rule whose text holds a forbidden token that no exempt token's text covers.
    fn refuse_forbidden(&self) -> Result<(), ConventionsError> {
        let texts = self
            .pair_rules
            .iter()
            .flat_map(|rule| [rule.front.as_str(), rule.back.as_str()])
            .chain(
                self.template_rules
                    .iter()
                    .map(|rule| rule.template.as_str()),
            )
            .chain(
                self.note_type_rules
                    .iter()
                    .map(|rule| rule.contains.as_str()),
            );
        for text in texts {
            let lowered = text.to_lowercase();
            let exempt = self
                .exempt_tokens
                .iter()
                .any(|token| lowered.contains(&token.to_lowercase()));
            let forbidden = FORBIDDEN_TOKENS.iter().any(|token| lowered.contains(token));
            if forbidden && !exempt {
                return Err(ConventionsError::ForbiddenLabel {
                    setting: CONVENTIONS_FILE,
                });
            }
        }
        Ok(())
    }
}

/// The refusal of a file that does not have `expected`'s shape.
const fn malformed(expected: &'static str) -> ConventionsError {
    ConventionsError::Malformed {
        setting: CONVENTIONS_FILE,
        expected,
    }
}

/// The object section `key` of `object`.
fn section<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    shape: &'static str,
) -> Result<&'a Map<String, Value>, ConventionsError> {
    object
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| malformed(shape))
}

/// The text field `key` of `object`: present and not blank.
fn name(
    object: &Map<String, Value>,
    key: &str,
    shape: &'static str,
) -> Result<String, ConventionsError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| malformed(shape))
}

/// The list `key` of `object`, each entry a non-blank name.
fn names(
    object: &Map<String, Value>,
    key: &str,
    shape: &'static str,
) -> Result<Vec<String>, ConventionsError> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| malformed(shape))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| malformed(shape))
        })
        .collect()
}

/// The direction field of a rule.
fn direction_of(object: &Map<String, Value>) -> Result<Direction, ConventionsError> {
    match object.get("direction").and_then(Value::as_str) {
        Some("recognition") => Ok(Direction::Recognition),
        Some("cued_recall") => Ok(Direction::CuedRecall),
        Some("production") => Ok(Direction::Production),
        _ => Err(malformed(DIRECTION_SHAPE)),
    }
}

/// The rules of the list `key`, each built from its object by `build`.
fn rules<T>(
    object: &Map<String, Value>,
    key: &str,
    build: impl Fn(&Map<String, Value>) -> Result<T, ConventionsError>,
) -> Result<Vec<T>, ConventionsError> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| malformed(DIRECTION_SHAPE))?
        .iter()
        .map(|value| {
            build(
                value
                    .as_object()
                    .ok_or_else(|| malformed(DIRECTION_SHAPE))?,
            )
        })
        .collect()
}

fn direction_rules(object: &Map<String, Value>) -> Result<DirectionRules, ConventionsError> {
    Ok(DirectionRules {
        pair_rules: rules(object, "pair_rules", |rule| {
            Ok(PairRule {
                front: name(rule, "front", DIRECTION_SHAPE)?,
                back: name(rule, "back", DIRECTION_SHAPE)?,
                direction: direction_of(rule)?,
            })
        })?,
        template_rules: rules(object, "template_rules", |rule| {
            Ok(TemplateRule {
                template: name(rule, "template", DIRECTION_SHAPE)?,
                direction: direction_of(rule)?,
            })
        })?,
        note_type_rules: rules(object, "note_type_rules", |rule| {
            Ok(NoteTypeRule {
                contains: name(rule, "contains", DIRECTION_SHAPE)?,
                direction: direction_of(rule)?,
            })
        })?,
        exempt_tokens: names(object, "exempt_tokens", DIRECTION_SHAPE)?,
    })
}

fn transfer_fields(object: &Map<String, Value>) -> Result<TransferFields, ConventionsError> {
    Ok(TransferFields {
        excluded_fields: names(object, "excluded_fields", TRANSFER_SHAPE)?,
        choice_prefix: name(object, "choice_prefix", TRANSFER_SHAPE)?,
        rank_field: name(object, "rank_field", TRANSFER_SHAPE)?,
        meaning_fields: names(object, "meaning_fields", TRANSFER_SHAPE)?,
    })
}

/// The path the setting names: an absolute file path.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ConventionsPath(PathBuf);

impl Setting for ConventionsPath {
    const SHAPE: &'static str = "an absolute file path";

    fn parse(text: &str) -> Option<Self> {
        let path = PathBuf::from(text);
        path.is_absolute().then_some(Self(path))
    }
}
