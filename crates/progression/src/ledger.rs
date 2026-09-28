//! The XP ledger (SPEC-040 R1 to R8): one `xp_ledger` row per grant, owned by this context
//! (docs/CONTEXT-MAP.md), written only through the grant port, and summed for the total and the
//! level each time they are read.
//!
//! The predecessor kept its grants in one ledger keyed by study day and source
//! (`database.py:GamifyStore.upsert_xp_grant` at `27ee2bc`). Here the key is the study day, the
//! source and the track, a `once` grant is held to one row per source and track across every study
//! day, and both rules are unique indexes of the table itself (ADR-040).

use deck_streak_kernel::{Db, KernelError, Track, UtcMillis};

use crate::grant::{GrantAnswer, GrantPort, GrantRequest};
use crate::xp::{Level, XpTotal, level_for};

/// The table the ledger lives in (`migrations/004001_progression_xp_ledger.sql`).
pub const XP_LEDGER_TABLE: &str = "xp_ledger";

/// `xp_ledger` in the service's own database: the grant port's one implementation.
#[derive(Clone, Debug)]
pub struct SqliteXpLedger {
    db: Db,
}

impl SqliteXpLedger {
    /// The ledger in `db`, whose migrations created `xp_ledger`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// The sum of every grant (R7).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn total(&self) -> Result<XpTotal, KernelError> {
        let total = sqlx::query_scalar!(
            r#"SELECT COALESCE(SUM(amount), 0) AS "total!: u64" FROM xp_ledger"#
        )
        .fetch_one(self.db.reader())
        .await?;
        Ok(XpTotal::new(total))
    }

    /// The sum of `track`'s grants (R7).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn track_total(&self, track: Track) -> Result<XpTotal, KernelError> {
        let track = track.as_str();
        let total = sqlx::query_scalar!(
            r#"SELECT COALESCE(SUM(amount), 0) AS "total!: u64" FROM xp_ledger WHERE track = ?1"#,
            track
        )
        .fetch_one(self.db.reader())
        .await?;
        Ok(XpTotal::new(total))
    }

    /// The level, derived from the total as it is read: no level is stored, so none can drift from
    /// the ledger (R8).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn level(&self) -> Result<Level, KernelError> {
        Ok(level_for(self.total().await?))
    }
}

impl GrantPort for SqliteXpLedger {
    async fn grant(
        &self,
        request: &GrantRequest,
        at: UtcMillis,
    ) -> Result<GrantAnswer, KernelError> {
        let day = request.study_day.epoch_day();
        let source = request.source.as_str();
        let track = request.track.as_str();
        let amount = request.amount.get();
        let scope = request.scope.as_str();
        let at = at.epoch_millis();
        let mut write = self.db.write().await?;
        sqlx::query!(
            "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            day,
            source,
            track,
            amount,
            scope,
            at
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
        Ok(GrantAnswer::Granted(request.amount))
    }
}
