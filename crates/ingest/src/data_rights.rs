//! Ingest's data-rights port (SPEC-022 R12): `sync_runs` is exported and erased, like every table
//! of the owner's data (CHARTER 13), through the kernel's port that `privacy` drives (#14).

use deck_streak_kernel::{DataRights, DataRightsError, Declaration, ExportedTable, PortFuture};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const INGEST_CONTEXT: &str = "ingest";
/// The table the sync record lives in (`migrations/002201_ingest_sync_runs.sql`).
pub const SYNC_RUNS_TABLE: &str = "sync_runs";

/// Ingest's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct IngestDataRights;

impl DataRights for IngestDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(INGEST_CONTEXT, Vec::new())
    }

    fn export<'a>(
        &'a self,
        _connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn erase<'a>(&'a self, _connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move { Ok(()) })
    }
}
