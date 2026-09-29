//! The run record's reads and its closed vocabularies (SPEC-022 R9, R10).

mod support;

use deck_streak_ingest::sync_runs::{
    ReasonCode, SqliteSyncRuns, StudyDayOutcome, SyncRun, SyncRunStore, Trigger,
};
use deck_streak_kernel::{StudyDay, UtcMillis};
use support::Fixture;

fn run(trigger: Trigger, day: i64, finished: i64, outcome: Result<(), ReasonCode>) -> SyncRun {
    SyncRun {
        trigger,
        started_at: UtcMillis::from_epoch_millis(finished - 10),
        finished_at: UtcMillis::from_epoch_millis(finished),
        study_day: StudyDay::from_epoch_day(day),
        outcome,
        attempts: 1,
        full_download: false,
    }
}

/// A record over a fresh database holding `runs`, in order.
fn record_of(runs: &[SyncRun]) -> (Fixture, SqliteSyncRuns, tokio::runtime::Runtime) {
    let fixture = Fixture::new("http://127.0.0.1:9/");
    let runtime = support::runtime();
    let record = SqliteSyncRuns::new(runtime.block_on(fixture.db()));
    for run in runs {
        runtime
            .block_on(record.record(run))
            .expect("the run is recorded");
    }
    (fixture, record, runtime)
}

#[test]
fn consecutive_failures_count_the_errors_since_the_last_run_that_did_not_fail() {
    let (_fixture, record, runtime) = record_of(&[
        run(Trigger::Scheduled, 1, 1_000, Err(ReasonCode::ServerError)),
        run(Trigger::Scheduled, 2, 2_000, Ok(())),
        run(Trigger::Scheduled, 3, 3_000, Err(ReasonCode::SyncTimeout)),
        run(Trigger::Owner, 3, 4_000, Err(ReasonCode::AuthRejected)),
    ]);
    let failures = runtime
        .block_on(record.consecutive_failures())
        .expect("a read");
    assert_eq!(failures, 2);
}

#[test]
fn the_last_success_is_when_the_latest_ok_run_finished() {
    let (_fixture, record, runtime) = record_of(&[
        run(Trigger::Scheduled, 1, 1_000, Ok(())),
        run(Trigger::Scheduled, 2, 7_000, Ok(())),
        run(Trigger::Scheduled, 3, 9_000, Err(ReasonCode::ServerError)),
    ]);
    let finished = runtime.block_on(record.last_success_at()).expect("a read");
    assert_eq!(finished, Some(UtcMillis::from_epoch_millis(7_000)));
}

#[test]
fn a_study_day_reads_its_success_before_its_failure_and_names_the_trigger() {
    let (_fixture, record, runtime) = record_of(&[
        run(Trigger::Scheduled, 5, 1_000, Ok(())),
        run(Trigger::Owner, 5, 2_000, Err(ReasonCode::ServerError)),
        run(Trigger::Owner, 6, 3_000, Err(ReasonCode::SyncTimeout)),
        run(Trigger::Scheduled, 7, 4_000, Err(ReasonCode::ServerError)),
        run(Trigger::Owner, 7, 5_000, Ok(())),
    ]);
    let outcome = |day| {
        runtime
            .block_on(record.study_day_outcome(StudyDay::from_epoch_day(day)))
            .expect("a read")
    };
    assert_eq!(
        outcome(5),
        Some(StudyDayOutcome {
            synced: true,
            trigger: Trigger::Scheduled
        })
    );
    assert_eq!(
        outcome(6),
        Some(StudyDayOutcome {
            synced: false,
            trigger: Trigger::Owner
        })
    );
    assert_eq!(
        outcome(7),
        Some(StudyDayOutcome {
            synced: true,
            trigger: Trigger::Owner
        })
    );
    assert_eq!(outcome(8), None);
}

#[test]
fn every_trigger_and_reason_parses_back_from_its_stored_word() {
    for trigger in [Trigger::Scheduled, Trigger::Owner] {
        assert_eq!(Trigger::parse(trigger.as_str()), Some(trigger));
    }
    assert_eq!(Trigger::parse("owner"), Some(Trigger::Owner));
    assert_eq!(Trigger::parse("other"), None);
    for code in ReasonCode::ALL {
        assert_eq!(ReasonCode::parse(code.as_str()), Some(code));
        assert_eq!(code.to_string(), code.as_str());
    }
    assert_eq!(ReasonCode::parse("not_a_reason"), None);
    assert_eq!(ReasonCode::parse(""), None);
}
