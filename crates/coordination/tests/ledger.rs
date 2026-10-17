//! The ledger's claim, release and record are the predecessor's, and two concurrent runs of one
//! daily job for one fire date act once (SPEC-027 A1, A2; R3, R4).

// An integration test is test code: its helpers panic on a failed database, and the examined
// counts are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use deck_streak_coordination::delivery::NoNotifier;
use deck_streak_coordination::jobs::{FireDate, Job, Schedule};
use deck_streak_coordination::ledger::{CronLedger, FireRow, Outcome, SqliteCronLedger};
use deck_streak_coordination::runner::{Decision, Done, Fire, Reason, Runner, Work};
use deck_streak_ingest::sync_runs::SqliteSyncRuns;
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, UtcMillis};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::Barrier;

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;

/// Rounds of the concurrent run: each opens a fresh database and starts both runs together.
const ROUNDS: usize = 24;

/// A daily job with no catch-up, at a local time no job of the table uses.
const DAILY: Job = Job {
    id: "synthetic_daily",
    schedule: Schedule::DailyAt { hour: 9, minute: 5 },
    catch_up: false,
};

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// A fresh database in a temporary directory, every migration applied.
async fn fresh() -> (TempDir, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (directory, db)
}

/// A row as the golden writes one: its instants as epoch milliseconds.
fn as_golden(row: &FireRow) -> Value {
    json!({
        "first_seen_at": row.first_seen_at.epoch_millis(),
        "updated_at": row.updated_at.epoch_millis(),
        "last_fire_at": row.last_fire_at.map(UtcMillis::epoch_millis),
        "ok_count": row.ok_count,
        "error_count": row.error_count,
        "catchup_count": row.catchup_count,
        "missed_count": row.missed_count,
        "last_outcome": row.last_outcome.as_str(),
    })
}

/// A job's work that counts its runs.
struct Counted {
    runs: Arc<AtomicUsize>,
}

impl Work for Counted {
    async fn perform(&self, _fire: &Fire) -> Result<Done, Reason> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        // Give the other run every chance to interleave while this one acts.
        tokio::task::yield_now().await;
        Ok(Done::Done)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_concurrent_runs_of_one_daily_job_for_one_fire_date_act_once() {
    // The fire at 09:05 on day 20000, both runs a minute after it, as a timer and a manual start.
    let day = 20_000;
    let now = UtcMillis::from_epoch_millis(day * DAY_MS + 9 * HOUR_MS + 6 * MINUTE_MS);
    for round in examined("round(s)", (1..=ROUNDS).collect()) {
        let (_directory, db) = fresh().await;
        let runs = Arc::new(AtomicUsize::new(0));
        let together = Arc::new(Barrier::new(2));
        let tasks: Vec<_> = (0..2)
            .map(|_| {
                let (db, runs, together) = (db.clone(), Arc::clone(&runs), Arc::clone(&together));
                tokio::spawn(async move {
                    let ledger = SqliteCronLedger::new(db.clone());
                    let sync_runs = SqliteSyncRuns::new(db);
                    let clock = ManualClock::new(now);
                    let runner = Runner::new(
                        &ledger,
                        &sync_runs,
                        &NoNotifier,
                        &clock,
                        StudyDayRule::default(),
                    );
                    together.wait().await;
                    runner.run(&DAILY, &Counted { runs }).await
                })
            })
            .collect();
        let mut decisions = Vec::new();
        for task in tasks {
            let report = task
                .await
                .expect("the run's task completes")
                .expect("the run reads and writes its ledger");
            decisions.push(report.decision);
        }
        assert_eq!(
            runs.load(Ordering::SeqCst),
            1,
            "round {round}: the job acted once: {decisions:?}"
        );
        let fire_date = FireDate::from_epoch_day(day);
        let ran = decisions
            .iter()
            .filter(|decision| {
                matches!(decision, Decision::Ran { fire_date: date, outcome: Outcome::Ok, .. } if *date == fire_date)
            })
            .count();
        let refused = decisions
            .iter()
            .filter(|decision| {
                matches!(decision, Decision::AlreadyClaimed { fire_date: date } if *date == fire_date)
            })
            .count();
        assert_eq!(
            (ran, refused),
            (1, 1),
            "round {round}: one run acted and the other found the claim: {decisions:?}"
        );
        let row = SqliteCronLedger::new(db.clone())
            .row(DAILY.id, fire_date)
            .await
            .expect("the ledger reads")
            .expect("the fire has its row");
        assert_eq!(
            (row.catchup_count, row.ok_count, row.last_outcome),
            (1, 1, Outcome::Ok),
            "round {round}: one claim and one success: {row:?}"
        );
        db.close().await;
    }
}

#[tokio::test]
async fn the_ledger_matches_the_predecessors_golden() {
    let mut cases = Vec::new();
    golden::each_case("cron_ledger", |case| {
        cases.push((case.input.clone(), case.output.clone()));
    });
    let mut steps_examined = 0_usize;
    for (input, output) in cases {
        let (_directory, db) = fresh().await;
        let ledger = SqliteCronLedger::new(db.clone());
        let job = input["job_id"].as_str().expect("a job id");
        let date = FireDate::from_epoch_day(input["fire_day"].as_i64().expect("a fire day"));
        let once_daily = output["once_daily"]
            .as_bool()
            .expect("whether the job fires once a day");
        let steps = input["steps"].as_array().expect("the steps");
        let expected = output["steps"].as_array().expect("each step's result");
        assert_eq!(steps.len(), expected.len(), "{input}");
        for (step, expected) in steps.iter().zip(expected) {
            let at = UtcMillis::from_epoch_millis(step["at_ms"].as_i64().expect("an instant"));
            let result = match step["op"].as_str().expect("an operation") {
                "claim" => json!(ledger.claim(job, date, at).await.expect("the claim")),
                "release" => {
                    ledger.release(job, date, at).await.expect("the release");
                    Value::Null
                }
                _ => {
                    let outcome = step["outcome"]
                        .as_str()
                        .and_then(Outcome::parse)
                        .expect("an outcome");
                    ledger
                        .record(job, date, outcome, once_daily, at)
                        .await
                        .expect("the record");
                    Value::Null
                }
            };
            assert_eq!(
                result, expected["result"],
                "the result of {step} in {input}"
            );
            let row = ledger
                .row(job, date)
                .await
                .expect("the row reads")
                .as_ref()
                .map_or(Value::Null, as_golden);
            assert_eq!(row, expected["row"], "the row after {step} in {input}");
            steps_examined += 1;
        }
        db.close().await;
    }
    println!("examined {steps_examined} step(s) of the ledger's golden");
    assert!(steps_examined > 0, "the golden's cases hold steps");
}
