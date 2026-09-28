//! The frontmatter the engine writes on every output (SPEC-044 R6; ADR-044).
//!
//! The engine, never the model, writes it, in persona-core's output contract v1: one
//! `key: <one-line JSON value>` line per key. Its `memory` lists exactly the reads the persona's
//! reader recorded, possibly none; a language output also carries its `cefr` band and its `lang`.
//! A duty writes its own further keys after these (SPEC-046).

use crate::memory::MemoryRead;
use crate::persona::{CefrBand, Duty, Persona, Subject, SubjectKind, TemplateId};

/// The schema id every persona output declares (persona-core's output contract v1).
pub const OUTPUT_SCHEMA: &str = "phx.persona.output.v1";

/// An output's frontmatter, as the engine writes it from what it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frontmatter {
    persona: TemplateId,
    subject: Subject,
    duty: Duty,
    cefr: Option<CefrBand>,
    lang: Option<String>,
    memory: Vec<String>,
}

impl Frontmatter {
    /// The frontmatter of `persona`'s output: `band` is written on a language output only, and
    /// `memory` is `reads`, the reader's record.
    #[must_use]
    pub fn new(persona: &Persona, band: Option<CefrBand>, reads: &[MemoryRead]) -> Self {
        let language = persona.subject().kind() == SubjectKind::Language;
        Self {
            persona: persona.template().clone(),
            subject: persona.subject().clone(),
            duty: persona.duty(),
            cefr: band.filter(|_| language),
            lang: persona.lang().map(str::to_owned),
            memory: reads.iter().map(MemoryRead::token).collect(),
        }
    }

    /// Its lines, each `key: <one-line JSON value>` and a line break, in the contract's order.
    #[must_use]
    pub fn lines(&self) -> String {
        format!(
            "schema: \"{OUTPUT_SCHEMA}\"\npersona: \"{}\"\nsubject: \"{}\"\nduty: \"{}\"\nmemory: []\n",
            self.persona.as_str(),
            self.subject,
            self.duty.name()
        )
    }

    /// The whole frontmatter, between its `---` delimiters.
    #[must_use]
    pub fn render(&self) -> String {
        format!("---\n{}---\n", self.lines())
    }
}
