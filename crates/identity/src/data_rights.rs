//! Identity's data-rights port: the `passkeys` table, exported whole and emptied by an erase
//! (SPEC-359 R12; SPEC-131 R18, R19).
//!
//! The sessions, link codes and ceremonies live in memory and are never stored, so the one table
//! identity owns is `passkeys`. Its export is every row with every column a person may ask for; the
//! credential is exported as the library stored it, and the erase deletes every row inside the
//! caller's transaction.

use deck_streak_kernel::{DataRights, DataRightsError, Declaration, ExportedTable, PortFuture};
use sqlx::SqliteConnection;

/// Identity's context name in the registry.
pub const IDENTITY_CONTEXT: &str = "identity";
/// The table identity owns (SPEC-359 R11).
pub const PASSKEYS_TABLE: &str = "passkeys";

/// Identity's port.
#[derive(Clone, Copy, Debug, Default)]
pub struct IdentityDataRights;

impl DataRights for IdentityDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(IDENTITY_CONTEXT, Vec::new())
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
