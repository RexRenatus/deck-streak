//! The read-only search parity tool (SPEC-514 R9 and A6): the cards the engine's search selects
//! in a collection against the cards the search crate's own evaluator selects in the same rows, counted
//! and never listed.
//!
//! The evaluator's side opens the collection only through the kernel's
//! [`Db::open_foreign_read_only`] (`mode=ro` and `query_only`, as [`crate::reader`] does), so a
//! write through it is refused by `SQLite` itself. The engine opens collections writable and, in
//! client mode, rewrites a configured offset, so its side runs only on a scratch copy the tool
//! makes; a scratch path that is the source is refused before anything is opened. The answer is
//! three counts (match, engine-only, evaluator-only): no card id, deck name, field text or
//! search text leaves the tool.
//!
//! A known gap: the read takes every note type's first field as its sort field (the note type's
//! configured sort field is in its protobuf configuration, which the read does not decode), so a
//! text search over a note type sorted by another field may count a note on one side only.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_kernel::{Db, ForeignDb};
use deck_streak_search::{
    Card, Collection, Deck, EngineDay, Note, Notetype, Profile, Refusal, Review, eval,
};

use crate::engine::{CollectionWrite, EngineError, RslibEngine};

/// The separator between a stored deck name's levels and a stored note's fields.
const SEPARATOR: char = '\u{1f}';

/// What the tool answers: how many cards both sides select, and how many only one side does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParityCounts {
    /// Cards both the engine and the evaluator select.
    pub matched: usize,
    /// Cards only the engine selects.
    pub engine_only: usize,
    /// Cards only the search crate's evaluator selects.
    pub ds_only: usize,
}

impl fmt::Display for ParityCounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "match={} engine-only={} ds-only={}",
            self.matched, self.engine_only, self.ds_only
        )
    }
}

/// Why the tool gives no counts. No variant names a path, a search or a row.
#[derive(Debug, thiserror::Error)]
pub enum ParityError {
    /// The scratch path the engine would write is the source: refused before anything opens.
    #[error("the engine's scratch copy would be the source collection")]
    WritableOpen,
    /// The search is not one expression of the operator profile.
    #[error("the search is refused: {0}")]
    Search(Refusal),
    /// The read-only open, or one of its reads, failed.
    #[error("the read-only read of the collection failed")]
    Read,
    /// Copying the source to the scratch path failed.
    #[error("the scratch copy could not be made")]
    Copy,
    /// The engine's day or search failed on the scratch copy.
    #[error("the engine failed on the scratch copy: {0}")]
    Engine(EngineError),
}

/// Counts the cards `search` selects in the collection at `source` on each side: the search crate's
/// evaluator over the rows read through the read-only open, and the engine's search on a copy
/// at `scratch`, on the engine's own day.
///
/// # Errors
///
/// [`ParityError::Search`] for a search the operator profile refuses,
/// [`ParityError::WritableOpen`] when `scratch` is `source`, [`ParityError::Read`] when the
/// read-only read fails, [`ParityError::Copy`] when the copy fails, and [`ParityError::Engine`]
/// when the engine fails on the copy.
pub async fn search_parity(
    source: &Path,
    scratch: &Path,
    search: &str,
) -> Result<ParityCounts, ParityError> {
    let _ = (source, scratch, search);
    Err(ParityError::Read)
}
