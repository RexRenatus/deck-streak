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

/// Each topic's stored reading (`migrations/004601_readings_and_attempts.sql`).
pub const READINGS_TABLE: &str = "readings";
/// Each model attempt at a reading (`migrations/004601_readings_and_attempts.sql`).
pub const READING_ATTEMPTS_TABLE: &str = "reading_attempts";

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
                TableRights {
                    table: READINGS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: READING_ATTEMPTS_TABLE,
                    disposition: Disposition::ExportAndErase,
                },
            ],
        )
    }

    #[allow(clippy::too_many_lines)] // one query per table, each with its own columns
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
            .fetch_all(&mut *connection)
            .await?;
            let readings = sqlx::query!(
                r#"SELECT id, topic, study_day, digest, persona, text, word_count,
                          reading_minutes, new_cards, note_count, card_ids, generated_at, version,
                          vault_status, vault_path, carried_nights, read_at, studied_count,
                          studied_verdict, studied_at, vault_tick, created_at
                   FROM readings ORDER BY generated_at, rowid"#
            )
            .fetch_all(&mut *connection)
            .await?;
            let attempts = sqlx::query!(
                r#"SELECT id AS "id!", run_id, topic, study_day, attempt, repair_gate, verdict,
                          cause, class, gate, turns, input_tokens, output_tokens, cost_micro_usd,
                          duration_ms, created_at
                   FROM reading_attempts ORDER BY id"#
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
                ExportedTable {
                    table: READINGS_TABLE,
                    rows: readings
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "topic": row.topic,
                                "study_day": row.study_day,
                                "digest": row.digest,
                                "persona": row.persona,
                                "text": row.text,
                                "word_count": row.word_count,
                                "reading_minutes": row.reading_minutes,
                                "new_cards": row.new_cards,
                                "note_count": row.note_count,
                                "card_ids": row.card_ids,
                                "generated_at": row.generated_at,
                                "version": row.version,
                                "vault_status": row.vault_status,
                                "vault_path": row.vault_path,
                                "carried_nights": row.carried_nights,
                                "read_at": row.read_at,
                                "studied_count": row.studied_count,
                                "studied_verdict": row.studied_verdict,
                                "studied_at": row.studied_at,
                                "vault_tick": row.vault_tick,
                                "created_at": row.created_at,
                            })
                        })
                        .collect(),
                },
                ExportedTable {
                    table: READING_ATTEMPTS_TABLE,
                    rows: attempts
                        .into_iter()
                        .map(|row| {
                            json!({
                                "id": row.id,
                                "run_id": row.run_id,
                                "topic": row.topic,
                                "study_day": row.study_day,
                                "attempt": row.attempt,
                                "repair_gate": row.repair_gate,
                                "verdict": row.verdict,
                                "cause": row.cause,
                                "class": row.class,
                                "gate": row.gate,
                                "turns": row.turns,
                                "input_tokens": row.input_tokens,
                                "output_tokens": row.output_tokens,
                                "cost_micro_usd": row.cost_micro_usd,
                                "duration_ms": row.duration_ms,
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
            // An attempt and a topic day name their run, so they go before the runs.
            sqlx::query!("DELETE FROM reading_attempts")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM reading_topic_days")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM readings")
                .execute(&mut *connection)
                .await?;
            sqlx::query!("DELETE FROM reading_runs")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}
