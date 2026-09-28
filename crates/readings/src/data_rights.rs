//! Readings' data-rights port (SPEC-045 R8; SPEC-021): `reading_topic_days` and `reading_runs` are
//! the owner's data, so both are exported and erased (CHARTER 13), through the kernel's port that
//! `privacy` drives. A topic day names the run that wrote it, so the erase empties the topic days
//! first, inside the caller's transaction.

use deck_streak_kernel::{
    DataRights, DataRightsError, Declaration, Disposition, ExportedTable, PortFuture, TableRights,
};
use serde_json::json;
use sqlx::SqliteConnection;

/// The context this port speaks for.
pub const READINGS_CONTEXT: &str = "readings";
/// Each topic's state for each study day (`migrations/004501_readings_topic_days_and_runs.sql`).
pub const READING_TOPIC_DAYS_TABLE: &str = "reading_topic_days";
/// Each resolution run (`migrations/004501_readings_topic_days_and_runs.sql`).
pub const READING_RUNS_TABLE: &str = "reading_runs";

/// Readings' implementation of the kernel's data-rights port.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReadingsDataRights;

impl DataRights for ReadingsDataRights {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            READINGS_CONTEXT,
            vec![
                TableRights {
                    table: READING_TOPIC_DAYS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: READING_RUNS_TABLE,
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
            let days = sqlx::query!(
                r#"SELECT id AS "id!", run_id, study_day, topic, state, class, reason, digest,
                          card_ids, note_ids, new_cards, created_at
                   FROM reading_topic_days ORDER BY id"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let runs = sqlx::query!(
                r#"SELECT id AS "id!", trigger, study_day, started_at, finished_at, outcome, class,
                          reason, unmapped_decks, created_at
                   FROM reading_runs ORDER BY id"#
            )
            .fetch_all(connection)
            .await?;
            Ok(vec![
                ExportedTable {
                    table: READING_TOPIC_DAYS_TABLE,
                    rows: days
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "run_id": row.run_id,
                                "study_day": row.study_day,
                                "topic": row.topic,
                                "state": row.state,
                                "class": row.class,
                                "reason": row.reason,
                                "digest": row.digest,
                                "card_ids": row.card_ids,
                                "note_ids": row.note_ids,
                                "new_cards": row.new_cards,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: READING_RUNS_TABLE,
                    rows: runs
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "trigger": row.trigger,
                                "study_day": row.study_day,
                                "started_at": row.started_at,
                                "finished_at": row.finished_at,
                                "outcome": row.outcome,
                                "class": row.class,
                                "reason": row.reason,
                                "unmapped_decks": row.unmapped_decks,
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
            // A topic day names its run, so the topic days go first.
            sqlx::query!("DELETE FROM reading_topic_days")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM reading_runs")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
