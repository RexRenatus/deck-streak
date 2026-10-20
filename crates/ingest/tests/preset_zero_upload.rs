//! Every preset path records zero uploads and leaves the copy unchanged (SPEC-387 R8, A7).
//!
//! Against the engine's own sync server behind the recording layer: the preset read, `list`, the
//! proposal's write and `verify`, and the read and the proposal refused because the copy's
//! configured UTC offset differs from the process's zone, each followed by the owner's sync,
//! record zero local changes, and the private copy's sha256 stays as its own sync left it. A
//! planted path that writes through the write port is seen first, so the check can fail. The test
//! pins the process's zone itself, as `skip_zero_upload` does: this target is compiled at edition
//! 2021 so that it sets `TZ` without `unsafe`, and its one test holds the target's lock for its
//! whole run.

// An integration test is test code: its helpers panic on a fixture that cannot be built, and it
// prints the examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

// The support module is shared with the crate's edition-2024 targets, which format it; formatted
// from this edition-2021 root it would be sorted the other way, so this root leaves it alone.
#[rustfmt::skip]
mod support;

use std::cell::Cell;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime};

use deck_streak_ingest::engine::{CollectionWrite, RslibEngine};
use deck_streak_ingest::preset::{
    answer, read_presets, ParameterField, Preset, PresetCommand, PresetError, PresetStore,
    ProposeOutcome,
};
use deck_streak_ingest::sync::OWNER_SYNC_DEBOUNCE_SECS;
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_kernel::{Db, UtcMillis};
use sha2::{Digest, Sha256};
use support::recording::{local_change, Recording};
use support::synthetic::{
    self, PresetSetup, PRESET_CARDS, PRESET_MAIN, PRESET_MAIN_FSRS6, PRESET_MAIN_RETENTION,
};
use support::{Fixture, SyncServer};
use tokio::runtime::Runtime;

const TEST: &str = "every_preset_path_records_zero_uploads_and_leaves_the_copy_unchanged";
/// The zone the service pins (SPEC-083 R3): a POSIX rule that names no zone file, at the zero
/// offset, with no daylight period.
const UTC: &str = "UTC0";
/// A POSIX rule five and a half hours east of UTC with no daylight period: the engine stores its
/// offset as -330 minutes west, so a sign slip in the zone check refuses a copy in this zone.
const EAST: &str = "IST-5:30";
/// [`EAST`]'s offset in minutes west of UTC, as the engine stores it.
const EAST_WEST: i32 = -330;
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

/// A deployment whose private copy the server filled with a preset collection, behind the
/// recording layer.
struct Scene {
    runtime: Runtime,
    _scratch: tempfile::TempDir,
    recording: Recording,
    _server: SyncServer,
    fixture: Fixture,
    db: Db,
    /// How far past the real instant the next sync's clock reads: each sync one debounce past the
    /// last, so the owner's sync that follows a path runs rather than answering debounced.
    ahead: Cell<i64>,
}

impl Scene {
    fn new(setup: PresetSetup) -> Self {
        let runtime = support::runtime();
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let base = scratch.path().join("server");
        let collection = support::server_collection(&base);
        synthetic::build_presets(&collection, setup);
        synthetic::as_served(&collection);
        let server = SyncServer::start(TEST, &base);
        let recording = Recording::start(server.endpoint());
        let fixture = Fixture::new(recording.endpoint());
        let db = runtime.block_on(fixture.db());
        let scene = Self {
            runtime,
            _scratch: scratch,
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

    /// The private copy's sha256.
    fn sha256(&self) -> Vec<u8> {
        let bytes = std::fs::read(self.fixture.copy()).expect("the private copy is read");
        Sha256::digest(bytes).to_vec()
    }

    /// The role's answer to `command`, over this scene's copy and database.
    fn answer(&self, command: PresetCommand) -> Result<String, PresetError> {
        self.runtime.block_on(answer(
            &RslibEngine,
            &self.db,
            &self.fixture.settings(),
            command,
            now(),
        ))
    }

    /// Runs `path`, then the owner's sync: whether the copy's sha256 changed, the local changes
    /// the layer saw, and how many requests it saw.
    fn after(&self, path: impl FnOnce(&Self)) -> (bool, Vec<String>, usize) {
        let before = self.sha256();
        self.recording.clear();
        path(self);
        let rewritten = self.sha256() != before;
        self.sync(Trigger::Owner);
        let requests = self.recording.requests();
        let local: Vec<String> = requests.iter().filter_map(local_change).collect();
        (rewritten, local, requests.len())
    }

    /// Runs `path`, then the owner's sync, and asserts the layer saw no local change and the
    /// private copy's sha256 stays as its own sync left it.
    fn zero(&self, name: &str, path: impl FnOnce(&Self)) -> usize {
        let (rewritten, local, requests) = self.after(path);
        assert!(!rewritten, "{name}: the private copy's sha256 changed");
        assert_eq!(
            local,
            Vec::<String>::new(),
            "{name}: a local change was recorded"
        );
        println!("{name}: {requests} request(s), 0 local change(s)");
        requests
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_preset_path_records_zero_uploads_and_leaves_the_copy_unchanged() {
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let east = zone(EAST);
    let setup = PresetSetup {
        utc_offset_west: Some(EAST_WEST),
        rollover: Some(4),
    };
    let scene = Scene::new(setup);

    // The plant first: a path that writes through the write port changes the copy, and the
    // owner's sync that follows uploads the change, so the check below can fail.
    let (rewritten, local, _) = scene.after(|scene| {
        RslibEngine
            .set_due_date(&scene.fixture.copy(), &[PRESET_CARDS[2].0], "5")
            .expect("the planted write runs");
    });
    assert!(
        rewritten && !local.is_empty(),
        "the planted write was not seen: sha256 changed {rewritten}, local changes {local:?}"
    );
    println!(
        "the planted write: sha256 changed {rewritten}, {} local change(s): {local:?}",
        local.len()
    );

    let mut paths = 0;
    paths += usize::from(
        scene.zero("the preset read", |scene| {
            let snapshot = scene
                .runtime
                .block_on(read_presets(&RslibEngine, &scene.fixture.settings(), now()))
                .expect("the copy is read");
            assert_eq!(
                snapshot.defaults.len(),
                21,
                "the engine's defaults are read"
            );
        }) > 0,
    );
    paths += usize::from(
        scene.zero("preset list", |scene| {
            scene.answer(PresetCommand::List).expect("the listing");
        }) > 0,
    );
    // The proposal's write is the store's own, with the defaults the read returned: the role's
    // `propose` is the read above followed by this write.
    let mut proposal = None;
    paths += usize::from(
        scene.zero("preset propose", |scene| {
            let snapshot = scene
                .runtime
                .block_on(read_presets(&RslibEngine, &scene.fixture.settings(), now()))
                .expect("the copy is read");
            let main = Preset {
                id: PRESET_MAIN,
                name: "Main".to_owned(),
                vector: PRESET_MAIN_FSRS6.to_vec(),
                field: ParameterField::Fsrs6,
                desired_retention: PRESET_MAIN_RETENTION,
                // A proposal records no deck.
                deck_ids: Vec::new(),
                non_new_cards: 5,
            };
            let store = PresetStore::new(scene.db.clone());
            match scene
                .runtime
                .block_on(store.propose(&main, &snapshot.defaults, now()))
                .expect("the proposal runs")
            {
                ProposeOutcome::Recorded(recorded) => proposal = Some(recorded.id),
                other => panic!("nothing was recorded: {other:?}"),
            }
        }) > 0,
    );
    let proposal = proposal.expect("the proposal was recorded");
    paths += usize::from(
        scene.zero("preset verify", |scene| {
            scene
                .answer(PresetCommand::Verify(proposal))
                .expect("the verify");
        }) > 0,
    );
    drop(scene);
    drop(east);

    // The copy's configured UTC offset differs from the process's zone: refused before any
    // engine read, and the copy stays as it was.
    let _utc = zone(UTC);
    let differs = Scene::new(setup);
    paths += usize::from(
        differs.zero("a refused read, the offset differing", |scene| {
            let read = scene.runtime.block_on(read_presets(
                &RslibEngine,
                &scene.fixture.settings(),
                now(),
            ));
            assert!(matches!(read, Err(PresetError::ZoneDiffers)), "{read:?}");
        }) > 0,
    );
    paths += usize::from(
        differs.zero("a refused proposal, the offset differing", |scene| {
            let answered = scene.answer(PresetCommand::Propose(PRESET_MAIN));
            assert!(
                matches!(answered, Err(PresetError::ZoneDiffers)),
                "{answered:?}"
            );
        }) > 0,
    );
    println!("examined {paths} path(s), each with its owner's sync");
    assert_eq!(paths, 6, "every path ran and its sync was recorded");
}
