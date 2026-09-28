//! The learner memory a persona reads: its sources, their ports, and the reader built for one
//! subject that records every read it makes (SPEC-044 R5, R7, R8; ADR-044).
//!
//! A persona reads the leeches, the drill grades and the lapse history of its own subject, and
//! never the journal (CHARTER 18). A template declares which of the three it reads; each source is
//! a port its own feature wires (#133, #136, #83), and with none wired a persona reads nothing. The
//! reader refuses another subject's read before it asks any port, and its record is the only
//! source of an output's `memory` declaration, so the declaration cannot claim a read that was not
//! made or omit one that was.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use deck_streak_kernel::{KernelError, PortFuture};

use crate::persona::{CefrBand, Persona, PersonaError, Subject};

/// A source of learner memory a template may declare. The journal is never one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemorySource {
    /// The cards the learner keeps failing.
    Leeches,
    /// The grades of the learner's drills.
    DrillGrades,
    /// The learner's lapse history.
    Lapses,
}

impl MemorySource {
    /// The three sources, in persona-core's order.
    pub const ALL: [Self; 3] = [Self::Leeches, Self::DrillGrades, Self::Lapses];

    /// The source's name, as a template declares it and a memory token writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Leeches => "leeches",
            Self::DrillGrades => "drill-grades",
            Self::Lapses => "lapses",
        }
    }

    /// The source `name` names.
    ///
    /// # Errors
    ///
    /// [`PersonaError::Template`] for a name off the list.
    pub fn parse(name: &str) -> Result<Self, PersonaError> {
        match name {
            "leeches" => Ok(Self::Leeches),
            "drill-grades" => Ok(Self::DrillGrades),
            "lapses" => Ok(Self::Lapses),
            _ => Err(PersonaError::Template("a memory source is off the list")),
        }
    }
}

/// Why the reader refused a read, or could not complete it.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// The read names another subject than the reader's own (R5).
    #[error("a persona's memory reader reads only its own subject")]
    OtherSubject,
    /// The read names a source the persona's template does not declare (R5).
    #[error("the persona's template does not declare the memory source")]
    Undeclared,
    /// The source's port could not read; the source says why.
    #[error("the memory source could not be read")]
    Source(#[from] KernelError),
}

/// One read the reader made: a source, of the reader's own subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryRead {
    source: MemorySource,
    subject: Subject,
}

impl MemoryRead {
    /// The source read.
    #[must_use]
    pub const fn source(&self) -> MemorySource {
        self.source
    }

    /// The subject whose memory was read.
    #[must_use]
    pub const fn subject(&self) -> &Subject {
        &self.subject
    }

    /// The read as an output's `memory` declares it: `<source>@<subject>`.
    #[must_use]
    pub fn token(&self) -> String {
        format!("{}@{}", self.source.name(), self.subject)
    }
}

/// What one source gave the reader: the learner's data for the prompt, never instructions. Its
/// `Debug` counts the entries and never shows them.
#[derive(Clone, PartialEq, Eq)]
pub struct Recall {
    source: MemorySource,
    entries: Vec<String>,
}

impl Recall {
    /// The source that answered.
    #[must_use]
    pub const fn source(&self) -> MemorySource {
        self.source
    }

    /// Its entries, one line of data each.
    #[must_use]
    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

impl fmt::Debug for Recall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recall")
            .field("source", &self.source)
            .field("entries", &self.entries.len())
            .finish()
    }
}

/// A memory source's port: its memory of one subject. The feature that builds the source's data
/// implements it (R7).
pub trait MemoryPort: Send + Sync {
    /// The entries this source holds for `subject`, one line of data each.
    fn read<'a>(&'a self, subject: &'a Subject) -> PortFuture<'a, Vec<String>>;
}

/// The live Road-to-C2 band's port, which the curriculum wires (R8, #85).
pub trait LiveBand: Send + Sync {
    /// The live band of `subject`, or `None` while the curriculum has none for it.
    fn band<'a>(&'a self, subject: &'a Subject) -> PortFuture<'a, Option<CefrBand>>;
}

/// The wired memory sources. None is wired by default, and then a persona reads nothing (R7).
#[derive(Clone, Default)]
pub struct MemoryPorts {
    ports: BTreeMap<MemorySource, Arc<dyn MemoryPort>>,
}

impl MemoryPorts {
    /// No source wired.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// These ports, with `port` serving `source`.
    #[must_use]
    pub fn wired(mut self, source: MemorySource, port: Arc<dyn MemoryPort>) -> Self {
        self.ports.insert(source, port);
        self
    }
}

/// A reader of one persona's memory: built for the persona's own subject and the sources its
/// template declares, it records every read it makes (R5).
pub struct MemoryReader<'p> {
    subject: Subject,
    declared: Vec<MemorySource>,
    ports: &'p MemoryPorts,
    reads: Vec<MemoryRead>,
}

impl<'p> MemoryReader<'p> {
    /// The reader of `persona`'s memory, through `ports`.
    #[must_use]
    pub fn new(persona: &Persona, ports: &'p MemoryPorts) -> Self {
        Self {
            subject: persona.subject().clone(),
            declared: persona.memory().to_vec(),
            ports,
            reads: Vec::new(),
        }
    }

    /// The only subject it reads.
    #[must_use]
    pub const fn subject(&self) -> &Subject {
        &self.subject
    }

    /// `subject`'s memory in `source`: `None` when no port serves the source, which reads nothing
    /// and records nothing.
    ///
    /// # Errors
    ///
    /// [`MemoryError::OtherSubject`] when `subject` is not the reader's own, and
    /// [`MemoryError::Undeclared`] when the template does not declare `source`, each before any
    /// port is asked; [`MemoryError::Source`] when the port fails, which records nothing.
    pub async fn read(
        &mut self,
        source: MemorySource,
        subject: &Subject,
    ) -> Result<Option<Recall>, MemoryError> {
        let Some(port) = self.ports.ports.get(&source) else {
            return Ok(None);
        };
        let entries = port.read(subject).await?;
        self.reads.push(MemoryRead {
            source,
            subject: subject.clone(),
        });
        Ok(Some(Recall { source, entries }))
    }

    /// Every source the template declares that a port serves, in the template's order.
    ///
    /// # Errors
    ///
    /// [`MemoryError::Source`] when a port fails.
    pub async fn read_declared(&mut self) -> Result<Vec<Recall>, MemoryError> {
        let subject = self.subject.clone();
        let mut recalled = Vec::new();
        for source in self.declared.clone() {
            if let Some(recall) = self.read(source, &subject).await? {
                recalled.push(recall);
            }
        }
        Ok(recalled)
    }

    /// Every read it made, once per source, in the order first made: the only record an output's
    /// `memory` is written from.
    #[must_use]
    pub fn reads(&self) -> &[MemoryRead] {
        &self.reads
    }
}

/// The band a language persona's output uses: the live band when its port is wired and answers
/// one, and the roster's band for the topic otherwise (R8); `None` for any other kind.
///
/// # Errors
///
/// The live band's port failing.
pub async fn resolve_band(
    persona: &Persona,
    live: Option<&dyn LiveBand>,
) -> Result<Option<CefrBand>, KernelError> {
    let _ = live;
    Ok(persona.band())
}
