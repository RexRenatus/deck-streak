//! The runner: the one entrance every timer reaches, `deckstreakd job <id>` (SPEC-027 R5, R7;
//! ADR-027).
//!
//! A run finds the job's latest scheduled instant at or before now. A `catch_up` job looks only at
//! the fires of `now`'s local calendar day, as the predecessor's boot-time catch-up did
//! (`scheduler.py:run_startup_catchup` at `27ee2bc`): with none elapsed it does nothing, and more
//! than [`CATCHUP_MAX_LATE_MIN`](crate::jobs::CATCHUP_MAX_LATE_MIN) minutes late it records
//! `missed` and exits without acting. A job that fires once a day, `sync` included, claims its (job,
//! fire date) before it acts and does nothing when the claim fails. After the work, the outcome is
//! recorded, and a job that returned "not delivered" after the notifier reported an attempted send
//! with no message id releases its claim. The decision is proved against `goldens/catchup.json`.
//!
//! No job but `sync` runs the sync cycle (ADR-037): every job reads its study day's sync outcome,
//! which the runner hands it, and never waits on a sync.
//!
//! The exit code is the page (R7): 0 when the job ran, skipped or was recorded `missed`; 1 on a
//! transition, which fails the unit so `OnFailure=` sends the one alert; 2 for an id the table does
//! not hold. The runner's ERROR line carries reason codes and integers only.

use std::fmt;
use std::future::Future;
use std::ops::ControlFlow;

use deck_streak_ingest::sync::SyncReport;
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, StudyDayOutcome};
use deck_streak_kernel::{Clock, Db, KernelError, StudyDay, StudyDayRule, UtcMillis};

use crate::delivery::{DeliveryCounts, DeliveryMarker};
use crate::jobs::{self, CATCHUP_MAX_LATE_MIN, FireDate, Job};
use crate::ledger::{CronLedger, Outcome, Recorded};
use crate::liveness::LivenessWork;
use crate::maintenance::MaintenanceWork;

const MINUTE_MS: i64 = 60_000;

/// A reason code and the integers that go with it: all that an ERROR line of the runner says, so
/// no text of a failure, a path or a value can reach the journal or the alert (R7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reason {
    /// The reason code.
    pub code: &'static str,
    /// Named integers: counts, seconds, minutes.
    pub figures: Vec<(&'static str, i64)>,
}

impl Reason {
    /// The reason `code`, with no figure.
    #[must_use]
    pub const fn new(code: &'static str) -> Self {
        Self {
            code,
            figures: Vec::new(),
        }
    }

    /// The same reason with the figure `name` = `value` added.
    #[must_use]
    pub fn with(mut self, name: &'static str, value: i64) -> Self {
        self.figures.push((name, value));
        self
    }
}

impl fmt::Display for Reason {
    /// The code, then each figure as `name=value`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)?;
        for (name, value) in &self.figures {
            write!(f, " {name}={value}")?;
        }
        Ok(())
    }
}

/// One fire of a job, as the runner hands it to the job's work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fire {
    /// The job.
    pub job: Job,
    /// The fire's local calendar date: the ledger's key.
    pub fire_date: FireDate,
    /// The scheduled instant the run answers.
    pub scheduled_at: UtcMillis,
    /// When the run started.
    pub started_at: UtcMillis,
    /// The study day the scheduled instant falls in.
    pub study_day: StudyDay,
    /// That study day's sync outcome, if a sync started in it (ADR-037): what a job that needs
    /// the day's data reads instead of syncing.
    pub sync: Option<StudyDayOutcome>,
}

/// What a job's work returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    /// The work is done.
    Done,
    /// A notification job attempted its send and nothing was delivered: the predecessor's falsy
    /// return.
    NotDelivered,
    /// The work is done and found what the owner must hear: page with these reasons.
    Page(Vec<Reason>),
}

/// The work of one job: what the runner calls once it has decided the job runs.
pub trait Work: Send + Sync {
    /// Does the job's work for `fire`.
    ///
    /// # Errors
    ///
    /// The failure's reason code and figures: the run records `error`, and the first error of the
    /// job's error streak pages.
    fn perform(&self, fire: &Fire) -> impl Future<Output = Result<Done, Reason>> + Send;
}

/// The sync cycle as the `sync` job runs it: SPEC-022's cycle with the `scheduled` trigger, through
/// `coordination::sync_cycle` (SPEC-023 extends the cycle; this port never changes with it).
pub trait SyncCycle: Send + Sync {
    /// Runs the one scheduled sync of the study day.
    ///
    /// # Errors
    ///
    /// The reason the cycle could not run or be recorded. A failed sync is not an error here: it
    /// is a recorded run with its reason code, in the report.
    fn run_scheduled(&self) -> impl Future<Output = Result<SyncReport, Reason>> + Send;
}

/// What the runner decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// A catch-up job with no fire elapsed yet on `now`'s local calendar day: nothing to do.
    NothingElapsed,
    /// A catch-up job more than the cap late: recorded `missed` (or suppressed), not run.
    Missed {
        /// The fire's date.
        fire_date: FireDate,
        /// Whether the `missed` was written or suppressed.
        recorded: Recorded,
    },
    /// The claim failed: the fire already has an attempt, so the job did nothing.
    AlreadyClaimed {
        /// The fire's date.
        fire_date: FireDate,
    },
    /// The job ran.
    Ran {
        /// The fire's date.
        fire_date: FireDate,
        /// `ok`, or `error` when the work failed.
        outcome: Outcome,
        /// Whether the claim was released after a failed delivery.
        released: bool,
    },
    /// The id names no job of the table.
    UnknownJob,
}

/// One run's decision, and the transitions it pages on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// What the runner decided.
    pub decision: Decision,
    /// The transitions this run pages on; none, most runs.
    pub pages: Vec<Reason>,
}

impl Report {
    /// The exit code of a run that pages: the unit fails, and `OnFailure=` sends the alert.
    pub const PAGE: u8 = 1;
    /// The exit code of an id the table does not hold.
    pub const UNKNOWN_JOB: u8 = 2;

    /// A report of `decision` that pages on nothing.
    #[must_use]
    pub const fn quiet(decision: Decision) -> Self {
        Self {
            decision,
            pages: Vec::new(),
        }
    }

    /// The run's exit code (R7).
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        if self.decision == Decision::UnknownJob {
            Self::UNKNOWN_JOB
        } else if self.pages.is_empty() {
            0
        } else {
            Self::PAGE
        }
    }
}

/// The runner, over the ports every job's run uses.
pub struct Runner<'a, L> {
    ledger: &'a L,
    sync_runs: &'a SqliteSyncRuns,
    marker: &'a dyn DeliveryMarker,
    clock: &'a dyn Clock,
    rule: StudyDayRule,
}

impl<'a, L: CronLedger> Runner<'a, L> {
    /// A runner writing `ledger`, reading study-day outcomes from `sync_runs`, the notifier's
    /// counts from `marker`, and the time from `clock`, under the study-day `rule`.
    #[must_use]
    pub const fn new(
        ledger: &'a L,
        sync_runs: &'a SqliteSyncRuns,
        marker: &'a dyn DeliveryMarker,
        clock: &'a dyn Clock,
        rule: StudyDayRule,
    ) -> Self {
        Self {
            ledger,
            sync_runs,
            marker,
            clock,
            rule,
        }
    }

    /// The ledger the runner writes.
    #[must_use]
    pub const fn ledger(&self) -> &'a L {
        self.ledger
    }

    /// Runs `job`'s `work` once, as a timer's fire of it (R5, R7).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the ledger or the study day's outcome cannot be read or
    /// written: the run then fails its unit, which pages.
    pub async fn run<W: Work>(&self, job: &Job, work: &W) -> Result<Report, KernelError> {
        let now = self.clock.now();
        let scheduled = match self.scheduled(job, now).await? {
            ControlFlow::Continue(scheduled) => scheduled,
            ControlFlow::Break(report) => return Ok(report),
        };
        let fire_date = FireDate::of(scheduled, self.rule.utc_offset());
        // The streak this run may break or extend, read before the run writes anything.
        let previous = self
            .ledger
            .latest(job.id)
            .await?
            .map(|row| row.last_outcome);
        if job.once_a_day() && !self.ledger.claim(job.id, fire_date, now).await? {
            tracing::info!(
                job = job.id,
                fire_date = fire_date.epoch_day(),
                "the fire already has an attempt: nothing to do"
            );
            return Ok(Report::quiet(Decision::AlreadyClaimed { fire_date }));
        }
        let study_day = self.rule.study_day(scheduled);
        let fire = Fire {
            job: *job,
            fire_date,
            scheduled_at: scheduled,
            started_at: now,
            study_day,
            sync: self.sync_runs.study_day_outcome(study_day).await?,
        };
        let before = self.marker.counts();
        let performed = work.perform(&fire).await;
        let outcome = if performed.is_ok() {
            Outcome::Ok
        } else {
            Outcome::Error
        };
        self.ledger
            .record(
                job.id,
                fire_date,
                outcome,
                job.once_a_day(),
                self.clock.now(),
            )
            .await?;
        let released = self.release_undelivered(&fire, &performed, before).await?;
        let pages = pages(&fire, performed, previous);
        tracing::info!(
            job = job.id,
            fire_date = fire_date.epoch_day(),
            outcome = outcome.as_str(),
            released,
            "the job ran"
        );
        Ok(Report {
            decision: Decision::Ran {
                fire_date,
                outcome,
                released,
            },
            pages,
        })
    }

    /// The scheduled instant a run of `job` at `now` answers, or, for a catch-up job that does not
    /// run, its report: no fire of the day has elapsed, or the fire is too late and is recorded
    /// `missed`.
    async fn scheduled(
        &self,
        job: &Job,
        now: UtcMillis,
    ) -> Result<ControlFlow<Report, UtcMillis>, KernelError> {
        if !job.catch_up {
            return Ok(ControlFlow::Continue(
                job.schedule.latest_at_or_before(now, self.rule),
            ));
        }
        let Some(scheduled) = job.schedule.latest_elapsed_today(now, self.rule) else {
            tracing::info!(
                job = job.id,
                "no fire of the day has elapsed: nothing to catch up"
            );
            return Ok(ControlFlow::Break(Report::quiet(Decision::NothingElapsed)));
        };
        let late = now.epoch_millis().saturating_sub(scheduled.epoch_millis());
        if late <= CATCHUP_MAX_LATE_MIN * MINUTE_MS {
            return Ok(ControlFlow::Continue(scheduled));
        }
        let fire_date = FireDate::of(scheduled, self.rule.utc_offset());
        let recorded = self
            .ledger
            .record(job.id, fire_date, Outcome::Missed, job.once_a_day(), now)
            .await?;
        tracing::warn!(
            job = job.id,
            fire_date = fire_date.epoch_day(),
            late_min = late / MINUTE_MS,
            suppressed = recorded == Recorded::Suppressed,
            "the fire is too late to run: it is recorded missed"
        );
        Ok(ControlFlow::Break(Report::quiet(Decision::Missed {
            fire_date,
            recorded,
        })))
    }

    /// Releases `fire`'s claim when the notifier positively reported a failed delivery: the work
    /// returned "not delivered", and a send was attempted during it with no message id back. A job
    /// that never engaged the notifier keeps its claim, so its fire latches instead of retrying.
    async fn release_undelivered(
        &self,
        fire: &Fire,
        performed: &Result<Done, Reason>,
        before: DeliveryCounts,
    ) -> Result<bool, KernelError> {
        if !(fire.job.once_a_day() && matches!(performed, Ok(Done::NotDelivered))) {
            return Ok(false);
        }
        let after = self.marker.counts();
        if after.attempted <= before.attempted || after.delivered > before.delivered {
            return Ok(false);
        }
        self.ledger
            .release(fire.job.id, fire.fire_date, self.clock.now())
            .await?;
        tracing::warn!(
            job = fire.job.id,
            fire_date = fire.fire_date.epoch_day(),
            "a send was attempted and nothing was delivered: the claim is released"
        );
        Ok(true)
    }

    /// Runs the table's job `id` with its own work: `sync` runs `cycle`, `maintenance` the
    /// database's upkeep on `db`, and `liveness` the dead-man watch and the drift check. No job
    /// but `sync` is handed the cycle.
    ///
    /// # Errors
    ///
    /// As [`Runner::run`].
    pub async fn run_job<C: SyncCycle>(
        &self,
        id: &str,
        cycle: &C,
        db: &Db,
    ) -> Result<Report, KernelError> {
        match jobs::job(id) {
            Some(job) if job == jobs::SYNC => self.run(&job, &SyncWork::new(cycle)).await,
            Some(job) if job == jobs::MAINTENANCE => {
                self.run(&job, &MaintenanceWork::new(db)).await
            }
            Some(job) if job == jobs::LIVENESS => {
                let watch = LivenessWork::new(self.ledger, self.sync_runs, db, self.rule);
                self.run(&job, &watch).await
            }
            _ => {
                tracing::error!("the job table holds no job of that id");
                Ok(Report::quiet(Decision::UnknownJob))
            }
        }
    }
}

/// The pages of `fire`'s run: what the work asked for, or its failure when it opens the job's error
/// streak, the job's `previous` outcome not being an error. A repeat failure only logs. Each page
/// is one ERROR line of reason codes and integers.
fn pages(fire: &Fire, performed: Result<Done, Reason>, previous: Option<Outcome>) -> Vec<Reason> {
    let (job, fire_date) = (fire.job.id, fire.fire_date.epoch_day());
    let pages = match performed {
        Ok(Done::Page(pages)) => pages,
        Err(reason) if previous == Some(Outcome::Error) => {
            tracing::warn!(job, fire_date, reason = %reason, "the job failed again: a repeat only logs");
            Vec::new()
        }
        Err(reason) => vec![reason],
        Ok(Done::Done | Done::NotDelivered) => Vec::new(),
    };
    for page in &pages {
        tracing::error!(job, fire_date, reason = %page, "the job pages");
    }
    pages
}

/// The `sync` job's work: the one scheduled sync of the study day, and a failed sync reported as
/// the job's failure with its reason code and attempts.
pub struct SyncWork<'a, C> {
    cycle: &'a C,
}

impl<'a, C> SyncWork<'a, C> {
    /// The work that runs `cycle`.
    #[must_use]
    pub const fn new(cycle: &'a C) -> Self {
        Self { cycle }
    }
}

impl<C: SyncCycle> Work for SyncWork<'_, C> {
    async fn perform(&self, _fire: &Fire) -> Result<Done, Reason> {
        match self.cycle.run_scheduled().await? {
            SyncReport::Ran { run, .. } => match run.outcome {
                Ok(()) => Ok(Done::Done),
                Err(code) => {
                    Err(Reason::new(code.as_str()).with("attempts", i64::from(run.attempts)))
                }
            },
            // Refused (a scheduled run already recorded this study day) or debounced: the day's
            // sync has its outcome, and this fire adds none.
            SyncReport::RefusedToday | SyncReport::Debounced { .. } => Ok(Done::Done),
        }
    }
}
