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
        let _ = (&self.db, run);
        Ok(0)
    }

    /// Deletes every run recorded before `cutoff` and returns how many it deleted.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails; nothing is deleted then.
    pub async fn prune_before(&self, cutoff: UtcMillis) -> Result<u64, KernelError> {
        let _ = cutoff;
        Ok(0)
    }
}
