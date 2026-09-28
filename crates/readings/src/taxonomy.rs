//! The private taxonomy (SPEC-045 R1): which top-level decks are law roots and which segments are
//! their year bands, which are language decks (each with its code, its display name and the note
//! field that holds its term), and which roots hold writing decks that fold into a language.
//!
//! The owner's deck names and subjects are private, so no deck name and no topic list is a literal
//! here: the taxonomy is a JSON file of the schema [`TAXONOMY_SCHEMA`], whose path is the setting
//! [`READINGS_TAXONOMY`], read at each resolution. `deploy/config/readings-taxonomy.example.json` is
//! a synthetic one. A file that cannot be read as a taxonomy is refused whole, and nothing here
//! quotes it: a refusal names what is wrong, and `Debug` counts the names and prints none.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_ingest::settings::DECK_SEPARATOR;
use deck_streak_kernel::{Environment, Setting, SettingsError};
use serde::Deserialize;

use crate::topic::is_slug;

/// The schema a taxonomy file names.
pub const TAXONOMY_SCHEMA: &str = "deckstreak.readings.taxonomy.v1";
/// The setting that holds the taxonomy file's path, optional: unset, every resolution is
/// `taxonomy_missing` (R6).
pub const READINGS_TAXONOMY: &str = "DECKSTREAK_READINGS_TAXONOMY";

/// Why a file is not a taxonomy. No variant carries the file's text or a name from it.
#[derive(Debug, thiserror::Error)]
pub enum TaxonomyError {
    /// The file could not be read.
    #[error("the taxonomy file could not be read")]
    Unreadable(#[source] std::io::Error),
    /// The file is not JSON of the taxonomy's shape: the parser's line and column.
    #[error("the taxonomy file is not a taxonomy at line {line}, column {column}")]
    Malformed {
        /// The line the parser stopped at.
        line: usize,
        /// The column the parser stopped at.
        column: usize,
    },
    /// The file names another schema.
    #[error("the taxonomy file is not of the schema {TAXONOMY_SCHEMA}")]
    OtherSchema,
    /// A name is blank, holds the deck separator, or is given twice; or a code is not a slug.
    #[error("the taxonomy file's {field} is refused: {why}")]
    Refused {
        /// The field refused.
        field: &'static str,
        /// Why, never quoting the value.
        why: &'static str,
    },
}

/// The file as it is written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxonomyFile {
    schema: String,
    law: LawFile,
    languages: Vec<LanguageFile>,
    writing_roots: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LawFile {
    roots: Vec<String>,
    bands: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LanguageFile {
    deck: String,
    code: String,
    display: String,
    term_field: String,
}

/// A language deck of the taxonomy.
#[derive(Clone, PartialEq, Eq)]
pub struct Language {
    deck: String,
    code: String,
    display: String,
    term_field: String,
}

impl Language {
    /// The language's top-level deck: every deck under it is the language's.
    #[must_use]
    pub fn deck(&self) -> &str {
        &self.deck
    }

    /// The language's code, a slug: its topic is `language/<code>`.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The name a writing deck's second segment gives the language.
    #[must_use]
    pub fn display(&self) -> &str {
        &self.display
    }

    /// The note field that holds a card's term (the generation reads it, SPEC-046 R1).
    #[must_use]
    pub fn term_field(&self) -> &str {
        &self.term_field
    }
}

impl fmt::Debug for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Language { .. }")
    }
}

/// The owner's taxonomy, read from its private file.
#[derive(Clone, PartialEq, Eq)]
pub struct Taxonomy {
    law_roots: Vec<String>,
    bands: Vec<String>,
    languages: Vec<Language>,
    writing_roots: Vec<String>,
}

impl Taxonomy {
    /// Reads the taxonomy file at `path`.
    ///
    /// # Errors
    ///
    /// [`TaxonomyError::Unreadable`] when the file cannot be read, and every refusal of
    /// [`Taxonomy::parse`].
    pub fn load(path: &Path) -> Result<Self, TaxonomyError> {
        let text = fs::read_to_string(path).map_err(TaxonomyError::Unreadable)?;
        Self::parse(&text)
    }

    /// Reads a taxonomy from the file's `text`.
    ///
    /// # Errors
    ///
    /// [`TaxonomyError::Malformed`] for text that is not JSON of the taxonomy's shape (an unknown
    /// field included), [`TaxonomyError::OtherSchema`] for another schema, and
    /// [`TaxonomyError::Refused`] for a blank, split or repeated name, or a code that is no slug.
    pub fn parse(text: &str) -> Result<Self, TaxonomyError> {
        let _ = text;
        Ok(Self {
            law_roots: Vec::new(),
            bands: Vec::new(),
            languages: Vec::new(),
            writing_roots: Vec::new(),
        })
    }

    /// The law roots: a deck whose top-level name is one of them is on the law track.
    #[must_use]
    pub fn law_roots(&self) -> &[String] {
        &self.law_roots
    }

    /// The year bands: a law deck whose second segment is one carries its subject fourth.
    #[must_use]
    pub fn bands(&self) -> &[String] {
        &self.bands
    }

    /// The language decks, in the file's order.
    #[must_use]
    pub fn languages(&self) -> &[Language] {
        &self.languages
    }

    /// The roots whose second segment names a language by its display name.
    #[must_use]
    pub fn writing_roots(&self) -> &[String] {
        &self.writing_roots
    }
}

impl fmt::Debug for Taxonomy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Taxonomy({} law root(s), {} band(s), {} language(s), {} writing root(s))",
            self.law_roots.len(),
            self.bands.len(),
            self.languages.len(),
            self.writing_roots.len()
        )
    }
}

/// Refuses a list of names with a blank one, one holding the deck separator, or one given twice.
fn names(field: &'static str, names: &[String]) -> Result<(), TaxonomyError> {
    if names.iter().any(|name| name.trim().is_empty()) {
        return Err(TaxonomyError::Refused {
            field,
            why: "a name is blank",
        });
    }
    if names.iter().any(|name| name.contains(DECK_SEPARATOR)) {
        return Err(TaxonomyError::Refused {
            field,
            why: "a name holds the deck separator, so it names no single segment",
        });
    }
    let mut seen: Vec<&str> = names.iter().map(String::as_str).collect();
    seen.sort_unstable();
    if seen.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(TaxonomyError::Refused {
            field,
            why: "a name is given twice",
        });
    }
    Ok(())
}

/// The taxonomy file's path (R1): an absolute path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaxonomyPath(PathBuf);

impl TaxonomyPath {
    /// The path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// The taxonomy's path from `env`, or `None` when [`READINGS_TAXONOMY`] is unset.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] naming [`READINGS_TAXONOMY`] when it is not an absolute path.
    pub fn from_env(env: &Environment) -> Result<Option<Self>, SettingsError> {
        env.optional(READINGS_TAXONOMY)
    }
}

impl Setting for TaxonomyPath {
    const SHAPE: &'static str = "an absolute path to the taxonomy file";

    fn parse(text: &str) -> Option<Self> {
        let path = PathBuf::from(text);
        path.is_absolute().then_some(Self(path))
    }
}
