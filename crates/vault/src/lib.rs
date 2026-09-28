//! # deck-streak-vault
//!
//! What this context owns: The anti-corruption layer for the owner's second-brain vault: the
//! versioned file contract, configured paths, atomic writes under ignored temp names, the
//! content rails that block executable Obsidian content, staged duty runs, the stats bridge,
//! drills and inbox capture.
//!
//! What it does not own: The vault's sync infrastructure, guards and backups, which stay in
//! private operations.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
//!
//! SPEC-042 builds the core: the configured paths and the start check ([`config`]), the file-system
//! port ([`fs`]), the atomic write ([`atomic`]), the content rails read from the vault-duties pack's
//! vendored `rails.json` ([`rails`]), the reading note's text ([`note`]), the readings date tree
//! ([`readings_tree`]) and the staged duty run's three-verb executor ([`staged`]). Every write
//! passes the rails, stays inside its confined folder, never overwrites a note and never deletes a
//! user file (ADR-042).
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

use std::io;

pub mod atomic;
pub mod config;
pub mod fs;
pub mod note;
pub mod rails;
pub mod readings_tree;
pub mod sha256;
pub mod staged;

pub use config::{FolderName, StartRefusal, VaultPaths, VaultRoot, VaultSettings};
pub use fs::{DirEntry, EntryKind, RealFs, VaultFile, VaultFs};
pub use note::{BodyHash, BoxLine, Malformed, TopicKey};
pub use rails::{RailRefusal, RailRow, Rails, RailsError};
pub use readings_tree::{
    BoxOutcome, Created, ReadingsTree, RollFailure, RollFailureReason, RollReport,
};
pub use staged::{
    Discard, DutyRun, Executor, GateError, Op, ProbeGate, RedClass, RunGate, RunOutcome, RunRefusal,
};

/// Why the vault adapter refused an operation, or could not complete it.
///
/// No variant carries a note's text or a configured path: a refusal names the rule, the rail or
/// the step, so an error can be logged without leaking the vault's contents or the host's paths.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    /// The vault features refuse to start (SPEC-042 R1).
    #[error("the vault features refuse to start: {0}")]
    Start(#[from] StartRefusal),
    /// A topic key is not a lowercase slug of `a-z`, `0-9`, `-` and `/` (R5).
    #[error("the topic key is not a lowercase slug of a-z, 0-9, - and /")]
    InvalidTopicKey,
    /// A frontmatter value holds a newline, a carriage return or a `]` (R5).
    #[error("the {field} value holds a newline, a carriage return or ]")]
    InvalidFrontmatterValue {
        /// The frontmatter key whose value was refused.
        field: &'static str,
    },
    /// A reading's body is empty, or holds a box line of its own, which would make a box's anchor
    /// ambiguous (R6, R9).
    #[error("the reading's body {0}")]
    InvalidBody(&'static str),
    /// The content rails refuse the bytes (R3): the refusal names the rail and the line, never the
    /// text.
    #[error(transparent)]
    Rails(#[from] RailRefusal),
    /// The write's resolved path, after `..` and symbolic links, leaves the folder it is confined
    /// to (R11).
    #[error("the write's resolved path leaves the folder it is confined to")]
    OutsideConfinement,
    /// A note already exists at the target, compared case-insensitively (R4, R6, R7, R8).
    #[error("a note already exists at the target")]
    NoteExists,
    /// No note exists where the operation needs one.
    #[error("no note exists at the target")]
    NoteMissing,
    /// The target is a symbolic link or not a regular file, which the adapter never reads or writes
    /// through (R11).
    #[error("the target is not a regular file")]
    NotARegularFile,
    /// A path the adapter needs as a folder is something else.
    #[error("a path the adapter needs as a folder is not one")]
    NotAFolder,
    /// A note is malformed (R8).
    #[error("the note is malformed: {0}")]
    Malformed(#[from] Malformed),
    /// The box line is missing: `box_anchor_missing` (R9).
    #[error("box_anchor_missing")]
    BoxAnchorMissing,
    /// The box line appears more than once: `box_anchor_ambiguous` (R9).
    #[error("box_anchor_ambiguous")]
    BoxAnchorAmbiguous,
    /// The note's body is not the body the adapter last wrote: `vault_note_edited` (R10).
    #[error("vault_note_edited")]
    NoteEdited,
    /// A rolled note did not read back as it was written, so its source stays (R7).
    #[error("the rolled note did not read back as written, so its source stays")]
    ReadBack,
    /// A file-system step failed; `step` names it.
    #[error("the vault could not {step}")]
    Io {
        /// The step that failed.
        step: &'static str,
        /// The operating system's reason.
        #[source]
        source: io::Error,
    },
}

impl VaultError {
    /// The error of the file-system step `step`, for `map_err`.
    pub(crate) fn io(step: &'static str) -> impl FnOnce(io::Error) -> Self {
        move |source| Self::Io { step, source }
    }
}
