//! Every model attempt a reading made (SPEC-046 R12): `reading_attempts`, kept ninety days.

use deck_streak_kernel::{KernelError, StudyDay, UtcMillis};

use crate::state::{AgentCause, ReadingGate};
use crate::store::{RunId, SqliteReadings, StoreError};
use crate::topic::TopicKey;

/// How many days an attempt is kept.
pub const RETAIN_DAYS: i64 = 90;

/// How an attempt ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttemptOutcome {
    /// Every gate passed.
    Passed,
    /// A gate refused the output, in this class.
    GateFailed {
        /// The gate.
        gate: ReadingGate,
        /// The class the gate reported.
        class: String,
    },
    /// The route could not run the attempt.
    Unavailable(AgentCause),
}

/// What an attempt measured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttemptTelemetry {
    /// Turns.
    pub turns: u32,
    /// Input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// The cost estimate, in millionths of a US dollar.
    pub cost_micro_usd: u64,
    /// The duration, in milliseconds.
    pub duration_ms: u64,
}

/// One attempt to record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptRecord {
    /// The run.
    pub run: RunId,
    /// The topic.
    pub topic: TopicKey,
    /// The study day.
    pub study_day: StudyDay,
    /// The attempt's number: 1, or 2 for the repair.
    pub attempt: u8,
    /// The gate the repair named; set on attempt 2 only.
    pub repair_gate: Option<ReadingGate>,
    /// How it ended.
    pub outcome: AttemptOutcome,
    /// What it measured.
    pub telemetry: AttemptTelemetry,
    /// When it was recorded.
    pub at: UtcMillis,
}

/// Milliseconds in a day.
const DAY_MILLIS: i64 = 86_400_000;

/// The cutoff before which an attempt recorded is no longer kept, as of `now`.
#[must_use]
pub const fn retention_cutoff(now: UtcMillis) -> UtcMillis {
    UtcMillis::from_epoch_millis(now.epoch_millis().saturating_sub(RETAIN_DAYS * DAY_MILLIS))
}

/// A count as the column holds it.
fn column(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

impl SqliteReadings {
    /// Records `attempt`.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn record_attempt(&self, attempt: &AttemptRecord) -> Result<(), KernelError> {
        let run_id = attempt.run.get();
        let topic = attempt.topic.as_str();
        let study_day = attempt.study_day.epoch_day();
        let number = i64::from(attempt.attempt);
        let repair_gate = attempt.repair_gate.map(ReadingGate::as_str);
        let (verdict, cause, class, gate) = match &attempt.outcome {
            AttemptOutcome::Passed => ("passed", None, None, None),
            AttemptOutcome::GateFailed { gate, class } => (
                "gate_failed",
                None,
                Some(class.as_str()),
                Some(gate.as_str()),
            ),
            AttemptOutcome::Unavailable(cause) => ("unavailable", Some(cause.as_str()), None, None),
        };
        let turns = i64::from(attempt.telemetry.turns);
        let input_tokens = column(attempt.telemetry.input_tokens);
        let output_tokens = column(attempt.telemetry.output_tokens);
        let cost = column(attempt.telemetry.cost_micro_usd);
        let duration = column(attempt.telemetry.duration_ms);
        let created_at = attempt.at.epoch_millis();
        let mut write = self.db.write().await?;
        sqlx::query!(
            "INSERT INTO reading_attempts \
             (run_id, topic, study_day, attempt, repair_gate, verdict, cause, class, gate, turns, \
              input_tokens, output_tokens, cost_micro_usd, duration_ms, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            run_id,
            topic,
            study_day,
            number,
            repair_gate,
            verdict,
            cause,
            class,
            gate,
            turns,
            input_tokens,
            output_tokens,
            cost,
            duration,
            created_at
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
        Ok(())
    }

    /// Every attempt of `run`, in order.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the read fails, or a row names no closed gate, cause or topic.
    pub async fn attempts(&self, run: RunId) -> Result<Vec<AttemptRecord>, StoreError> {
        let run_id = run.get();
        let rows = sqlx::query!(
            r#"SELECT id AS "id!", topic, study_day, attempt, repair_gate, verdict, cause, class,
                      gate, turns, input_tokens, output_tokens, cost_micro_usd, duration_ms,
                      created_at
               FROM reading_attempts WHERE run_id = ?1 ORDER BY id"#,
            run_id
        )
        .fetch_all(self.db.reader())
        .await?;
        rows.into_iter()
            .map(|row| {
                let unreadable = StoreError::Unreadable {
                    table: "reading_attempts",
                    id: row.id,
                };
                let outcome = match row.verdict.as_str() {
                    "passed" => Some(AttemptOutcome::Passed),
                    "gate_failed" => row
                        .gate
                        .as_deref()
                        .and_then(ReadingGate::parse)
                        .zip(row.class.clone())
                        .map(|(gate, class)| AttemptOutcome::GateFailed { gate, class }),
                    "unavailable" => row
                        .cause
                        .as_deref()
                        .and_then(AgentCause::parse)
                        .map(AttemptOutcome::Unavailable),
                    _ => None,
                };
                let record = (|| {
                    Some(AttemptRecord {
                        run,
                        topic: TopicKey::parse(&row.topic)?,
                        study_day: StudyDay::from_epoch_day(row.study_day),
                        attempt: u8::try_from(row.attempt).ok()?,
                        repair_gate: match row.repair_gate.as_deref() {
                            Some(name) => Some(ReadingGate::parse(name)?),
                            None => None,
                        },
                        outcome: outcome?,
                        telemetry: AttemptTelemetry {
                            turns: u32::try_from(row.turns).ok()?,
                            input_tokens: u64::try_from(row.input_tokens).ok()?,
                            output_tokens: u64::try_from(row.output_tokens).ok()?,
                            cost_micro_usd: u64::try_from(row.cost_micro_usd).ok()?,
                            duration_ms: u64::try_from(row.duration_ms).ok()?,
                        },
                        at: UtcMillis::from_epoch_millis(row.created_at),
                    })
                })();
                record.ok_or(unreadable)
            })
            .collect()
    }

    /// Deletes the attempts recorded before `cutoff`, and returns how many.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn prune_attempts_before(&self, cutoff: UtcMillis) -> Result<u64, KernelError> {
        let cutoff = cutoff.epoch_millis();
        let mut write = self.db.write().await?;
        let deleted = sqlx::query!("DELETE FROM reading_attempts WHERE created_at < ?1", cutoff)
            .execute(&mut *write)
            .await?
            .rows_affected();
        write.commit().await?;
        Ok(deleted)
    }
}
