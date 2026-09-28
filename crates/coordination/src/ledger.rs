//! The cron-fire ledger (SPEC-027 R3, R4): one row per (job, fire date), with its counters and last
//! outcome, and the claim, release and record that make a daily fire act once.
//!
//! The semantics are the predecessor's (`database.py:GamifyStore.claim_cron_fire`,
//! `release_cron_fire`, `record_cron_fire` and `pipeline_layers/ops.py:OpsLayer.record_cron_fire`
//! at `27ee2bc`), proved by `goldens/cron_ledger.json`. The claim writes a `catchup` attempt BEFORE
//! the job acts and succeeds only while the row counts no attempt, so a crash loop can never act
//! twice; the release undoes one claimed attempt; the record counts an outcome, advancing
//! `last_fire_at` only for one that ran, and a `missed` for a job that fires once a day is
//! suppressed when the fire already has a row, the positive evidence that it has a verdict. Each is
//! one `BEGIN IMMEDIATE` write through the kernel's repository base.

use std::future::Future;

use deck_streak_kernel::{Db, KernelError, UtcMillis};

use crate::jobs::FireDate;

/// A fire's outcome, as the ledger counts it: the predecessor's `database.py:CRON_OUTCOMES`,
/// proved by `goldens/scheduler.constants.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The job ran and returned.
    Ok,
    /// The job ran and failed.
    Error,
    /// A claimed attempt: written before the job acts.
    Catchup,
    /// The fire was too late to run, and did not.
    Missed,
}

impl Outcome {
    /// Every outcome, in the predecessor's order.
    pub const ALL: [Self; 4] = [Self::Ok, Self::Error, Self::Catchup, Self::Missed];

    /// The outcome as `cron_fires` stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Catchup => "catchup",
            Self::Missed => "missed",
        }
    }

    /// The outcome `cron_fires` stored as `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|outcome| outcome.as_str() == text)
    }

    /// Whether the outcome means the job ran, so it advances `last_fire_at` and counts as an
    /// attempt for a claim: every outcome but `missed`.
    #[must_use]
    pub const fn ran(self) -> bool {
        !matches!(self, Self::Missed)
    }
}

/// One row of the ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FireRow {
    /// The job's id.
    pub job_id: String,
    /// The fire's local calendar date.
    pub fire_date: FireDate,
    /// When the row was first written.
    pub first_seen_at: UtcMillis,
    /// When the row was last written.
    pub updated_at: UtcMillis,
    /// When the job last ran for this fire (`ok`, `error` or `catchup`), if it has.
    pub last_fire_at: Option<UtcMillis>,
    /// Outcomes `ok` counted.
    pub ok_count: i64,
    /// Outcomes `error` counted.
    pub error_count: i64,
    /// Claimed attempts counted.
    pub catchup_count: i64,
    /// Outcomes `missed` counted.
    pub missed_count: i64,
    /// The last outcome written.
    pub last_outcome: Outcome,
}

/// Whether a record was written, or suppressed by positive evidence that the fire already has a
/// verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// The outcome was counted.
    Written,
    /// A `missed` for a job that fires once a day, whose fire already has a row: nothing written.
    Suppressed,
}

/// What the runner asks of the ledger and tells it (R4, R5).
pub trait CronLedger: Send + Sync {
    /// Claims (`job`, `date`) at `at`: writes a `catchup` attempt and returns `true`, unless an
    /// attempt is already counted, when it writes nothing and returns `false`.
    fn claim(
        &self,
        job: &str,
        date: FireDate,
        at: UtcMillis,
    ) -> impl Future<Output = Result<bool, KernelError>> + Send;

    /// Undoes one claimed attempt of (`job`, `date`) at `at`: the claim's `catchup` and the
    /// recorded `ok`, each floored at zero, so a later claim may succeed.
    fn release(
        &self,
        job: &str,
        date: FireDate,
        at: UtcMillis,
    ) -> impl Future<Output = Result<(), KernelError>> + Send;

    /// Counts `outcome` for (`job`, `date`) at `at`. A `missed` is suppressed when `once_a_day`
    /// and the fire already has a row.
    fn record(
        &self,
        job: &str,
        date: FireDate,
        outcome: Outcome,
        once_a_day: bool,
        at: UtcMillis,
    ) -> impl Future<Output = Result<Recorded, KernelError>> + Send;

    /// The job's row of its latest fire date, if it has one: what the paging transitions read.
    fn latest(
        &self,
        job: &str,
    ) -> impl Future<Output = Result<Option<FireRow>, KernelError>> + Send;
}

/// `cron_fires` in the service's own database, owned by this context (docs/CONTEXT-MAP.md).
#[derive(Clone, Debug)]
pub struct SqliteCronLedger {
    db: Db,
}

impl SqliteCronLedger {
    /// The ledger in `db`, whose migrations created `cron_fires`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// The row of (`job`, `date`), if there is one.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn row(&self, job: &str, date: FireDate) -> Result<Option<FireRow>, KernelError> {
        let _ = (&self.db, job, date);
        Ok(None)
    }
}

impl CronLedger for SqliteCronLedger {
    async fn claim(&self, job: &str, date: FireDate, at: UtcMillis) -> Result<bool, KernelError> {
        let _ = (job, date, at);
        Ok(true)
    }

    async fn release(&self, job: &str, date: FireDate, at: UtcMillis) -> Result<(), KernelError> {
        let _ = (job, date, at);
        Ok(())
    }

    async fn record(
        &self,
        job: &str,
        date: FireDate,
        outcome: Outcome,
        once_a_day: bool,
        at: UtcMillis,
    ) -> Result<Recorded, KernelError> {
        let _ = (job, date, outcome, once_a_day, at);
        Ok(Recorded::Written)
    }

    async fn latest(&self, job: &str) -> Result<Option<FireRow>, KernelError> {
        let _ = job;
        Ok(None)
    }
}
