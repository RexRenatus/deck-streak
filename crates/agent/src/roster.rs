//! The private roster: which template teaches each topic, the four slots each template it uses is
//! filled with, and each language topic's band until the live band exists (SPEC-044 R2, R3, R10;
//! ADR-044).
//!
//! The roster is a `deckstreak.agent.roster.v1` JSON file outside the repository, named by the
//! setting [`ROSTER`] and placed by the private rail. Nothing of it reaches a log: the roster's
//! `Debug` shows its counts, the path's shows nothing, and a refusal names the rule it applies,
//! never a key or a value.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_kernel::Setting;
use serde_json::{Map, Value};

use crate::persona::{
    CefrBand, Duty, Persona, PersonaError, Slot, Slots, SubjectKind, TemplateId, TemplateSet,
};

/// The schema id of the private roster.
pub const ROSTER_SCHEMA: &str = "deckstreak.agent.roster.v1";
/// The setting naming the roster file by an absolute path.
pub const ROSTER: &str = "DECKSTREAK_AGENT_ROSTER";

/// Where the private roster is, an absolute path. Its `Debug` never shows the path.
#[derive(Clone, PartialEq, Eq)]
pub struct RosterPath(PathBuf);

impl RosterPath {
    /// The roster at `path`, or `None` when the path is not absolute.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Option<Self> {
        let path = path.into();
        path.is_absolute().then_some(Self(path))
    }

    /// The roster's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Debug for RosterPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RosterPath(..)")
    }
}

impl Setting for RosterPath {
    const SHAPE: &'static str = "an absolute file path";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// A readings topic's key, as the roster binds it: a lowercase slug whose groups `/` joins.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TopicKey(String);

impl TopicKey {
    /// The key `key`, or `None` when it is not a lowercase slug of `a-z`, `0-9`, `-` and `/`.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        key.split('/')
            .all(crate::persona::is_slug)
            .then(|| Self(key.to_owned()))
    }

    /// The key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One topic's binding: its template, and for a language mentor the band to write at.
#[derive(Clone, PartialEq, Eq)]
struct Binding {
    template: TemplateId,
    band: Option<CefrBand>,
}

/// The private roster, read and checked against the templates it binds.
#[derive(Clone, PartialEq, Eq)]
pub struct Roster {
    templates: TemplateSet,
    personas: BTreeMap<TemplateId, Slots>,
    topics: BTreeMap<TopicKey, Binding>,
}

impl Roster {
    /// The roster the file at `path` holds, checked against `templates`.
    ///
    /// # Errors
    ///
    /// [`PersonaError::RosterUnreadable`] when the file cannot be read, and every refusal of
    /// [`Roster::parse`].
    pub fn load(path: &RosterPath, templates: TemplateSet) -> Result<Self, PersonaError> {
        let text = fs::read_to_string(path.path()).map_err(PersonaError::RosterUnreadable)?;
        Self::parse(&text, templates)
    }

    /// The roster `text` holds, checked against `templates`.
    ///
    /// # Errors
    ///
    /// [`PersonaError::Roster`] naming a schema rule the text breaks;
    /// [`PersonaError::UnknownTemplate`] for an entry that names no template of `templates`;
    /// [`PersonaError::UnknownSlot`], [`PersonaError::SlotUnfilled`] and
    /// [`PersonaError::SlotNotPlainText`] for a persona entry's slots.
    pub fn parse(text: &str, templates: TemplateSet) -> Result<Self, PersonaError> {
        let document: Value =
            serde_json::from_str(text).map_err(|_| PersonaError::Roster("it is not JSON"))?;
        let roster = object(Some(&document), "it is not a JSON object")?;
        only(
            roster,
            &["schema", "personas", "topics"],
            "it holds a key the schema does not name",
        )?;
        if roster.get("schema").and_then(Value::as_str) != Some(ROSTER_SCHEMA) {
            return Err(PersonaError::Roster(
                "schema is not deckstreak.agent.roster.v1",
            ));
        }
        let mut personas = BTreeMap::new();
        for (id, entry) in object(roster.get("personas"), "personas is not an object")? {
            let id = known(&templates, id)?;
            let entry = object(Some(entry), "a persona entry is not an object")?;
            if entry.keys().any(|key| Slot::parse(key).is_none()) {
                return Err(PersonaError::UnknownSlot);
            }
            let value = |slot: Slot| {
                entry
                    .get(slot.name())
                    .and_then(Value::as_str)
                    .ok_or(PersonaError::SlotUnfilled { slot: slot.name() })
            };
            let slots = Slots::new(
                value(Slot::Name)?,
                value(Slot::Bio)?,
                value(Slot::Voice)?,
                value(Slot::Personality)?,
            )?;
            personas.insert(id, slots);
        }
        let mut topics = BTreeMap::new();
        for (key, entry) in object(roster.get("topics"), "topics is not an object")? {
            let key = TopicKey::parse(key)
                .ok_or(PersonaError::Roster("a topic key is not a lowercase slug"))?;
            let entry = object(Some(entry), "a topic entry is not an object")?;
            only(
                entry,
                &["template", "cefr"],
                "a topic entry holds a key the schema does not name",
            )?;
            let template = entry
                .get("template")
                .and_then(Value::as_str)
                .ok_or(PersonaError::Roster("a topic names no template"))?;
            let template = known(&templates, template)?;
            if !personas.contains_key(&template) {
                return Err(PersonaError::SlotUnfilled {
                    slot: Slot::Name.name(),
                });
            }
            let language = templates
                .get(&template)
                .is_some_and(|bound| bound.subject().kind() == SubjectKind::Language);
            let band = match (language, entry.get("cefr")) {
                (true, Some(band)) => Some(band.as_str().and_then(CefrBand::parse).ok_or(
                    PersonaError::Roster("a topic's cefr is not a band from A1 to C2"),
                )?),
                (true, None) => {
                    return Err(PersonaError::Roster("a language topic names no cefr band"));
                }
                (false, None) => None,
                (false, Some(_)) => {
                    return Err(PersonaError::Roster(
                        "only a language topic names a cefr band",
                    ));
                }
            };
            topics.insert(key, Binding { template, band });
        }
        Ok(Self {
            templates,
            personas,
            topics,
        })
    }

    /// The persona that teaches `topic` for `duty`, its slots filled from the roster.
    ///
    /// # Errors
    ///
    /// [`PersonaError::TopicUnbound`] when the roster binds no template to `topic`, and
    /// [`PersonaError::DutyNotOffered`] when the topic's template does not offer `duty`.
    pub fn persona(&self, topic: &TopicKey, duty: Duty) -> Result<Persona, PersonaError> {
        let binding = self.topics.get(topic).ok_or(PersonaError::TopicUnbound)?;
        let template = self
            .templates
            .get(&binding.template)
            .ok_or(PersonaError::UnknownTemplate)?;
        let slots = self
            .personas
            .get(&binding.template)
            .ok_or(PersonaError::SlotUnfilled {
                slot: Slot::Name.name(),
            })?;
        template.instantiate(slots, duty, binding.band)
    }
}

/// The JSON object `value` is, refused as `refusal` when it is absent or anything else.
fn object<'v>(
    value: Option<&'v Value>,
    refusal: &'static str,
) -> Result<&'v Map<String, Value>, PersonaError> {
    value
        .and_then(Value::as_object)
        .ok_or(PersonaError::Roster(refusal))
}

/// Refuses `object` as `refusal` when it holds a key `keys` does not name, so a misspelt key never
/// passes silently.
fn only(
    object: &Map<String, Value>,
    keys: &[&str],
    refusal: &'static str,
) -> Result<(), PersonaError> {
    if object.keys().all(|key| keys.contains(&key.as_str())) {
        Ok(())
    } else {
        Err(PersonaError::Roster(refusal))
    }
}

/// The id `id`, when it names a template of `templates`.
fn known(templates: &TemplateSet, id: &str) -> Result<TemplateId, PersonaError> {
    TemplateId::parse(id)
        .filter(|id| templates.get(id).is_some())
        .ok_or(PersonaError::UnknownTemplate)
}

impl fmt::Debug for Roster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Roster")
            .field("templates", &self.templates.len())
            .field("personas", &self.personas.len())
            .field("topics", &self.topics.len())
            .finish()
    }
}
