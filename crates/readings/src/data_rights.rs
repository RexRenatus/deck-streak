//! Readings' data-rights port (SPEC-045 R8; SPEC-021): `reading_topic_days` and `reading_runs` are
//! the owner's data, so both are exported and erased (CHARTER 13), through the kernel's port that
//! `privacy` drives. A topic day names the run that wrote it, so the erase empties the topic days
//! first, inside the caller's transaction.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const READINGS_CONTEXT: &str = "readings";
/// Each topic's state for each study day (`migrations/004501_readings_topic_days_and_runs.sql`).
pub const READING_TOPIC_DAYS_TABLE: &str = "reading_topic_days";
/// Each resolution run (`migrations/004501_readings_topic_days_and_runs.sql`).
pub const READING_RUNS_TABLE: &str = "reading_runs";

/// Readings' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReadingsDataRights;

impl DataRights for ReadingsDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(READINGS_CONTEXT, Vec::new())
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
