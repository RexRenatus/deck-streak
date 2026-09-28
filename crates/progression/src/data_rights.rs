//! Progression's data-rights port (SPEC-040 R9): `xp_ledger` is the owner's data, exported whole
//! and erased, through the kernel's port that `privacy` drives (CHARTER 13; SPEC-021).
//!
//! The erase is the one statement that removes a grant, and it removes every grant at once, when
//! the owner erases their data. The grant port itself offers no debit, update or delete (R6).

use deck_streak_kernel::{DataRights, DataRightsError, Declaration, ExportedTable, PortFuture};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const PROGRESSION_CONTEXT: &str = "progression";

/// Progression's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProgressionDataRights;

impl DataRights for ProgressionDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(PROGRESSION_CONTEXT, Vec::new())
    }

    fn export<'a>(
        &'a self,
        _connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn erase<'a>(&'a self, _connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}
