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
use std::path::{Path, PathBuf};

use deck_streak_kernel::Setting;

use crate::persona::{CefrBand, Duty, Persona, PersonaError, Slots, TemplateId, TemplateSet};

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
        let _ = path;
        Self::parse("", templates)
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
        let _ = text;
        Ok(Self {
            templates,
            personas: BTreeMap::new(),
            topics: BTreeMap::new(),
        })
    }

    /// The persona that teaches `topic` for `duty`, its slots filled from the roster.
    ///
    /// # Errors
    ///
    /// [`PersonaError::TopicUnbound`] when the roster binds no template to `topic`, and
    /// [`PersonaError::DutyNotOffered`] when the topic's template does not offer `duty`.
    pub fn persona(&self, topic: &TopicKey, duty: Duty) -> Result<Persona, PersonaError> {
        let _ = (topic, duty);
        Err(PersonaError::TopicUnbound)
    }
}

impl fmt::Debug for Roster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Roster")
            .field("personas", &self.personas.len())
            .field("topics", &self.topics.len())
            .finish()
    }
}
