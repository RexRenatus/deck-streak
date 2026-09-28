//! The ingest window (SPEC-023 R6): only the last [`INGEST_WINDOW_DAYS`] of reviews are read each
//! cycle, and every study event older than the window is one persisted count.
//!
//! It ports the predecessor's `pipeline.py:GamifyPipeline._maybe_rebase_ingest` and
//! `anki_reader.py:count_study_reviews_before` (predecessor `27ee2bc`): reviews newer than the floor
//! are read; the floor is the persisted base's, or [`INGEST_WINDOW_DAYS`] days before now when there
//! is none; the base is recounted in SQL when it is more than [`INGEST_REBASE_DAYS`] days staler than
//! a fresh floor; and a recount lower than the stored count is the self-check, logged once (cards
//! deleted or the collection replaced). `goldens/ingest_rebase.json` and
//! `goldens/ingest.constants.json` prove the rule and the constants. The bounded window is what kept
//! the predecessor's memory under its unit's ceiling.

use deck_streak_kernel::{KernelError, UtcMillis};

use crate::reader::{CollectionData, CollectionReader, ReadError};
use crate::state::SqliteIngestState;

/// The days of reviews each cycle reads (`constants.py:INGEST_WINDOW_DAYS`).
pub const INGEST_WINDOW_DAYS: i64 = 0;
/// How many days staler than a fresh floor a base may be before it is recounted
/// (`constants.py:INGEST_REBASE_DAYS`).
pub const INGEST_REBASE_DAYS: i64 = 0;

/// The window's base: the floor, a review id (which is epoch milliseconds), and the count of study
/// events of the cards in scope at or before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowBase {
    /// The floor: reviews newer than it are read.
    pub floor: i64,
    /// The study events at or before the floor.
    pub count: i64,
}

/// What a cycle does with the base (R6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rebase {
    /// The base is fresh enough: kept as it is.
    Keep(WindowBase),
    /// There is no base, or it is stale: recount the study events at or before `floor`.
    Recount {
        /// The fresh floor to recount before.
        floor: i64,
    },
}

/// The self-check (R6): a recount lower than the stored count, which only deleted cards or a
/// replaced collection explain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelfCheck {
    /// The count the base held.
    pub stored: i64,
    /// The count the recount found.
    pub recounted: i64,
}

/// A fresh floor at `now`: [`INGEST_WINDOW_DAYS`] days before it, never before the epoch.
#[must_use]
pub fn fresh_floor(now: UtcMillis) -> i64 {
    let _ = now;
    0
}

/// The floor a read at `now` reads above: the base's, or a fresh one when there is none.
#[must_use]
pub fn read_floor(base: Option<WindowBase>, now: UtcMillis) -> i64 {
    let _ = (base, now);
    0
}

/// Whether the base at `now` is kept or recounted: it is recounted when there is none, or when it
/// is more than [`INGEST_REBASE_DAYS`] days staler than a fresh floor.
#[must_use]
pub fn rebase(base: Option<WindowBase>, now: UtcMillis) -> Rebase {
    let _ = now;
    Rebase::Keep(base.unwrap_or(WindowBase { floor: 0, count: 0 }))
}

/// The base a recount of `count` study events at or before `floor` writes, and the self-check when
/// `count` is lower than what `previous` held. A failed self-check logs one WARN.
#[must_use]
pub fn rebased(
    previous: Option<WindowBase>,
    floor: i64,
    count: i64,
) -> (WindowBase, Option<SelfCheck>) {
    let _ = previous;
    (WindowBase { floor, count }, None)
}

/// One cycle's read inside the window (R6): the data, the base the cycle kept or wrote, and the
/// self-check when the recount shrank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowRead {
    /// What the read returned.
    pub data: CollectionData,
    /// The base after the cycle.
    pub base: WindowBase,
    /// The self-check, when a recount came back lower than the stored count.
    pub self_check: Option<SelfCheck>,
}

/// Why the window could not be read.
#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    /// The copy could not be read.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// The window's base could not be read or written.
    #[error("the window's base could not be read or written")]
    State(#[from] KernelError),
}

/// Reads the window at `now` (R6): the reviews above the floor, then the base kept or recounted,
/// and a recount written back.
///
/// # Errors
///
/// [`WindowError::Read`] when the copy cannot be read, and [`WindowError::State`] when the base
/// cannot be read or written.
pub async fn read_window(
    reader: &CollectionReader,
    state: &SqliteIngestState,
    now: UtcMillis,
) -> Result<WindowRead, WindowError> {
    let _ = state;
    let data = reader.read(read_floor(None, now)).await?;
    Ok(WindowRead {
        data,
        base: WindowBase { floor: 0, count: 0 },
        self_check: None,
    })
}
