//! The learner memory a persona reads: its sources (SPEC-044 R5, R7; ADR-044).
//!
//! A persona reads the leeches, the drill grades and the lapse history of its own subject, and
//! never the journal (CHARTER 18). A template declares which of the three it reads.

use crate::persona::PersonaError;

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
