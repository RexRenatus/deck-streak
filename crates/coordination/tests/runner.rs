//! The runner catches up at most six hours late, releases a claim only after a failed delivery,
//! matches the predecessor's catch-up decision, pages a failing sync once per episode, and lets no
//! job but `sync` run the sync cycle (SPEC-027 A3 to A6, A11, A12; R5 to R7).
//!
//! The runner tests register a synthetic `catch_up` notification job with a fake `DeliveryMarker`,
//! because no real notification job exists before W1; every clock is a `ManualClock`.

// An integration test is test code: its helpers panic on a failed database, and the examined
// counts are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::delivery::{DeliveryCounts, DeliveryMarker, NoNotifier};
use deck_streak_coordination::jobs::{FireDate, Job, Schedule};
use deck_streak_coordination::ledger::{CronLedger, FireRow, Outcome, Recorded, SqliteCronLedger};
use deck_streak_coordination::runner::{
    Decision, Done, Fire, Reason, Report, Runner, SyncCycle, Work,
};
use deck_streak_ingest::sync::{SYNC_RETRY_ATTEMPTS, SyncReport};
use deck_streak_ingest::sync_runs::{
    ReasonCode, SqliteSyncRuns, StudyDayOutcome, SyncRun, SyncRunStore, Trigger,
};
use deck_streak_kernel::{Clock, Db, KernelError, ManualClock, StudyDayRule, UtcMillis, UtcOffset};
use serde_json::{Value, json};
use tempfile::TempDir;

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
/// A synthetic day, as an epoch day number.
const DAY: i64 = 20_000;

/// The synthetic notification job: a daily catch-up job at 09:05 local.
const NOTIFY: Job = Job {
    id: "synthetic_notification",
    schedule: Schedule::DailyAt { hour: 9, minute: 5 },
    catch_up: true,
};

/// A synthetic job that needs the study day's data: daily at 09:05, no catch-up.
const READER: Job = Job {
    id: "synthetic_reader",
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

/// The instant `hour`:`minute` UTC on epoch day `day`.
const fn at(day: i64, hour: i64, minute: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(day * DAY_MS + hour * HOUR_MS + minute * MINUTE_MS)
}

/// A fresh database in a temporary directory, every migration applied.
async fn fresh() -> (TempDir, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (directory, db)
}

/// A fake notifier's marker, which the synthetic job moves.
#[derive(Default)]
struct FakeMarker {
    attempted: AtomicU64,
    delivered: AtomicU64,
}

impl DeliveryMarker for FakeMarker {
    fn counts(&self) -> DeliveryCounts {
        DeliveryCounts {
            attempted: self.attempted.load(Ordering::SeqCst),
            delivered: self.delivered.load(Ordering::SeqCst),
        }
    }
}

/// The synthetic notification job's work: it counts its runs, moves the marker by its sends, and
/// returns whether it delivered, or fails.
struct Notification<'a> {
    marker: &'a FakeMarker,
    runs: AtomicUsize,
    attempts: u64,
    deliveries: u64,
    delivered: bool,
    fails: bool,
}

impl<'a> Notification<'a> {
    fn new(marker: &'a FakeMarker, attempts: u64, deliveries: u64, delivered: bool) -> Self {
        Self {
            marker,
            runs: AtomicUsize::new(0),
            attempts,
            deliveries,
            delivered,
            fails: false,
        }
    }

    fn runs(&self) -> usize {
        self.runs.load(Ordering::SeqCst)
    }
}

impl Work for Notification<'_> {
    async fn perform(&self, _fire: &Fire) -> Result<Done, Reason> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        self.marker
            .attempted
            .fetch_add(self.attempts, Ordering::SeqCst);
        self.marker
            .delivered
            .fetch_add(self.deliveries, Ordering::SeqCst);
        if self.fails {
            Err(Reason::new("synthetic_failure"))
        } else if self.delivered {
            Ok(Done::Done)
        } else {
            Ok(Done::NotDelivered)
        }
    }
}

/// The ledger row of `job`'s fire on `day`.
async fn row(db: &Db, job: &Job, day: i64) -> Option<FireRow> {
    SqliteCronLedger::new(db.clone())
        .row(job.id, FireDate::from_epoch_day(day))
        .await
        .expect("the ledger reads")
}

#[tokio::test]
async fn a_catch_up_job_five_hours_late_is_claimed_and_run_once() {
    let (_directory, db) = fresh().await;
    let ledger = SqliteCronLedger::new(db.clone());
    let sync_runs = SqliteSyncRuns::new(db.clone());
    let marker = FakeMarker::default();
    let late = UtcMillis::from_epoch_millis(at(DAY, 9, 5).epoch_millis() + 5 * HOUR_MS);
    let clock = ManualClock::new(late);
    let runner = Runner::new(
        &ledger,
        &sync_runs,
        &marker,
        &clock,
        StudyDayRule::default(),
    );
    let work = Notification::new(&marker, 1, 1, true);

    let first = runner.run(&NOTIFY, &work).await.expect("the run");
    assert_eq!(
        first.decision,
        Decision::Ran {
            fire_date: FireDate::from_epoch_day(DAY),
            outcome: Outcome::Ok,
            released: false,
        }
    );
    assert_eq!(work.runs(), 1, "the late fire ran");
    let claimed = row(&db, &NOTIFY, DAY).await.expect("the fire has its row");
    assert_eq!(
        (
            claimed.catchup_count,
            claimed.ok_count,
            claimed.last_outcome
        ),
        (1, 1, Outcome::Ok)
    );
    assert_eq!(
        claimed.first_seen_at, late,
        "claimed before it acted, at the run"
    );

    // A second start of the same fire, a minute later, finds the claim and does nothing.
    clock.advance(std::time::Duration::from_secs(60));
    let second = runner.run(&NOTIFY, &work).await.expect("the second run");
    assert_eq!(
        second.decision,
        Decision::AlreadyClaimed {
            fire_date: FireDate::from_epoch_day(DAY)
        }
    );
    assert_eq!(work.runs(), 1, "run once");
    assert_eq!(first.exit_code(), 0);
    assert_eq!(second.exit_code(), 0);
}

#[tokio::test]
async fn a_catch_up_job_seven_hours_late_is_recorded_missed_and_not_run() {
    let (_directory, db) = fresh().await;
    let ledger = SqliteCronLedger::new(db.clone());
    let sync_runs = SqliteSyncRuns::new(db.clone());
    let marker = FakeMarker::default();
    let late = UtcMillis::from_epoch_millis(at(DAY, 9, 5).epoch_millis() + 7 * HOUR_MS);
    let clock = ManualClock::new(late);
    let runner = Runner::new(
        &ledger,
        &sync_runs,
        &marker,
        &clock,
        StudyDayRule::default(),
    );
    let work = Notification::new(&marker, 1, 1, true);

    let report = runner.run(&NOTIFY, &work).await.expect("the run");
    assert_eq!(
        report.decision,
        Decision::Missed {
            fire_date: FireDate::from_epoch_day(DAY),
            recorded: Recorded::Written,
        }
    );
    let missed = row(&db, &NOTIFY, DAY).await.expect("the miss has its row");
    assert_eq!(
        (
            missed.missed_count,
            missed.catchup_count,
            missed.ok_count,
            missed.last_outcome
        ),
        (1, 0, 0, Outcome::Missed)
    );
    assert_eq!(missed.first_seen_at, late);
    assert_eq!(
        missed.last_fire_at, None,
        "a miss never advances the last fire"
    );
    assert_eq!(work.runs(), 0, "a fire seven hours late is not run");
    assert_eq!(report.exit_code(), 0, "a miss is recorded, not paged");
}

/// A ledger that answers the claim with a fixed value and records every call, as the golden's stub
/// pipeline does.
struct Recording {
    claim: bool,
    events: Arc<Mutex<Vec<Value>>>,
}

impl Recording {
    fn push(&self, event: Value) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

impl CronLedger for Recording {
    async fn claim(&self, job: &str, date: FireDate, _at: UtcMillis) -> Result<bool, KernelError> {
        self.push(json!(["claim", job, date.epoch_day()]));
        Ok(self.claim)
    }

    async fn release(&self, job: &str, date: FireDate, _at: UtcMillis) -> Result<(), KernelError> {
        self.push(json!(["release", job, date.epoch_day()]));
        Ok(())
    }

    async fn record(
        &self,
        job: &str,
        date: FireDate,
        outcome: Outcome,
        _once_a_day: bool,
        _at: UtcMillis,
    ) -> Result<Recorded, KernelError> {
        self.push(json!(["record", job, date.epoch_day(), outcome.as_str()]));
        Ok(Recorded::Written)
    }

    async fn latest(&self, _job: &str) -> Result<Option<FireRow>, KernelError> {
        Ok(None)
    }
}

/// The golden's stub body: it records its invocation, moves the marker, and returns its delivery
/// or fails.
struct Body<'a> {
    events: Arc<Mutex<Vec<Value>>>,
    marker: &'a FakeMarker,
    attempts: u64,
    deliveries: u64,
    delivered: bool,
    raises: bool,
}

impl Work for Body<'_> {
    async fn perform(&self, fire: &Fire) -> Result<Done, Reason> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(json!(["invoke", fire.job.id]));
        self.marker
            .attempted
            .fetch_add(self.attempts, Ordering::SeqCst);
        self.marker
            .delivered
            .fetch_add(self.deliveries, Ordering::SeqCst);
        if self.raises {
            Err(Reason::new("synthetic_failure"))
        } else if self.delivered {
            Ok(Done::Done)
        } else {
            Ok(Done::NotDelivered)
        }
    }
}

#[tokio::test]
async fn the_catch_up_decision_matches_the_predecessors_golden() {
    let mut cases = Vec::new();
    golden::each_case("catchup", |case| {
        cases.push((case.input.clone(), case.output.clone()));
    });
    let (_directory, db) = fresh().await;
    let sync_runs = SqliteSyncRuns::new(db.clone());
    // The golden's instants are local times; the decision is the same at any offset.
    let offsets = [0, 330, -300];
    let mut decided = 0_usize;
    for (input, output) in &cases {
        let id: &'static str = Box::leak(
            input["job_id"]
                .as_str()
                .expect("a job id")
                .to_owned()
                .into_boxed_str(),
        );
        let hour = u8::try_from(input["hour"].as_u64().expect("an hour")).expect("an hour");
        let minute = u8::try_from(input["minute"].as_u64().expect("a minute")).expect("a minute");
        let job = Job {
            id,
            schedule: Schedule::DailyAt { hour, minute },
            catch_up: true,
        };
        let flag = |name: &str| input[name].as_bool().expect("a flag");
        let count = |name: &str| input[name].as_u64().expect("a count");
        for minutes in offsets {
            let offset = UtcOffset::from_minutes(minutes).expect("an offset");
            let rule = StudyDayRule::new(StudyDayRule::default().rollover_hour(), offset);
            let local = input["now_ms"].as_i64().expect("an instant");
            let clock = ManualClock::new(UtcMillis::from_epoch_millis(
                local - i64::from(minutes) * MINUTE_MS,
            ));
            let events = Arc::new(Mutex::new(Vec::new()));
            let ledger = Recording {
                claim: flag("claim"),
                events: Arc::clone(&events),
            };
            let marker = FakeMarker::default();
            let body = Body {
                events: Arc::clone(&events),
                marker: &marker,
                attempts: count("attempts"),
                deliveries: count("deliveries"),
                delivered: flag("delivered"),
                raises: flag("raises"),
            };
            let runner = Runner::new(&ledger, &sync_runs, &marker, &clock, rule);
            let report = runner.run(&job, &body).await.expect("the run");
            let mut events = events
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            if !report.pages.is_empty() {
                events.push(json!(["alert"]));
            }
            let ran: Vec<&str> = match report.decision {
                Decision::Ran {
                    outcome: Outcome::Ok,
                    ..
                } => vec![id],
                _ => Vec::new(),
            };
            assert_eq!(
                json!({ "ran": ran, "events": events }),
                *output,
                "the decision for {input} at offset {minutes}"
            );
            decided += 1;
        }
    }
    println!("examined {decided} decision(s) over the golden's cases and offsets");
    assert_eq!(decided, cases.len() * offsets.len());
}

#[tokio::test]
async fn an_attempted_send_without_a_message_id_releases_the_claim() {
    // Each scenario: its sends, whether a message id came back, and whether the claim survives.
    let scenarios = [
        ("an attempted send with no message id", 1, 0, true),
        ("a job that never engaged the notifier", 0, 0, false),
        ("a send that returned a message id", 1, 1, false),
    ];
    for (what, attempts, deliveries, released) in examined("scenario(s)", scenarios.to_vec()) {
        let (_directory, db) = fresh().await;
        let ledger = SqliteCronLedger::new(db.clone());
        let sync_runs = SqliteSyncRuns::new(db.clone());
        let marker = FakeMarker::default();
        let clock = ManualClock::new(at(DAY, 9, 5));
        let runner = Runner::new(
            &ledger,
            &sync_runs,
            &marker,
            &clock,
            StudyDayRule::default(),
        );
        let work = Notification::new(&marker, attempts, deliveries, false);

        let first = runner.run(&NOTIFY, &work).await.expect("the run");
        assert_eq!(
            first.decision,
            Decision::Ran {
                fire_date: FireDate::from_epoch_day(DAY),
                outcome: Outcome::Ok,
                released,
            },
            "{what}"
        );
        let after = row(&db, &NOTIFY, DAY).await.expect("the fire has its row");
        let counted = if released { 0 } else { 1 };
        assert_eq!(
            (after.catchup_count, after.ok_count),
            (counted, counted),
            "{what}: {after:?}"
        );

        // A later start within the window claims again only when the claim was released.
        clock.advance(std::time::Duration::from_secs(600));
        let second = runner.run(&NOTIFY, &work).await.expect("the later run");
        let expected_runs = if released { 2 } else { 1 };
        assert_eq!(work.runs(), expected_runs, "{what}: {second:?}");
    }
}

/// A sync cycle that answers each call with the next scripted outcome, and records it in
/// `sync_runs` as the real cycle would.
struct ScriptedCycle<'a> {
    calls: AtomicUsize,
    script: Mutex<Vec<Result<(), ReasonCode>>>,
    sync_runs: &'a SqliteSyncRuns,
    clock: &'a ManualClock,
}

impl<'a> ScriptedCycle<'a> {
    fn new(
        script: Vec<Result<(), ReasonCode>>,
        sync_runs: &'a SqliteSyncRuns,
        clock: &'a ManualClock,
    ) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            script: Mutex::new(script.into_iter().rev().collect()),
            sync_runs,
            clock,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl SyncCycle for ScriptedCycle<'_> {
    async fn run_scheduled(&self) -> Result<SyncReport, Reason> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let outcome = self
            .script
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()
            .unwrap_or(Ok(()));
        let now = self.clock.now();
        let run = SyncRun {
            trigger: Trigger::Scheduled,
            started_at: now,
            finished_at: now,
            study_day: StudyDayRule::default().study_day(now),
            outcome,
            attempts: if outcome.is_ok() {
                1
            } else {
                SYNC_RETRY_ATTEMPTS
            },
            full_download: false,
        };
        self.sync_runs
            .record(&run)
            .await
            .map_err(|_| Reason::new("sync_record_failed"))?;
        Ok(SyncReport::Ran {
            run,
            waits_seconds: Vec::new(),
        })
    }
}

#[tokio::test]
async fn the_third_consecutive_sync_failure_pages_once() {
    let (_directory, db) = fresh().await;
    let ledger = SqliteCronLedger::new(db.clone());
    let sync_runs = SqliteSyncRuns::new(db.clone());
    let clock = ManualClock::new(at(DAY, 4, 8));
    let runner = Runner::new(
        &ledger,
        &sync_runs,
        &NoNotifier,
        &clock,
        StudyDayRule::default(),
    );
    // Each study day's one scheduled sync fails after its bounded retries, three days running;
    // then it succeeds; then it fails again.
    let failure = Err(ReasonCode::NetworkUnreachable);
    let script = vec![failure, failure, failure, Ok(()), failure];
    let cycle = ScriptedCycle::new(script.clone(), &sync_runs, &clock);
    let mut reports: Vec<Report> = Vec::new();
    for day in 0..5 {
        clock.set(at(DAY + day, 4, 8));
        reports.push(runner.run_job("sync", &cycle, &db).await.expect("the run"));
    }
    let exits: Vec<u8> = reports.iter().map(Report::exit_code).collect();
    assert_eq!(
        exits,
        [1, 0, 0, 0, 1],
        "the first failure of each episode pages, and a repeat only logs: {reports:?}"
    );
    let attempts = i64::from(SYNC_RETRY_ATTEMPTS);
    assert_eq!(
        reports[0].pages,
        [Reason::new("network_unreachable").with("attempts", attempts)],
        "the page carries the reason code and the attempts"
    );
    assert_eq!(cycle.calls(), 5, "one scheduled sync per study day");
    for (day, outcome) in (0..5).zip(&script) {
        let recorded = row(&db, &jobs_sync(), DAY + day)
            .await
            .expect("each day's sync has its row");
        let expected = if outcome.is_ok() {
            Outcome::Ok
        } else {
            Outcome::Error
        };
        assert_eq!(recorded.last_outcome, expected, "day {day}: {recorded:?}");
    }
    // Three consecutive failures: three errors in sync_runs, and exactly one page among them.
    let paged = reports[..3]
        .iter()
        .filter(|report| report.exit_code() == Report::PAGE)
        .count();
    assert_eq!(paged, 1, "three consecutive failures page once");
}

/// The table's `sync` job.
const fn jobs_sync() -> Job {
    deck_streak_coordination::jobs::SYNC
}

/// A job's work that keeps the study day's sync outcome the runner handed it.
struct Reader {
    seen: Mutex<Vec<Option<StudyDayOutcome>>>,
}

impl Work for Reader {
    async fn perform(&self, fire: &Fire) -> Result<Done, Reason> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(fire.sync);
        Ok(Done::Done)
    }
}

#[tokio::test]
async fn no_job_but_sync_runs_the_sync_cycle() {
    let (_directory, db) = fresh().await;
    let ledger = SqliteCronLedger::new(db.clone());
    let sync_runs = SqliteSyncRuns::new(db.clone());
    let clock = ManualClock::new(at(DAY, 4, 8));
    let runner = Runner::new(
        &ledger,
        &sync_runs,
        &NoNotifier,
        &clock,
        StudyDayRule::default(),
    );
    let cycle = ScriptedCycle::new(vec![Ok(())], &sync_runs, &clock);

    // The day's one scheduled sync runs the cycle once.
    let synced = runner.run_job("sync", &cycle, &db).await.expect("the sync");
    assert!(
        matches!(
            synced.decision,
            Decision::Ran {
                outcome: Outcome::Ok,
                ..
            }
        ),
        "{synced:?}"
    );
    assert_eq!(cycle.calls(), 1, "the sync job runs the cycle");

    // Every other job of the table runs without it.
    let others = [
        ("maintenance", at(DAY, 4, 29)),
        ("liveness", at(DAY, 5, 15)),
    ];
    for (id, when) in examined("other job(s) of the table", others.to_vec()) {
        clock.set(when);
        let report = runner.run_job(id, &cycle, &db).await.expect("the run");
        assert!(
            matches!(
                report.decision,
                Decision::Ran {
                    outcome: Outcome::Ok,
                    ..
                }
            ),
            "{id}: {report:?}"
        );
        assert_eq!(cycle.calls(), 1, "{id} ran the sync cycle");
    }

    // A job that needs the study day's data reads the day's sync outcome and never syncs.
    clock.set(at(DAY, 9, 6));
    let reader = Reader {
        seen: Mutex::new(Vec::new()),
    };
    let report = runner
        .run(&READER, &reader)
        .await
        .expect("the reader's run");
    assert!(
        matches!(
            report.decision,
            Decision::Ran {
                outcome: Outcome::Ok,
                ..
            }
        ),
        "{report:?}"
    );
    let seen = reader
        .seen
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(
        seen,
        [Some(StudyDayOutcome {
            synced: true,
            trigger: Trigger::Scheduled,
        })],
        "the reader was handed the study day's sync outcome"
    );
    assert_eq!(cycle.calls(), 1, "the reader ran the sync cycle");
}
