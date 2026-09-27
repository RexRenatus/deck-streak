//! The parity oracle's one golden reader (ADR-029). This is the red-first stub: it has the reader's
//! surface and accepts every golden, so SPEC-029's tests compile and fail by assertion.
#![allow(dead_code)]

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

/// The one schema a golden may carry.
pub const SCHEMA: &str = "phx.parity-golden.v1";

/// One golden, as the generator writes it.
#[derive(Debug, Default, Deserialize)]
pub struct Golden {
    pub schema: String,
    pub kind: String,
    pub function: String,
    pub source_commit: String,
    pub generator: String,
    pub generator_sha256: String,
    pub registry: String,
    pub registry_sha256: String,
    pub inputs: String,
    pub seed: u64,
    pub adapter: Option<String>,
    pub note: Option<String>,
    pub cases: Vec<Case>,
}

/// One case of a golden.
#[derive(Debug, Deserialize)]
pub struct Case {
    pub input: Value,
    pub output: Value,
    pub class: Option<String>,
}

/// Why a golden is refused.
#[derive(Debug)]
pub enum GoldenError {
    Unreadable { path: PathBuf, reason: String },
    Malformed { path: PathBuf, reason: String },
    Schema { path: PathBuf, found: String },
    NoCase { path: PathBuf },
}

/// How many cases of which function a test examined.
pub struct Examined {
    pub function: String,
    pub count: usize,
}

impl fmt::Display for Examined {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "examined {} case(s) of {}", self.count, self.function)
    }
}

/// Parses a golden's text.
pub fn parse(path: &Path, text: &str) -> Result<Golden, GoldenError> {
    let _ = (path, text);
    Ok(Golden::default())
}

/// Reads a golden from a file.
pub fn read(path: &Path) -> Result<Golden, GoldenError> {
    parse(path, "")
}

/// Runs `check` over every case of the named committed golden.
pub fn each_case(name: &str, check: impl FnMut(&Case)) -> Examined {
    let _ = (name, check);
    Examined {
        function: String::new(),
        count: 0,
    }
}
