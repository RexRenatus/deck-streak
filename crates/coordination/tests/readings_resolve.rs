//! The readings' resolve use case (SPEC-045 R4, R8, R9, R11): it reads the last sync from ingest's
//! record, the taxonomy from its file and the private copy through ingest's read, runs the readings'
//! resolution, and records the run and every topic that ended the day.
//!
//! Each test runs the use case over a temporary deployment: a copy the engine itself created, which
//! holds its default deck and no review, the service's database, and a taxonomy file written here
//! whose one language deck is that default deck, so the copy names one topic. Every clock is a
//! `ManualClock`, and every value is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deck_streak_coordination::readings::resolve::{ResolveParts, Resolved, resolve_study_day};
use deck_streak_ingest::engine::{AnkiEngine, RslibEngine};
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::settings::{STATE_DIRECTORY, SYNC_ENDPOINT, ScopeSettings, SyncSettings};
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger};
use deck_streak_kernel::{
    Db, Environment, Hour, ManualClock, Offload, OffloadWorkers, StudyDay, StudyDayRule, UtcMillis,
    UtcOffset,
};
use deck_streak_readings::day_set::EngineQueue;
use deck_streak_readings::state::{CouldNotTell, RunOutcome, TopicState};
use deck_streak_readings::store::{ReadingRun, RunTrigger, SqliteReadings};

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// An hour past the 04:00 rollover (the default rule) that starts study day 20000.
const START: i64 = 20_000 * DAY_MS + 5 * HOUR_MS;
/// A taxonomy whose one language deck is the engine's default deck.
const TAXONOMY: &str = r#"{
  "schema": "deckstreak.readings.taxonomy.v1",
  "law": {"roots": ["Casebook"], "bands": []},
  "languages": [
    {"deck": "Default", "code": "qaa", "display": "Alpha", "term_field": "Headword"}
  ],
  "writing_roots": []
}"#;

/// A deployment in a scratch directory, removed when it drops.
struct Deployment {
    _scratch: tempfile::TempDir,
    settings: SyncSettings,
    taxonomy: PathBuf,
    queue_scratch: PathBuf,
    clock: Arc<ManualClock>,
    db: Db,
    runs: SqliteSyncRuns,
    store: SqliteReadings,
}

impl Deployment {
    async fn new() -> Self {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let state = scratch.path().join("state");
        let queue_scratch = scratch.path().join("day-set");
        for folder in [&state, &queue_scratch] {
            fs::create_dir_all(folder).expect("a folder");
        }
        let settings = SyncSettings::from_env(&Environment::from_vars([
            (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
            (STATE_DIRECTORY, state.as_os_str()),
        ]))
        .expect("the settings");
        // The copy: the engine creates an empty collection where none exists, through its port.
        RslibEngine
            .new_card_queue(&settings.copy_path())
            .expect("the engine creates the copy");
        let taxonomy = scratch.path().join("taxonomy.json");
        fs::write(&taxonomy, TAXONOMY).expect("the taxonomy file");
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("the database");
        Self {
            runs: SqliteSyncRuns::new(db.clone()),
            store: SqliteReadings::new(db.clone()),
            clock: Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START))),
            _scratch: scratch,
            settings,
            taxonomy,
            queue_scratch,
            db,
        }
    }

    /// The use case's parts, reading the taxonomy at `taxonomy`.
    fn parts(&self, taxonomy: Option<&Path>) -> ResolveParts<RslibEngine> {
        self.parts_with(taxonomy, StudyDayRule::default())
    }

    /// The use case's parts, configured with `rule`.
    fn parts_with(&self, taxonomy: Option<&Path>, rule: StudyDayRule) -> ResolveParts<RslibEngine> {
        let offload = Offload::new(
            OffloadWorkers::new(1).expect("one worker"),
            self.clock.clone(),
        );
        let reader =
            CollectionReader::new(&self.settings, ScopeSettings::default(), offload.clone());
        ResolveParts::new(
            self.runs.clone(),
            reader,
            EngineQueue::new(
                RslibEngine,
                &self.settings,
                self.queue_scratch.clone(),
                offload,
            ),
            self.store.clone(),
            taxonomy.map(Path::to_path_buf),
            self.clock.clone(),
            rule,
        )
    }

    /// Records a sync `minutes` before the start, succeeded or failed.
    async fn synced(&self, outcome: Result<(), ReasonCode>) {
        let at = UtcMillis::from_epoch_millis(START - 60 * 60_000);
        self.runs
            .record(&SyncRun {
                trigger: Trigger::Scheduled,
                started_at: at,
                finished_at: at,
                study_day: StudyDay::from_epoch_day(20_000),
                outcome,
                attempts: 1,
                full_download: false,
            })
            .await
            .expect("the sync is recorded");
    }

    /// The one run recorded, and every topic day of its study day as (topic, state).
    async fn recorded(&self) -> (ReadingRun, Vec<(String, TopicState)>) {
        let runs = self.store.runs().await.expect("the runs");
        assert_eq!(runs.len(), 1, "one run is recorded");
        let days = self
            .store
            .topic_days(StudyDay::from_epoch_day(20_000))
            .await
            .expect("the topic days");
        (
            runs[0].1,
            days.into_iter()
                .map(|stored| (stored.day.topic.as_str().to_owned(), stored.day.state))
                .collect(),
        )
    }
}

/// The run the use case records for `outcome` at the start, finishing when the clock says.
fn expected_run(trigger: RunTrigger, outcome: RunOutcome) -> ReadingRun {
    ReadingRun {
        trigger,
        study_day: StudyDay::from_epoch_day(20_000),
        started_at: UtcMillis::from_epoch_millis(START),
        finished_at: UtcMillis::from_epoch_millis(START),
        outcome,
        unmapped_decks: 0,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_last_sync_records_every_topic_could_not_tell() {
    let deployment = Deployment::new().await;
    deployment.synced(Err(ReasonCode::ServerError)).await;
    let Resolved { run, resolution } = resolve_study_day(
        &deployment.parts(Some(&deployment.taxonomy)),
        RunTrigger::Scheduled,
    )
    .await
    .expect("the resolution is recorded");
    let sync_failed = RunOutcome::CouldNotTell(CouldNotTell::SyncFailed);
    assert_eq!(resolution.outcome, sync_failed);
    let (recorded, days) = deployment.recorded().await;
    assert_eq!(recorded, expected_run(RunTrigger::Scheduled, sync_failed));
    assert_eq!(
        days,
        [(
            "language/qaa".to_owned(),
            TopicState::CouldNotTell(CouldNotTell::SyncFailed)
        )]
    );
    assert_eq!(run.get(), 1);
    deployment.db.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_silent_days_record_every_topic_paused() {
    let deployment = Deployment::new().await;
    deployment.synced(Ok(())).await;
    resolve_study_day(
        &deployment.parts(Some(&deployment.taxonomy)),
        RunTrigger::Owner,
    )
    .await
    .expect("the resolution is recorded");
    let (recorded, days) = deployment.recorded().await;
    assert_eq!(
        recorded,
        expected_run(RunTrigger::Owner, RunOutcome::Paused)
    );
    assert_eq!(days, [("language/qaa".to_owned(), TopicState::Paused)]);
    deployment.db.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_taxonomy_records_a_config_fault_and_no_topic() {
    let deployment = Deployment::new().await;
    deployment.synced(Ok(())).await;
    let missing = RunOutcome::CouldNotTell(CouldNotTell::TaxonomyMissing);
    // No taxonomy configured, then one whose file is not there: both are missing.
    let absent_file = deployment.taxonomy.with_file_name("absent.json");
    for (index, taxonomy) in [None, Some(absent_file.as_path())].into_iter().enumerate() {
        let Resolved { resolution, .. } =
            resolve_study_day(&deployment.parts(taxonomy), RunTrigger::Scheduled)
                .await
                .expect("the resolution is recorded");
        assert_eq!(resolution.outcome, missing, "{taxonomy:?}");
        assert_eq!(resolution.topics.len(), 0);
        let runs = deployment.store.runs().await.expect("the runs");
        assert_eq!(runs.len(), index + 1);
        assert_eq!(runs[index].1, expected_run(RunTrigger::Scheduled, missing));
    }
    deployment.db.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unreadable_copy_records_a_rail_that_could_not_open_it() {
    let deployment = Deployment::new().await;
    deployment.synced(Ok(())).await;
    fs::remove_file(deployment.settings.copy_path()).expect("the copy is removed");
    deployment
        .clock
        .set(UtcMillis::from_epoch_millis(START + 1_000));
    let Resolved { resolution, .. } = resolve_study_day(
        &deployment.parts(Some(&deployment.taxonomy)),
        RunTrigger::Scheduled,
    )
    .await
    .expect("the resolution is recorded");
    let open_failed = RunOutcome::CouldNotTell(CouldNotTell::CollectionOpenFailed);
    assert_eq!(resolution.outcome, open_failed);
    let (recorded, days) = deployment.recorded().await;
    let started = UtcMillis::from_epoch_millis(START + 1_000);
    assert_eq!(
        recorded,
        ReadingRun {
            started_at: started,
            finished_at: started,
            ..expected_run(RunTrigger::Scheduled, open_failed)
        }
    );
    assert_eq!(days.len(), 0, "no topic is named without a read");
    deployment.db.close().await;
}

/// The UTC instant study day `day` begins at, written from the definition.
fn begins(day: i64, hour: i64, offset: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS - offset * 60_000
}

/// Answers a card at `at` in the copy's review log, as a study review the pause gate counts.
async fn review_at(deployment: &Deployment, at: i64) {
    use sqlx::Connection;
    let options =
        sqlx::sqlite::SqliteConnectOptions::new().filename(deployment.settings.copy_path());
    let mut connection = sqlx::sqlite::SqliteConnection::connect_with(&options)
        .await
        .expect("the copy opens");
    sqlx::query(
        "INSERT INTO cards (id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, reps, \
         lapses, left, odue, odid, flags, data) \
         VALUES (1, 1, 1, 0, 0, 0, 2, 2, 1, 1, 2500, 1, 0, 0, 0, 0, 0, '')",
    )
    .execute(&mut connection)
    .await
    .expect("a card of the default deck");
    sqlx::query(
        "INSERT INTO revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
         VALUES (?, 1, 0, 3, 1, 0, 2500, 1000, 1)",
    )
    .bind(at)
    .execute(&mut connection)
    .await
    .expect("a review");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_resolution_reads_the_pause_window_and_the_day_in_the_configured_rule() {
    let mut examined = 0;
    for offset in [-720_i64, -300, -210, 0, 330, 345, 540, 840] {
        for hour in [0_i64, 4] {
            let rule = StudyDayRule::new(
                Hour::new(u8::try_from(hour).expect("an hour")).expect("an hour"),
                UtcOffset::from_minutes(i16::try_from(offset).expect("minutes"))
                    .expect("an offset"),
            );
            let today = begins(20_002, hour, offset) + HOUR_MS;
            // The window is study days 20001 and 20000: a review at the rollover that starts 20000
            // is inside it, and one a millisecond earlier is not.
            for (at, outcome) in [
                (begins(20_000, hour, offset), RunOutcome::Resolved),
                (begins(20_000, hour, offset) - 1, RunOutcome::Paused),
            ] {
                let deployment = Deployment::new().await;
                deployment.synced(Ok(())).await;
                review_at(&deployment, at).await;
                deployment.clock.set(UtcMillis::from_epoch_millis(today));
                let Resolved { resolution, .. } = resolve_study_day(
                    &deployment.parts_with(Some(&deployment.taxonomy), rule),
                    RunTrigger::Owner,
                )
                .await
                .expect("the resolution is recorded");
                assert_eq!(
                    resolution.outcome, outcome,
                    "offset {offset}, hour {hour}, review {at}: the pause window is read in the configured rule"
                );
                let runs = deployment.store.runs().await.expect("the runs");
                assert_eq!(
                    runs[0].1.study_day,
                    StudyDay::from_epoch_day(20_002),
                    "offset {offset}, hour {hour}: the run is dated by the configured day"
                );
                deployment.db.close().await;
            }
            examined += 1;
        }
    }
    println!("examined {examined} configured rule(s)");
    assert_eq!(examined, 16, "eight offsets by two rollover hours");
}
