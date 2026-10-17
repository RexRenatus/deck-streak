//! Every path but the take and the undo uploads nothing (SPEC-083 R28, A6; the take's arms here,
//! the undo's are E4c's).
//!
//! Against the engine's own sync server behind the recording layer: a scheduled sync, an owner
//! sync, a preview, each refused take, an aborted take and a failed one, and the owner's sync that
//! follows each, record zero uploads and zero local changes, and the private copy's bytes stay as
//! its own sync left them; nothing is requested after a refused, aborted or failed take's answer,
//! over a wait longer than the syncer's timeout. The test pins the process's zone itself, as the
//! service pins its own (SPEC-083 section 3): this target is compiled at edition 2021 so that it
//! sets `TZ` without `unsafe`, and its one test holds the target's lock for its whole run.

// An integration test is test code: its helpers panic on a fixture that cannot be built, and it
// prints the examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

// The support module is shared with the crate's edition-2024 targets, which format it; formatted
// from this edition-2021 root it would be sorted the other way, so this root leaves it alone.
#[rustfmt::skip]
mod support;

use std::cell::Cell;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime};

use deck_streak_ingest::engine::RslibEngine;
use deck_streak_ingest::settings::SkipSearch;
use deck_streak_ingest::skip::{FailReason, SkipRefusal, SkipStore, SKIP_MAX_CARDS};
use deck_streak_ingest::skip_write::{
    preview, take, NoHooks, Preview, TakeAnswer, TakeHooks, TakePorts, TakeRequest,
};
use deck_streak_ingest::sync::{OWNER_SYNC_DEBOUNCE_SECS, SYNC_TIMEOUT_SECS};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_ingest::write_class_stop::WriteClassStop;
use deck_streak_kernel::{Db, Environment, StudyDay, StudyDayRule, UtcMillis};
use support::recording::{local_change, Recording};
use support::synthetic::{self, SkipCard, SkipSetup, SKIP_CARDS};
use support::{Fixture, SyncServer};
use tokio::runtime::Runtime;

const TEST: &str = "every_path_but_the_take_and_the_undo_records_zero_uploads";
/// The zone the service pins (SPEC-083 R3): a POSIX rule that names no zone file, at the zero
/// offset, with no daylight period.
const UTC: &str = "UTC0";
/// How long the test waits after it changes `TZ`: chrono reads the variable again at most once a
/// second on each thread.
const ZONE_SETTLES: Duration = Duration::from_millis(1100);

/// The target's one lock: the zone is the process's, and `cargo test` runs a target's tests on
/// threads of one process.
static LOCK: Mutex<()> = Mutex::new(());

/// Takes the target's one lock and sets the zone the test runs in, waiting for chrono to read it
/// again when it changed.
fn zone(rule: &str) -> MutexGuard<'static, ()> {
    let held = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    if std::env::var("TZ").ok().as_deref() != Some(rule) {
        std::env::set_var("TZ", rule);
        thread::sleep(ZONE_SETTLES);
    }
    held
}

fn now() -> UtcMillis {
    UtcMillis::from_system_time(SystemTime::now())
}

fn today() -> StudyDay {
    StudyDayRule::default().study_day(now())
}

/// Changes the partial backup, so the take's restore check fails.
struct SpoilTheBackup;

impl TakeHooks for SpoilTheBackup {
    fn after_backup(&self, partial: &Path) {
        std::fs::write(partial, b"not the working copy").expect("the partial is changed");
    }
}

/// A deployment whose private copy the server filled, behind the recording layer.
struct Scene {
    runtime: Runtime,
    scratch: tempfile::TempDir,
    recording: Recording,
    _server: SyncServer,
    fixture: Fixture,
    db: Db,
    store: SkipStore,
    stop: WriteClassStop,
    search: SkipSearch,
    /// How far past the real instant the next sync's clock reads: each sync one debounce past the
    /// last, so the owner's sync that follows a path runs rather than answering debounced.
    ahead: Cell<i64>,
}

impl Scene {
    fn new(cards: &[(i64, SkipCard)], setup: SkipSetup) -> Self {
        let runtime = support::runtime();
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let base = scratch.path().join("server");
        let collection = support::server_collection(&base);
        synthetic::build_skip(&collection, cards, setup);
        synthetic::as_served(&collection);
        let server = SyncServer::start(TEST, &base);
        let recording = Recording::start(server.endpoint());
        let fixture = Fixture::new(recording.endpoint());
        let db = runtime.block_on(fixture.db());
        let scene = Self {
            store: SkipStore::new(db.clone()),
            stop: WriteClassStop::new(db.clone()),
            search: SkipSearch::from_env(&Environment::from_vars(Vec::<(&str, &str)>::new()))
                .expect("the default search"),
            runtime,
            scratch,
            recording,
            _server: server,
            fixture,
            db,
            ahead: Cell::new(0),
        };
        scene.sync(Trigger::Owner);
        scene
    }

    fn sync(&self, trigger: Trigger) {
        let syncer = self.fixture.syncer(
            RslibEngine,
            SqliteSyncRuns::new(self.db.clone()),
            support::clock_at(now().epoch_millis() + self.ahead.get()),
        );
        self.ahead
            .set(self.ahead.get() + (OWNER_SYNC_DEBOUNCE_SECS + 1) * 1000);
        self.runtime
            .block_on(syncer.sync(trigger))
            .expect("the sync runs");
    }

    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.fixture.copy()).expect("the private copy is read")
    }

    fn preview(&self) -> Preview {
        self.runtime
            .block_on(preview(
                &RslibEngine,
                &self.fixture.settings(),
                &self.search,
                &self.stop,
                today(),
                StudyDayRule::default(),
            ))
            .expect("the preview answers")
    }

    fn take(&self, digest: Option<String>, hooks: Arc<dyn TakeHooks>) -> TakeAnswer {
        let skip = self
            .runtime
            .block_on(self.store.begin(today(), None, now()))
            .expect("the take's row is begun");
        let settings = self.fixture.settings();
        let credentials = self.fixture.credentials();
        let ports = TakePorts {
            writer: &RslibEngine,
            settings: &settings,
            search: &self.search,
            stop: &self.stop,
            store: &self.store,
            credentials: &credentials,
        };
        let request = TakeRequest {
            skip,
            day: today(),
            rule: StudyDayRule::default(),
            digest,
            now: now(),
        };
        self.runtime.block_on(take(&ports, &request, hooks))
    }

    fn digest(&self) -> String {
        match self.preview() {
            Preview::Listed { digest, .. } => digest,
            other => panic!("the preview listed nothing: {other:?}"),
        }
    }

    /// Runs `path`, then the owner's sync, and asserts the layer saw no upload and no local change,
    /// and that the private copy's bytes stay as its own sync left them.
    fn zero(&self, name: &str, path: impl FnOnce(&Self)) -> usize {
        let mine = self.bytes();
        self.recording.clear();
        path(self);
        assert!(
            self.bytes() == mine,
            "{name}: the private copy's bytes are its own sync's"
        );
        self.sync(Trigger::Owner);
        let requests = self.recording.requests();
        let changes: Vec<String> = requests.iter().filter_map(local_change).collect();
        assert_eq!(
            changes,
            Vec::<String>::new(),
            "{name}: a local change was recorded"
        );
        println!("{name}: {} request(s), 0 local change(s)", requests.len());
        requests.len()
    }
}

/// The take answered `failed` with `reason`.
fn failed_with(answer: &TakeAnswer, reason: FailReason) {
    assert!(
        matches!(answer, TakeAnswer::Failed { reason: got, .. } if *got == reason),
        "{reason:?}: {answer:?}"
    );
}

#[test]
fn every_path_but_the_take_and_the_undo_records_zero_uploads() {
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let scene = Scene::new(&SKIP_CARDS, SkipSetup::UTC);
    let mut paths = 0;
    paths +=
        usize::from(scene.zero("a scheduled sync", |scene| scene.sync(Trigger::Scheduled)) > 0);
    paths += usize::from(scene.zero("an owner sync", |scene| scene.sync(Trigger::Owner)) > 0);
    paths += usize::from(
        scene.zero("a preview", |scene| {
            assert!(matches!(scene.preview(), Preview::Listed { .. }));
        }) > 0,
    );
    paths += usize::from(
        scene.zero("a refused take: already_skipped", |scene| {
            let first = scene
                .runtime
                .block_on(scene.store.begin(today(), None, now()))
                .expect("the first row");
            let again = scene
                .runtime
                .block_on(scene.store.begin(today(), None, now()));
            assert!(
                matches!(again, Err(SkipRefusal::AlreadySkipped)),
                "{again:?}"
            );
            scene
                .runtime
                .block_on(scene.store.settle_failed(first, FailReason::EngineFailed))
                .expect("the day is freed");
        }) > 0,
    );
    paths += usize::from(
        scene.zero("a refused take: preview_changed", |scene| {
            let answer = scene.take(None, Arc::new(NoHooks));
            failed_with(&answer, FailReason::PreviewChanged);
        }) > 0,
    );
    paths += usize::from(
        scene.zero("a failed take: backup_check_failed", |scene| {
            let answer = scene.take(Some(scene.digest()), Arc::new(SpoilTheBackup));
            failed_with(&answer, FailReason::BackupCheckFailed);
        }) > 0,
    );
    let other = scene.scratch.path().join("other.anki2");
    std::fs::copy(scene.fixture.copy(), &other).expect("the other client's copy");
    paths += usize::from(
        scene.zero("an aborted take: full_sync_required", |scene| {
            let digest = Some(scene.digest());
            support::upload_from_another_client(&scene.runtime, &other, scene.recording.endpoint());
            scene.recording.clear();
            let answer = scene.take(digest, Arc::new(NoHooks));
            failed_with(&answer, FailReason::FullSyncRequired);
        }) > 0,
    );

    // Nothing is requested after the answers, over a wait longer than the syncer's timeout.
    scene.recording.clear();
    let wait = Duration::from_secs_f64(SYNC_TIMEOUT_SECS + 1.0);
    scene
        .runtime
        .block_on(async { tokio::time::sleep(wait).await });
    assert_eq!(
        scene.recording.requests().len(),
        0,
        "a request after the answers"
    );

    let large = Scene::new(
        &synthetic::skip_due_reviews(SKIP_MAX_CARDS + 1),
        SkipSetup::UTC,
    );
    paths += usize::from(
        large.zero("a refused take: too_many_cards", |scene| {
            let answer = scene.take(Some(scene.digest()), Arc::new(NoHooks));
            failed_with(&answer, FailReason::TooManyCards);
        }) > 0,
    );

    // The collection's configured UTC offset differs from the process's zone.
    let west = SkipSetup {
        utc_offset_west: Some(-330),
        ..SkipSetup::UTC
    };
    let differs = Scene::new(&SKIP_CARDS, west);
    paths += usize::from(
        differs.zero("a preview, the offset differing", |scene| {
            assert_eq!(scene.preview(), Preview::Refused(FailReason::ZoneDiffers));
        }) > 0,
    );
    paths += usize::from(
        differs.zero("a refused take, the offset differing", |scene| {
            let answer = scene.take(None, Arc::new(NoHooks));
            failed_with(&answer, FailReason::ZoneDiffers);
        }) > 0,
    );
    println!("examined {paths} path(s), each with its owner's sync");
    assert_eq!(paths, 10, "every path ran and its sync was recorded");
}
