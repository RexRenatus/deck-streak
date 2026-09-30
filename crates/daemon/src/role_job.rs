//! The `job` role: runs one job of the table by its id, and exits (SPEC-027 R5, R7, R11; ADR-027).
//!
//! Each job is a `oneshot` unit its timer starts, so the role sends no `sd_notify(3)` message: it
//! installs logging and reads its settings like every role, opens the database under the open lock,
//! runs the job once through `coordination::runner`, and exits with the runner's code. 0 means the
//! job ran, skipped or was recorded `missed`; 1 is a page, which fails the unit so `OnFailure=` sends
//! the one alert; 2 is an id the table does not hold.
//!
//! The ports are joined here, in the composition root. The notifier's marker is `NoNotifier`: no job
//! of the W0 table sends a message, so the role builds no bot transport and loads no bot token, and
//! the first job that sends joins wiring's `TransportMarker` (SPEC-026 R10; #20, #27). The `sync`
//! job's cycle builds its syncer, its reader and its change gate from the sync's own settings only
//! when `sync` runs, so the other jobs start without them (SPEC-023 R12); so it loads its recompute,
//! wiring's `RecomputeSetup`: the owner's courses, agreed with the readings taxonomy, their digest
//! recorded, and the fold (SPEC-071 R1, R3, R4, R15). No obligation source is registered yet: each deadline-bearing feature
//! registers its own.

use std::sync::Arc;

use deck_streak_coordination::delivery::NoNotifier;
use deck_streak_coordination::drills::{DrillNotes, DrillPostbackWork, RealFs};
use deck_streak_coordination::jobs::{DRILL_POSTBACK, Job, SYNC};
use deck_streak_coordination::ledger::SqliteCronLedger;
use deck_streak_coordination::obligations::Obligations;
use deck_streak_coordination::runner::{Reason, Runner, SyncCycle};
use deck_streak_coordination::sync_cycle::{CycleError, CycleParts, sync_cycle};
use deck_streak_daemon::sync_request::owner_request_pending;
use deck_streak_daemon::wiring::{
    self, OwnerSyncCycle, RecomputeSetup, StateDirectory, WiringError,
};
use deck_streak_ingest::engine::RslibEngine;
use deck_streak_ingest::gate::ChangeGate;
use deck_streak_ingest::settings::{ScopeSettings, SyncSettings};
use deck_streak_ingest::state::{RefusalReason, SqliteIngestState};
use deck_streak_ingest::sync::{SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    Clock, CredentialLoader, CredentialsDirectory, Db, Environment, KernelError, KernelSettings,
    Offload, Redactor, SettingsError, StudyDayRule, SystemClock,
};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_vault::{Rails, RailsError, VaultSettings};

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
    /// The vault's content rails could not be read (SPEC-110).
    #[error("the vault's content rails could not be read")]
    Rails(#[source] RailsError),
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
        offload: offload.clone(),
        rule,
    };
    if job.id == SYNC.id {
        serve_owner_request(env, redactor, &db, &offload, rule).await;
    }
    let report = if job.id == DRILL_POSTBACK.id {
        // The drill post-back joins the table without a sync cycle (SPEC-110 R9): its work is the
        // vault's graded drills, so it runs through the runner's own `run`.
        let settings = VaultSettings::from_env(env)?;
        let rails = Rails::vendored().map_err(JobRoleError::Rails)?;
        let notes = DrillNotes::open(&settings, RealFs, rails);
        let grants = SqliteXpLedger::new(db.clone());
        runner
            .run(job, &DrillPostbackWork::new(notes, &db, &grants))
            .await
    } else {
        runner.run_job(job.id, &cycle, &db).await
    };
    db.close().await;
    Ok(report.map_err(JobRoleError::Ledger)?.exit_code())
}

/// Serves the owner's stored request, when there is one (SPEC-059; ADR-066): one owner cycle, with
/// R17's reuse window, before the scheduled run. Only the stored flag says the owner asked: the
/// file that rang the doorbell is never read. A refusal is logged and the scheduled run goes on,
/// so the timer's own fire is never lost to a request.
async fn serve_owner_request(
    env: &Environment,
    redactor: &Redactor,
    db: &Db,
    offload: &Offload,
    rule: StudyDayRule,
) {
    match owner_request_pending(db).await {
        Ok(false) => {}
        Ok(true) => {
            let recompute = match RecomputeSetup::load(env, db).await {
                Ok(recompute) => recompute,
                Err(error) => {
                    tracing::error!(%error, "the recompute refuses the owner's request");
                    record_refusal(db, RefusalReason::RecomputeRefused).await;
                    return;
                }
            };
            let cycle = OwnerSyncCycle::new(
                env.clone(),
                redactor.clone(),
                db.clone(),
                offload.clone(),
                rule,
                recompute,
            );
            match cycle.run().await {
                Ok(answer) => tracing::info!(?answer, "the owner's request was served"),
                Err(refusal) => {
                    tracing::error!(reason = refusal.reason, "the owner's request was refused");
                    if let Some(reason) = RefusalReason::parse(refusal.reason) {
                        record_refusal(db, reason).await;
                    }
                }
            }
        }
        Err(error) => tracing::error!(%error, "the owner's request could not be read"),
    }
}

/// Records the refusal of the owner's request, so the flag is clear and the owner is answered with
/// the code (SPEC-128). A failed write is logged and changes nothing else: the scheduled run goes on.
async fn record_refusal(db: &Db, reason: RefusalReason) {
    if let Err(error) = SqliteIngestState::new(db.clone())
        .record_refusal(reason, SystemClock.now())
        .await
    {
        tracing::error!(%error, "the owner's refusal could not be recorded");
    }
}

/// The `sync` job's cycle in production: SPEC-022's syncer over Anki's engine, SPEC-023's reader and
/// change gate, built from the sync's settings and credentials when the job runs, through
/// `coordination::sync_cycle`.
struct ScheduledSync<'a> {
    env: &'a Environment,
    redactor: &'a Redactor,
    db: &'a Db,
    offload: Offload,
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
        let scope = ScopeSettings::from_env(self.env).map_err(|refusal| {
            tracing::error!(%refusal, "the read's scope refuses it");
            Reason::new("scope_settings_refused")
        })?;
        let recompute = RecomputeSetup::load(self.env, self.db)
            .await
            .map_err(|refusal| {
                tracing::error!(%refusal, "the recompute refuses the sync");
                Reason::new("recompute_refused")
            })?;
        let clock = Arc::new(SystemClock);
        let reader = recompute.reader(&settings, scope, self.offload.clone());
        let syncer = Syncer::new(
            RslibEngine,
            SqliteSyncRuns::new(self.db.clone()),
            settings,
            CredentialLoader::new(directory, self.redactor.clone()),
            clock.clone(),
            self.rule,
        );
        let gate = ChangeGate::new(self.db.clone(), self.rule, clock.clone());
        let parts = recompute.cycle(
            CycleParts::new(syncer, reader, gate, Obligations::new(), clock),
            self.db.clone(),
            self.rule,
        );
        sync_cycle(&parts, Trigger::Scheduled)
            .await
            .map(|report| report.sync)
            .map_err(|error| cycle_failed(&error))
    }
}

/// The reason a cycle that could not run to its end is recorded with: the sync's record, an
/// obligation, or the recompute after the sync. A failed sync is not one: it is a recorded run.
fn cycle_failed(error: &CycleError) -> Reason {
    tracing::error!(%error, "the sync cycle could not run to its end");
    Reason::new(match error {
        CycleError::History(_) | CycleError::Sync(_) => "sync_record_failed",
        CycleError::Obligations(_) => "obligations_unreadable",
        CycleError::Gate(_) | CycleError::Window(_) | CycleError::Recompute(_) => {
            "recompute_failed"
        }
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use deck_streak_coordination::runner::Reason;
    use deck_streak_coordination::sync_cycle::CycleError;
    use deck_streak_ingest::gate::GateError;
    use deck_streak_ingest::reader::ReadError;
    use deck_streak_ingest::sync::SyncError;
    use deck_streak_ingest::window::WindowError;
    use deck_streak_kernel::KernelError;

    use super::cycle_failed;

    /// A cause no step of a cycle names: the mapping reads the step, never the cause.
    const fn cause() -> KernelError {
        KernelError::LoggingInstalled
    }

    /// One failure of every kind a cycle can stop with, each inner kind of the gate and the window
    /// included, and the reason code the `sync` job records it with.
    fn every_failure() -> Vec<(CycleError, &'static str)> {
        vec![
            (CycleError::History(cause()), "sync_record_failed"),
            (
                CycleError::Sync(SyncError::Store(cause())),
                "sync_record_failed",
            ),
            (CycleError::Obligations(cause()), "obligations_unreadable"),
            (
                CycleError::Gate(GateError::Read(ReadError::WriteRefused)),
                "recompute_failed",
            ),
            (
                CycleError::Gate(GateError::Record(cause())),
                "recompute_failed",
            ),
            (
                CycleError::Window(WindowError::Read(ReadError::WriteRefused)),
                "recompute_failed",
            ),
            (
                CycleError::Window(WindowError::State(cause())),
                "recompute_failed",
            ),
            (CycleError::Recompute(cause()), "recompute_failed"),
        ]
    }

    /// The failure's kind. The match is exhaustive, so a kind added to `CycleError` does not compile
    /// here until `every_failure` gives it a case and a code.
    const fn kind(error: &CycleError) -> &'static str {
        match error {
            CycleError::History(_) => "history",
            CycleError::Sync(_) => "sync",
            CycleError::Obligations(_) => "obligations",
            CycleError::Gate(_) => "gate",
            CycleError::Window(_) => "window",
            CycleError::Recompute(_) => "recompute",
        }
    }

    #[test]
    fn a_cycle_that_cannot_finish_is_recorded_with_its_steps_reason_code() {
        let failures = every_failure();
        let kinds: BTreeSet<&str> = failures.iter().map(|(error, _)| kind(error)).collect();
        assert_eq!(
            kinds,
            BTreeSet::from([
                "gate",
                "history",
                "obligations",
                "recompute",
                "sync",
                "window"
            ]),
            "every kind of failure has a case"
        );
        for (error, code) in &failures {
            assert_eq!(cycle_failed(error), Reason::new(code), "{error:?}");
        }
    }
}
