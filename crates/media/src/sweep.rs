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

/// One file's fate: kept for the first reason that holds, live, pinned, in grace or imported, and
/// deleted when none does.
#[must_use]
pub fn fate(facts: Facts, policy: SweepPolicy) -> Fate {
    if facts.referenced {
        Fate::Keep(Keep::Live)
    } else if facts.pinned {
        Fate::Keep(Keep::Pinned)
    } else if in_grace(facts.added_ms, policy) {
        Fate::Keep(Keep::InGrace)
    } else if facts.imported {
        Fate::Keep(Keep::Imported)
    } else {
        Fate::Delete
    }
}

/// Whether a file added at `added_ms` is within the grace window at the sweep's instant: added at
/// or after the instant, or less than `grace_ms` before it.
#[must_use]
pub fn in_grace(added_ms: i64, policy: SweepPolicy) -> bool {
    if added_ms >= policy.now_ms {
        return true;
    }
    let age = i128::from(policy.now_ms) - i128::from(added_ms);
    let grace = i128::from(policy.grace_ms);
    // The witness's mutant: the sweep reads no grace window.
    #[cfg(feature = "kani-witness-sweep")]
    let grace = 0;
    age < grace
}

/// Whether `name` is pinned: it starts with `_`.
#[must_use]
pub fn pinned(name: &str) -> bool {
    name.starts_with(PINNED_PREFIX)
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

/// The sweep plan for the files `stored` against the names `referenced`, at `policy`'s instant.
#[must_use]
pub fn plan_sweep(
    stored: &[StoredFile],
    referenced: &BTreeSet<String>,
    policy: SweepPolicy,
) -> SweepPlan {
    let mut files: Vec<&StoredFile> = stored.iter().collect();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    let mut plan = SweepPlan::default();
    for file in files {
        let facts = Facts {
            referenced: referenced.contains(&file.name),
            pinned: pinned(&file.name),
            imported: file.imported,
            added_ms: file.added_ms,
        };
        match fate(facts, policy) {
            Fate::Keep(reason) => plan.kept.push((file.name.clone(), reason)),
            Fate::Delete => plan.delete.push(file.name.clone()),
        }
    }
    let held: BTreeSet<&str> = stored.iter().map(|file| file.name.as_str()).collect();
    plan.missing = referenced
        .iter()
        .filter(|name| !held.contains(name.as_str()))
        .cloned()
        .collect();
    plan
}
