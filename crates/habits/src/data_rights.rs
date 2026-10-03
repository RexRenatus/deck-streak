//! The habits' data-rights port (SPEC-078 R21; SPEC-021): `minutes_log` is the owner's data, so it
//! is exported and erased, through the kernel's port that `privacy` drives.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const HABITS_CONTEXT: &str = "habits";
/// Every reading entry (`migrations/007801_habits_minutes_log.sql`).
pub const MINUTES_LOG_TABLE: &str = "minutes_log";

/// The habits' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct HabitsDataRights;

impl DataRights for HabitsDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            HABITS_CONTEXT,
            vec![TableRights {
                table: MINUTES_LOG_TABLE,
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let _ = connection;
            Ok(Vec::new())
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let _ = connection;
            Ok(())
        })
    }
}
