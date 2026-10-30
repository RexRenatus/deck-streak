//! Coordination's data-rights port (SPEC-027 R10): `cron_fires` is EXEMPT from export and erase
//! (CHARTER 13), through the kernel's port that `privacy` drives (#14). `instrument_reports` holds the
//! owner's findings about their own collection, so it is exported and erased (SPEC-094 R9).
//!
//! The ledger holds no data of the owner's, only which scheduled fires ran; and an erase that
//! emptied it would re-arm the catch-up double-send guard, so a notification already sent that day
//! could be claimed and sent again. The predecessor kept it out of its erase for the same reason
//! (`database.py`, migration 18).

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::{Row, SqliteConnection};

/// The context this port speaks for.
pub const COORDINATION_CONTEXT: &str = "coordination";
/// The cron-fire ledger (`migrations/002701_coordination_cron_fires.sql`).
pub const CRON_FIRES_TABLE: &str = "cron_fires";

/// The latest report of each instrument (`migrations/009401_coordination_instrument_reports.sql`).
pub const INSTRUMENT_REPORTS_TABLE: &str = "instrument_reports";

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
            vec![
                TableRights {
                    table: CRON_FIRES_TABLE,
                    disposition: Disposition::Exempt {
                        reason: CRON_FIRES_EXEMPTION,
                    },
                },
                TableRights {
                    table: INSTRUMENT_REPORTS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
            ],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        // `cron_fires` is exempt: an export carries none of it.
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT instrument, study_day, schema_version, report_json, created_at \
                 FROM instrument_reports ORDER BY instrument",
            )
            .fetch_all(connection)
            .await?;
            let rows = rows
                .iter()
                .map(|row| {
                    json!({
                        "instrument": row.get::<String, _>("instrument"),
                        "study_day": row.get::<i64, _>("study_day"),
                        "schema_version": row.get::<i64, _>("schema_version"),
                        "report_json": row.get::<String, _>("report_json"),
                        "created_at": row.get::<i64, _>("created_at"),
                    })
                })
                .collect();
            Ok(vec![ExportedTable {
                table: INSTRUMENT_REPORTS_TABLE,
                rows,
            }])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        // `cron_fires` is exempt: an erase leaves it as it was.
        Box::pin(async move {
            sqlx::query("DELETE FROM instrument_reports")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
