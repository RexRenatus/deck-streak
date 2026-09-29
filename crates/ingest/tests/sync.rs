//! The sync against the engine's own server, and its guards (SPEC-022 A4 to A7, A15 to A17).
//!
//! A4 to A7 and A15 run the real engine against the engine's sync server, which runs in a child
//! process over a small synthetic collection; A15 puts the recording layer between them. A16 and
//! A17 run the syncer's guards on a manual clock over a scripted engine, which counts every request
//! the syncer makes. Every test waits zero-length waits (`RetrySchedule::IMMEDIATE`).

mod support;

use std::fs;
use std::time::Duration;

use deck_streak_ingest::engine::{AnkiEngine, RslibEngine, SyncLogin, SyncOutcome};
use deck_streak_ingest::sync::{OWNER_SYNC_DEBOUNCE_SECS, SyncReport};
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, SyncRun, Trigger};
use deck_streak_kernel::Db;
use support::recording::{Recording, local_change};
use support::synthetic::{self, Shape, Side};
use support::{Fixture, ScriptedEngine, Step, SyncServer};

/// An instant in a study day, one hour past its 04:00 rollover in UTC (the default rule).
const IN_A_STUDY_DAY: i64 = 20_000 * 86_400_000 + 5 * 3_600_000;
/// The other client's reviews a sync pulls.
const OTHER_REVIEWS: usize = 10;

/// The engine's sync server over a new scratch directory, serving `shape` (or no collection).
fn start_server(test: &str, shape: Option<Shape>) -> (tempfile::TempDir, SyncServer) {
    let scratch =
        tempfile::tempdir().unwrap_or_else(|error| panic!("a scratch directory: {error}"));
    let base = scratch.path().join("server");
    let collection = support::server_collection(&base);
    if let Some(shape) = shape {
        synthetic::build_shaped(&collection, Side::Server, shape);
    }
    let server = SyncServer::start(test, &base);
    (scratch, server)
}

fn ran(report: &SyncReport) -> &SyncRun {
    match report {
        SyncReport::Ran { run, .. } => run,
        other => panic!("the sync did not run: {other:?}"),
    }
}

async fn rows(db: &Db) -> Vec<(String, i64)> {
    sqlx::query_as("SELECT trigger, study_day FROM sync_runs ORDER BY id")
        .fetch_all(db.reader())
        .await
        .unwrap_or_else(|error| panic!("the record is read: {error}"))
}

#[test]
fn a_sync_pulls_a_review_made_on_another_client() {
    const TEST: &str = "a_sync_pulls_a_review_made_on_another_client";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let (scratch, server) = start_server(TEST, Some(Shape::SMALL));
    let fixture = Fixture::new(server.endpoint());
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(
        RslibEngine,
        SqliteSyncRuns::new(db),
        support::clock_at(IN_A_STUDY_DAY),
    );
    let first = runtime
        .block_on(syncer.sync(Trigger::Owner))
        .expect("recorded");
    assert_eq!(ran(&first).outcome, Ok(()), "{first:?}");
    let before = synthetic::counts(&fixture.copy());
    assert_eq!(
        before.cards,
        Shape::SMALL.cards,
        "the first sync fetched the server's collection"
    );

    let other = scratch.path().join("other.anki2");
    fs::copy(fixture.copy(), &other).expect("the other client starts from the same collection");
    support::review_on_another_client(&runtime, &other, server.endpoint(), OTHER_REVIEWS);

    let second = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    let run = ran(&second);
    assert_eq!(run.outcome, Ok(()), "{second:?}");
    assert!(
        !run.full_download,
        "a change on the server is an incremental sync"
    );
    assert_eq!(
        synthetic::counts(&fixture.copy()).reviews,
        before.reviews + OTHER_REVIEWS,
        "the sync pulled the other client's reviews"
    );
}

#[test]
fn a_second_sync_with_no_change_pulls_nothing() {
    const TEST: &str = "a_second_sync_with_no_change_pulls_nothing";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let (_scratch, server) = start_server(TEST, Some(Shape::SMALL));
    let fixture = Fixture::new(server.endpoint());
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(
        RslibEngine,
        SqliteSyncRuns::new(db),
        support::clock_at(IN_A_STUDY_DAY),
    );
    let first = runtime
        .block_on(syncer.sync(Trigger::Owner))
        .expect("recorded");
    assert!(
        ran(&first).full_download,
        "the first sync downloads the collection: {first:?}"
    );
    let (counts, stamp) = (
        synthetic::counts(&fixture.copy()),
        synthetic::modified(&fixture.copy()),
    );

    let second = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    let run = ran(&second);
    assert_eq!(
        (run.outcome, run.attempts, run.full_download),
        (Ok(()), 1, false),
        "{second:?}"
    );
    assert_eq!(
        synthetic::counts(&fixture.copy()),
        counts,
        "nothing was pulled"
    );
    assert_eq!(
        synthetic::modified(&fixture.copy()),
        stamp,
        "the copy is as it was"
    );
}

#[test]
fn a_full_sync_demand_downloads_and_never_uploads() {
    const TEST: &str = "a_full_sync_demand_downloads_and_never_uploads";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let (scratch, server) = start_server(TEST, Some(Shape::SMALL));
    let recording = Recording::start(server.endpoint());
    let fixture = Fixture::new(recording.endpoint());
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(
        RslibEngine,
        SqliteSyncRuns::new(db),
        support::clock_at(IN_A_STUDY_DAY),
    );
    let first = runtime
        .block_on(syncer.sync(Trigger::Owner))
        .expect("recorded");
    assert!(
        ran(&first).full_download,
        "a copy that does not exist yet is downloaded: {first:?}"
    );

    // The owner's other client changes the schema and uploads its collection, with reviews the
    // copy does not have: the server now demands a full sync of every other client.
    let other = scratch.path().join("other.anki2");
    fs::copy(fixture.copy(), &other).expect("the other client starts from the same collection");
    support::review_on_another_client(&runtime, &other, server.endpoint(), OTHER_REVIEWS);
    support::upload_from_another_client(&runtime, &other, server.endpoint());
    let uploaded = synthetic::counts(&other);
    recording.clear();

    let demanded = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    let run = ran(&demanded);
    assert_eq!(
        (run.outcome, run.full_download),
        (Ok(()), true),
        "{demanded:?}"
    );
    assert_eq!(
        synthetic::counts(&fixture.copy()),
        uploaded,
        "the copy is the server's collection"
    );
    let methods: Vec<String> = recording
        .requests()
        .into_iter()
        .map(|request| request.method)
        .collect();
    assert!(
        methods.iter().any(|method| method == "download"),
        "{methods:?}"
    );
    assert!(
        !methods.iter().any(|method| method == "upload"),
        "{methods:?}"
    );
}

#[test]
fn a_server_with_no_collection_is_refused_with_full_upload_required() {
    const TEST: &str = "a_server_with_no_collection_is_refused_with_full_upload_required";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let (_scratch, server) = start_server(TEST, None);
    let fixture = Fixture::new(server.endpoint());
    let built = synthetic::build_shaped(&fixture.copy(), Side::Client, Shape::SMALL);
    let stamp = synthetic::modified(&fixture.copy());
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(
        RslibEngine,
        SqliteSyncRuns::new(db),
        support::clock_at(IN_A_STUDY_DAY),
    );
    let refused = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    let run = ran(&refused);
    assert_eq!(
        run.outcome,
        Err(ReasonCode::FullUploadRequired),
        "{refused:?}"
    );
    assert!(!run.full_download);
    assert_eq!(
        synthetic::counts(&fixture.copy()),
        built,
        "the copy is untouched"
    );
    assert_eq!(
        synthetic::modified(&fixture.copy()),
        stamp,
        "the copy is untouched"
    );
}

/// The census over one scenario's requests: none uploads or carries a local change, and each
/// method the scenario must send was sent.
#[allow(
    clippy::print_stdout,
    reason = "the census reports the requests it read"
)]
fn census(scenario: &str, recording: &Recording, sent: &[&str]) {
    let requests = recording.requests();
    let methods: Vec<&str> = requests
        .iter()
        .map(|request| request.method.as_str())
        .collect();
    println!("census {scenario}: {methods:?}");
    for method in sent {
        assert!(
            methods.contains(method),
            "{scenario} sent no {method}: {methods:?}"
        );
    }
    let changes: Vec<String> = requests.iter().filter_map(local_change).collect();
    assert_eq!(
        changes,
        Vec::<String>::new(),
        "{scenario} sent a local change or an upload"
    );
    recording.clear();
}

#[test]
fn a_sync_run_sends_no_upload_and_no_local_change() {
    const TEST: &str = "a_sync_run_sends_no_upload_and_no_local_change";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let clock = support::clock_at(IN_A_STUDY_DAY);
    let (scratch, server) = start_server(TEST, Some(Shape::SMALL));
    let recording = Recording::start(server.endpoint());
    let fixture = Fixture::new(recording.endpoint());
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(RslibEngine, SqliteSyncRuns::new(db), clock.clone());

    // A full-sync demand: the copy does not exist yet.
    runtime
        .block_on(syncer.sync(Trigger::Owner))
        .expect("recorded");
    census(
        "a full-sync demand",
        &recording,
        &["hostKey", "meta", "download"],
    );

    // A normal sync that pulls another client's reviews.
    let other = scratch.path().join("other.anki2");
    fs::copy(fixture.copy(), &other).expect("the other client starts from the same collection");
    support::review_on_another_client(&runtime, &other, server.endpoint(), OTHER_REVIEWS);
    runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    census(
        "a normal sync",
        &recording,
        &[
            "hostKey",
            "meta",
            "start",
            "applyChanges",
            "chunk",
            "applyChunk",
            "finish",
        ],
    );

    // Nothing new.
    clock.advance(Duration::from_secs(
        2 * OWNER_SYNC_DEBOUNCE_SECS.unsigned_abs(),
    ));
    runtime
        .block_on(syncer.sync(Trigger::Owner))
        .expect("recorded");
    census("nothing new", &recording, &["hostKey", "meta"]);

    // An empty server, with a populated copy.
    let (_empty_scratch, empty) = start_server(TEST, None);
    let empty_recording = Recording::start(empty.endpoint());
    let populated = Fixture::new(empty_recording.endpoint());
    synthetic::build_shaped(&populated.copy(), Side::Client, Shape::SMALL);
    let db = runtime.block_on(populated.db());
    let refusing = populated.syncer(RslibEngine, SqliteSyncRuns::new(db), clock.clone());
    let refused = runtime
        .block_on(refusing.sync(Trigger::Scheduled))
        .expect("recorded");
    assert_eq!(
        ran(&refused).outcome,
        Err(ReasonCode::FullUploadRequired),
        "{refused:?}"
    );
    census("an empty server", &empty_recording, &["hostKey", "meta"]);
}

#[test]
fn a_second_scheduled_sync_in_one_study_day_is_refused() {
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let engine = ScriptedEngine::new([Step::Answer(SyncOutcome::Synced)]);
    let clock = support::clock_at(IN_A_STUDY_DAY);
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(
        engine.clone(),
        SqliteSyncRuns::new(db.clone()),
        clock.clone(),
    );

    let first = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    assert_eq!(ran(&first).outcome, Ok(()), "{first:?}");
    let recorded = runtime.block_on(rows(&db));
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(
        recorded[0].0, "scheduled",
        "the first run is recorded as scheduled"
    );

    // Three hours on, the same study day: refused before any request, and no row.
    clock.advance(Duration::from_hours(3));
    let second = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    assert_eq!(second, SyncReport::RefusedToday);
    assert_eq!(engine.normal_syncs(), 1, "the refused run made no request");
    assert_eq!(
        runtime.block_on(rows(&db)).len(),
        1,
        "the refused run left no row"
    );

    // The next study day's scheduled run syncs.
    clock.advance(Duration::from_hours(24));
    let next = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    assert_eq!(ran(&next).outcome, Ok(()), "{next:?}");
    let recorded = runtime.block_on(rows(&db));
    assert_eq!(recorded.len(), 2, "{recorded:?}");
    assert_eq!(
        recorded[1].1,
        recorded[0].1 + 1,
        "one scheduled run on each study day"
    );
}

#[test]
fn an_owner_trigger_within_five_minutes_of_a_success_returns_it_without_syncing() {
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let engine = ScriptedEngine::new([Step::Answer(SyncOutcome::Synced)]);
    let clock = support::clock_at(IN_A_STUDY_DAY);
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(
        engine.clone(),
        SqliteSyncRuns::new(db.clone()),
        clock.clone(),
    );
    let first = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("recorded");
    let success = ran(&first).clone();

    // One millisecond short of five minutes: that success, with no request and no row.
    let debounce = Duration::from_secs(OWNER_SYNC_DEBOUNCE_SECS.unsigned_abs());
    clock.advance(debounce.saturating_sub(Duration::from_millis(1)));
    let soon = runtime
        .block_on(syncer.sync(Trigger::Owner))
        .expect("recorded");
    assert_eq!(soon, SyncReport::Debounced { last: success });
    assert_eq!(
        engine.normal_syncs(),
        1,
        "the debounced trigger made no request"
    );
    assert_eq!(
        runtime.block_on(rows(&db)).len(),
        1,
        "the debounced trigger left no row"
    );

    // Five minutes on: it syncs, and is recorded as the owner's.
    clock.advance(Duration::from_millis(1));
    let later = runtime
        .block_on(syncer.sync(Trigger::Owner))
        .expect("recorded");
    assert_eq!(ran(&later).outcome, Ok(()), "{later:?}");
    assert_eq!(engine.normal_syncs(), 2);
    let recorded = runtime.block_on(rows(&db));
    assert_eq!(recorded.len(), 2, "{recorded:?}");
    assert_eq!(
        recorded[1].0, "owner",
        "the owner's trigger is recorded as the owner's"
    );
}

#[test]
fn the_engine_tells_a_sync_with_no_change_from_one_that_exchanged_changes() {
    const TEST: &str = "the_engine_tells_a_sync_with_no_change_from_one_that_exchanged_changes";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let (scratch, server) = start_server(TEST, Some(Shape::SMALL));
    let login = SyncLogin::new(server.endpoint(), support::USERNAME, support::PASSWORD);
    let runtime = support::runtime();
    let copy = scratch.path().join("copy.anki2");
    runtime
        .block_on(RslibEngine.full_download(&copy, &login))
        .expect("the copy starts as a full download");
    let unchanged = runtime
        .block_on(RslibEngine.normal_sync(&copy, &login))
        .expect("the copy syncs");
    assert_eq!(unchanged, SyncOutcome::NoChanges);

    let other = scratch.path().join("other.anki2");
    fs::copy(&copy, &other).expect("the other client starts from the same collection");
    support::review_on_another_client(&runtime, &other, server.endpoint(), OTHER_REVIEWS);
    let changed = runtime
        .block_on(RslibEngine.normal_sync(&copy, &login))
        .expect("the copy syncs");
    assert_eq!(changed, SyncOutcome::Synced);
}
