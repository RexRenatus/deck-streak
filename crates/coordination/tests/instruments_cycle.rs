//! The sync cycle's instruments step (SPEC-094 R7): after a sync's recompute, the cycle runs the
//! instruments step once, so a weekly instrument that is due has its report stored; a cycle built
//! without instruments stores nothing.
//!
//! Each test runs one whole cycle over a temporary deployment: an engine whose sync finds no
//! change, an empty copy the engine itself created, and the service's database. Every clock is a
//! `ManualClock`. Every value is synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use deck_streak_coordination::instruments::{BoxFuture, InstrumentRunner, Instruments};
use deck_streak_coordination::obligations::Obligations;
use deck_streak_coordination::sync_cycle::{CycleParts, sync_cycle};
use deck_streak_ingest::engine::{
    AnkiEngine, EngineError, NewCardQueue, RslibEngine, SyncLogin, SyncOutcome,
};
use deck_streak_ingest::gate::ChangeGate;
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::settings::{
    STATE_DIRECTORY, SYNC_ENDPOINT, SYNC_PASSWORD, SYNC_USERNAME, ScopeSettings, SyncSettings,
};
use deck_streak_ingest::sync::{RetrySchedule, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_insights::instrument::{Cadence, ReportEnvelope};
use deck_streak_insights::registry::{Row, State};
use deck_streak_kernel::{
    Clock, CredentialLoader, CredentialsDirectory, Db, Environment, ManualClock, Offload,
    OffloadWorkers, Redactor, StudyDayRule, UtcMillis,
};
use serde_json::json;

const DAY_MS: i64 = 86_400_000;
/// A study day, as its epoch day number.
const DAY: i64 = 20_000;

static ROWS: [Row; 1] = [Row {
    id: "alpha",
    cadence: Cadence::Weekly,
    state: State::Live,
}];

/// An engine whose every sync finds no change.
#[derive(Clone, Copy)]
struct Engine;

impl AnkiEngine for Engine {
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

/// An instrument that counts its runs.
struct Counting(Arc<AtomicUsize>);

impl InstrumentRunner for Counting {
    fn id(&self) -> &'static str {
        "alpha"
    }
    fn schema_version(&self) -> u32 {
        1
    }
    fn run(&self, study_day: i64) -> BoxFuture<'_, Result<ReportEnvelope, String>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            Ok(ReportEnvelope {
                instrument: "alpha".to_owned(),
                study_day,
                schema_version: 1,
                failed_reads: Vec::new(),
                report: json!({ "ran": true }),
            })
        })
    }
}

struct Deployment {
    _scratch: tempfile::TempDir,
    instruments: Arc<Instruments>,
    runs: Arc<AtomicUsize>,
    with: CycleParts<Engine>,
    without: CycleParts<Engine>,
}

async fn deployment() -> Deployment {
    let scratch = tempfile::tempdir().expect("a scratch");
    let state = scratch.path().join("state");
    let credentials = scratch.path().join("credentials");
    for folder in [&state, &credentials] {
        fs::create_dir_all(folder).expect("a folder");
    }
    for (id, value) in [
        (SYNC_USERNAME, "synthetic-owner\n"),
        (SYNC_PASSWORD, "synthetic-password\n"),
    ] {
        fs::write(credentials.join(id), value).expect("a credential");
    }
    let settings = SyncSettings::from_env(&Environment::from_vars([
        (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
        (STATE_DIRECTORY, state.as_os_str()),
    ]))
    .expect("the settings");
    RslibEngine
        .new_card_queue(&settings.copy_path())
        .expect("the engine creates the copy");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
        DAY * DAY_MS + 8 * 3_600_000,
    )));
    let shared: Arc<dyn Clock> = clock.clone();
    let runs = Arc::new(AtomicUsize::new(0));
    let instruments = Arc::new(
        Instruments::new(
            db.clone(),
            &state,
            Arc::clone(&shared),
            StudyDayRule::default(),
            vec![Arc::new(Counting(Arc::clone(&runs))) as Arc<dyn InstrumentRunner>],
        )
        .with_rows(&ROWS),
    );
    let parts = |db: &Db| {
        let credentials = CredentialsDirectory::new(credentials.clone())
            .expect("an absolute credentials directory");
        let loader = CredentialLoader::new(credentials, Redactor::new());
        let syncer = Syncer::new(
            Engine,
            SqliteSyncRuns::new(db.clone()),
            settings.clone(),
            loader,
            Arc::clone(&shared),
            StudyDayRule::default(),
        )
        .with_schedule(RetrySchedule::IMMEDIATE);
        let offload = Offload::new(
            OffloadWorkers::new(1).expect("one worker"),
            Arc::clone(&shared),
        );
        let reader = CollectionReader::new(&settings, ScopeSettings::default(), offload);
        let gate = ChangeGate::new(db.clone(), StudyDayRule::default(), Arc::clone(&shared));
        CycleParts::new(
            syncer,
            reader,
            gate,
            Obligations::new(),
            Arc::clone(&shared),
        )
    };
    let with = parts(&db).with_instruments(Arc::clone(&instruments));
    let without = parts(&db);
    Deployment {
        _scratch: scratch,
        instruments,
        runs,
        with,
        without,
    }
}

#[tokio::test]
async fn a_cycle_with_instruments_runs_the_step_after_its_sync() {
    let deployment = deployment().await;

    let _report = sync_cycle(&deployment.with, Trigger::Owner)
        .await
        .expect("the cycle runs");

    assert_eq!(deployment.runs.load(Ordering::SeqCst), 1, "the step ran");
    let stored = deployment
        .instruments
        .store()
        .get("alpha")
        .await
        .expect("get")
        .expect("the due instrument's report is stored");
    assert_eq!(stored.study_day, DAY);
    assert_eq!(stored.report["report"]["ran"], true);
}

#[tokio::test]
async fn a_cycle_without_instruments_stores_no_report() {
    let deployment = deployment().await;

    let _report = sync_cycle(&deployment.without, Trigger::Owner)
        .await
        .expect("the cycle runs");

    assert_eq!(deployment.runs.load(Ordering::SeqCst), 0, "no step ran");
    assert!(
        deployment
            .instruments
            .store()
            .get("alpha")
            .await
            .expect("get")
            .is_none()
    );
}
