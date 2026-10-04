//! The skip day's take on a working copy, against the engine's own sync server behind the recording
//! layer (SPEC-083 A5, A9, A25, A26, A28, A29, A34, A36, A38 to A40, A44, A47 to A49, A53, A54 and
//! A56). This target is compiled at edition 2021 so that its tests set the process's zone without
//! `unsafe` (SPEC-083 section 3), and every test in it holds the target's one lock for its whole run.

// An integration test is test code: its helpers panic on a fixture that cannot be built, and it
// prints the examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

// The support module is shared with the crate's edition-2024 targets, which format it; formatted
// from this edition-2021 root it would be sorted the other way, so this root leaves it alone.
#[rustfmt::skip]
mod support;

use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime};

use deck_streak_ingest::engine::{CollectionWrite, RslibEngine};
use deck_streak_ingest::reader::is_study_event;
use deck_streak_ingest::settings::SkipSearch;
use deck_streak_ingest::skip::{
    skip_spec, FailReason, SkipId, SkipState, SkipStore, SKIP_MAX_CARDS, SKIP_SPREAD_MAX_DAYS,
    SKIP_SPREAD_MIN_DAYS,
};
use deck_streak_ingest::skip_write::{
    erase_backups, list_digest, moved_counts, preview, take, Counts, NoHooks, Preview, PreviewCard,
    TakeAnswer, TakeHooks, TakePorts, TakeRequest,
};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_ingest::write_class_stop::{ClassStop, StopSetter, WriteClassStop};
use deck_streak_kernel::{Db, Environment, Hour, StudyDay, StudyDayRule, UtcMillis, UtcOffset};
use sha2::{Digest, Sha256};
use support::recording::{local_change, CardRow, Recording};
use support::synthetic::{self, SkipCard, SkipSetup, SKIP_CARDS, SKIP_FLOOR};
use support::{Fixture, SyncServer};
use tokio::runtime::Runtime;

/// An endpoint no test here contacts.
const ENDPOINT: &str = "http://127.0.0.1:9/";
/// The zone the service pins and each test here sets itself (SPEC-083 R3): a POSIX rule that names
/// no zone file, at the zero offset, with no daylight period.
const UTC: &str = "UTC0";
/// How long a test waits after it changes `TZ`: chrono reads the variable again at most once a
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

/// Changes the process's zone to `rule` while the caller holds the target's lock, and waits until
/// chrono reads it.
fn rezone(_held: &MutexGuard<'static, ()>, rule: &str) {
    std::env::set_var("TZ", rule);
    thread::sleep(ZONE_SETTLES);
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn a_reschedule_by_the_engine_is_not_a_study_event() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let copy = fixture.copy();
    synthetic::build_skip(&copy, &SKIP_CARDS, SkipSetup::UTC);
    let moved = synthetic::skip_moved(&SKIP_CARDS);
    assert_eq!(
        synthetic::review_log(&copy),
        Vec::new(),
        "the collection holds no review-log row before the reschedule"
    );

    RslibEngine
        .set_due_date(
            &copy,
            &moved,
            &skip_spec(SKIP_SPREAD_MIN_DAYS, SKIP_SPREAD_MAX_DAYS),
        )
        .expect("the engine reschedules the cards");

    // The positive artifact: the engine's Set Due Date wrote one review-log row for each moved
    // card, of type 4 with ease 0 (R18), and no other row.
    let rows = examined(
        "review-log row(s) the reschedule wrote",
        synthetic::review_log(&copy),
    );
    let mut cards: Vec<i64> = rows.iter().map(|row| row.card).collect();
    cards.sort_unstable();
    assert_eq!(cards, moved, "one row for each moved card, and no other");
    for row in &rows {
        assert_eq!(
            (row.kind, row.ease),
            (4, 0),
            "a reschedule's row is of type 4 with ease 0: {row:?}"
        );
        assert!(
            !is_study_event(row.kind, row.ease),
            "the read's rule counts a reschedule as a study event: {row:?}"
        );
    }

    // The read counts none of them: a skip day stays a day with no study.
    let reader = synthetic::reader(&fixture.settings(), "", None, support::clock_at(0));
    let read = support::runtime()
        .block_on(reader.read(0))
        .expect("the copy reads");
    examined("card(s) the read saw", read.cards);
    assert_eq!(
        read.reviews,
        Vec::new(),
        "the read counted a reschedule as a study event"
    );
}

/// The configured skip search when none is set: the default (R3).
fn default_search() -> SkipSearch {
    SkipSearch::from_env(&Environment::from_vars(Vec::<(&str, &str)>::new()))
        .expect("the default search is one expression")
}

/// The D20 digest worked by hand: SHA-256 over the ascending ids, each in decimal ended by a line
/// feed, its first 16 bytes as lowercase hexadecimal.
fn digest_by_hand(ids: &[i64]) -> String {
    let mut ascending = ids.to_vec();
    ascending.sort_unstable();
    let mut text = String::new();
    for id in ascending {
        writeln!(text, "{id}").expect("a String takes the text");
    }
    let sum = Sha256::digest(text.as_bytes());
    let mut hex = String::new();
    for byte in sum.iter().take(16) {
        write!(hex, "{byte:02x}").expect("a String takes the text");
    }
    hex
}

#[test]
fn the_preview_names_a_set_stop_and_who_set_it() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let runtime = support::runtime();
    let stop = WriteClassStop::new(runtime.block_on(fixture.db()));
    let since = UtcMillis::from_epoch_millis(1_700_000_000_999);
    runtime
        .block_on(stop.set_by_counts("review_log_rows", since))
        .expect("the stop is set");

    // No collection is built: a preview that read one while the stop is set would fail here.
    let shown = runtime
        .block_on(preview(
            &RslibEngine,
            &fixture.settings(),
            &default_search(),
            &stop,
            today(StudyDayRule::default()),
            StudyDayRule::default(),
        ))
        .expect("the preview answers");
    assert_eq!(
        shown,
        Preview::Stopped(ClassStop::Stopped {
            set_by: StopSetter::Counts,
            reason: "review_log_rows".to_owned(),
            since,
        }),
        "the preview names who set the stop, why and since when, and lists nothing"
    );

    // A stop already set keeps who set it, why and since when.
    runtime
        .block_on(stop.set_by_counts("cards", UtcMillis::from_epoch_millis(1)))
        .expect("a second set is a no-op");
    assert_eq!(
        runtime.block_on(stop.read_stop()),
        ClassStop::Stopped {
            set_by: StopSetter::Counts,
            reason: "review_log_rows".to_owned(),
            since,
        }
    );
}

#[test]
fn a_missing_stop_row_reads_as_stopped() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let stop = WriteClassStop::new(db.clone());
    assert_eq!(
        runtime.block_on(stop.read_stop()),
        ClassStop::Running,
        "the migration seeds the stop not set"
    );
    runtime.block_on(async {
        sqlx::query("DELETE FROM write_class_stop")
            .execute(db.reader())
            .await
            .expect("the row is removed");
    });
    let read = runtime.block_on(stop.read_stop());
    assert_eq!(read, ClassStop::Unread, "a missing row reads as unread");
    assert!(read.is_stopped(), "an unread stop stops every write");
    let shown = runtime
        .block_on(preview(
            &RslibEngine,
            &fixture.settings(),
            &default_search(),
            &stop,
            today(StudyDayRule::default()),
            StudyDayRule::default(),
        ))
        .expect("the preview answers");
    assert_eq!(shown, Preview::Stopped(ClassStop::Unread));
}

#[test]
fn the_class_stop_is_one_row_and_a_second_is_refused() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    // A row every other check admits: only the singleton key can refuse it.
    let second = runtime.block_on(async {
        sqlx::query("INSERT INTO write_class_stop (id, stopped, created_at) VALUES (2, 0, 0)")
            .execute(db.reader())
            .await
    });
    assert!(second.is_err(), "a second stop row was admitted");
    let rows: Vec<i64> = runtime.block_on(async {
        sqlx::query_scalar("SELECT id FROM write_class_stop ORDER BY id")
            .fetch_all(db.reader())
            .await
            .expect("the rows are read")
    });
    assert_eq!(rows, [1], "the class's stop is one row, id 1");
}

#[test]
fn the_preview_lists_the_cards_and_a_changed_list_writes_nothing() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let copy = fixture.copy();
    let planned = synthetic::build_skip(&copy, &SKIP_CARDS, SkipSetup::UTC);
    let runtime = support::runtime();
    let stop = WriteClassStop::new(runtime.block_on(fixture.db()));
    let search = default_search();
    let skip_deck = planned.decks[synthetic::SKIP_DECK];

    // The list: exactly the cards the default search moves, each card of every other kind left out,
    // ascending by id, each under its top-level deck, with its digest. The private copy is unchanged.
    let shown = preview_unchanged(&runtime, &fixture, &search, &stop);
    let Preview::Listed { cards, digest } = shown else {
        panic!("the preview listed nothing: {shown:?}");
    };
    let cards = examined("previewed card(s)", cards);
    let moved = synthetic::skip_moved(&SKIP_CARDS);
    let ids: Vec<i64> = cards.iter().map(|card| card.id).collect();
    assert_eq!(
        ids, moved,
        "the preview lists exactly the cards the search moves"
    );
    for card in &cards {
        assert_eq!(card.top_level_deck, skip_deck, "{card:?}");
    }
    assert_eq!(digest.len(), 32, "{digest}");
    assert!(
        digest
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "{digest}"
    );
    assert_eq!(digest, digest_by_hand(&moved));
    assert_eq!(digest, list_digest(&moved));

    // A changed collection changes the list and its digest, and the preview still writes nothing.
    let joined = SKIP_FLOOR + 12;
    synthetic::run_sql(
        &copy,
        &format!("update cards set due = due - 10 where id = {joined}"),
    );
    let shown = preview_unchanged(&runtime, &fixture, &search, &stop);
    let Preview::Listed {
        cards,
        digest: changed,
    } = shown
    else {
        panic!("the preview listed nothing: {shown:?}");
    };
    let mut grown = moved.clone();
    grown.push(joined);
    let ids: Vec<i64> = cards.iter().map(|card: &PreviewCard| card.id).collect();
    assert_eq!(ids, grown, "the card now due joins the list");
    assert_ne!(changed, digest, "a changed list has a changed digest");
    assert_eq!(changed, digest_by_hand(&grown));

    // The take's arm: a confirm with no digest, or with the digest of the list before the change,
    // writes nothing, sends no request (the endpoint is one no test contacts) and answers
    // preview_changed with the new preview.
    let store = SkipStore::new(runtime.block_on(fixture.db()));
    let settings = fixture.settings();
    let credentials = fixture.credentials();
    let rule = StudyDayRule::default();
    for confirm in [None, Some(digest.clone())] {
        let skip = runtime
            .block_on(store.begin(today(rule), None, now()))
            .expect("the take's row is begun");
        let before = std::fs::read(&copy).expect("the copy is read");
        let ports = TakePorts {
            writer: &RslibEngine,
            settings: &settings,
            search: &search,
            stop: &stop,
            store: &store,
            credentials: &credentials,
        };
        let request = TakeRequest {
            skip,
            day: today(rule),
            rule,
            digest: confirm.clone(),
            now: now(),
        };
        let answer = runtime.block_on(take(&ports, &request, no_hooks()));
        let TakeAnswer::Failed {
            reason: FailReason::PreviewChanged,
            preview: Some(Preview::Listed { cards, digest: new }),
        } = answer
        else {
            panic!("confirm {confirm:?}: the take did not answer preview_changed: {answer:?}");
        };
        let ids: Vec<i64> = cards.iter().map(|card| card.id).collect();
        assert_eq!(ids, grown, "the answer carries the new preview");
        assert_eq!(new, changed);
        assert_eq!(std::fs::read(&copy).expect("the copy is read"), before);
        let state = runtime
            .block_on(store.records())
            .expect("read")
            .into_iter()
            .find(|record| record.id == skip)
            .expect("the row")
            .state;
        assert_eq!(state, SkipState::Failed(FailReason::PreviewChanged));
    }
}

/// One preview of the fixture's private copy under the default rule, which leaves the copy's bytes
/// as they were.
fn preview_unchanged(
    runtime: &Runtime,
    fixture: &Fixture,
    search: &SkipSearch,
    stop: &WriteClassStop,
) -> Preview {
    let before = std::fs::read(fixture.copy()).expect("the copy is read");
    let shown = runtime
        .block_on(preview(
            &RslibEngine,
            &fixture.settings(),
            search,
            stop,
            today(StudyDayRule::default()),
            StudyDayRule::default(),
        ))
        .expect("the preview answers");
    assert_eq!(
        std::fs::read(fixture.copy()).expect("the copy is read"),
        before
    );
    shown
}

/// The instant now.
fn now() -> UtcMillis {
    UtcMillis::from_system_time(SystemTime::now())
}

/// The study day now under `rule`.
fn today(rule: StudyDayRule) -> StudyDay {
    rule.study_day(now())
}

/// A zone whose offset is not a whole number of hours and observes no daylight saving (A5, A44).
const IST: &str = "IST-5:30";

/// The study-day rule of [`IST`] with the engine's default rollover hour.
fn ist_rule() -> StudyDayRule {
    StudyDayRule::new(
        Hour::new(4).expect("an hour"),
        UtcOffset::from_minutes(330).expect("an offset"),
    )
}

/// A skip collection configured for [`IST`].
const IST_SETUP: SkipSetup = SkipSetup {
    utc_offset_west: Some(-330),
    rollover: 4,
    fsrs: false,
};

/// The point between the take's steps where a test acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Point {
    AfterConverge,
    AfterBackup,
    AfterSnapshot,
    BeforePush,
}

/// One act at one point.
struct At(Point, Box<dyn Fn(&Path) + Send + Sync>);

impl At {
    fn act(&self, point: Point, path: &Path) {
        if self.0 == point {
            (self.1)(path);
        }
    }
}

impl TakeHooks for At {
    fn after_converge(&self, working: &Path) {
        self.act(Point::AfterConverge, working);
    }
    fn after_backup(&self, partial: &Path) {
        self.act(Point::AfterBackup, partial);
    }
    fn after_snapshot(&self, working: &Path) {
        self.act(Point::AfterSnapshot, working);
    }
    fn before_push(&self, working: &Path) {
        self.act(Point::BeforePush, working);
    }
}

fn at(point: Point, act: impl Fn(&Path) + Send + Sync + 'static) -> Arc<dyn TakeHooks> {
    Arc::new(At(point, Box::new(act)))
}

fn no_hooks() -> Arc<dyn TakeHooks> {
    Arc::new(NoHooks)
}

/// A deployment: its private copy, its ledger and, when the test needs one, the engine's own sync
/// server behind the recording layer. Without a server the endpoint is one no test contacts, so a
/// take that sent a request would fail on it.
struct Scene {
    runtime: Runtime,
    scratch: tempfile::TempDir,
    recording: Option<Recording>,
    _server: Option<SyncServer>,
    fixture: Fixture,
    db: Db,
    store: SkipStore,
    stop: WriteClassStop,
    rule: StudyDayRule,
}

impl Scene {
    fn build(
        test: Option<&str>,
        seeded: Option<(&[(i64, SkipCard)], SkipSetup)>,
        rule: StudyDayRule,
    ) -> Self {
        let runtime = support::runtime();
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let base = scratch.path().join("server");
        if let Some((cards, setup)) = seeded {
            let served = support::server_collection(&base);
            synthetic::build_skip(&served, cards, setup);
            synthetic::as_served(&served);
        }
        let server = test.map(|test| {
            let _ = support::server_collection(&base);
            SyncServer::start(test, &base)
        });
        let recording = server
            .as_ref()
            .map(|server| Recording::start(server.endpoint()));
        let fixture = Fixture::new(recording.as_ref().map_or(ENDPOINT, Recording::endpoint));
        let db = runtime.block_on(fixture.db());
        Self {
            store: SkipStore::new(db.clone()),
            stop: WriteClassStop::new(db.clone()),
            runtime,
            scratch,
            recording,
            _server: server,
            fixture,
            db,
            rule,
        }
    }

    /// The server holds `cards` built with `setup`, and the owner's sync pulls them.
    fn served(test: &str, cards: &[(i64, SkipCard)], setup: SkipSetup, rule: StudyDayRule) -> Self {
        let scene = Self::build(Some(test), Some((cards, setup)), rule);
        scene.owner_sync();
        scene.clear();
        scene
    }

    /// No server: the private copy alone holds `cards` built with `setup`.
    fn offline(cards: &[(i64, SkipCard)], setup: SkipSetup, rule: StudyDayRule) -> Self {
        let scene = Self::build(None, None, rule);
        synthetic::build_skip(&scene.fixture.copy(), cards, setup);
        scene
    }

    fn owner_sync(&self) {
        let syncer = self.fixture.syncer(
            RslibEngine,
            SqliteSyncRuns::new(self.db.clone()),
            support::clock_at(now().epoch_millis()),
        );
        self.runtime
            .block_on(syncer.sync(Trigger::Owner))
            .expect("the owner's sync runs");
    }

    fn endpoint(&self) -> &str {
        self.recording
            .as_ref()
            .map_or(ENDPOINT, Recording::endpoint)
    }

    fn clear(&self) {
        if let Some(recording) = &self.recording {
            recording.clear();
        }
    }

    fn copy(&self) -> PathBuf {
        self.fixture.copy()
    }

    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.copy()).expect("the private copy is read")
    }

    /// Another client's collection, a copy of the private copy as it stands.
    fn other(&self, name: &str) -> PathBuf {
        let other = self.scratch.path().join(name);
        std::fs::copy(self.copy(), &other).expect("the other client starts from the copy");
        other
    }

    fn preview(&self) -> Preview {
        self.runtime
            .block_on(preview(
                &RslibEngine,
                &self.fixture.settings(),
                &default_search(),
                &self.stop,
                today(self.rule),
                self.rule,
            ))
            .expect("the preview answers")
    }

    fn digest(&self) -> String {
        match self.preview() {
            Preview::Listed { digest, .. } => digest,
            other => panic!("the preview listed nothing: {other:?}"),
        }
    }

    fn begin(&self) -> SkipId {
        self.runtime
            .block_on(self.store.begin(today(self.rule), None, now()))
            .expect("the take's row is begun")
    }

    fn take(&self, digest: Option<String>, hooks: Arc<dyn TakeHooks>) -> (SkipId, TakeAnswer) {
        let skip = self.begin();
        (skip, self.take_on(skip, digest, hooks))
    }

    fn take_on(
        &self,
        skip: SkipId,
        digest: Option<String>,
        hooks: Arc<dyn TakeHooks>,
    ) -> TakeAnswer {
        self.take_with(skip, digest, hooks, &default_search())
    }

    fn take_with(
        &self,
        skip: SkipId,
        digest: Option<String>,
        hooks: Arc<dyn TakeHooks>,
        search: &SkipSearch,
    ) -> TakeAnswer {
        let settings = self.fixture.settings();
        let credentials = self.fixture.credentials();
        let ports = TakePorts {
            writer: &RslibEngine,
            settings: &settings,
            search,
            stop: &self.stop,
            store: &self.store,
            credentials: &credentials,
        };
        let request = TakeRequest {
            skip,
            day: today(self.rule),
            rule: self.rule,
            digest,
            now: now(),
        };
        self.runtime.block_on(take(&ports, &request, hooks))
    }

    fn state(&self, skip: SkipId) -> SkipState {
        self.runtime
            .block_on(self.store.records())
            .expect("the records are read")
            .into_iter()
            .find(|record| record.id == skip)
            .expect("the take's row")
            .state
    }

    /// Every request the recording layer saw that carried a local change.
    fn changes(&self) -> Vec<String> {
        self.recording.as_ref().map_or_else(Vec::new, |recording| {
            recording
                .requests()
                .iter()
                .filter_map(local_change)
                .collect()
        })
    }

    fn requests(&self) -> usize {
        self.recording
            .as_ref()
            .map_or(0, |recording| recording.requests().len())
    }

    /// The cards the pushes carried, ascending by id.
    fn pushed(&self) -> Vec<CardRow> {
        let mut cards: Vec<CardRow> = self.recording.as_ref().map_or_else(Vec::new, |recording| {
            recording
                .requests()
                .iter()
                .filter(|request| request.method == "applyChunk")
                .flat_map(support::recording::Recorded::cards)
                .collect()
        });
        cards.sort_by_key(|card| card.id);
        cards
    }

    /// The files beside the private copy whose names begin with `prefix`, sorted.
    fn beside(&self, prefix: &str) -> Vec<String> {
        let directory = self
            .copy()
            .parent()
            .expect("the copy's directory")
            .to_owned();
        let mut names: Vec<String> = std::fs::read_dir(directory)
            .expect("the copy's directory is listed")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.starts_with(prefix))
            .collect();
        names.sort();
        names
    }

    /// The snapshot rows of `skip`: card, prior due, left due and left modification time.
    fn snapshot(&self, skip: SkipId) -> Vec<(i64, i64, Option<i64>, Option<i64>)> {
        snapshot_of(&self.runtime, &self.db, skip)
    }

    /// Sets the class's stop from a hook's own thread, as the owner or a count would.
    fn stop_setter(&self) -> impl Fn(&Path) + Send + Sync + 'static {
        let path = self.fixture.scratch().join("deckstreak.db");
        move |_: &Path| {
            let runtime = support::runtime();
            runtime.block_on(async {
                let db = Db::open(&path).await.expect("the ledger opens");
                WriteClassStop::new(db)
                    .set_by_counts("planted", now())
                    .await
                    .expect("the stop is set");
            });
        }
    }

    /// Asserts a take ended `failed` with `reason` and wrote nothing: no request carried a change,
    /// the private copy's bytes are `before`, and no working copy is left.
    fn wrote_nothing(&self, skip: SkipId, answer: &TakeAnswer, reason: FailReason, before: &[u8]) {
        assert!(
            matches!(answer, TakeAnswer::Failed { reason: got, .. } if *got == reason),
            "the take answers {reason}: {answer:?}"
        );
        assert_eq!(self.state(skip), SkipState::Failed(reason));
        assert_eq!(
            self.changes(),
            Vec::<String>::new(),
            "no request carried a change"
        );
        assert!(
            self.bytes() == before,
            "the private copy's bytes are unchanged"
        );
        assert_eq!(
            self.beside("skip-working"),
            Vec::<String>::new(),
            "the working copy is discarded"
        );
    }
}

fn snapshot_of(
    runtime: &Runtime,
    db: &Db,
    skip: SkipId,
) -> Vec<(i64, i64, Option<i64>, Option<i64>)> {
    runtime
        .block_on(
            sqlx::query_as::<_, (i64, i64, Option<i64>, Option<i64>)>(
                "SELECT card_id, prior_due, left_due, left_mtime FROM skip_card_snapshot \
                 WHERE skip_id = ?1 ORDER BY card_id",
            )
            .bind(skip.get())
            .fetch_all(db.reader()),
        )
        .expect("the snapshot is read")
}

/// An empty collection another client uploads whole: the server then holds no cards, so only an
/// upload could answer its full-sync demand.
fn upload_empty(runtime: &Runtime, scratch: &Path, endpoint: &str) {
    let empty = scratch.join("empty.anki2");
    synthetic::build_skip(&empty, &[], SkipSetup::UTC);
    support::upload_from_another_client(runtime, &empty, endpoint);
}

/// The take's push carried no upload, no grave, no note, and never a configured UTC offset other
/// than the collection's (A5).
fn pushed_no_other_change(recording: &Recording) {
    for request in recording.requests() {
        assert_ne!(request.method, "upload", "no upload");
        assert_ne!(request.method, "applyGraves", "no grave");
        if request.method == "applyChunk" {
            assert!(
                request.body["chunk"]["notes"]
                    .as_array()
                    .is_none_or(Vec::is_empty),
                "no note is pushed"
            );
        }
        if let Some(settings) = request.settings() {
            assert_eq!(
                settings
                    .get(support::UTC_OFFSET_KEY)
                    .and_then(serde_json::Value::as_i64),
                Some(-330),
                "the configured UTC offset is never changed"
            );
        }
    }
}

#[test]
fn a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows() {
    const TEST: &str = "a_take_pushes_exactly_the_previewed_cards_and_their_review_log_rows";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(IST);
    let moved = synthetic::skip_moved(&SKIP_CARDS);
    for fsrs in [false, true] {
        let setup = SkipSetup { fsrs, ..IST_SETUP };
        let scene = Scene::served(TEST, &SKIP_CARDS, setup, ist_rule());

        // The preview under the zone: the study day's cards, and none in the filtered deck.
        let Preview::Listed { cards, digest } = scene.preview() else {
            panic!("the preview under {IST} listed nothing");
        };
        let listed: Vec<i64> = cards.iter().map(|card| card.id).collect();
        assert_eq!(
            listed, moved,
            "fsrs {fsrs}: the preview lists the study day's cards"
        );

        // After the preview, another client makes a card due today that the preview did not list:
        // the take's converge brings it in and the search then selects it, yet it never moves.
        let joined = SKIP_FLOOR + 12;
        let other = scene.other("other.anki2");
        let today_due = RslibEngine
            .due_cards(&other, "prop:due=0")
            .expect("the other client's copy reads")[0]
            .due;
        synthetic::change_cards(&other, &[(joined, today_due)]);
        support::sync_on_another_client(&scene.runtime, &other, scene.endpoint());
        scene.clear();

        let before = scene.bytes();
        let (skip, answer) = scene.take(Some(digest), no_hooks());
        assert_eq!(
            answer,
            TakeAnswer::Accepted {
                moved: moved.clone(),
                left_alone: vec![joined],
                read_back: Vec::new()
            },
            "fsrs {fsrs}: the card the preview did not list is left alone"
        );
        let pushed = examined("pushed card(s)", scene.pushed());
        let ids: Vec<i64> = pushed.iter().map(|card| card.id).collect();
        assert_eq!(
            ids, moved,
            "fsrs {fsrs}: the push carries exactly the previewed cards"
        );
        let snapshot = scene.snapshot(skip);
        assert_eq!(
            snapshot.len(),
            moved.len(),
            "fsrs {fsrs}: one snapshot row a card"
        );
        for (card, (id, prior_due, left_due, left_mtime)) in pushed.iter().zip(&snapshot) {
            assert_eq!(card.id, *id);
            assert!(
                (SKIP_SPREAD_MIN_DAYS..=SKIP_SPREAD_MAX_DAYS).contains(&(card.due - prior_due)),
                "fsrs {fsrs}: the card moves within the day spec's range: {card:?}"
            );
            assert_eq!(
                Some(card.due),
                *left_due,
                "the pushed card is the state the take recorded"
            );
            assert_eq!(
                Some(card.mtime),
                *left_mtime,
                "the pushed card is the state the take recorded"
            );
            assert_eq!(
                (card.ctype, card.queue),
                (2, 2),
                "the card stays a review card"
            );
        }
        let recording = scene.recording.as_ref().expect("the layer");
        let mut rows: Vec<(i64, i64, i64)> = recording
            .requests()
            .iter()
            .filter(|request| request.method == "applyChunk")
            .flat_map(support::recording::Recorded::revlog)
            .map(|row| (row.cid, row.kind, row.ease))
            .collect();
        rows.sort_unstable();
        let expected: Vec<(i64, i64, i64)> = moved.iter().map(|id| (*id, 4, 0)).collect();
        assert_eq!(
            rows, expected,
            "fsrs {fsrs}: one review-log row of type 4 with ease 0 a card"
        );
        pushed_no_other_change(recording);
        assert!(
            scene.bytes() == before,
            "fsrs {fsrs}: the private copy's bytes are unchanged"
        );
        assert_eq!(scene.beside("skip-working"), Vec::<String>::new());
    }
}

#[test]
fn a_full_sync_demand_at_the_converge_aborts_the_take_writing_nothing() {
    const TEST: &str = "a_full_sync_demand_at_the_converge_aborts_the_take_writing_nothing";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    // One a download would resolve: another client forced a one-way sync.
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let other = scene.other("other.anki2");
    support::upload_from_another_client(&scene.runtime, &other, scene.endpoint());
    scene.clear();
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, no_hooks());
    scene.wrote_nothing(skip, &answer, FailReason::FullSyncRequired, &before);
    no_snapshot_and_no_backup(&scene, skip);

    // One only an upload would resolve: the server holds an empty collection.
    let scene = Scene::build(Some(TEST), None, StudyDayRule::default());
    synthetic::build_skip(&scene.copy(), &SKIP_CARDS, SkipSetup::UTC);
    let digest = Some(scene.digest());
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, no_hooks());
    scene.wrote_nothing(skip, &answer, FailReason::FullSyncRequired, &before);
    no_snapshot_and_no_backup(&scene, skip);
}

/// A take the converge aborted got no further (R21): no snapshot row and no backup beside the
/// private copy.
fn no_snapshot_and_no_backup(scene: &Scene, skip: SkipId) {
    assert_eq!(scene.snapshot(skip), Vec::new(), "no snapshot row");
    assert_eq!(
        scene.beside("skip-backup"),
        Vec::<String>::new(),
        "no backup"
    );
}

#[test]
fn a_full_sync_demand_at_the_push_aborts_the_take_writing_nothing() {
    const TEST: &str = "a_full_sync_demand_at_the_push_aborts_the_take_writing_nothing";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    for download in [true, false] {
        let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
        let digest = Some(scene.digest());
        let other = scene.other("other.anki2");
        let endpoint = scene.endpoint().to_owned();
        let scratch = scene.scratch.path().to_owned();
        let forced = at(Point::AfterConverge, move |_| {
            let runtime = support::runtime();
            if download {
                support::upload_from_another_client(&runtime, &other, &endpoint);
            } else {
                upload_empty(&runtime, &scratch, &endpoint);
            }
        });
        let before = scene.bytes();
        let (skip, answer) = scene.take(digest, forced);
        // The other client's own upload is the only change the layer saw.
        let changes = scene.changes();
        assert_eq!(
            changes.len(),
            1,
            "download {download}: only the other client's upload: {changes:?}"
        );
        assert!(changes[0].starts_with("upload"), "{changes:?}");
        assert!(
            matches!(
                answer,
                TakeAnswer::Failed {
                    reason: FailReason::FullSyncRequired,
                    ..
                }
            ),
            "download {download}: {answer:?}"
        );
        assert_eq!(
            scene.state(skip),
            SkipState::Failed(FailReason::FullSyncRequired)
        );
        assert_eq!(scene.pushed(), Vec::new(), "no request carried a card");
        assert!(
            scene.bytes() == before,
            "the private copy's bytes are unchanged"
        );
        assert_eq!(scene.beside("skip-working"), Vec::<String>::new());
    }
}

#[test]
fn the_prior_state_is_recorded_before_any_card_changes() {
    const TEST: &str = "the_prior_state_is_recorded_before_any_card_changes";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let moved = synthetic::skip_moved(&SKIP_CARDS);
    let skip = scene.begin();
    let path = scene.fixture.scratch().join("deckstreak.db");
    let seen = Arc::new(Mutex::new(None));
    let saw = Arc::clone(&seen);
    // The take stops between the prior state's commit and the reschedule.
    let stopped = at(Point::AfterSnapshot, move |working| {
        let runtime = support::runtime();
        let db = runtime.block_on(Db::open(&path)).expect("the ledger opens");
        let rows = snapshot_of(&runtime, &db, skip);
        let dues: Vec<i64> = synthetic::review_log(working)
            .iter()
            .map(|row| row.card)
            .collect();
        *saw.lock().unwrap_or_else(PoisonError::into_inner) = Some((rows, dues));
        panic!("the take stops here");
    });
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        scene.take_on(skip, digest, stopped)
    }));
    assert!(stop.is_err(), "the planted stop ended the take");
    let (rows, revlog) = seen
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
        .expect("the take reached the point after the prior state's commit");
    let rows = examined("snapshot row(s) committed before the reschedule", rows);
    let ids: Vec<i64> = rows.iter().map(|row| row.0).collect();
    assert_eq!(ids, moved, "a prior state for each moved card");
    for row in &rows {
        assert_eq!(
            (row.2, row.3),
            (None, None),
            "no left state before the reschedule: {row:?}"
        );
    }
    assert_eq!(
        revlog,
        Vec::<i64>::new(),
        "no card changed before the prior state's commit"
    );
    assert_eq!(scene.snapshot(skip), rows, "the rows stay after the stop");
    assert_eq!(scene.pushed(), Vec::new(), "no request carried a card");
}

#[test]
fn a_card_reviewed_during_the_take_is_listed_to_the_owner() {
    const TEST: &str = "a_card_reviewed_during_the_take_is_listed_to_the_owner";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    for point in [Point::AfterSnapshot, Point::BeforePush] {
        let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
        let digest = Some(scene.digest());
        let other = scene.other("other.anki2");
        let endpoint = scene.endpoint().to_owned();
        let reviewer = other.clone();
        let review = at(point, move |_| {
            if point == Point::BeforePush {
                // A tie keeps the working copy's card: land a whole second after the reschedule.
                thread::sleep(Duration::from_millis(2100));
            }
            support::review_card_on_another_client(
                &support::runtime(),
                &reviewer,
                &endpoint,
                SKIP_FLOOR + 1,
            );
        });
        let (_, answer) = scene.take(digest, review);
        let studied: Vec<i64> = synthetic::review_log(&other)
            .iter()
            .filter(|row| is_study_event(row.kind, row.ease))
            .map(|row| row.card)
            .collect();
        assert_eq!(
            studied.len(),
            1,
            "{point:?}: the other client answered one card"
        );
        let TakeAnswer::Accepted {
            moved, read_back, ..
        } = answer
        else {
            panic!("{point:?}: the take was not accepted: {answer:?}");
        };
        assert!(
            moved.contains(&studied[0]),
            "{point:?}: the reviewed card was one the take moved"
        );
        assert_eq!(
            read_back, studied,
            "{point:?}: the read-back lists the reviewed card"
        );
    }
}

#[test]
fn the_card_guard_refuses_a_large_set_and_an_empty_set_writes_nothing() {
    const TEST: &str = "the_card_guard_refuses_a_large_set_and_an_empty_set_writes_nothing";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let large = synthetic::skip_due_reviews(SKIP_MAX_CARDS + 1);
    let scene = Scene::served(TEST, &large, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, no_hooks());
    scene.wrote_nothing(skip, &answer, FailReason::TooManyCards, &before);

    // A day with no card to move: the skip is recorded and no card is written.
    let none = [
        (SKIP_FLOOR + 1, SkipCard::New),
        (SKIP_FLOOR + 2, SkipCard::OtherDay(5)),
    ];
    let scene = Scene::served(TEST, &none, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, no_hooks());
    assert_eq!(
        answer,
        TakeAnswer::Accepted {
            moved: Vec::new(),
            left_alone: Vec::new(),
            read_back: Vec::new()
        }
    );
    assert_eq!(
        scene.state(skip),
        SkipState::Pending,
        "the record keeps the skip for its settlement"
    );
    assert_eq!(
        scene.changes(),
        Vec::<String>::new(),
        "no request carried a change"
    );
    assert!(
        scene.bytes() == before,
        "the private copy's bytes are unchanged"
    );
    assert_eq!(
        scene.beside("skip-backup"),
        Vec::<String>::new(),
        "no backup without a write"
    );
}

/// A36's bound: a set of exactly [`SKIP_MAX_CARDS`] cards passes the card guard. Its take reaches
/// the backup, whose planted change then ends it with nothing written.
#[test]
fn the_card_guard_admits_a_set_of_exactly_its_bound() {
    const TEST: &str = "the_card_guard_admits_a_set_of_exactly_its_bound";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let bound = synthetic::skip_due_reviews(SKIP_MAX_CARDS);
    let scene = Scene::served(TEST, &bound, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let changed = at(Point::AfterBackup, |partial| {
        let mut bytes = std::fs::read(partial).expect("the partial backup is read");
        bytes.push(0);
        std::fs::write(partial, bytes).expect("the partial backup is changed");
    });
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, changed);
    scene.wrote_nothing(skip, &answer, FailReason::BackupCheckFailed, &before);
}

#[test]
fn a_custom_search_moves_only_the_study_days_due_review_cards() {
    const TEST: &str = "a_custom_search_moves_only_the_study_days_due_review_cards";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let custom = |search: &str| {
        SkipSearch::from_env(&Environment::from_vars([(
            deck_streak_ingest::settings::SKIP_SEARCH,
            search,
        )]))
    };
    let wide = custom(&format!("deck:{}", synthetic::SKIP_DECK)).expect("one expression");
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let shown = scene
        .runtime
        .block_on(preview(
            &RslibEngine,
            &scene.fixture.settings(),
            &wide,
            &scene.stop,
            today(scene.rule),
            scene.rule,
        ))
        .expect("the preview answers");
    let Preview::Listed { cards, digest } = shown else {
        panic!("the preview listed nothing: {shown:?}");
    };
    let moved = synthetic::skip_moved(&SKIP_CARDS);
    let listed: Vec<i64> = cards.iter().map(|card| card.id).collect();
    assert_eq!(
        listed, moved,
        "the preview lists only the study day's due review cards"
    );
    let skip = scene.begin();
    let answer = scene.take_with(skip, Some(digest), no_hooks(), &wide);
    assert!(
        matches!(&answer, TakeAnswer::Accepted { moved: got, .. } if *got == moved),
        "{answer:?}"
    );
    let pushed: Vec<i64> = examined("pushed card(s)", scene.pushed())
        .iter()
        .map(|card| card.id)
        .collect();
    assert_eq!(
        pushed, moved,
        "the push carries only the study day's due review cards"
    );
    assert!(
        custom(&format!("deck:{0}) or (deck:{0}", synthetic::SKIP_DECK)).is_err(),
        "a search that closes the wrap's group is refused before any preview"
    );
}

#[test]
fn a_card_that_changed_between_the_preview_and_the_converge_is_left_alone() {
    const TEST: &str = "a_card_that_changed_between_the_preview_and_the_converge_is_left_alone";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let other = scene.other("other.anki2");
    let joined = SKIP_FLOOR + 12;
    let rescheduled = SKIP_FLOOR + 1;
    let due_today = synthetic::skip_moved(&SKIP_CARDS);
    let today_due = RslibEngine
        .due_cards(&scene.copy(), "prop:due=0")
        .expect("the copy reads")[0]
        .due;
    synthetic::change_cards(&other, &[(joined, today_due), (rescheduled, today_due + 7)]);
    support::sync_on_another_client(&scene.runtime, &other, scene.endpoint());
    scene.clear();
    let (_, answer) = scene.take(digest, no_hooks());
    let kept: Vec<i64> = due_today
        .iter()
        .copied()
        .filter(|id| *id != rescheduled)
        .collect();
    assert_eq!(
        answer,
        TakeAnswer::Accepted {
            moved: kept.clone(),
            left_alone: vec![rescheduled, joined],
            read_back: Vec::new()
        }
    );
    let pushed: Vec<i64> = scene.pushed().iter().map(|card| card.id).collect();
    assert_eq!(pushed, kept, "the push carries neither card");
}

#[test]
fn the_takes_backup_passes_its_restore_check_before_any_card_changes() {
    const TEST: &str = "the_takes_backup_passes_its_restore_check_before_any_card_changes";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    // A backup whose check fails ends the take with nothing written and nothing pushed.
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let changed = at(Point::AfterBackup, |partial| {
        let mut bytes = std::fs::read(partial).expect("the partial backup is read");
        bytes.push(0);
        std::fs::write(partial, bytes).expect("the partial backup is changed");
    });
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, changed);
    scene.wrote_nothing(skip, &answer, FailReason::BackupCheckFailed, &before);
    assert_eq!(
        scene.snapshot(skip),
        Vec::new(),
        "no prior state before a passed check"
    );
    assert_eq!(
        scene.beside("skip-backup"),
        Vec::<String>::new(),
        "no backup and no partial is kept"
    );

    // A backup that passes: it is checked before any card changes.
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let saw = Arc::clone(&seen);
    let unchanged = at(Point::AfterBackup, move |partial| {
        saw.lock().unwrap_or_else(PoisonError::into_inner).extend(
            synthetic::review_log(partial)
                .into_iter()
                .map(|row| row.card),
        );
    });
    let (skip, answer) = scene.take(digest, unchanged);
    assert!(matches!(answer, TakeAnswer::Accepted { .. }), "{answer:?}");
    assert_eq!(
        *seen.lock().unwrap_or_else(PoisonError::into_inner),
        Vec::<i64>::new()
    );
    assert_eq!(
        scene.beside("skip-backup"),
        vec![format!("skip-backup-{}.anki2", skip.get())]
    );
}

#[test]
fn the_takes_counts_move_only_the_review_log_rows_and_the_due_count() {
    const TEST: &str = "the_takes_counts_move_only_the_review_log_rows_and_the_due_count";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let (_, answer) = scene.take(digest, no_hooks());
    assert!(matches!(answer, TakeAnswer::Accepted { .. }), "{answer:?}");
    assert_eq!(
        scene.runtime.block_on(scene.stop.read_stop()),
        ClassStop::Running
    );

    // A planted extra change: one card deleted after the prior state's commit.
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let planted = at(Point::AfterSnapshot, |working| {
        synthetic::delete_cards(working, &[SKIP_FLOOR + 5]);
    });
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, planted);
    scene.wrote_nothing(skip, &answer, FailReason::CountsMoved, &before);
    let ClassStop::Stopped { set_by, reason, .. } = scene.runtime.block_on(scene.stop.read_stop())
    else {
        panic!("the moved count did not set the class's stop");
    };
    assert_eq!((set_by, reason.as_str()), (StopSetter::Counts, "cards"));
}

/// A48's comparison (R35) on counts the test states: only the review-log rows and the reschedule's
/// rows up by the moved cards and the due count down by them pass, and any other move names the
/// first count that moved. The values make adding, subtracting, multiplying and dividing by the
/// moved count disagree.
#[test]
fn the_counts_name_the_first_count_a_reschedule_does_not_explain() {
    let _zone = zone(UTC);
    let moved = 3;
    let before = Counts {
        cards: 40,
        notes: 30,
        review_log_rows: 10,
        reschedule_rows: 4,
        cards_by_queue_and_type: vec![((0, 0), 15), ((2, 2), 25)],
        due: 12,
    };
    let after = Counts {
        review_log_rows: 13,
        reschedule_rows: 7,
        due: 9,
        ..before.clone()
    };
    assert_eq!(
        moved_counts(&before, &after, moved),
        None,
        "a reschedule of {moved} cards explains every count"
    );
    let rescheduled_moves = vec![
        (
            "cards",
            Counts {
                cards: 41,
                ..after.clone()
            },
        ),
        (
            "notes",
            Counts {
                notes: 31,
                ..after.clone()
            },
        ),
        (
            "cards_by_queue_and_type",
            Counts {
                cards_by_queue_and_type: vec![((0, 0), 16), ((2, 2), 24)],
                ..after.clone()
            },
        ),
        (
            "review_log_rows",
            Counts {
                review_log_rows: 14,
                ..after.clone()
            },
        ),
        (
            "reschedule_rows",
            Counts {
                reschedule_rows: 8,
                ..after.clone()
            },
        ),
        (
            "due",
            Counts {
                due: 10,
                ..after.clone()
            },
        ),
    ];
    for (name, moved_after) in examined("count(s) that moved", rescheduled_moves) {
        assert_eq!(
            moved_counts(&before, &moved_after, moved),
            Some(name),
            "{name} moved"
        );
    }
}

#[test]
fn the_take_refuses_while_the_classs_stop_is_set() {
    const TEST: &str = "the_take_refuses_while_the_classs_stop_is_set";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    (scene.stop_setter())(Path::new(""));
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, no_hooks());
    scene.wrote_nothing(skip, &answer, FailReason::WritesStopped, &before);
    assert_eq!(
        answer,
        TakeAnswer::Failed {
            reason: FailReason::WritesStopped,
            preview: None
        },
        "the refusal is answered, not silent"
    );
    assert_eq!(scene.requests(), 0, "no request at all");

    // A stop set during a take ends it before its push.
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let digest = Some(scene.digest());
    let set = at(Point::BeforePush, scene.stop_setter());
    let before = scene.bytes();
    let (skip, answer) = scene.take(digest, set);
    scene.wrote_nothing(skip, &answer, FailReason::WritesStopped, &before);
}

#[test]
fn the_backup_sits_beside_the_copy_owner_only_and_one_is_kept() {
    const TEST: &str = "the_backup_sits_beside_the_copy_owner_only_and_one_is_kept";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let _zone = zone(UTC);
    let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    let directory = scene
        .copy()
        .parent()
        .expect("the copy's directory")
        .to_owned();
    let older = directory.join("skip-backup-999.anki2");
    std::fs::write(&older, b"an older backup").expect("an older backup is planted");

    // A take whose check fails keeps the older backup and leaves no partial file.
    let digest = Some(scene.digest());
    let changed = at(Point::AfterBackup, |partial| {
        std::fs::write(partial, b"not the working copy").expect("the partial backup is changed");
    });
    let (_, answer) = scene.take(digest, changed);
    assert!(
        matches!(
            answer,
            TakeAnswer::Failed {
                reason: FailReason::BackupCheckFailed,
                ..
            }
        ),
        "{answer:?}"
    );
    assert_eq!(
        scene.beside("skip-backup"),
        vec!["skip-backup-999.anki2".to_owned()]
    );

    // A take whose check passes replaces it: one backup, owner-only, named for its skip.
    let digest = Some(scene.digest());
    let (skip, answer) = scene.take(digest, no_hooks());
    assert!(matches!(answer, TakeAnswer::Accepted { .. }), "{answer:?}");
    let name = format!("skip-backup-{}.anki2", skip.get());
    assert_eq!(scene.beside("skip-backup"), vec![name.clone()]);
    let mode = std::fs::metadata(directory.join(&name))
        .expect("the backup")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "the backup is the owner's only");
}

#[test]
fn the_backup_erase_removes_every_backup_and_nothing_else() {
    let _zone = zone(UTC);
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let directory = scratch.path();
    let removed = [
        "skip-backup-1.anki2",
        "skip-backup-22.anki2",
        "skip-backup-3.anki2.partial",
    ];
    let kept = [
        "collection.anki2",
        "collection.lock",
        "skip-backup-.anki2",
        "skip-backup-x.anki2",
        "skip-backup-4.anki2.bak",
        "my-skip-backup-5.anki2",
    ];
    for name in removed.iter().chain(&kept) {
        std::fs::write(directory.join(name), name.as_bytes()).expect("a file is planted");
    }
    std::fs::create_dir(directory.join("skip-backup-6.anki2")).expect("a directory is planted");
    let count = erase_backups(directory).expect("the erase runs");
    assert_eq!(count, removed.len(), "it answers what it removed");
    let mut left: Vec<String> = std::fs::read_dir(directory)
        .expect("listed")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    left.sort();
    let mut expected: Vec<String> = kept.iter().map(|name| (*name).to_owned()).collect();
    expected.push("skip-backup-6.anki2".to_owned());
    expected.sort();
    assert_eq!(examined("file(s) left beside the copy", left), expected);
}

/// Asserts the preview and a take each refuse with `reason`, before any request or write.
fn refuses(scene: &Scene, reason: FailReason, case: &str) {
    let before = scene.bytes();
    assert_eq!(
        scene.preview(),
        Preview::Refused(reason),
        "{case}: the preview refuses"
    );
    let (skip, answer) = scene.take(None, no_hooks());
    scene.wrote_nothing(skip, &answer, reason, &before);
}

#[test]
fn a_take_holds_to_the_study_day_and_writes_no_setting_when_the_engines_day_or_zone_differs() {
    const TEST: &str =
        "a_take_holds_to_the_study_day_and_writes_no_setting_when_the_engines_day_or_zone_differs";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    // Each condition planted alone, on the private copy.
    {
        let _zone = zone(UTC);
        let later = SkipSetup {
            rollover: 5,
            ..SkipSetup::UTC
        };
        let scene = Scene::offline(&SKIP_CARDS, later, StudyDayRule::default());
        refuses(
            &scene,
            FailReason::EngineDayDiffers,
            "the engine's rollover hour differs",
        );
        let scene = Scene::offline(&SKIP_CARDS, IST_SETUP, StudyDayRule::default());
        refuses(
            &scene,
            FailReason::ZoneDiffers,
            "the configured offset differs",
        );
    }
    {
        let _zone = zone(IST);
        let missing = SkipSetup {
            utc_offset_west: None,
            ..IST_SETUP
        };
        let scene = Scene::offline(&SKIP_CARDS, missing, ist_rule());
        refuses(
            &scene,
            FailReason::ZoneDiffers,
            "no configured offset, a zone off zero",
        );
    }

    // Another client changes the server's setting after the private copy's last sync: the private
    // copy passes, the converge brings the change, and the take pushes nothing.
    let utc = zone(UTC);
    let cases: [(&str, Option<i64>, FailReason); 3] = [
        (support::UTC_OFFSET_KEY, Some(-330), FailReason::ZoneDiffers),
        (support::ROLLOVER_KEY, Some(5), FailReason::EngineDayDiffers),
        (support::UTC_OFFSET_KEY, None, FailReason::ZoneDiffers),
    ];
    for (key, value, reason) in cases {
        let scene = Scene::served(TEST, &SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
        let digest = Some(scene.digest());
        let other = scene.other("other.anki2");
        // A client keeps a configured offset only in a zone of that offset: the engine rewrites
        // it to the zone's before every sync it makes (`local_utc_offset_for_user`).
        let elsewhere = key == support::UTC_OFFSET_KEY && value == Some(-330);
        if elsewhere {
            rezone(&utc, IST);
        }
        match value {
            Some(value) => support::change_setting_on_another_client(
                &scene.runtime,
                &other,
                scene.endpoint(),
                key,
                value,
            ),
            None => support::remove_setting_on_another_client(
                &scene.runtime,
                &other,
                scene.endpoint(),
                key,
            ),
        }
        if elsewhere {
            rezone(&utc, UTC);
        }
        scene.clear();
        let before = scene.bytes();
        let (skip, answer) = scene.take(digest, no_hooks());
        scene.wrote_nothing(skip, &answer, reason, &before);
    }
    drop(utc);

    // A zone that observes daylight saving refuses, whether its daylight period is in effect now
    // or begins later in the current or the next calendar year, and when it is set after the
    // service started under a rule with none. A rule's start is read in standard time and its end
    // in daylight time, so `later` starts at 19:00 UTC and ends at 21:00 UTC on the year's last day.
    let in_effect = ("AAA-1BBB,J1/0,J365/23", 120);
    let later = ("AAA-1BBB,J365/20,J365/23", 60);
    for (rule, minutes) in [in_effect, later, in_effect] {
        let _zone = zone(rule);
        let setup = SkipSetup {
            utc_offset_west: Some(-minutes),
            rollover: 4,
            fsrs: false,
        };
        let at_rule = StudyDayRule::new(
            Hour::new(4).expect("an hour"),
            UtcOffset::from_minutes(i16::try_from(minutes).expect("minutes")).expect("an offset"),
        );
        let scene = Scene::offline(&SKIP_CARDS, setup, at_rule);
        refuses(&scene, FailReason::ZoneObservesDaylightSaving, rule);
    }
}

/// The source of the take's module, read as text by the zone census.
const SKIP_WRITE_SOURCE: &str = include_str!("../src/skip_write.rs");

#[test]
fn the_skip_refuses_a_zone_the_service_does_not_pin() {
    let unpinned: [Option<&str>; 9] = [
        None,
        Some(""),
        Some("localtime"),
        Some(":/etc/localtime"),
        Some("/etc/localtime"),
        Some("Etc/../UTC"),
        Some("GMT0"),
        Some("Etc/UTC"),
        Some("5:30IST"),
    ];
    for value in unpinned {
        let _held = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        match value {
            Some(value) => std::env::set_var("TZ", value),
            None => std::env::remove_var("TZ"),
        }
        thread::sleep(ZONE_SETTLES);
        let scene = Scene::offline(&SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
        refuses(&scene, FailReason::ZoneNotPinned, value.unwrap_or("unset"));
    }
    for (rule, setup, study) in [
        (UTC, SkipSetup::UTC, StudyDayRule::default()),
        (IST, IST_SETUP, ist_rule()),
    ] {
        let _zone = zone(rule);
        let scene = Scene::offline(&SKIP_CARDS, setup, study);
        assert!(
            matches!(scene.preview(), Preview::Listed { .. }),
            "{rule} is pinned"
        );
    }

    // Only the pin reads TZ or opens a zone directory, and nothing names the host's zone file.
    let mut in_pin = false;
    let mut reads = Vec::new();
    // The module's own tests come last (clippy's items_after_test_module refuses code after
    // them) and one of them hands a child process its zone, so the scan ends where they begin.
    for (number, line) in SKIP_WRITE_SOURCE
        .lines()
        .enumerate()
        .take_while(|(_, line)| *line != "#[cfg(test)]")
    {
        if line.starts_with("fn zone_pin") || line.starts_with("pub fn zone_pin") {
            in_pin = true;
        }
        if line.contains("\"TZ\"") || line.contains("zoneinfo") {
            reads.push((number + 1, in_pin));
        }
        if in_pin && line == "}" {
            in_pin = false;
        }
        assert!(
            !line.contains("/etc/localtime"),
            "line {}: names the host's zone file",
            number + 1
        );
    }
    for (number, inside) in examined("line(s) reading TZ or a zone directory", reads) {
        assert!(inside, "line {number} reads the zone outside the pin");
    }
}

/// A44's grammar: the pin refuses a rule outside the POSIX grammar and pins one inside it, each
/// arm of the parser judged by a rule that breaks only that arm and names no zone file.
#[test]
fn the_pin_refuses_a_rule_outside_the_posix_grammar() {
    let held = zone(UTC);
    let outside = vec![
        // text after the offset that is no daylight zone's name
        "UTC0X",
        // an offset past 24 hours
        "UTC25",
        // a quoted name under three characters
        "<U>0",
        // a quoted name holding a character outside letters, digits, `+` and `-`
        "<U.C>0",
        // a Julian day below 1
        "AAA0BBB,J0,J1",
        // a day that is not decimal digits
        "AAA0BBB,J+1,J2",
        // a month past 12
        "AAA0BBB,M13.1.0,M1.1.0",
    ];
    for rule in examined("rule(s) outside the grammar", outside) {
        rezone(&held, rule);
        let scene = Scene::offline(&SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
        assert_eq!(
            scene.preview(),
            Preview::Refused(FailReason::ZoneNotPinned),
            "{rule}: the pin refuses it"
        );
    }

    // A quoted name inside the grammar is pinned, letters and digits or a sign among them, and the
    // preview lists.
    for quoted in ["<UTC>0", "<+00>0"] {
        rezone(&held, quoted);
        let scene = Scene::offline(&SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
        assert!(
            matches!(scene.preview(), Preview::Listed { .. }),
            "{quoted} is pinned"
        );
    }

    // A month rule inside the grammar is pinned, then refused for its daylight period.
    let month = "AAA0BBB,M3.2.0,M11.1.0";
    rezone(&held, month);
    let scene = Scene::offline(&SKIP_CARDS, SkipSetup::UTC, StudyDayRule::default());
    assert_eq!(
        scene.preview(),
        Preview::Refused(FailReason::ZoneObservesDaylightSaving),
        "{month} is pinned and observes daylight saving"
    );
}
