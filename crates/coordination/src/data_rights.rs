//! Coordination's data-rights port (SPEC-027 R10): `cron_fires` is EXEMPT from export and erase
//! (CHARTER 13), through the kernel's port that `privacy` drives (#14).
//!
//! The ledger holds no data of the owner's, only which scheduled fires ran; and an erase that
//! emptied it would re-arm the catch-up double-send guard, so a notification already sent that day
//! could be claimed and sent again. The predecessor kept it out of its erase for the same reason
//! (`database.py`, migration 18).

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const COORDINATION_CONTEXT: &str = "coordination";
/// The cron-fire ledger (`migrations/002701_coordination_cron_fires.sql`).
pub const CRON_FIRES_TABLE: &str = "cron_fires";

/// Why the ledger survives an erase.
const CRON_FIRES_EXEMPTION: &str = "the cron-fire ledger records which scheduled fires ran and \
    holds none of the owner's data, and an erase must never re-arm the catch-up double-send guard: \
    emptied, it would let a notification already sent that day be claimed and sent again";

/// Coordination's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct CoordinationDataRights;

impl DataRights for CoordinationDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            COORDINATION_CONTEXT,
            vec![TableRights {
                table: CRON_FIRES_TABLE,
                disposition: Disposition::Exempt {
                    reason: CRON_FIRES_EXEMPTION,
                },
            }],
        )
    }

    fn export<'a>(
        &'a self,
        _connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        // The one table is exempt: an export carries none of it.
        Box::pin(async { Ok(Vec::new()) })
    }

    fn erase<'a>(&'a self, _connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        // The one table is exempt: an erase leaves it as it was.
        Box::pin(async { Ok(()) })
    }
}
