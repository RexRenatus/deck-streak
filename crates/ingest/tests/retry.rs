//! The retry loop and its record (SPEC-022 A8 to A11, R8, R9), and the login's refusal of an empty
//! credential, which it reads through the kernel's loader (SPEC-066 A6).
//!
//! A8 and A10 run the predecessor's schedule on tokio's paused time, so no test sleeps: the waits
//! and timeouts pass the moment nothing else can run. They hand the syncer an in-memory record,
//! because sqlx's pool times its acquire on the same paused clock (SPEC-022 section 7). A11 runs
//! the real engine and the real record, with zero-length waits.

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use std::fs;
use std::net::TcpListener;
use std::time::Duration;

use deck_streak_ingest::engine::{EngineError, RslibEngine, SyncOutcome};
use deck_streak_ingest::settings::{SYNC_PASSWORD, SYNC_USERNAME};
use deck_streak_ingest::sync::{
    COLLECTION_OPEN_RETRIES, COLLECTION_OPEN_RETRY_BASE_SECS, RetrySchedule, SYNC_RETRY_ATTEMPTS,
    SYNC_RETRY_BASE_SECS, SYNC_RETRY_JITTER_FRAC, SYNC_TIMEOUT_SECS, SyncReport,
};
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, Trigger};
use support::{Fixture, MemoryRuns, ScriptedEngine, Step};

/// A runtime whose clock is paused: a wait passes the moment nothing else can run.
fn paused() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap_or_else(|error| panic!("a paused runtime: {error}"))
}

fn seconds(value: &serde_json::Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("a list of waits: {value}"))
        .iter()
        .map(|wait| {
            wait.as_f64()
                .unwrap_or_else(|| panic!("a wait in seconds: {wait}"))
        })
        .collect()
}

#[test]
fn the_retry_schedule_matches_the_predecessors_golden() {
    let examined = golden::each_case("sync_retry", |case| {
        let failures = case.input["failures"].as_u64().expect("failures");
        let failing = match case.input["kind"].as_str() {
            Some("error") => Step::Fail(EngineError::ServerError),
            Some("timeout") => Step::Hang,
            other => panic!("a failure kind the golden does not name: {other:?}"),
        };
        let steps = (0..failures)
            .map(|_| failing.clone())
            .chain([Step::Answer(SyncOutcome::Synced)]);
        let engine = ScriptedEngine::new(steps);
        let mut draws = seconds(&case.input["jitter_draws"]).into_iter();
        let fixture = Fixture::new("http://127.0.0.1:9/");
        let syncer = fixture
            .syncer(engine.clone(), MemoryRuns::default(), support::clock_at(0))
            .with_schedule(RetrySchedule::PREDECESSOR)
            .with_jitter(move || draws.next().expect("the golden draws one jitter per wait"));
        let report = paused()
            .block_on(syncer.sync(Trigger::Owner))
            .expect("the record is in memory");
        let SyncReport::Ran { run, waits_seconds } = report else {
            panic!("an owner's first sync runs: {report:?}");
        };
        assert_eq!(
            waits_seconds,
            seconds(&case.output["waits_seconds"]),
            "the waits for {}",
            case.input
        );
        assert_eq!(
            u64::from(run.attempts),
            case.output["attempts"].as_u64().expect("attempts"),
            "the attempts for {}",
            case.input
        );
        let expected = match case.output["error"].as_str() {
            None => Ok(()),
            Some("server_error") => Err(ReasonCode::ServerError),
            Some("sync_timeout") => Err(ReasonCode::SyncTimeout),
            Some(other) => panic!("a reason the golden does not name: {other}"),
        };
        assert_eq!(run.outcome, expected, "the outcome for {}", case.input);
        assert_eq!(
            case.output["ok"].as_bool(),
            Some(run.outcome.is_ok()),
            "{}",
            case.input
        );
    });
    assert!(examined.count >= 16, "{examined}");
}

#[test]
fn the_sync_constants_equal_the_predecessors() {
    let examined = golden::each_case("sync.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        let port = match name {
            "constants.SYNC_RETRY_ATTEMPTS" => f64::from(SYNC_RETRY_ATTEMPTS),
            "constants.SYNC_RETRY_BASE_SECS" => SYNC_RETRY_BASE_SECS,
            "constants.SYNC_RETRY_JITTER_FRAC" => SYNC_RETRY_JITTER_FRAC,
            "constants.SYNC_TIMEOUT_SECS" => SYNC_TIMEOUT_SECS,
            "constants.COLLECTION_OPEN_RETRIES" => f64::from(COLLECTION_OPEN_RETRIES),
            "constants.COLLECTION_OPEN_RETRY_BASE_SECS" => COLLECTION_OPEN_RETRY_BASE_SECS,
            other => panic!("a constant the port does not hold: {other}"),
        };
        let predecessor = case.output.as_f64().expect("a number");
        assert_eq!(
            port.to_bits(),
            predecessor.to_bits(),
            "{name}: the port holds {port}, the predecessor {predecessor}"
        );
    });
    assert_eq!(examined.count, 6, "{examined}");
}

#[test]
fn an_attempt_past_its_timeout_is_recorded_as_sync_timeout() {
    let engine = ScriptedEngine::new([Step::Hang]);
    let runs = MemoryRuns::default();
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let syncer = fixture
        .syncer(engine.clone(), runs.clone(), support::clock_at(0))
        .with_schedule(RetrySchedule::PREDECESSOR)
        .with_jitter(|| 0.0);
    let taken = paused().block_on(async {
        let started = tokio::time::Instant::now();
        syncer
            .sync(Trigger::Scheduled)
            .await
            .expect("the record is in memory");
        started.elapsed()
    });
    let recorded = runs.runs();
    assert_eq!(recorded.len(), 1, "one run, one record: {recorded:?}");
    assert_eq!(recorded[0].outcome, Err(ReasonCode::SyncTimeout));
    assert_eq!(recorded[0].attempts, SYNC_RETRY_ATTEMPTS);
    assert_eq!(recorded[0].trigger, Trigger::Scheduled);
    assert_eq!(engine.normal_syncs(), 3, "every attempt was made");
    // Three timeouts and the two waits between them passed on the paused clock.
    let least = Duration::from_secs_f64(3.0 * SYNC_TIMEOUT_SECS + 2.0 + 4.0);
    assert!(
        taken >= least,
        "the run took {taken:?} of paused time, less than {least:?}"
    );
}

#[test]
fn a_failed_sync_records_a_bounded_reason_code_and_no_error_text() {
    // A loopback port nothing listens on: the real engine fails to connect, and its error text
    // names the endpoint and, when it opens the copy, the path.
    let closed = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = closed.local_addr().expect("its address").port();
    drop(closed);
    let endpoint = format!("http://127.0.0.1:{port}/");
    let fixture = Fixture::new(&endpoint);
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let syncer = fixture.syncer(
        RslibEngine,
        SqliteSyncRuns::new(db.clone()),
        support::clock_at(0),
    );
    let report = runtime
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("the record is written");
    assert!(
        matches!(&report, SyncReport::Ran { run, .. } if run.outcome == Err(ReasonCode::NetworkUnreachable)),
        "{report:?}"
    );
    let rows: Vec<(String, String, Option<String>, i64)> = runtime
        .block_on(
            sqlx::query_as("SELECT trigger, status, reason, attempts FROM sync_runs")
                .fetch_all(db.reader()),
        )
        .expect("the record is read");
    assert_eq!(rows.len(), 1, "one failed run, one row: {rows:?}");
    let (trigger, status, reason, attempts) = &rows[0];
    assert_eq!((trigger.as_str(), status.as_str()), ("scheduled", "error"));
    assert_eq!(reason.as_deref(), Some("network_unreachable"));
    assert_eq!(*attempts, i64::from(SYNC_RETRY_ATTEMPTS));
    // The row's only text columns hold closed-set words, so none can carry the endpoint, the
    // account or a path.
    let text_columns: Vec<String> = runtime
        .block_on(
            sqlx::query_scalar(
                "SELECT name FROM pragma_table_info('sync_runs') WHERE type = 'TEXT' ORDER BY cid",
            )
            .fetch_all(db.reader()),
        )
        .expect("the table's columns are read");
    assert_eq!(text_columns, ["trigger", "status", "reason"]);
    assert!(Trigger::parse(trigger).is_some(), "{trigger}");
    assert!(ReasonCode::parse(reason.as_deref().unwrap_or_default()).is_some());
    let secrets = [
        endpoint.as_str(),
        support::USERNAME,
        support::PASSWORD,
        &fixture.scratch().display().to_string(),
    ]
    .map(str::to_owned);
    for text in [trigger, status, reason.as_ref().expect("a reason")] {
        assert!(
            !secrets.iter().any(|secret| text.contains(secret.as_str())),
            "the row holds {text:?}"
        );
    }
}

/// One attempt, so the reopens are the only retries in play.
const ONE_ATTEMPT: RetrySchedule = RetrySchedule {
    attempts: 1,
    ..RetrySchedule::PREDECESSOR
};

/// Runs one scheduled sync of `engine` under `schedule` on the paused clock, bounded by a paused
/// hour so a loop that never ends fails instead of hanging. Returns the run, its waits and the
/// paused time it took.
fn one_sync(
    engine: &ScriptedEngine,
    schedule: RetrySchedule,
) -> (deck_streak_ingest::sync_runs::SyncRun, Vec<f64>, Duration) {
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let syncer = fixture
        .syncer(engine.clone(), MemoryRuns::default(), support::clock_at(0))
        .with_schedule(schedule);
    paused().block_on(async {
        let started = tokio::time::Instant::now();
        let report = tokio::time::timeout(Duration::from_hours(1), syncer.sync(Trigger::Scheduled))
            .await
            .unwrap_or_else(|_| panic!("the run ends within a paused hour"))
            .unwrap_or_else(|error| panic!("the record is in memory: {error}"));
        let SyncReport::Ran { run, waits_seconds } = report else {
            panic!("a scheduled first sync runs: {report:?}");
        };
        (run, waits_seconds, started.elapsed())
    })
}

#[test]
fn a_locked_collection_is_reopened_three_times_with_doubling_waits() {
    let engine = ScriptedEngine::new([
        Step::Fail(EngineError::CollectionLocked),
        Step::Fail(EngineError::CollectionLocked),
        Step::Fail(EngineError::CollectionLocked),
        Step::Answer(SyncOutcome::Synced),
    ]);
    let (run, waits, taken) = one_sync(&engine, ONE_ATTEMPT);
    assert_eq!(run.outcome, Ok(()));
    assert_eq!(run.attempts, 1);
    assert!(waits.is_empty(), "no attempt failed: {waits:?}");
    assert_eq!(engine.normal_syncs(), 4, "the first open and three reopens");
    assert_eq!(taken, Duration::from_secs_f64(0.25 + 0.5 + 1.0));
}

#[test]
fn a_collection_locked_past_its_reopens_is_recorded_as_locked() {
    let engine = ScriptedEngine::new([Step::Fail(EngineError::CollectionLocked)]);
    let (run, _, taken) = one_sync(&engine, ONE_ATTEMPT);
    assert_eq!(run.outcome, Err(ReasonCode::CollectionLocked));
    assert_eq!(engine.normal_syncs(), 4, "the first open and three reopens");
    assert_eq!(taken, Duration::from_secs_f64(0.25 + 0.5 + 1.0));
}

#[test]
fn the_reopen_wait_doubles_from_its_base() {
    let schedule = RetrySchedule::PREDECESSOR;
    let waits: Vec<f64> = (0..4).map(|reopen| schedule.reopen_wait(reopen)).collect();
    assert_eq!(waits, [0.25, 0.5, 1.0, 2.0]);
    let slower = RetrySchedule {
        open_retry_base_secs: 3.0,
        ..schedule
    };
    assert_eq!(slower.reopen_wait(2).to_bits(), 12.0_f64.to_bits());
}

#[test]
fn the_immediate_schedule_waits_for_nothing_and_keeps_the_predecessors_counts() {
    let expected = RetrySchedule {
        attempts: 3,
        base_secs: 0.0,
        jitter_frac: 0.0,
        attempt_timeout_secs: 300.0,
        open_retries: 3,
        open_retry_base_secs: 0.0,
    };
    assert_eq!(RetrySchedule::IMMEDIATE, expected);
}

#[test]
fn the_default_jitter_draws_stay_inside_their_fraction_of_each_wait() {
    let schedule = RetrySchedule {
        attempts: 40,
        base_secs: 1.0,
        open_retry_base_secs: 0.0,
        ..RetrySchedule::PREDECESSOR
    };
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let engine = ScriptedEngine::new([Step::Fail(EngineError::ServerError)]);
    let syncer = fixture
        .syncer(engine, MemoryRuns::default(), support::clock_at(0))
        .with_schedule(schedule);
    let report = paused()
        .block_on(syncer.sync(Trigger::Scheduled))
        .expect("the record is in memory");
    let SyncReport::Ran { waits_seconds, .. } = report else {
        panic!("a scheduled first sync runs: {report:?}");
    };
    assert_eq!(
        waits_seconds.len(),
        39,
        "a wait after every attempt but the last"
    );
    for (index, wait) in waits_seconds.iter().enumerate() {
        let base = 2_f64.powi(i32::try_from(index).expect("a small index"));
        let draw = (wait - base) / (base * SYNC_RETRY_JITTER_FRAC);
        assert!(
            (0.0..1.0).contains(&draw),
            "wait {index} was {wait}s, a draw of {draw}"
        );
    }
    let draws: std::collections::BTreeSet<u64> = waits_seconds
        .iter()
        .enumerate()
        .map(|(index, wait)| {
            let base = 2_f64.powi(i32::try_from(index).expect("a small index"));
            ((wait - base) / base).to_bits()
        })
        .collect();
    assert!(draws.len() > 30, "the draws were not spread: {draws:?}");
}

#[test]
fn an_empty_sync_credential_is_recorded_missing_and_never_reaches_the_engine() {
    // The login reads both of its credentials through the kernel's loader, so each, empty in each
    // form the loader refuses, records the run as missing credentials with no attempt made, and
    // the engine, which counts every sync it is asked for, is never asked (SPEC-066 A6).
    for id in [SYNC_USERNAME, SYNC_PASSWORD] {
        for form in ["", "\n"] {
            let engine = ScriptedEngine::new([Step::Answer(SyncOutcome::NoChanges)]);
            let runs = MemoryRuns::default();
            let fixture = Fixture::new("http://127.0.0.1:9/");
            fs::write(fixture.scratch().join("credentials").join(id), form)
                .expect("the credential is rewritten");
            let syncer = fixture.syncer(engine.clone(), runs.clone(), support::clock_at(0));
            paused()
                .block_on(syncer.sync(Trigger::Scheduled))
                .expect("the record is in memory");
            let recorded = runs.runs();
            assert_eq!(recorded.len(), 1, "{id} {form:?}: {recorded:?}");
            assert_eq!(
                recorded[0].outcome,
                Err(ReasonCode::MissingCredentials),
                "{id} {form:?}"
            );
            assert_eq!(recorded[0].attempts, 0, "{id} {form:?}");
            assert_eq!(
                engine.normal_syncs(),
                0,
                "{id} {form:?}: an empty login reached the engine"
            );
        }
    }
}
