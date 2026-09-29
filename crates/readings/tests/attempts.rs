//! The attempts' retention (SPEC-046 R10): ninety days, counted in whole days.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_readings::attempts::{
    AttemptOutcome, AttemptRecord, AttemptTelemetry, RETAIN_DAYS, retention_cutoff,
};
use deck_streak_readings::state::{AgentCause, ReadingGate, RunOutcome};
use deck_streak_readings::store::{ReadingRun, RunId, RunTrigger, SqliteReadings};
use deck_streak_readings::topic::TopicKey;

#[test]
fn an_attempt_is_kept_for_exactly_ninety_whole_days() {
    assert_eq!(RETAIN_DAYS, 90, "ninety days");
    let now = UtcMillis::from_epoch_millis(1_000_000_000_000);
    assert_eq!(
        retention_cutoff(now).epoch_millis(),
        1_000_000_000_000 - 90 * 86_400_000,
        "the cutoff is ninety days of 86 400 000 milliseconds back"
    );
}

/// A store over a migrated temporary database, with one run to hang attempts on.
async fn store_with_run(directory: &tempfile::TempDir) -> (Db, SqliteReadings, RunId) {
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let store = SqliteReadings::new(db.clone());
    let run = store
        .record_run(&ReadingRun {
            trigger: RunTrigger::Owner,
            study_day: StudyDay::from_epoch_day(20_000),
            started_at: UtcMillis::from_epoch_millis(1_000),
            finished_at: UtcMillis::from_epoch_millis(2_000),
            outcome: RunOutcome::Resolved,
            unmapped_decks: 0,
        })
        .await
        .expect("a run");
    (db, store, run)
}

fn attempt(run: RunId, topic: &str, outcome: AttemptOutcome, at: i64) -> AttemptRecord {
    AttemptRecord {
        run,
        topic: TopicKey::parse(topic).expect("a topic"),
        study_day: StudyDay::from_epoch_day(20_000),
        attempt: 2,
        repair_gate: Some(ReadingGate::Roster),
        outcome,
        telemetry: AttemptTelemetry {
            turns: 3,
            input_tokens: 40,
            output_tokens: 50,
            cost_micro_usd: 60,
            duration_ms: 70,
        },
        at: UtcMillis::from_epoch_millis(at),
    }
}

#[tokio::test]
async fn every_kind_of_attempt_reads_back_as_it_was_recorded() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let (db, store, run) = store_with_run(&directory).await;
    let recorded = vec![
        attempt(run, "law/a", AttemptOutcome::Passed, 5),
        attempt(
            run,
            "law/b",
            AttemptOutcome::GateFailed {
                gate: ReadingGate::Anchors,
                class: "anchor-missing".to_owned(),
            },
            6,
        ),
        attempt(
            run,
            "law/c",
            AttemptOutcome::Unavailable(AgentCause::ProxyUnreachable),
            7,
        ),
    ];
    for one in &recorded {
        store.record_attempt(one).await.expect("an attempt");
    }
    let read = store.attempts(run).await.expect("the attempts read back");
    assert_eq!(read, recorded);
    db.close().await;
}

#[tokio::test]
async fn pruning_deletes_only_the_attempts_before_the_cutoff_and_counts_them() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let (db, store, run) = store_with_run(&directory).await;
    for (topic, at) in [("law/old", 10), ("law/older", 20), ("law/new", 100)] {
        store
            .record_attempt(&attempt(run, topic, AttemptOutcome::Passed, at))
            .await
            .expect("an attempt");
    }
    let deleted = store
        .prune_attempts_before(UtcMillis::from_epoch_millis(50))
        .await
        .expect("the prune");
    assert_eq!(deleted, 2, "the two before the cutoff");
    let left = store.attempts(run).await.expect("the attempts");
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].topic.as_str(), "law/new");
    db.close().await;
}
