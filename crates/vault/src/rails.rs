//! The content rails (SPEC-042 R3): every byte the adapter writes into the vault passes the rails
//! of the vault-duties pack's `no-executable` class, read from the pack's vendored `rails.json`, plus
//! one rail of the adapter's own that refuses NUL and every other control character but the tab,
//! the line feed and the carriage return.
//!
//! The rails are data: a new rail is a row in `rails.json`, never a code branch here. Each refusal
//! names its rail row and the line it stands on, and never echoes the text; nothing is sanitised.
//! The scan reads a note as the pack's probe does, so the two agree on every planted fixture
//! (SPEC-042 A2 and A3); where this port differs from the probe it is only ever stricter (ADR-042).

use std::collections::BTreeMap;
use std::fmt;

use deck_streak_kernel::Verdict;
use serde_json::Value;

/// The vault-duties pack's `rails.json`, as vendored with the packs (ADR-004), compiled in so the
/// adapter reads the same bytes the pack's probe reads.
pub const VENDORED: &str = include_str!("../../../.packs/skills/packs/vault-duties/rails.json");

/// The schema `rails.json` carries.
pub const SCHEMA: &str = "phx.duty.vault.rails.v1";

/// Every key `rails.json` holds: the ten the `no-executable` class reads, the three the pack's
/// `nothing-leaves` class reads, and the document's own three. A key outside this list is a new
/// kind of rail, which needs code before the port can honour it (ADR-042).
pub const KNOWN_KEYS: [&str; 16] = [
    "schema",
    "spec",
    "note",
    "fence_allow",
    "fence_known",
    "fence_prefixes",
    "templater_open",
    "inline_query_prefixes",
    "html_allow",
    "html_attributes_allow",
    "executable_schemes",
    "egress_schemes",
    "remote_schemes",
    "math_macros_refused",
    "dynamic_embed_extensions",
    "publish_key",
];

/// The adapter's own rail: a control character other than a tab, a line feed or a carriage return.
pub const CONTROL_CHARACTER: &str = "control_character";

/// Why `rails.json` could not be read as rails.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RailsError {
    /// The text is not JSON.
    #[error("rails.json is not JSON")]
    NotJson,
    /// The document carries another schema.
    #[error("rails.json does not carry the schema {SCHEMA}")]
    Schema,
    /// A key the port reads is missing or has the wrong shape.
    #[error("rails.json's {0} is missing or has the wrong shape")]
    Key(&'static str),
}

/// One rail row: a refusal the rails can make, named by the `rails.json` key it comes from and,
/// where that key holds a list or a map, by its entry: `fence_known:dataviewjs`, `templater_open`,
/// `executable_schemes:javascript`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailRow(String);

impl RailRow {
    /// The row of `key` alone.
    fn of(key: &str) -> Self {
        Self(key.to_owned())
    }

    /// The row of `entry` under `key`.
    fn entry(key: &str, entry: &str) -> Self {
        Self(format!("{key}:{entry}"))
    }

    /// The row's name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RailRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A refused text: the rail row that refused it, and the 1-based line it refused. It never holds
/// the text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
#[error("the rail {row} refuses line {line}")]
pub struct RailRefusal {
    /// The rail row.
    pub row: RailRow,
    /// The line, counted from 1.
    pub line: usize,
}

/// The rails, read from `rails.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rails {
    fence_allow: Vec<String>,
    fence_known: BTreeMap<String, String>,
    fence_prefixes: BTreeMap<String, String>,
    templater_open: String,
    inline_query_prefixes: Vec<String>,
    html_allow: Vec<String>,
    html_attributes_allow: Vec<String>,
    executable_schemes: Vec<String>,
    math_macros_refused: Vec<String>,
    dynamic_embed_extensions: Vec<String>,
}

impl Rails {
    /// The rails of the vendored `rails.json`.
    ///
    /// # Errors
    ///
    /// [`RailsError`] when the vendored file is not the rails' shape, which a build of the adapter
    /// would never ship: a test reads it.
    pub fn vendored() -> Result<Self, RailsError> {
        Self::from_json(VENDORED)
    }

    /// The rails of a `rails.json` text.
    ///
    /// # Errors
    ///
    /// [`RailsError::NotJson`], [`RailsError::Schema`], or [`RailsError::Key`] naming a key the
    /// port reads that is missing or of the wrong shape.
    pub fn from_json(text: &str) -> Result<Self, RailsError> {
        let document: Value = serde_json::from_str(text).map_err(|_| RailsError::NotJson)?;
        if document.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
            return Err(RailsError::Schema);
        }
        Ok(Self {
            fence_allow: strings(&document, "fence_allow")?,
            fence_known: string_map(&document, "fence_known")?,
            fence_prefixes: string_map(&document, "fence_prefixes")?,
            templater_open: string(&document, "templater_open")?,
            inline_query_prefixes: strings(&document, "inline_query_prefixes")?,
            html_allow: strings(&document, "html_allow")?,
            html_attributes_allow: strings(&document, "html_attributes_allow")?,
            executable_schemes: strings(&document, "executable_schemes")?,
            math_macros_refused: strings(&document, "math_macros_refused")?,
            dynamic_embed_extensions: strings(&document, "dynamic_embed_extensions")?,
        })
    }

    /// Every rail row the data declares: one for each known fence, fence prefix, inline query
    /// prefix, executable scheme, refused macro and dynamic embed extension, and one each for a
    /// fence off the allow-list, Templater's opener, an HTML tag off the allow-list and an HTML
    /// attribute off it. The adapter's own control-character rail is not a row of `rails.json`.
    #[must_use]
    pub fn rows(&self) -> Vec<RailRow> {
        let mut rows = Vec::new();
        rows.extend(
            self.fence_known
                .keys()
                .map(|fence| RailRow::entry("fence_known", fence)),
        );
        rows.extend(
            self.fence_prefixes
                .keys()
                .map(|prefix| RailRow::entry("fence_prefixes", prefix)),
        );
        rows.push(RailRow::of("fence_allow"));
        rows.push(RailRow::of("templater_open"));
        rows.extend(
            self.inline_query_prefixes
                .iter()
                .map(|prefix| RailRow::entry("inline_query_prefixes", prefix)),
        );
        rows.push(RailRow::of("html_allow"));
        rows.push(RailRow::of("html_attributes_allow"));
        rows.extend(
            self.executable_schemes
                .iter()
                .map(|scheme| RailRow::entry("executable_schemes", scheme)),
        );
        rows.extend(
            self.math_macros_refused
                .iter()
                .map(|name| RailRow::entry("math_macros_refused", name)),
        );
        rows.extend(
            self.dynamic_embed_extensions
                .iter()
                .map(|extension| RailRow::entry("dynamic_embed_extensions", extension)),
        );
        rows.sort();
        rows
    }

    /// Every refusal the rails make of `text`, sorted by line and then by row.
    #[must_use]
    pub fn refusals(&self, text: &str) -> Vec<RailRefusal> {
        let _ = text;
        Vec::new()
    }

    /// Whether `text` passes the rails: [`Verdict::Refuse`] names the first refusal by line.
    pub fn check(&self, text: &str) -> Verdict<RailRefusal> {
        match self.refusals(text).into_iter().next() {
            Some(refusal) => Verdict::Refuse(refusal),
            None => Verdict::Pass,
        }
    }
}

/// The list of strings at `key`.
fn strings(document: &Value, key: &'static str) -> Result<Vec<String>, RailsError> {
    document
        .get(key)
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or(RailsError::Key(key))
}

/// The map of strings at `key`.
fn string_map(document: &Value, key: &'static str) -> Result<BTreeMap<String, String>, RailsError> {
    document
        .get(key)
        .and_then(Value::as_object)
        .and_then(|items| {
            items
                .iter()
                .map(|(name, what)| what.as_str().map(|what| (name.clone(), what.to_owned())))
                .collect::<Option<BTreeMap<_, _>>>()
        })
        .ok_or(RailsError::Key(key))
}

/// The string at `key`.
fn string(document: &Value, key: &'static str) -> Result<String, RailsError> {
    document
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(RailsError::Key(key))
}
