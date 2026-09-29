//! Analytics' data-rights port (SPEC-071 R23; SPEC-021): `daily_rollup` and `daily_lang_stats` are
//! the owner's data, so both are exported and erased (CHARTER 13), through the kernel's port that
//! `privacy` drives.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

use crate::rollup::RollupRow;

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
        Declaration::new(
            ANALYTICS_CONTEXT,
            vec![
                TableRights {
                    table: DAILY_ROLLUP_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: DAILY_LANG_STATS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
            ],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let rollups = sqlx::query_as!(
                RollupRow,
                r#"SELECT study_day AS "study_day!", reviews, learn_count, review_count,
                          relearn_count, filtered_count, seconds, answered, passed,
                          true_retention, graduations, decks_studied, avg_answer_seconds,
                          young_answered, young_passed, mature_answered, mature_passed,
                          mature_count, young_count, leech_active, backlog, due_today,
                          card_state_src, score, consistency, retention, workload, volume,
                          mastery, score_at_close, settled_at, fingerprint, created_at, updated_at
                   FROM daily_rollup ORDER BY study_day"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let languages = sqlx::query!(
                "SELECT study_day, course, reviews, seconds, answered, passed, created_at \
                 FROM daily_lang_stats ORDER BY study_day, course"
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: DAILY_ROLLUP_TABLE,
                    rows: rollups.iter().map(RollupRow::to_json).collect(),
                },
                ExportedTable {
                    table: DAILY_LANG_STATS_TABLE,
                    rows: languages
                        .into_iter()
                        .map(|row| {
                            json!({
                                "study_day": row.study_day,
                                "course": row.course,
                                "reviews": row.reviews,
                                "seconds": row.seconds,
                                "answered": row.answered,
                                "passed": row.passed,
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
            sqlx::query!("DELETE FROM daily_lang_stats")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM daily_rollup")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
