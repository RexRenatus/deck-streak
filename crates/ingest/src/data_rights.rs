//! Ingest's data-rights port (SPEC-022 R12): `sync_runs` is exported and erased, like every table
//! of the owner's data (CHARTER 13), through the kernel's port that `privacy` drives (#14).

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const INGEST_CONTEXT: &str = "ingest";
/// The table the sync record lives in (`migrations/002201_ingest_sync_runs.sql`).
pub const SYNC_RUNS_TABLE: &str = "sync_runs";
/// The table the change gate's anchor, the rescore flag and the window's base live in
/// (`migrations/002301_ingest_state.sql`).
pub const INGEST_STATE_TABLE: &str = "ingest_state";

/// Ingest's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct IngestDataRights;

impl DataRights for IngestDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            INGEST_CONTEXT,
            vec![TableRights {
                table: SYNC_RUNS_TABLE,
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let rows = sqlx::query!(
                r#"SELECT id AS "id!", trigger, study_day, started_at, finished_at, status, reason,
                          attempts, full_download, created_at
                   FROM sync_runs ORDER BY id"#
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![ExportedTable {
                table: SYNC_RUNS_TABLE,
                rows: rows
                    .into_iter()
                    .map(|row| {
                        json!({
                            "id": row.id,
                            "trigger": row.trigger,
                            "study_day": row.study_day,
                            "started_at": row.started_at,
                            "finished_at": row.finished_at,
                            "status": row.status,
                            "reason": row.reason,
                            "attempts": row.attempts,
                            "full_download": row.full_download,
                            "created_at": row.created_at,
                        })
                    })
                    .collect(),
            }])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM sync_runs")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
