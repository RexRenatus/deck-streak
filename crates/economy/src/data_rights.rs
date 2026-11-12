//! The economy's data-rights port (SPEC-082 R18; SPEC-021): `coin_ledger` is the owner's data, so
//! it is exported and erased, and `economy_state` is one row, exported and reset in place to no
//! pass and no surcharge (CHARTER 13), through the kernel's port that `privacy` drives.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const ECONOMY_CONTEXT: &str = "economy";
/// Every coin movement (`migrations/008201_economy_wallet_and_shop.sql`).
pub const COIN_LEDGER_TABLE: &str = "coin_ledger";
/// The shop's one row: the active pass's end and the surcharge's end (the same migration).
pub const ECONOMY_STATE_TABLE: &str = "economy_state";

/// The columns of `economy_state` an erase clears: no pass and no surcharge.
const ECONOMY_STATE_CLEARED: [&str; 2] = ["pass_ends_at", "surcharge_ends_at"];

/// The values an erase writes into `economy_state`'s one row.
fn economy_state_reset() -> Map<String, Value> {
    ECONOMY_STATE_CLEARED
        .iter()
        .map(|column| ((*column).to_owned(), Value::Null))
        .collect()
}

/// The economy's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct EconomyDataRights;

impl DataRights for EconomyDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            ECONOMY_CONTEXT,
            vec![
                TableRights {
                    table: COIN_LEDGER_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: ECONOMY_STATE_TABLE,
                    disposition: Disposition::ResetInPlace {
                        row: economy_state_reset(),
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
            let movements = sqlx::query!(
                "SELECT id, study_day, source, reference, delta, created_at \
                 FROM coin_ledger ORDER BY id"
            )
            .fetch_all(&mut *connection)
            .await?;
            let state = sqlx::query!(
                "SELECT id, pass_ends_at, surcharge_ends_at, created_at \
                 FROM economy_state ORDER BY id"
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: COIN_LEDGER_TABLE,
                    rows: movements
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "study_day": row.study_day,
                                "source": row.source,
                                "reference": row.reference,
                                "delta": row.delta,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: ECONOMY_STATE_TABLE,
                    rows: state
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "pass_ends_at": row.pass_ends_at,
                                "surcharge_ends_at": row.surcharge_ends_at,
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
            sqlx::query!("DELETE FROM coin_ledger")
                .execute(&mut *connection)
                .await?;
            // The one row stays: it is reset to no pass and no surcharge, never deleted.
            sqlx::query!(
                "UPDATE economy_state SET pass_ends_at = NULL, surcharge_ends_at = NULL \
                 WHERE id = 1"
            )
            .execute(connection)
            .await?;
            Ok(())
        })
    }
}
