//! The quests data-rights port (SPEC-081 R21; SPEC-021): the chest tables.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::Map;
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const QUESTS_CONTEXT: &str = "quests";
/// Every chest (`migrations/008101_quests_chests_and_tokens.sql`).
pub const CHESTS_TABLE: &str = "chests";
/// The pity counters' one row.
pub const PITY_TABLE: &str = "pity";
/// Every double-XP token.
pub const XP_TOKENS_TABLE: &str = "xp_tokens";
/// The chest settings' one row.
pub const CHEST_SETTINGS_TABLE: &str = "chest_settings";

/// The quests' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct QuestsDataRights;

impl DataRights for QuestsDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            QUESTS_CONTEXT,
            vec![
                TableRights {
                    table: CHESTS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: PITY_TABLE,
                    disposition: Disposition::ResetInPlace { row: Map::new() },
                },
                TableRights {
                    table: XP_TOKENS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: CHEST_SETTINGS_TABLE,
                    disposition: Disposition::ResetInPlace { row: Map::new() },
                },
            ],
        )
    }

    fn export<'a>(
        &'a self,
        _connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move { Ok(Vec::<ExportedTable>::new()) })
    }

    fn erase<'a>(&'a self, _connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move { Ok(()) })
    }
}
