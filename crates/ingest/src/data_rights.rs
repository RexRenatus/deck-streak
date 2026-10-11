//! Ingest's data-rights port (SPEC-022 R12, SPEC-023 R13): `sync_runs` is exported and erased, like
//! every table of the owner's data, and the singleton `ingest_state` is exported and reset in place
//! (CHARTER 13), through the kernel's port that `privacy` drives (#14). A reset `ingest_state` reads
//! as never recomputed, so the next cycle runs the recompute and recounts the window.
//! The declared write class's stop, `write_class_stop`, is exempt (SPEC-083 R36): only the
//! owner's command clears it, and an erase that cleared it would be a second path.
//! The decks the learner keeps away from AI, `sensitive_decks`, are exported and erased by delete
//! (SPEC-381 R10), so after an erase every deck is readable again until the learner keeps one away.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const INGEST_CONTEXT: &str = "ingest";
/// The table the sync record lives in (`migrations/002201_ingest_sync_runs.sql`).
pub const SYNC_RUNS_TABLE: &str = "sync_runs";
/// The table a skip day's record lives in (`migrations/008301_ingest_skip_days.sql`).
pub const SKIP_DAYS_TABLE: &str = "skip_days";
/// The table a skip's card snapshot lives in (`migrations/008302_ingest_skip_card_snapshot.sql`).
pub const SKIP_CARD_SNAPSHOT_TABLE: &str = "skip_card_snapshot";
/// The table the change gate's anchor, the rescore flag and the window's base live in
/// (`migrations/002301_ingest_state.sql`).
pub const INGEST_STATE_TABLE: &str = "ingest_state";
/// The table the marked decks live in (`migrations/038101_ingest_sensitive_decks.sql`, SPEC-381 R10).
pub const SENSITIVE_DECKS_TABLE: &str = "sensitive_decks";
/// The table the declared write class's stop lives in
/// (`migrations/008303_ingest_write_class_stop.sql`).
pub const WRITE_CLASS_STOP_TABLE: &str = "write_class_stop";

/// Why the class's stop survives an erase (SPEC-083 R36, ADR-321 D16).
const WRITE_CLASS_STOP_EXEMPTION: &str = "only the owner's command clears the class's stop, and \
    an erase that cleared it would be a second path; the row names who set the stop, why and \
    when, and holds none of the owner's data";

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

/// The skip record, exported whole: every column, one row per take.
async fn export_skip_days(connection: &mut SqliteConnection) -> Result<ExportedTable, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", study_day, due_count, state, reason, cards_moved,
                  tariff_unfunded, undone, undone_at, created_at
           FROM skip_days ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(ExportedTable {
        table: SKIP_DAYS_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "study_day": row.study_day,
                    "due_count": row.due_count,
                    "state": row.state,
                    "reason": row.reason,
                    "cards_moved": row.cards_moved,
                    "tariff_unfunded": row.tariff_unfunded,
                    "undone": row.undone,
                    "undone_at": row.undone_at,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
}

/// The decks kept away from AI (SPEC-381 R10), exported whole: every column, one row per mark.
async fn export_sensitive_decks(
    connection: &mut SqliteConnection,
) -> Result<ExportedTable, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT deck_id AS "deck_id!", created_at FROM sensitive_decks ORDER BY deck_id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(ExportedTable {
        table: SENSITIVE_DECKS_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "deck_id": row.deck_id,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
}

/// The skip card snapshot, exported whole: card ids and scheduling only.
async fn export_skip_card_snapshot(
    connection: &mut SqliteConnection,
) -> Result<ExportedTable, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", skip_id, card_id, prior_due, prior_queue, prior_type,
                  prior_interval, prior_ease_factor, prior_original_deck_id,
                  prior_original_due, left_due, left_queue, left_type, left_interval,
                  left_ease_factor, left_mtime, created_at
           FROM skip_card_snapshot ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(ExportedTable {
        table: SKIP_CARD_SNAPSHOT_TABLE,
        rows: rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "skip_id": row.skip_id,
                    "card_id": row.card_id,
                    "prior_due": row.prior_due,
                    "prior_queue": row.prior_queue,
                    "prior_type": row.prior_type,
                    "prior_interval": row.prior_interval,
                    "prior_ease_factor": row.prior_ease_factor,
                    "prior_original_deck_id": row.prior_original_deck_id,
                    "prior_original_due": row.prior_original_due,
                    "left_due": row.left_due,
                    "left_queue": row.left_queue,
                    "left_type": row.left_type,
                    "left_interval": row.left_interval,
                    "left_ease_factor": row.left_ease_factor,
                    "left_mtime": row.left_mtime,
                    "created_at": row.created_at,
                })
            })
            .collect(),
    })
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
                    table: SKIP_DAYS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: SKIP_CARD_SNAPSHOT_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: SENSITIVE_DECKS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: INGEST_STATE_TABLE,
                    disposition: Disposition::ResetInPlace {
                        row: ingest_state_reset(),
                    },
                },
                TableRights {
                    table: WRITE_CLASS_STOP_TABLE,
                    disposition: Disposition::Exempt {
                        reason: WRITE_CLASS_STOP_EXEMPTION,
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
            .fetch_all(&mut *connection)
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
                export_skip_days(&mut *connection).await?,
                export_skip_card_snapshot(&mut *connection).await?,
                export_sensitive_decks(connection).await?,
                state,
            ])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM sync_runs")
                .execute(&mut *connection)
                .await?;
            // The snapshot rows reference their skip, so they go first.
            sqlx::query!("DELETE FROM skip_card_snapshot")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM skip_days")
                .execute(&mut *connection)
                .await?;
            // Every mark goes, so after an erase every deck is readable again (SPEC-381 R10).
            sqlx::query!("DELETE FROM sensitive_decks")
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
