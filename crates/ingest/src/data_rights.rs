//! Ingest's data-rights port (SPEC-022 R12, SPEC-023 R13): `sync_runs` is exported and erased, like
//! every table of the owner's data, and the singleton `ingest_state` is exported and reset in place
//! (CHARTER 13), through the kernel's port that `privacy` drives (#14). A reset `ingest_state` reads
//! as never recomputed, so the next cycle runs the recompute and recounts the window.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const INGEST_CONTEXT: &str = "ingest";
/// The table the sync record lives in (`migrations/002201_ingest_sync_runs.sql`).
pub const SYNC_RUNS_TABLE: &str = "sync_runs";
/// The table the change gate's anchor, the rescore flag and the window's base live in
/// (`migrations/002301_ingest_state.sql`).
pub const INGEST_STATE_TABLE: &str = "ingest_state";

/// The columns an erase clears in `ingest_state`: the anchor, the window's base and the refused
/// owner request (SPEC-128 R6).
const INGEST_STATE_CLEARED: [&str; 10] = [
    "anchor_newest_review_id",
    "anchor_card_count",
    "anchor_card_fingerprint",
    "anchor_study_day",
    "anchor_recomputed_at",
    "anchor_settings_generation",
    "window_floor",
    "window_count",
    "refused_at",
    "refused_reason",
];

/// The row an erase leaves in `ingest_state`: no anchor, no pending rescore, no base, no refused request.
fn ingest_state_reset() -> Map<String, Value> {
    let mut row: Map<String, Value> = INGEST_STATE_CLEARED
        .iter()
        .map(|column| ((*column).to_owned(), Value::Null))
        .collect();
    row.insert("rescore_pending".to_owned(), Value::from(0));
    row
}

/// Ingest's implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct IngestDataRights;

impl DataRights for IngestDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            INGEST_CONTEXT,
            vec![
                TableRights {
                    table: SYNC_RUNS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: INGEST_STATE_TABLE,
                    disposition: Disposition::ResetInPlace {
                        row: ingest_state_reset(),
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
            let rows = sqlx::query!(
                r#"SELECT id AS "id!", trigger, study_day, started_at, finished_at, status, reason,
                          attempts, full_download, created_at
                   FROM sync_runs ORDER BY id"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let state = sqlx::query!(
                r#"SELECT id AS "id!", anchor_newest_review_id, anchor_card_count,
                          anchor_card_fingerprint, anchor_study_day, anchor_recomputed_at,
                          anchor_settings_generation, rescore_pending, window_floor, window_count,
                          refused_at, refused_reason, created_at
                   FROM ingest_state ORDER BY id"#
            )
            .fetch_all(connection)
            .await?;
            let state = ExportedTable {
                table: INGEST_STATE_TABLE,
                rows: state
                    .into_iter()
                    .map(|row| {
                        json!({
                            "id": row.id,
                            "anchor_newest_review_id": row.anchor_newest_review_id,
                            "anchor_card_count": row.anchor_card_count,
                            "anchor_card_fingerprint": row.anchor_card_fingerprint,
                            "anchor_study_day": row.anchor_study_day,
                            "anchor_recomputed_at": row.anchor_recomputed_at,
                            "anchor_settings_generation": row.anchor_settings_generation,
                            "rescore_pending": row.rescore_pending,
                            "window_floor": row.window_floor,
                            "window_count": row.window_count,
                            "refused_at": row.refused_at,
                            "refused_reason": row.refused_reason,
                            "created_at": row.created_at,
                        })
                    })
                    .collect(),
            };
            Ok(vec![
                ExportedTable {
                    table: SYNC_RUNS_TABLE,
                    rows: rows
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "trigger": row.trigger,
                                "study_day": row.study_day,
                                "started_at": row.started_at,
                                "finished_at": row.finished_at,
                                "status": row.status,
                                "reason": row.reason,
                                "attempts": row.attempts,
                                "full_download": row.full_download,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                state,
            ])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM sync_runs")
                .execute(&mut *connection)
                .await?;
            // The declared reset row (`ingest_state_reset`): the row itself, and when it was made,
            // are kept.
            sqlx::query!(
                "UPDATE ingest_state SET anchor_newest_review_id = NULL, anchor_card_count = NULL, \
                 anchor_card_fingerprint = NULL, anchor_study_day = NULL, \
                 anchor_recomputed_at = NULL, anchor_settings_generation = NULL, \
                 rescore_pending = 0, window_floor = NULL, window_count = NULL, \
                 refused_at = NULL, refused_reason = NULL WHERE id = 1"
            )
            .execute(connection)
            .await?;
            Ok(())
        })
    }
}
