//! Progression's data-rights port (SPEC-040 R9): `xp_ledger` is the owner's data, exported whole
//! and erased, through the kernel's port that `privacy` drives (CHARTER 13; SPEC-021).
//!
//! The erase is the one statement that removes a grant, and it removes every grant at once, when
//! the owner erases their data. The grant port itself offers no debit, update or delete (R6).

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

use crate::ledger::XP_LEDGER_TABLE;
use crate::settle::XP_SETTLEMENT_TABLE;

/// The table the day buffs live in (`migrations/007202_progression_buffs.sql`).
pub const BUFFS_TABLE: &str = "buffs";

/// The context this port speaks for.
pub const PROGRESSION_CONTEXT: &str = "progression";

/// Progression's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProgressionDataRights;

impl DataRights for ProgressionDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            PROGRESSION_CONTEXT,
            vec![
                TableRights {
                    table: XP_LEDGER_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: XP_SETTLEMENT_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: BUFFS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
            ],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let rows = sqlx::query!(
                r#"SELECT id AS "id!", study_day, source, track, amount, scope, created_at
                   FROM xp_ledger ORDER BY id"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let settled = sqlx::query!(
                r#"SELECT id AS "id!", study_day, source, track, amount, closed, created_at
                   FROM xp_settlement ORDER BY id"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let buffs = sqlx::query!(
                "SELECT study_day, kind, created_at FROM buffs ORDER BY study_day, kind"
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: XP_LEDGER_TABLE,
                    rows: rows
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "study_day": row.study_day,
                                "source": row.source,
                                "track": row.track,
                                "amount": row.amount,
                                "scope": row.scope,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: XP_SETTLEMENT_TABLE,
                    rows: settled
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "study_day": row.study_day,
                                "source": row.source,
                                "track": row.track,
                                "amount": row.amount,
                                "closed": row.closed,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: BUFFS_TABLE,
                    rows: buffs
                        .into_iter()
                        .map(|row| {
                            json!({
                                "study_day": row.study_day,
                                "kind": row.kind,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
            ])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            // Every grant, at once: the owner's erase, and the only delete a grant ever meets.
            sqlx::query!("DELETE FROM xp_ledger")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM xp_settlement")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM buffs")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
