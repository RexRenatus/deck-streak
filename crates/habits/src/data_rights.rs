//! The habits' data-rights port (SPEC-078 R21; SPEC-021): `minutes_log` and `writing_log` are the
//! owner's data, so each is exported and erased, through the kernel's port that `privacy` drives.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const HABITS_CONTEXT: &str = "habits";
/// Every reading entry (`migrations/007801_habits_minutes_log.sql`).
pub const MINUTES_LOG_TABLE: &str = "minutes_log";
/// Every writing confirmation (`migrations/007802_habits_writing_log.sql`).
pub const WRITING_LOG_TABLE: &str = "writing_log";

/// The habits' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct HabitsDataRights;

impl DataRights for HabitsDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            HABITS_CONTEXT,
            vec![
                TableRights {
                    table: MINUTES_LOG_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: WRITING_LOG_TABLE,
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
            let entries = sqlx::query!(
                "SELECT id, code, study_day, minutes, note, created_at \
                 FROM minutes_log ORDER BY id"
            )
            .fetch_all(&mut *connection)
            .await?;
            let confirmations = sqlx::query!(
                "SELECT code, study_day, created_at FROM writing_log ORDER BY study_day, code"
            )
            .fetch_all(&mut *connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: MINUTES_LOG_TABLE,
                    rows: entries
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "code": row.code,
                                "study_day": row.study_day,
                                "minutes": row.minutes,
                                "note": row.note,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: WRITING_LOG_TABLE,
                    rows: confirmations
                        .into_iter()
                        .map(|row| {
                            json!({
                                "code": row.code,
                                "study_day": row.study_day,
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
            sqlx::query!("DELETE FROM minutes_log")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM writing_log")
                .execute(&mut *connection)
                .await?;
            Ok(())
        })
    }
}
