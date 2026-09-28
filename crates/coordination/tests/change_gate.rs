//! The obligations registry and the change gate (SPEC-023 A18, A19): a registered deadline that
//! passes runs the recompute on a cycle in which nothing else changed, exactly once, and a deadline
//! not yet due, or already served, lets the gate skip.
//!
//! Each test runs whole sync cycles over a temporary deployment: an engine whose every sync finds no
//! change, an empty copy the engine itself created, the service's database, and a synthetic
//! obligation source registered in the registry, as a deadline-bearing feature registers its own.
//! No feature's real deadlines exist yet. Every clock is a `ManualClock`.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::obligations::{ObligationSource, Obligations};
use deck_streak_coordination::sync_cycle::{CycleParts, Recompute, sync_cycle};
use deck_streak_ingest::engine::{
    AnkiEngine, EngineError, NewCardQueue, RslibEngine, SyncLogin, SyncOutcome,
};
use deck_streak_ingest::gate::{ChangeGate, Deadline, RunReason};
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::settings::{
    STATE_DIRECTORY, SYNC_ENDPOINT, SYNC_PASSWORD, SYNC_USERNAME, ScopeSettings, SyncSettings,
};
use deck_streak_ingest::sync::Syncer;
use deck_streak_ingest::sync_runs::{RunStatus, SqliteSyncRuns, Trigger};
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, Db, Environment, ManualClock, Offload, OffloadWorkers,
    PortFuture, Redactor, StudyDayRule, UtcMillis,
};

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
/// A study day's 05:00 in UTC, an hour past its 04:00 rollover (the default rule).
const START: i64 = 20_000 * DAY_MS + 5 * 60 * MINUTE_MS;
/// The label the synthetic source gives its deadlines.
const LABEL: &str = "synthetic_deadline";

/// An engine whose every sync finds no change: the collection stays as it is between cycles.
#[derive(Clone, Copy, Default)]
struct Unchanging;

impl AnkiEngine for Unchanging {
    fn new_card_queue(&self, _collection: &Path) -> Result<NewCardQueue, EngineError> {
        Ok(NewCardQueue::default())
    }

    async fn normal_sync(
        &self,
        _collection: &Path,
        _login: &SyncLogin,
    ) -> Result<SyncOutcome, EngineError> {
        Ok(SyncOutcome::NoChanges)
    }

    async fn full_download(
        &self,
        _collection: &Path,
        _login: &SyncLogin,
    ) -> Result<(), EngineError> {
        Ok(())
    }
}

/// A synthetic obligation source: the deadlines a test opens.
#[derive(Clone, Default)]
struct Synthetic(Arc<Mutex<Vec<Deadline>>>);

impl Synthetic {
    fn open(&self, at: i64) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Deadline {
                label: LABEL,
                at: UtcMillis::from_epoch_millis(at),
            });
    }
}

impl ObligationSource for Synthetic {
    fn name(&self) -> &'static str {
        "synthetic"
    }

    fn deadlines(&self, _now: UtcMillis) -> PortFuture<'_, Vec<Deadline>> {
        let open = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        Box::pin(async move { Ok(open) })
    }
}

/// A deployment in a scratch directory, removed when it drops.
struct Deployment {
    _scratch: tempfile::TempDir,
    clock: Arc<ManualClock>,
    source: Synthetic,
    runs: SqliteSyncRuns,
    cycle: CycleParts<Unchanging>,
}

impl Deployment {
    async fn new() -> Self {
        let scratch = tempfile::tempdir().unwrap_or_else(|error| panic!("a scratch: {error}"));
        let state = scratch.path().join("state");
        let credentials = scratch.path().join("credentials");
        for folder in [&state, &credentials] {
            fs::create_dir_all(folder).unwrap_or_else(|error| panic!("a folder: {error}"));
        }
        for (id, value) in [
            (SYNC_USERNAME, "synthetic-owner\n"),
            (SYNC_PASSWORD, "synthetic-password\n"),
        ] {
            fs::write(credentials.join(id), value)
                .unwrap_or_else(|error| panic!("a credential: {error}"));
        }
        let settings = SyncSettings::from_env(&Environment::from_vars([
            (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
            (STATE_DIRECTORY, state.as_os_str()),
        ]))
        .unwrap_or_else(|error| panic!("the settings: {error}"));
        // The copy: the engine creates an empty collection where none exists, through its port.
        RslibEngine
            .new_card_queue(&settings.copy_path())
            .unwrap_or_else(|error| panic!("the engine creates the copy: {error}"));
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .unwrap_or_else(|error| panic!("the database: {error}"));
        let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START)));
        let loader = CredentialLoader::new(
            CredentialsDirectory::new(credentials)
                .unwrap_or_else(|| panic!("an absolute credentials directory")),
            Redactor::new(),
        );
        let runs = SqliteSyncRuns::new(db.clone());
        let syncer = Syncer::new(
            Unchanging,
            runs.clone(),
            settings.clone(),
            loader,
            clock.clone(),
            StudyDayRule::default(),
        );
        let offload = Offload::new(
            OffloadWorkers::new(1).unwrap_or_else(|| panic!("one worker")),
            clock.clone(),
        );
        let reader = CollectionReader::new(&settings, ScopeSettings::default(), offload);
        let gate = ChangeGate::new(db, StudyDayRule::default(), clock.clone());
        let source = Synthetic::default();
        let cycle = CycleParts::new(
            syncer,
            reader,
            gate,
            Obligations::new().with(source.clone()),
            clock.clone(),
        );
        Self {
            _scratch: scratch,
            clock,
            source,
            runs,
            cycle,
        }
    }

    /// One owner-triggered cycle, `minutes` after the start: far enough apart that no owner sync
    /// is debounced into the one before it.
    async fn cycle_at(&self, minutes: i64) -> Recompute {
        self.clock
            .set(UtcMillis::from_epoch_millis(START + minutes * MINUTE_MS));
        sync_cycle(&self.cycle, Trigger::Owner)
            .await
            .unwrap_or_else(|error| panic!("the cycle at {minutes} minute(s): {error}"))
            .recompute
    }

    async fn last_status(&self) -> Option<RunStatus> {
        self.runs
            .history()
            .await
            .unwrap_or_else(|error| panic!("the record is read: {error}"))
            .last
    }
}

fn ran_for(recompute: &Recompute) -> Option<RunReason> {
    match recompute {
        Recompute::Ran { reason, .. } => Some(*reason),
        Recompute::Skipped => None,
    }
}

#[tokio::test]
async fn a_registered_deadline_that_passes_runs_the_recompute_on_an_unchanged_cycle() {
    let deployment = Deployment::new().await;
    assert_eq!(deployment.cycle.obligations().names(), ["synthetic"]);
    let first = deployment.cycle_at(0).await;
    assert!(
        ran_for(&first).is_some(),
        "the first cycle recomputes: {first:?}"
    );

    // A deadline passes between two cycles in which nothing in the collection changed.
    deployment.source.open(START + 5 * MINUTE_MS);
    let due = deployment.cycle_at(10).await;
    assert_eq!(
        ran_for(&due),
        Some(RunReason::DeadlineDue { label: LABEL }),
        "the deadline alone runs the recompute: {due:?}"
    );
    // Exactly once: the recompute served it, so the next unchanged cycle skips, and says so.
    assert_eq!(deployment.cycle_at(20).await, Recompute::Skipped);
    assert_eq!(deployment.last_status().await, Some(RunStatus::Skipped));
    // The skipped cycle still synced: a skip never skips the sync.
    let synced = deployment
        .runs
        .last_success_at()
        .await
        .unwrap_or_else(|error| panic!("the record is read: {error}"));
    assert_eq!(
        synced,
        Some(UtcMillis::from_epoch_millis(START + 20 * MINUTE_MS))
    );
}

#[tokio::test]
async fn a_deadline_not_yet_due_or_already_served_lets_the_gate_skip() {
    let deployment = Deployment::new().await;
    let first = deployment.cycle_at(0).await;
    assert!(
        ran_for(&first).is_some(),
        "the first cycle recomputes: {first:?}"
    );

    // Not yet due: tomorrow's deadline never holds today's gate open.
    deployment.source.open(START + DAY_MS);
    assert_eq!(deployment.cycle_at(10).await, Recompute::Skipped);
    assert_eq!(deployment.last_status().await, Some(RunStatus::Skipped));

    // Already served: a deadline that passed is served by the one recompute it runs, and the
    // cycles after it skip while it stays open in its source.
    deployment.source.open(START + 15 * MINUTE_MS);
    assert_eq!(
        ran_for(&deployment.cycle_at(20).await),
        Some(RunReason::DeadlineDue { label: LABEL })
    );
    assert_eq!(deployment.cycle_at(30).await, Recompute::Skipped);
    assert_eq!(deployment.cycle_at(40).await, Recompute::Skipped);
    assert_eq!(deployment.last_status().await, Some(RunStatus::Skipped));
}
