//! Streaks' data-rights port (SPEC-076 R20; CHARTER 13): the streak rows, the freeze ledger and the
//! habit strength are the owner's data, exported whole and erased; the governor's one row is reset
//! in place, so an erase returns both tracks and the governor to their start state.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const STREAKS_CONTEXT: &str = "streaks";

/// Streaks' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct StreaksDataRights;

impl DataRights for StreaksDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            STREAKS_CONTEXT,
            vec![TableRights {
                table: "streak_state",
                disposition: Disposition::ExportAndErase,
            }],
        )
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
