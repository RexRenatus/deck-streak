//! Coordination's data-rights port (SPEC-027 R10): `cron_fires` is EXEMPT from export and erase
//! (CHARTER 13), through the kernel's port that `privacy` drives (#14).
//!
//! The ledger holds no data of the owner's, only which scheduled fires ran; and an erase that
//! emptied it would re-arm the catch-up double-send guard, so a notification already sent that day
//! could be claimed and sent again. The predecessor kept it out of its erase for the same reason
//! (`database.py`, migration 18).

use deck_streak_kernel::{DataRights, DataRightsError, Declaration, ExportedTable, PortFuture};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const COORDINATION_CONTEXT: &str = "coordination";
/// The cron-fire ledger (`migrations/002701_coordination_cron_fires.sql`).
pub const CRON_FIRES_TABLE: &str = "cron_fires";

/// Coordination's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct CoordinationDataRights;

impl DataRights for CoordinationDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(COORDINATION_CONTEXT, Vec::new())
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
