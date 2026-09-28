//! The public persona templates, and the vocabulary they are written in (SPEC-044 R1, R4;
//! ADR-044).
//!
//! A template is a public, neutral document in persona-core's template schema v1: a frontmatter of
//! `key: <one-line JSON value>` lines naming its id, subject, language, duties and memory sources,
//! then a body of marked sections carrying the four roster slots. The engine loads a template only
//! when each slot still stands in its section, so no template it instantiates carries a name, a
//! bio, a voice or a personality of its own.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::Value;

use crate::memory::MemorySource;

/// The schema id every persona template declares (persona-core's template schema v1).
pub const TEMPLATE_SCHEMA: &str = "phx.persona.template.v1";

/// The public templates, compiled into the engine from `agent/personas/` (ADR-044): the text a
/// build instantiates is the text the box run judged.
const PUBLIC: [&str; 18] = [
    include_str!("../../../agent/personas/es.persona.md"),
    include_str!("../../../agent/personas/fr.persona.md"),
    include_str!("../../../agent/personas/ja.persona.md"),
    include_str!("../../../agent/personas/ko.persona.md"),
    include_str!("../../../agent/personas/law-business-associations.persona.md"),
    include_str!("../../../agent/personas/law-civil-procedure.persona.md"),
    include_str!("../../../agent/personas/law-conflict-of-laws.persona.md"),
    include_str!("../../../agent/personas/law-constitutional-law.persona.md"),
    include_str!("../../../agent/personas/law-contracts.persona.md"),
    include_str!("../../../agent/personas/law-criminal-law-and-procedure.persona.md"),
    include_str!("../../../agent/personas/law-evidence.persona.md"),
    include_str!("../../../agent/personas/law-family-law.persona.md"),
    include_str!("../../../agent/personas/law-professional-responsibility.persona.md"),
    include_str!("../../../agent/personas/law-real-property.persona.md"),
    include_str!("../../../agent/personas/law-secured-transactions.persona.md"),
    include_str!("../../../agent/personas/law-torts.persona.md"),
    include_str!("../../../agent/personas/law-trusts-and-estates.persona.md"),
    include_str!("../../../agent/personas/zh.persona.md"),
];

/// Each section the template schema gives roster slots, with the slots it must carry
/// (persona-core's contract, `template_sections`).
const SLOTTED: [(&str, &[Slot]); 4] = [
    ("identity", &[Slot::Name, Slot::Bio]),
    ("voice", &[Slot::Voice]),
    ("personality", &[Slot::Personality]),
    ("disclosure", &[Slot::Name]),
];

/// Why the engine refused a template, a roster or an instantiation.
///
/// No variant carries a slot's value, a topic key, a template id or a path: a refusal reaches the
/// journal, and the roster's contents stay out of every log (SPEC-044 R10). A refusal names the
/// rule, and for a slot the slot's public name.
#[derive(Debug, thiserror::Error)]
pub enum PersonaError {
    /// A template does not read as the template schema v1; the reason names the rule.
    #[error("a persona template does not read as the template schema: {0}")]
    Template(&'static str),
    /// A template's roster slot is filled, or missing from the section that carries it (R4).
    #[error("a persona template's {slot} slot is filled, or missing from its section")]
    SlotFilled {
        /// The slot's public name.
        slot: &'static str,
    },
    /// A template or a roster names a slot that is not one of the four (R3).
    #[error("a persona template or roster names a slot that is not one of the four")]
    UnknownSlot,
    /// A template names the journal as a memory source (CHARTER 18, R5).
    #[error("the journal is never a memory source")]
    Journal,
}

/// The kind of a subject: the part of `<kind>/<area>` before the slash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SubjectKind {
    /// A language, taught by its mentor.
    Language,
    /// A law subject, taught by its professor.
    Law,
    /// A test's preparation.
    TestPrep,
    /// Anything else.
    General,
}

impl SubjectKind {
    /// Every kind persona-core's contract lists.
    pub const ALL: [Self; 4] = [Self::Language, Self::Law, Self::TestPrep, Self::General];

    /// The kind as a subject spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Language => "language",
            Self::Law => "law",
            Self::TestPrep => "test-prep",
            Self::General => "general",
        }
    }

    /// The kind spelled `name`, or `None` for a kind the contract does not list.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// A persona's subject, `<kind>/<area>`: the scope of everything it may read.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Subject {
    kind: SubjectKind,
    text: String,
}

impl Subject {
    /// The subject `text`, or `None` when it is not a known kind, a slash and an area slug.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (kind, area) = text.split_once('/')?;
        let kind = SubjectKind::parse(kind)?;
        is_slug(area).then(|| Self {
            kind,
            text: text.to_owned(),
        })
    }

    /// The subject's kind.
    #[must_use]
    pub const fn kind(&self) -> SubjectKind {
        self.kind
    }

    /// The subject as a template and an output write it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// A template's id: a slug, unique across the templates.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateId(String);

impl TemplateId {
    /// The id `text`, or `None` when it is not a lowercase slug.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        is_slug(text).then(|| Self(text.to_owned()))
    }

    /// The id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One kind of AI work a persona does: persona-core's duty registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Duty {
    /// The day's pre-study reading.
    DailyReading,
    /// A drill, graded against the corpus.
    DrillCoach,
    /// An explanation, a mnemonic and a contrast for a card the learner keeps failing.
    LeechDoctor,
    /// Corrections of a writing sample.
    WritingTutor,
    /// One chat turn, corrected as it goes.
    ConversationPartner,
    /// A timed practice set with explanations.
    PracticeQuestions,
}

impl Duty {
    /// Every duty in the registry.
    pub const ALL: [Self; 6] = [
        Self::DailyReading,
        Self::DrillCoach,
        Self::LeechDoctor,
        Self::WritingTutor,
        Self::ConversationPartner,
        Self::PracticeQuestions,
    ];

    /// The duty's id, as a template and an output write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DailyReading => "daily-reading",
            Self::DrillCoach => "drill-coach",
            Self::LeechDoctor => "leech-doctor",
            Self::WritingTutor => "writing-tutor",
            Self::ConversationPartner => "conversation-partner",
            Self::PracticeQuestions => "practice-questions",
        }
    }

    /// The duty `name`, or `None` for a duty off the registry.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|duty| duty.name() == name)
    }
}

/// A CEFR band, A1 to C2: the level a language mentor writes at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CefrBand {
    /// A1.
    A1,
    /// A2.
    A2,
    /// B1.
    B1,
    /// B2.
    B2,
    /// C1.
    C1,
    /// C2.
    C2,
}

impl CefrBand {
    /// Every band, lowest first.
    pub const ALL: [Self; 6] = [Self::A1, Self::A2, Self::B1, Self::B2, Self::C1, Self::C2];

    /// The band as an output's `cefr` writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::A1 => "A1",
            Self::A2 => "A2",
            Self::B1 => "B1",
            Self::B2 => "B2",
            Self::C1 => "C1",
            Self::C2 => "C2",
        }
    }

    /// The band `name`, or `None` for anything but A1 to C2.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|band| band.name() == name)
    }
}

/// One of the four roster slots a template carries unfilled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slot {
    /// `{{name}}`.
    Name,
    /// `{{bio}}`.
    Bio,
    /// `{{voice}}`.
    Voice,
    /// `{{personality}}`.
    Personality,
}

impl Slot {
    /// The four slots.
    pub const ALL: [Self; 4] = [Self::Name, Self::Bio, Self::Voice, Self::Personality];

    /// The slot's public name, as the roster keys it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Bio => "bio",
            Self::Voice => "voice",
            Self::Personality => "personality",
        }
    }

    /// The slot's token in a template.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Name => "{{name}}",
            Self::Bio => "{{bio}}",
            Self::Voice => "{{voice}}",
            Self::Personality => "{{personality}}",
        }
    }

    /// The slot `name`, or `None` for a name that is not one of the four.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|slot| slot.name() == name)
    }
}

/// A public persona template, loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template {
    id: TemplateId,
    subject: Subject,
    lang: Option<String>,
    duties: Vec<Duty>,
    memory: Vec<MemorySource>,
    text: String,
    body: String,
}

impl Template {
    /// The template `text` reads as.
    ///
    /// # Errors
    ///
    /// [`PersonaError::Template`] naming the rule it breaks, [`PersonaError::SlotFilled`] for a
    /// slot missing from its section, [`PersonaError::UnknownSlot`] for any other `{{...}}` token,
    /// and [`PersonaError::Journal`] for a template that declares the journal.
    pub fn parse(text: &str) -> Result<Self, PersonaError> {
        let rest = text
            .strip_prefix("---\n")
            .ok_or(PersonaError::Template("line 1 is not ---"))?;
        let (head, body) = rest
            .split_once("\n---\n")
            .ok_or(PersonaError::Template("the frontmatter is never closed"))?;
        let fields = frontmatter(head)?;
        if fields.get("schema").and_then(Value::as_str) != Some(TEMPLATE_SCHEMA) {
            return Err(PersonaError::Template(
                "schema is not phx.persona.template.v1",
            ));
        }
        let id = fields
            .get("template")
            .and_then(Value::as_str)
            .and_then(TemplateId::parse)
            .ok_or(PersonaError::Template("template is not a slug"))?;
        let subject = fields
            .get("subject")
            .and_then(Value::as_str)
            .and_then(Subject::parse)
            .ok_or(PersonaError::Template(
                "subject is not a known kind and an area",
            ))?;
        let lang = match (subject.kind(), fields.get("lang")) {
            (SubjectKind::Language, Some(Value::String(tag))) => Some(tag.clone()),
            (SubjectKind::Language, _) => {
                return Err(PersonaError::Template("a language template names no lang"));
            }
            (_, None) => None,
            (_, Some(_)) => {
                return Err(PersonaError::Template(
                    "only a language template names a lang",
                ));
            }
        };
        let duties = names(&fields, "duties", "duties is not a list of names")?
            .into_iter()
            .map(|name| {
                Duty::parse(name).ok_or(PersonaError::Template("a duty is off the registry"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if duties.is_empty() {
            return Err(PersonaError::Template("the template offers no duty"));
        }
        let memory = names(&fields, "memory", "memory is not a list of names")?
            .into_iter()
            .map(MemorySource::parse)
            .collect::<Result<Vec<_>, _>>()?;
        slots_in_place(text, body)?;
        Ok(Self {
            id,
            subject,
            lang,
            duties,
            memory,
            text: text.to_owned(),
            body: body.to_owned(),
        })
    }

    /// The template's id.
    #[must_use]
    pub const fn id(&self) -> &TemplateId {
        &self.id
    }

    /// The template's subject: the scope of every read its persona makes.
    #[must_use]
    pub const fn subject(&self) -> &Subject {
        &self.subject
    }

    /// A language mentor's BCP 47 tag; `None` for any other kind.
    #[must_use]
    pub fn lang(&self) -> Option<&str> {
        self.lang.as_deref()
    }

    /// The duties the template offers.
    #[must_use]
    pub fn duties(&self) -> &[Duty] {
        &self.duties
    }

    /// The memory sources the template declares.
    #[must_use]
    pub fn memory(&self) -> &[MemorySource] {
        &self.memory
    }

    /// The whole template, as loaded.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The template's body: everything after its frontmatter.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// The templates the engine instantiates from, by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateSet {
    templates: BTreeMap<TemplateId, Template>,
}

impl TemplateSet {
    /// The public templates, compiled into the engine from `agent/personas/`.
    ///
    /// # Errors
    ///
    /// The refusal of the first template that does not load.
    pub fn public() -> Result<Self, PersonaError> {
        Self::new(
            PUBLIC
                .iter()
                .map(|text| Template::parse(text))
                .collect::<Result<Vec<_>, _>>()?,
        )
    }

    /// The set of `templates`.
    ///
    /// # Errors
    ///
    /// [`PersonaError::Template`] when two templates share an id.
    pub fn new(templates: Vec<Template>) -> Result<Self, PersonaError> {
        let mut set = BTreeMap::new();
        for template in templates {
            if set.insert(template.id.clone(), template).is_some() {
                return Err(PersonaError::Template("two templates share an id"));
            }
        }
        Ok(Self { templates: set })
    }

    /// The template `id`, or `None` when the set holds none.
    #[must_use]
    pub fn get(&self, id: &TemplateId) -> Option<&Template> {
        self.templates.get(id)
    }

    /// How many templates the set holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Whether the set holds no template.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

/// The frontmatter's fields: one `key: <one-line JSON value>` per line, each key once.
fn frontmatter(head: &str) -> Result<BTreeMap<&str, Value>, PersonaError> {
    let mut fields = BTreeMap::new();
    for line in head.split('\n') {
        let (key, value) = line.split_once(": ").ok_or(PersonaError::Template(
            "a frontmatter line is not key: JSON",
        ))?;
        let value = serde_json::from_str(value)
            .map_err(|_| PersonaError::Template("a frontmatter value is not one JSON value"))?;
        if fields.insert(key, value).is_some() {
            return Err(PersonaError::Template("a frontmatter key is repeated"));
        }
    }
    Ok(fields)
}

/// The names the field `key` lists, refused as `refusal` unless it is a list of strings.
fn names<'f>(
    fields: &'f BTreeMap<&str, Value>,
    key: &str,
    refusal: &'static str,
) -> Result<Vec<&'f str>, PersonaError> {
    fields
        .get(key)
        .and_then(Value::as_array)
        .ok_or(PersonaError::Template(refusal))?
        .iter()
        .map(|item| item.as_str().ok_or(PersonaError::Template(refusal)))
        .collect()
}

/// Refuses a template unless each slot stands in the section that carries it, and no other
/// `{{...}}` token is anywhere in `text` (R4).
fn slots_in_place(text: &str, body: &str) -> Result<(), PersonaError> {
    for (id, slots) in SLOTTED {
        let section = section(body, id);
        if let Some(slot) = slots.iter().find(|slot| !section.contains(slot.token())) {
            return Err(PersonaError::SlotFilled { slot: slot.name() });
        }
    }
    let mut rest = text;
    while let Some((_, after)) = rest.split_once("{{") {
        let (inner, tail) = after.split_once("}}").ok_or(PersonaError::UnknownSlot)?;
        Slot::parse(inner).ok_or(PersonaError::UnknownSlot)?;
        rest = tail;
    }
    Ok(())
}

/// The lines of the section marked `id` in `body`: from its `##` heading to the next `#` or `##`
/// heading, as persona-core reads a section; empty when no heading carries the marker.
fn section(body: &str, id: &str) -> String {
    let marker = format!("<!-- section:{id} -->");
    body.lines()
        .skip_while(|line| !(line.starts_with("## ") && line.ends_with(&marker)))
        .skip(1)
        .take_while(|line| !line.starts_with("# ") && !line.starts_with("## "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `text` is a lowercase slug: groups of `a-z` and `0-9` joined by single hyphens.
fn is_slug(text: &str) -> bool {
    text.split('-').all(|group| {
        !group.is_empty()
            && group
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}
