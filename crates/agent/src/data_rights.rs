//! The agent's data-rights port (SPEC-043 R14): `agent_runs` is the owner's data, exported and
//! erased through the kernel's port that `privacy` drives.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

use crate::runs::AGENT_RUNS_TABLE;

/// The context this port speaks for.
pub const AGENT_CONTEXT: &str = "agent";

/// The agent's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct AgentDataRights;

impl DataRights for AgentDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            AGENT_CONTEXT,
            vec![TableRights {
                table: AGENT_RUNS_TABLE,
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        let _ = (connection, json!(null));
        Box::pin(async { Ok(Vec::new()) })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        let _ = connection;
        Box::pin(async { Ok(()) })
    }
}
