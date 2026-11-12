//! The record of each duty run (SPEC-043 R11, R12, R16): `agent_runs`.

use deck_streak_kernel::{Db, KernelError, UtcMillis};

use crate::verdict::{Telemetry, Verdict};

/// The table this context owns (`migrations/004301_agent_runs.sql`).
pub const AGENT_RUNS_TABLE: &str = "agent_runs";

/// How long a run's record is kept, in days (`privacy.json`'s `agent-runs` retention).
pub const RETENTION_DAYS: i64 = 90;

/// One run to record.
#[derive(Clone, Debug)]
pub struct RunRecord<'a> {
    /// The duty's name.
    pub duty: &'a str,
    /// The persona template's id.
    pub template: &'a str,
    /// The subject the run was for.
    pub subject: &'a str,
    /// How it ended.
    pub verdict: &'a Verdict,
    /// What the runner measured; `None` when nothing ran or the run failed before a reply.
    pub telemetry: Option<Telemetry>,
    /// When the run ended.
    pub at: UtcMillis,
}

/// The `agent_runs` writer.
#[derive(Clone, Debug)]
pub struct AgentRuns {
    db: Db,
}

impl AgentRuns {
    /// A writer over `db`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// Records one run and returns its id.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails; nothing is written then.
    pub async fn record(&self, run: &RunRecord<'_>) -> Result<i64, KernelError> {
        let verdict = run.verdict.name();
        let cause = match run.verdict {
            Verdict::Unavailable(cause) => Some(cause.as_str()),
            _ => None,
        };
        let class = match run.verdict {
            Verdict::Withheld(withheld) => Some(withheld.class.as_str()),
            _ => None,
        };
        let telemetry = match run.verdict {
            Verdict::AiRouteAbsent => None,
            Verdict::Delivered(delivered) => Some(delivered.telemetry),
            _ => Some(run.telemetry.unwrap_or_default()),
        };
        let turns = telemetry.map(|t| i64::from(t.turns));
        let input = telemetry.map(|t| i64::try_from(t.input_tokens).unwrap_or(i64::MAX));
        let output = telemetry.map(|t| i64::try_from(t.output_tokens).unwrap_or(i64::MAX));
        let cost = telemetry.map(|t| i64::try_from(t.cost_micro_usd).unwrap_or(i64::MAX));
        let duration = telemetry.map(|t| i64::try_from(t.duration_ms).unwrap_or(i64::MAX));
        let at = run.at.epoch_millis();
        let mut write = self.db.write().await?;
        let id = sqlx::query_scalar!(
            r#"INSERT INTO agent_runs
                   (duty, template, subject, verdict, cause, class, turns, input_tokens,
                    output_tokens, cost_micro_usd, duration_ms, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
               RETURNING id AS "id!: i64""#,
            run.duty,
            run.template,
            run.subject,
            verdict,
            cause,
            class,
            turns,
            input,
            output,
            cost,
            duration,
            at
        )
        .fetch_one(&mut *write)
        .await?;
        write.commit().await?;
        Ok(id)
    }

    /// Deletes every run recorded before `cutoff` and returns how many it deleted.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails; nothing is deleted then.
    pub async fn prune_before(&self, cutoff: UtcMillis) -> Result<u64, KernelError> {
        let cutoff = cutoff.epoch_millis();
        let mut write = self.db.write().await?;
        let done = sqlx::query!("DELETE FROM agent_runs WHERE created_at < ?1", cutoff)
            .execute(&mut *write)
            .await?;
        write.commit().await?;
        Ok(done.rows_affected())
    }
}
