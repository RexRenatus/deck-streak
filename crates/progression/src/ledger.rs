//! The XP ledger (SPEC-040 R1 to R8): one `xp_ledger` row per grant, owned by this context
//! (docs/CONTEXT-MAP.md), written only through the grant port, and summed for the total and the
//! level each time they are read.
//!
//! The predecessor kept its grants in one ledger keyed by study day and source
//! (`database.py:GamifyStore.upsert_xp_grant` at `27ee2bc`). Here the key is the study day, the
//! source and the track, a `once` grant is held to one row per source and track across every study
//! day, and both rules are unique indexes of the table itself (ADR-040).

use deck_streak_kernel::{Db, KernelError, Track, UtcMillis};
use sqlx::SqliteConnection;

use crate::grant::{GrantAnswer, GrantPort, GrantRequest};
use crate::xp::{Level, XpAmount, XpTotal, level_for};

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

    /// The sum of every grant and every settlement (R7, SPEC-072 R10).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn total(&self) -> Result<XpTotal, KernelError> {
        let total = sqlx::query_scalar!(
            r#"SELECT (SELECT COALESCE(SUM(amount), 0) FROM xp_ledger)
                    + (SELECT COALESCE(SUM(amount), 0) FROM xp_settlement) AS "total!: u64""#
        )
        .fetch_one(self.db.reader())
        .await?;
        Ok(XpTotal::new(total))
    }

    /// The sum of `track`'s grants and settlements (R7, SPEC-072 R10).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn track_total(&self, track: Track) -> Result<XpTotal, KernelError> {
        let track = track.as_str();
        let total = sqlx::query_scalar!(
            r#"SELECT (SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE track = ?1)
                    + (SELECT COALESCE(SUM(amount), 0) FROM xp_settlement WHERE track = ?1)
                    AS "total!: u64""#,
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

/// Write `request` on `connection`, which the caller holds inside its own write transaction, so a
/// grant made during the fold lands in the same commit as the day it belongs to (SPEC-076 R26).
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn grant_on(
    connection: &mut SqliteConnection,
    request: &GrantRequest,
    at: UtcMillis,
) -> Result<GrantAnswer, KernelError> {
    let day = request.study_day.epoch_day();
    let source = request.source.as_str();
    let track = request.track.as_str();
    let amount = request.amount.get();
    let scope = request.scope.as_str();
    let at = at.epoch_millis();
    // The insert's own conflict with the ledger's two unique indexes is the existence check, so
    // the key lives in the migration alone (ADR-040); a conflict writes nothing.
    let written = sqlx::query!(
        "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT DO NOTHING",
        day,
        source,
        track,
        amount,
        scope,
        at
    )
    .execute(&mut *connection)
    .await?
    .rows_affected();
    let answer = if written == 1 {
        GrantAnswer::Granted(request.amount)
    } else {
        // The row that holds the key, read in the same transaction: the same study day, source
        // and track, or for a `once` request the `once` row of that source and track.
        let held = sqlx::query_scalar!(
            r#"SELECT amount AS "amount!: u32" FROM xp_ledger
                   WHERE source = ?1 AND track = ?2
                     AND (study_day = ?3 OR (?4 = 'once' AND scope = 'once'))
                   ORDER BY id LIMIT 1"#,
            source,
            track,
            day,
            scope
        )
        .fetch_one(&mut *connection)
        .await?;
        GrantAnswer::AlreadyGranted(XpAmount::new(held))
    };
    Ok(answer)
}

impl GrantPort for SqliteXpLedger {
    async fn grant(
        &self,
        request: &GrantRequest,
        at: UtcMillis,
    ) -> Result<GrantAnswer, KernelError> {
        let mut write = self.db.write().await?;
        let answer = grant_on(&mut write, request, at).await?;
        write.commit().await?;
        Ok(answer)
    }
}
