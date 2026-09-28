//! The `job` role: runs one job of the table by its id, and exits (SPEC-027 R5, R7, R11; ADR-027).
//!
//! Each job is a `oneshot` unit its timer starts, so the role sends no `sd_notify(3)` message: it
//! installs logging and reads its settings like every role, opens the database under the open lock,
//! runs the job once through `coordination::runner`, and exits with the runner's code. 0 means the
//! job ran, skipped or was recorded `missed`; 1 is a page, which fails the unit so `OnFailure=` sends
//! the one alert; 2 is an id the table does not hold.
//!
//! The ports are joined here, in the composition root. The notifier's marker is `NoNotifier` until
//! the bot's transport exists (SPEC-026), and the `sync` job's cycle builds its syncer from the
//! sync's own settings only when `sync` runs, so the other jobs start without them.

use std::sync::Arc;

use deck_streak_coordination::delivery::NoNotifier;
use deck_streak_coordination::jobs::Job;
use deck_streak_coordination::ledger::SqliteCronLedger;
use deck_streak_coordination::runner::{Reason, Runner, SyncCycle};
use deck_streak_coordination::sync_cycle::sync_cycle;
use deck_streak_daemon::wiring::{self, StateDirectory, WiringError};
use deck_streak_ingest::engine::RslibEngine;
use deck_streak_ingest::settings::SyncSettings;
use deck_streak_ingest::sync::{SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, Db, Environment, KernelError, KernelSettings, Offload,
    Redactor, SettingsError, StudyDayRule, SystemClock,
};

/// Why the `job` role stopped before its job could report.
#[derive(Debug, thiserror::Error)]
pub enum JobRoleError {
    /// A setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The database could not be opened.
    #[error("the database could not be opened")]
    Database(#[source] WiringError),
    /// The ledger or the study day's outcome could not be read or written.
    #[error("the job's ledger could not be read or written")]
    Ledger(#[source] KernelError),
}

/// Runs `job` once, and returns the process's exit code.
///
/// # Errors
///
/// Every refusal of [`JobRoleError`]: a setting refuses start, the database cannot be opened, or
/// the ledger cannot be read or written. Each fails the unit, as a page does.
pub async fn run(env: &Environment, redactor: &Redactor, job: &Job) -> Result<u8, JobRoleError> {
    let kernel = KernelSettings::from_env(env)?;
    let state = StateDirectory::from_env(env)?;
    let offload = Offload::new(kernel.offload_workers, Arc::new(SystemClock));
    let db = wiring::open_database(&offload, &state)
        .await
        .map_err(JobRoleError::Database)?;
    let ledger = SqliteCronLedger::new(db.clone());
    let sync_runs = SqliteSyncRuns::new(db.clone());
    let rule = kernel.study_day_rule;
    let runner = Runner::new(&ledger, &sync_runs, &NoNotifier, &SystemClock, rule);
    let cycle = ScheduledSync {
        env,
        redactor,
        db: &db,
        rule,
    };
    let report = runner.run_job(job.id, &cycle, &db).await;
    db.close().await;
    Ok(report.map_err(JobRoleError::Ledger)?.exit_code())
}

/// The `sync` job's cycle in production: SPEC-022's syncer over Anki's engine, built from the
/// sync's settings and credentials when the job runs, through `coordination::sync_cycle`.
struct ScheduledSync<'a> {
    env: &'a Environment,
    redactor: &'a Redactor,
    db: &'a Db,
    rule: StudyDayRule,
}

impl SyncCycle for ScheduledSync<'_> {
    async fn run_scheduled(&self) -> Result<SyncReport, Reason> {
        let settings = SyncSettings::from_env(self.env).map_err(|refusal| {
            tracing::error!(%refusal, "the sync's settings refuse it");
            Reason::new("sync_settings_refused")
        })?;
        let directory = CredentialsDirectory::from_env(self.env).map_err(|refusal| {
            tracing::error!(%refusal, "the sync's credentials directory refuses it");
            Reason::new("credentials_directory_refused")
        })?;
        let syncer = Syncer::new(
            RslibEngine,
            SqliteSyncRuns::new(self.db.clone()),
            settings,
            CredentialLoader::new(directory, self.redactor.clone()),
            Arc::new(SystemClock),
            self.rule,
        );
        sync_cycle(&syncer, Trigger::Scheduled)
            .await
            .map_err(|_| Reason::new("sync_record_failed"))
    }
}
