//! The parity oracle's one golden reader (ADR-029).
//!
//! A golden is what the predecessor's own function returned over seeded synthetic inputs, written
//! by `tools/parity-oracle/generate.py` in the schema `phx.parity-golden.v1`. This file is compiled
//! into a proving crate's integration tests, and never into production code:
//!
//! ```text
//! #[path = "../../../tools/parity-oracle/golden.rs"]
//! mod golden;
//! ```
//!
//! It is one file included by path, rather than a crate or a copy in each crate, so every crate
//! reads a golden one way and a change here is tested by every crate that includes it. A crate
//! that includes it adds `serde` and `serde_json` to its dev-dependencies, and nothing else.

// Each including crate calls only the part of the reader its tests need.
#![allow(dead_code)]

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

/// The one schema a golden may carry: a golden of any other is refused, never read loosely.
pub const SCHEMA: &str = "phx.parity-golden.v1";

/// One golden: the predecessor function it records, where it came from, and its cases.
///
/// Every field but `adapter` and `note` is required, so a golden that lost its provenance is
/// refused rather than read.
#[derive(Debug, Deserialize)]
pub struct Golden {
    /// Always [`SCHEMA`].
    pub schema: String,
    /// `function`, `adapter` or `constants`: how the generator reached the predecessor.
    pub kind: String,
    /// The predecessor function the cases came from, relative to its package, or a constants
    /// golden's own name.
    pub function: String,
    /// The predecessor's commit the golden was generated at.
    pub source_commit: String,
    /// The generator's path in this repository.
    pub generator: String,
    /// The generator's sha256 when it wrote this golden.
    pub generator_sha256: String,
    /// The registry module that registered this golden.
    pub registry: String,
    /// The registry module's sha256 when the generator loaded it.
    pub registry_sha256: String,
    /// Always `synthetic`: no case holds a row of anyone's data.
    pub inputs: String,
    /// The seed of the `random.Random` the case builder drew from.
    pub seed: u64,
    /// An adapter golden's glue, by name.
    pub adapter: Option<String>,
    /// An adapter golden's note on what its glue builds.
    pub note: Option<String>,
    /// At least one case.
    pub cases: Vec<Case>,
}

/// One case: the input the predecessor was called with, and the output it returned.
#[derive(Debug, Deserialize)]
pub struct Case {
    /// The case's input.
    pub input: Value,
    /// The predecessor's output: a day as its epoch day number, an instant as epoch milliseconds.
    pub output: Value,
    /// The boundary the case sits on (`rollover`, `negative`, `offset`, `tie`), if any.
    pub class: Option<String>,
}

/// Why a golden is refused.
#[derive(Debug)]
pub enum GoldenError {
    /// The file could not be read.
    Unreadable {
        /// The golden's path.
        path: PathBuf,
        /// The operating system's reason.
        reason: String,
    },
    /// The text is not JSON, or a field is missing or of the wrong type.
    Malformed {
        /// The golden's path.
        path: PathBuf,
        /// The parser's reason, naming the field.
        reason: String,
    },
    /// The golden names another schema, or none.
    Schema {
        /// The golden's path.
        path: PathBuf,
        /// The schema it names, as JSON, or `none`.
        found: String,
    },
    /// The golden holds no case, so it proves nothing.
    NoCase {
        /// The golden's path.
        path: PathBuf,
    },
}

impl fmt::Display for GoldenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, reason } => {
                write!(f, "{}: cannot be read: {reason}", path.display())
            }
            Self::Malformed { path, reason } => {
                write!(f, "{}: is not a golden: {reason}", path.display())
            }
            Self::Schema { path, found } => {
                write!(
                    f,
                    "{}: has schema {found}, not \"{SCHEMA}\"",
                    path.display()
                )
            }
            Self::NoCase { path } => {
                write!(f, "{}: holds no case, so it proves nothing", path.display())
            }
        }
    }
}

impl std::error::Error for GoldenError {}

/// How many cases of which predecessor function a test examined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Examined {
    /// The golden's `function`.
    pub function: String,
    /// The cases examined, never zero.
    pub count: usize,
}

impl fmt::Display for Examined {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "examined {} case(s) of {}", self.count, self.function)
    }
}

/// The schema alone, read first, so a golden of another shape is refused for its schema rather
/// than for a field that shape happens to lack.
#[derive(Deserialize)]
struct Header {
    schema: Option<Value>,
}

/// Parses a golden's text; `path` names it in a refusal.
///
/// # Errors
///
/// [`GoldenError::Malformed`] for text that is not JSON or lacks a field, [`GoldenError::Schema`]
/// for a golden of another schema, and [`GoldenError::NoCase`] for a golden with no case.
pub fn parse(path: &Path, text: &str) -> Result<Golden, GoldenError> {
    let malformed = |error: serde_json::Error| GoldenError::Malformed {
        path: path.to_path_buf(),
        reason: error.to_string(),
    };
    let header: Header = serde_json::from_str(text).map_err(malformed)?;
    if header.schema.as_ref().and_then(Value::as_str) != Some(SCHEMA) {
        return Err(GoldenError::Schema {
            path: path.to_path_buf(),
            found: header
                .schema
                .map_or_else(|| "none".to_owned(), |schema| schema.to_string()),
        });
    }
    let golden: Golden = serde_json::from_str(text).map_err(malformed)?;
    if golden.cases.is_empty() {
        return Err(GoldenError::NoCase {
            path: path.to_path_buf(),
        });
    }
    Ok(golden)
}

/// Reads the golden at `path`.
///
/// # Errors
///
/// [`GoldenError::Unreadable`] when the file cannot be read, and every refusal of [`parse`].
pub fn read(path: &Path) -> Result<Golden, GoldenError> {
    let text = fs::read_to_string(path).map_err(|error| GoldenError::Unreadable {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;
    parse(path, &text)
}

/// Where the committed golden `name` lives. Every crate sits at `crates/<context>/`, two levels
/// below the repository root, so the including crate's manifest directory finds it.
#[must_use]
pub fn committed(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/parity-oracle/goldens")
        .join(format!("{name}.json"))
}

/// Runs `check` over every case of the committed golden `name`, prints
/// `examined N case(s) of <function>`, and returns that report.
///
/// # Panics
///
/// When the golden is refused, or when it yields no case: an enumeration that examined nothing
/// proved nothing (the tdd pack).
#[allow(
    clippy::print_stdout,
    reason = "the examined count is the test's report (the tdd pack)"
)]
pub fn each_case(name: &str, mut check: impl FnMut(&Case)) -> Examined {
    let golden = read(&committed(name)).unwrap_or_else(|refusal| panic!("{refusal}"));
    for case in &golden.cases {
        check(case);
    }
    let examined = Examined {
        function: golden.function,
        count: golden.cases.len(),
    };
    println!("{examined}");
    assert!(
        examined.count > 0,
        "{examined}: a golden that yields no case proves nothing"
    );
    examined
}
