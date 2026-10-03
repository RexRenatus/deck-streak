//! The quests data-rights port (SPEC-081 R21; SPEC-021): `chests` and `xp_tokens` are the owner's
//! data, exported and erased; `pity` and `chest_settings` are one row each, exported and reset in
//! place to the migration's seeds (CHARTER 13), through the kernel's port that `privacy` drives.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::{Map, Value, json};
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

/// The values an erase writes into `pity`'s one row: both counters at 0.
fn pity_reset() -> Map<String, Value> {
    [("since_epic", 0), ("since_legendary", 0)]
        .into_iter()
        .map(|(column, value)| (column.to_owned(), Value::from(value)))
        .collect()
}

/// The values an erase writes into `chest_settings`' one row: the predecessor's defaults, three
/// chests a study day and the vault from the local hour 21.
fn chest_settings_reset() -> Map<String, Value> {
    [("per_day_max", 3), ("vault_hour", 21)]
        .into_iter()
        .map(|(column, value)| (column.to_owned(), Value::from(value)))
        .collect()
}

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
                    disposition: Disposition::ResetInPlace { row: pity_reset() },
                },
                TableRights {
                    table: XP_TOKENS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: CHEST_SETTINGS_TABLE,
                    disposition: Disposition::ResetInPlace {
                        row: chest_settings_reset(),
                    },
                },
            ],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let chests = sqlx::query!(
                "SELECT id, study_day, origin, session_start, rarity, payout_xp, state, choice, \
                 announced, created_at FROM chests ORDER BY id"
            )
            .fetch_all(&mut *connection)
            .await?;
            let pity = sqlx::query!(
                "SELECT id, since_epic, since_legendary, created_at FROM pity ORDER BY id"
            )
            .fetch_all(&mut *connection)
            .await?;
            let tokens = sqlx::query!(
                "SELECT id, chest_id, granted_at, activated_at, window_ends_at, consumed, \
                 created_at FROM xp_tokens ORDER BY id"
            )
            .fetch_all(&mut *connection)
            .await?;
            let settings = sqlx::query!(
                "SELECT id, per_day_max, vault_hour, created_at FROM chest_settings ORDER BY id"
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: CHESTS_TABLE,
                    rows: chests
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "study_day": row.study_day,
                                "origin": row.origin,
                                "session_start": row.session_start,
                                "rarity": row.rarity,
                                "payout_xp": row.payout_xp,
                                "state": row.state,
                                "choice": row.choice,
                                "announced": row.announced,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: PITY_TABLE,
                    rows: pity
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "since_epic": row.since_epic,
                                "since_legendary": row.since_legendary,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: XP_TOKENS_TABLE,
                    rows: tokens
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "chest_id": row.chest_id,
                                "granted_at": row.granted_at,
                                "activated_at": row.activated_at,
                                "window_ends_at": row.window_ends_at,
                                "consumed": row.consumed,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: CHEST_SETTINGS_TABLE,
                    rows: settings
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "per_day_max": row.per_day_max,
                                "vault_hour": row.vault_hour,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
            ])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM chests")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM xp_tokens")
                .execute(&mut *connection)
                .await?;
            // The two single rows stay: each is reset to its seed, never deleted.
            sqlx::query!("UPDATE pity SET since_epic = 0, since_legendary = 0 WHERE id = 1")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("UPDATE chest_settings SET per_day_max = 3, vault_hour = 21 WHERE id = 1")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
