//! Identity's data-rights port: the `passkeys` table, exported whole and emptied by an erase
//! (SPEC-359 R12; SPEC-131 R18, R19).
//!
//! The sessions, link codes and ceremonies live in memory and are never stored, so the one table
//! identity owns is `passkeys`. Its export is every row with every column a person may ask for; the
//! credential is exported as the library stored it, and the erase deletes every row inside the
//! caller's transaction.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
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
        Declaration::new(
            IDENTITY_CONTEXT,
            vec![TableRights {
                table: PASSKEYS_TABLE,
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            // The two byte columns as uppercase hex, the credential as the library stored it.
            let passkeys = sqlx::query!(
                r#"SELECT id, telegram_user_id,
                       hex(credential_id) AS "credential_id!: String",
                       hex(user_handle) AS "user_handle!: String",
                       credential, counter, backup_state, created_at, last_used_at
                   FROM passkeys ORDER BY id"#
            )
            .fetch_all(&mut *connection)
            .await?;
            Ok(vec![ExportedTable {
                table: PASSKEYS_TABLE,
                rows: passkeys
                    .into_iter()
                    .map(|row| {
                        json!({
                            "id": row.id,
                            "telegram_user_id": row.telegram_user_id,
                            "credential_id": row.credential_id,
                            "user_handle": row.user_handle,
                            "credential": row.credential,
                            "counter": row.counter,
                            "backup_state": row.backup_state,
                            "created_at": row.created_at,
                            "last_used_at": row.last_used_at,
                        })
                    })
                    .collect(),
            }])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM passkeys")
                .execute(&mut *connection)
                .await?;
            Ok(())
        })
    }
}
