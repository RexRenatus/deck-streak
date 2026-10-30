//! The sweep plan: which stored files a sweep deletes, and why each other one stays (SPEC-513 R6;
//! ADR-531 D6).
//!
//! A file stays when a note, a template or a CSS names it (live), when its name starts with `_`
//! (pinned: a template or a CSS may load it by a name no scan sees, as the engine's own media
//! check keeps it), when it was added within the grace window (in grace: a note that names it may
//! not be saved yet), or when an import brought it (imported: its name is the deck's, kept as it
//! was written). Every other file is deleted. The plan also lists every name a note writes that
//! no stored file has (missing).

use std::collections::BTreeSet;

/// The first character of a pinned name.
pub const PINNED_PREFIX: char = '_';

/// One file the media store holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFile {
    /// The file's name.
    pub name: String,
    /// When the file was added, in milliseconds since the Unix epoch.
    pub added_ms: i64,
    /// Whether an import brought the file.
    pub imported: bool,
}

/// When a sweep runs and how long a new file is kept unnamed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SweepPolicy {
    /// The sweep's instant, in milliseconds since the Unix epoch.
    pub now_ms: i64,
    /// How long after it is added a file stays though nothing names it, in milliseconds.
    pub grace_ms: u64,
}

/// Why a file stays, in the order the sweep asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Keep {
    /// A note, a template or a CSS names it.
    Live,
    /// Its name starts with `_`.
    Pinned,
    /// It was added within the grace window, or after the sweep's instant.
    InGrace,
    /// An import brought it.
    Imported,
}

/// What a sweep does with one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// The file stays, for the first reason that holds.
    Keep(Keep),
    /// The file is deleted.
    Delete,
}

/// What the sweep knows of one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    /// Whether a note, a template or a CSS names it.
    pub referenced: bool,
    /// Whether its name starts with `_`.
    pub pinned: bool,
    /// Whether an import brought it.
    pub imported: bool,
    /// When it was added, in milliseconds since the Unix epoch.
    pub added_ms: i64,
}

/// One file's fate. STUB: every file is kept as live.
#[must_use]
pub fn fate(_facts: Facts, _policy: SweepPolicy) -> Fate {
    Fate::Keep(Keep::Live)
}

/// Whether a file added at `added_ms` is within the grace window. STUB: always.
#[must_use]
pub fn in_grace(_added_ms: i64, _policy: SweepPolicy) -> bool {
    true
}

/// Whether `name` is pinned. STUB: never.
#[must_use]
pub fn pinned(_name: &str) -> bool {
    false
}

/// A sweep's plan: the names it deletes, each name it keeps with its reason, and each name a note
/// writes that no stored file has. Each list is in name order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SweepPlan {
    /// The files the sweep deletes.
    pub delete: Vec<String>,
    /// The files that stay, each with the first reason that holds.
    pub kept: Vec<(String, Keep)>,
    /// The names something writes that no stored file has.
    pub missing: Vec<String>,
}

/// The sweep plan for the files `stored` against the names `referenced`. STUB: keeps every file
/// as live and lists nothing missing.
#[must_use]
pub fn plan_sweep(
    stored: &[StoredFile],
    _referenced: &BTreeSet<String>,
    _policy: SweepPolicy,
) -> SweepPlan {
    SweepPlan {
        delete: Vec::new(),
        kept: stored
            .iter()
            .map(|file| (file.name.clone(), Keep::Live))
            .collect(),
        missing: Vec::new(),
    }
}
