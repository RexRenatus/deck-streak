//! The vault's data-rights port (SPEC-110 R17). This is the inert shape: it declares nothing yet.

use deck_streak_kernel::{DataRights, DataRightsError, Declaration, ExportedTable, PortFuture};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const VAULT_CONTEXT: &str = "vault";

/// The vault's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct VaultDataRights;

impl DataRights for VaultDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(VAULT_CONTEXT, Vec::new())
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
