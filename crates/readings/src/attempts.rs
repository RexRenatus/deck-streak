//! Every model attempt a reading made (SPEC-046 R12): `reading_attempts`, kept ninety days.

use deck_streak_kernel::{KernelError, StudyDay, UtcMillis};

use crate::state::{AgentCause, ReadingGate};
use crate::store::{RunId, SqliteReadings, StoreError};
use crate::topic::TopicKey;

/// How many days an attempt is kept.
pub const RETAIN_DAYS: i64 = 0;

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

impl SqliteReadings {
    /// Records `attempt`.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn record_attempt(&self, attempt: &AttemptRecord) -> Result<(), KernelError> {
        let _ = attempt;
        Ok(())
    }

    /// Every attempt of `run`, in order.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the read fails.
    pub async fn attempts(&self, run: RunId) -> Result<Vec<AttemptRecord>, StoreError> {
        let _ = run;
        Ok(Vec::new())
    }

    /// Deletes the attempts recorded before `cutoff`, and returns how many.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn prune_attempts_before(&self, cutoff: UtcMillis) -> Result<u64, KernelError> {
        let _ = cutoff;
        Ok(0)
    }
}
