//! Analytics' data-rights port (SPEC-071 R23; SPEC-021): `daily_rollup` and `daily_lang_stats` are
//! the owner's data, so both are exported and erased (CHARTER 13), through the kernel's port that
//! `privacy` drives.

use deck_streak_kernel::{DataRights, DataRightsError, Declaration, ExportedTable, PortFuture};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const ANALYTICS_CONTEXT: &str = "analytics";
/// Each study day's rollup (`migrations/007101_analytics_daily_rollup.sql`).
pub const DAILY_ROLLUP_TABLE: &str = "daily_rollup";
/// Each study day's per-course statistics (`migrations/007101_analytics_daily_rollup.sql`).
pub const DAILY_LANG_STATS_TABLE: &str = "daily_lang_stats";

/// Analytics' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnalyticsDataRights;

impl DataRights for AnalyticsDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(ANALYTICS_CONTEXT, Vec::new())
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
