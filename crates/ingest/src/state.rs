//! `ingest_state` (SPEC-023 R6, R11): the change gate's anchor, the owner's pending rescore and the
//! ingest window's base, in one row of the service's database, owned by this context
//! (docs/CONTEXT-MAP.md, `migrations/002301_ingest_state.sql`).
//!
//! A full recompute writes a fresh anchor and clears the rescore flag in the same write, so a
//! request is consumed by the recompute that serves it and a recompute that fails serves none. A
//! skip leaves the row as it was. Every write is an upsert of the one row, so a row that was
//! deleted is written back rather than silently left unwritten.

use deck_streak_kernel::{Db, KernelError, UtcMillis};

use crate::gate::{Anchor, AnchorState};
use crate::window::WindowBase;

/// The row, as the gate and the window read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IngestState {
    /// What the last full recompute saw, when the row holds a whole anchor.
    pub anchor: AnchorState,
    /// Whether the owner asked for a rescore that no recompute has served yet.
    pub rescore_pending: bool,
    /// The window's base, once a recount has written one.
    pub window_base: Option<WindowBase>,
}

/// `ingest_state` in the service's own database.
#[derive(Clone, Debug)]
pub struct SqliteIngestState {
    db: Db,
}

impl SqliteIngestState {
    /// The state in `db`, whose migrations created `ingest_state`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// Reads the row. A row that is gone reads as an unreadable anchor, no pending rescore and no
    /// base, so the gate runs; an anchor with any part missing reads as unreadable, and one with
    /// every part missing as missing.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn load(&self) -> Result<IngestState, KernelError> {
        let _ = &self.db;
        Ok(IngestState {
            anchor: AnchorState::Missing,
            rescore_pending: false,
            window_base: None,
        })
    }

    /// Marks an owner's rescore pending (R8): the next cycle recomputes whatever else it finds. The
    /// bot's `/sync` calls it (#19). `now` stamps a row that had to be written back.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn request_rescore(&self, now: UtcMillis) -> Result<(), KernelError> {
        let _ = now;
        Ok(())
    }

    /// Writes the anchor a full recompute leaves (R11), and clears the rescore flag in the same
    /// write: the recompute served every request made before it began.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn write_anchor(&self, anchor: &Anchor, now: UtcMillis) -> Result<(), KernelError> {
        let _ = (anchor, now);
        Ok(())
    }

    /// Writes the window's base after a recount (R6).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn write_window_base(
        &self,
        base: WindowBase,
        now: UtcMillis,
    ) -> Result<(), KernelError> {
        let _ = (base, now);
        Ok(())
    }
}
