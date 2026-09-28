//! `ingest_state` (SPEC-023 R6, R11): the change gate's anchor, the owner's pending rescore and the
//! ingest window's base, in one row of the service's database, owned by this context
//! (docs/CONTEXT-MAP.md, `migrations/002301_ingest_state.sql`).
//!
//! A full recompute writes a fresh anchor and clears the rescore flag in the same write, so a
//! request is consumed by the recompute that serves it and a recompute that fails serves none. A
//! skip leaves the row as it was. Every write is an upsert of the one row, so a row that was
//! deleted is written back rather than silently left unwritten.

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};

use crate::gate::{Anchor, AnchorState, Probe};
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
        let row = sqlx::query!(
            r#"SELECT anchor_newest_review_id AS "anchor_newest_review_id?",
                      anchor_card_count AS "anchor_card_count?",
                      anchor_card_fingerprint AS "anchor_card_fingerprint?",
                      anchor_study_day AS "anchor_study_day?",
                      anchor_recomputed_at AS "anchor_recomputed_at?",
                      anchor_settings_generation AS "anchor_settings_generation?",
                      rescore_pending AS "rescore_pending!",
                      window_floor AS "window_floor?",
                      window_count AS "window_count?"
               FROM ingest_state WHERE id = 1"#
        )
        .fetch_optional(self.db.reader())
        .await?;
        let Some(row) = row else {
            return Ok(IngestState {
                anchor: AnchorState::Unreadable,
                rescore_pending: false,
                window_base: None,
            });
        };
        let anchor = match (
            row.anchor_newest_review_id,
            row.anchor_card_count,
            row.anchor_card_fingerprint,
            row.anchor_study_day,
            row.anchor_recomputed_at,
            row.anchor_settings_generation,
        ) {
            (
                Some(newest_review_id),
                Some(card_count),
                Some(card_fingerprint),
                Some(study_day),
                Some(recomputed_at),
                Some(settings_generation),
            ) => AnchorState::Present(Anchor {
                probe: Probe {
                    newest_review_id,
                    card_count,
                    card_fingerprint,
                },
                study_day: StudyDay::from_epoch_day(study_day),
                recomputed_at: UtcMillis::from_epoch_millis(recomputed_at),
                settings_generation,
            }),
            (None, None, None, None, None, None) => AnchorState::Missing,
            _ => AnchorState::Unreadable,
        };
        let window_base = row
            .window_floor
            .zip(row.window_count)
            .map(|(floor, count)| WindowBase { floor, count });
        Ok(IngestState {
            anchor,
            rescore_pending: row.rescore_pending != 0,
            window_base,
        })
    }

    /// Marks an owner's rescore pending (R8): the next cycle recomputes whatever else it finds. The
    /// bot's `/sync` calls it (#19). `now` stamps a row that had to be written back.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn request_rescore(&self, now: UtcMillis) -> Result<(), KernelError> {
        let created_at = now.epoch_millis();
        let mut write = self.db.write().await?;
        sqlx::query!(
            "INSERT INTO ingest_state (id, rescore_pending, created_at) VALUES (1, 1, ?1) \
             ON CONFLICT (id) DO UPDATE SET rescore_pending = 1",
            created_at
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
        Ok(())
    }

    /// Writes the anchor a full recompute leaves (R11), and clears the rescore flag in the same
    /// write: the recompute served every request made before it began.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn write_anchor(&self, anchor: &Anchor, now: UtcMillis) -> Result<(), KernelError> {
        let newest_review_id = anchor.probe.newest_review_id;
        let card_count = anchor.probe.card_count;
        let card_fingerprint = anchor.probe.card_fingerprint;
        let study_day = anchor.study_day.epoch_day();
        let recomputed_at = anchor.recomputed_at.epoch_millis();
        let settings_generation = anchor.settings_generation;
        let created_at = now.epoch_millis();
        let mut write = self.db.write().await?;
        sqlx::query!(
            "INSERT INTO ingest_state \
             (id, anchor_newest_review_id, anchor_card_count, anchor_card_fingerprint, \
              anchor_study_day, anchor_recomputed_at, anchor_settings_generation, rescore_pending, \
              created_at) \
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, 0, ?7) \
             ON CONFLICT (id) DO UPDATE SET \
             anchor_newest_review_id = excluded.anchor_newest_review_id, \
             anchor_card_count = excluded.anchor_card_count, \
             anchor_card_fingerprint = excluded.anchor_card_fingerprint, \
             anchor_study_day = excluded.anchor_study_day, \
             anchor_recomputed_at = excluded.anchor_recomputed_at, \
             anchor_settings_generation = excluded.anchor_settings_generation, \
             rescore_pending = 0",
            newest_review_id,
            card_count,
            card_fingerprint,
            study_day,
            recomputed_at,
            settings_generation,
            created_at
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
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
        let created_at = now.epoch_millis();
        let mut write = self.db.write().await?;
        sqlx::query!(
            "INSERT INTO ingest_state (id, rescore_pending, window_floor, window_count, created_at) \
             VALUES (1, 0, ?1, ?2, ?3) \
             ON CONFLICT (id) DO UPDATE SET window_floor = excluded.window_floor, \
             window_count = excluded.window_count",
            base.floor,
            base.count,
            created_at
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
        Ok(())
    }
}
