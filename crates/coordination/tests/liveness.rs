//! The dead-man watch pages once per episode when no sync succeeded within its window, and a
//! maintenance fire drifting past the tolerance pages (SPEC-027 A7, A8; R7, R8).

// An integration test is test code: its helpers panic on a failed database, and the examined
// counts are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::sync::atomic::{AtomicUsize, Ordering};

use deck_streak_coordination::delivery::NoNotifier;
use deck_streak_coordination::ledger::SqliteCronLedger;
use deck_streak_coordination::liveness::{
    DEAD_MAN_WINDOW_SECS, LIVENESS_BOOT_GRACE_SECS, SyncHistory, check, rollover_skew_minutes,
    signed_skew_minutes,
};
use deck_streak_coordination::runner::{Reason, Report, Runner, SyncCycle};
use deck_streak_ingest::sync::SyncReport;
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger};
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, UtcMillis};
use tempfile::TempDir;

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
/// A synthetic day, as an epoch day number.
const DAY: i64 = 20_000;

/// One day of the drift test: the maintenance fire's hour and minute, then the pages of the first
/// check after it and of a later one.
type Day = ((i64, i64), Vec<Reason>, Vec<Reason>);

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

/// A sync cycle no job of these tests may run: it counts the calls that would be a defect.
#[derive(Default)]
struct NoCycle {
    calls: AtomicUsize,
}

impl SyncCycle for NoCycle {
    async fn run_scheduled(&self) -> Result<SyncReport, Reason> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(Reason::new("no_cycle_in_this_test"))
    }
}

/// Records one sync run that started and finished at `at`.
async fn record_sync(sync_runs: &SqliteSyncRuns, at: UtcMillis, outcome: Result<(), ReasonCode>) {
    sync_runs
        .record(&SyncRun {
            trigger: Trigger::Scheduled,
            started_at: at,
            finished_at: at,
            study_day: StudyDayRule::default().study_day(at),
            outcome,
            attempts: 1,
            full_download: false,
        })
        .await
        .expect("the run is recorded");
}

/// Runs the liveness job at every hourly slot (minute 14) from `from` to `to`, both included, and
/// returns each check's instant with its report.
async fn hourly(
    runner: &Runner<'_, SqliteCronLedger>,
    clock: &ManualClock,
    db: &Db,
    cycle: &NoCycle,
    from: UtcMillis,
    to: UtcMillis,
) -> Vec<(UtcMillis, Report)> {
    let mut checks = Vec::new();
    let mut now = from;
    while now <= to {
        clock.set(now);
        let report = runner
            .run_job("liveness", cycle, db)
            .await
            .expect("the check runs");
        checks.push((now, report));
        now = UtcMillis::from_epoch_millis(now.epoch_millis() + HOUR_MS);
    }
    checks
}

/// The checks among `checks` that paged, by their instants.
fn paged(checks: &[(UtcMillis, Report)]) -> Vec<UtcMillis> {
    checks
        .iter()
        .filter(|(_, report)| report.exit_code() == Report::PAGE)
        .map(|(now, _)| *now)
        .collect()
}

#[tokio::test]
async fn the_dead_man_watch_pages_once_when_no_sync_succeeded_within_the_window() {
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
    let cycle = NoCycle::default();

    // The window is one study day plus the catch-up window: 30 hours.
    assert_eq!(DEAD_MAN_WINDOW_SECS, 30 * 3600);

    // A success at 04:08, then two days of hourly checks with no other success.
    record_sync(&sync_runs, at(DAY, 4, 8), Ok(())).await;
    let checks = hourly(
        &runner,
        &clock,
        &db,
        &cycle,
        at(DAY, 5, 14),
        at(DAY + 2, 3, 14),
    )
    .await;
    println!("examined {} hourly check(s)", checks.len());
    // 30 hours after 04:08 is 10:08 the next day: the check at 10:14 is the first to find it dead.
    assert_eq!(
        paged(&checks),
        [at(DAY + 1, 10, 14)],
        "one page for the episode"
    );
    let (_, first) = checks
        .iter()
        .find(|(now, _)| *now == at(DAY + 1, 10, 14))
        .expect("the paging check");
    assert_eq!(
        first.pages,
        [Reason::new("sync_dead").with("silent_secs", 30 * 3600 + 6 * 60)],
        "the page carries the reason code and the seconds since the last success"
    );

    // The sync recovers, then stops again: the next episode pages once more.
    record_sync(&sync_runs, at(DAY + 2, 4, 8), Ok(())).await;
    let later = hourly(
        &runner,
        &clock,
        &db,
        &cycle,
        at(DAY + 2, 4, 14),
        at(DAY + 4, 4, 14),
    )
    .await;
    assert_eq!(
        paged(&later),
        [at(DAY + 3, 10, 14)],
        "the second episode pages once"
    );
    assert_eq!(
        cycle.calls.load(Ordering::SeqCst),
        0,
        "the watch never syncs"
    );

    // With no success ever, the first recorded attempt starts the boot grace: 15 minutes.
    let (_directory, fresh_db) = fresh().await;
    let ledger = SqliteCronLedger::new(fresh_db.clone());
    let sync_runs = SqliteSyncRuns::new(fresh_db.clone());
    let runner = Runner::new(
        &ledger,
        &sync_runs,
        &NoNotifier,
        &clock,
        StudyDayRule::default(),
    );
    assert_eq!(LIVENESS_BOOT_GRACE_SECS, 900);
    record_sync(&sync_runs, at(DAY, 4, 8), Err(ReasonCode::AuthRejected)).await;
    let boot = hourly(
        &runner,
        &clock,
        &fresh_db,
        &cycle,
        at(DAY, 4, 14),
        at(DAY, 9, 14),
    )
    .await;
    assert_eq!(
        paged(&boot),
        [at(DAY, 5, 14)],
        "the grace passed at 04:23: 05:14 pages once"
    );
}

#[tokio::test]
async fn a_maintenance_fire_drifting_past_the_tolerance_pages() {
    // The skew arithmetic is the predecessor's.
    golden::each_case("signed_skew", |case| {
        let a = case.input["a_min"].as_i64().expect("a minute");
        let b = case.input["b_min"].as_i64().expect("a minute");
        assert_eq!(
            signed_skew_minutes(a, b),
            case.output.as_i64().expect("a skew"),
            "the skew of {}",
            case.input
        );
    });
    golden::each_case("rollover_skew", |case| {
        let field = |name: &str| case.input[name].as_i64().expect("an integer");
        assert_eq!(
            rollover_skew_minutes(
                field("cron_hour"),
                field("scheduler_utc_offset_min"),
                field("collection_rollover_hour"),
                field("collection_utc_offset_min"),
            ),
            case.output.as_i64().expect("a skew"),
            "the rollover skew of {}",
            case.input
        );
    });

    // Each day the maintenance job fires at a time of the timer's, then the liveness watch checks
    // at 05:14 and again at 06:14. The slot is 04:28, the default rollover hour at minute 28.
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
    let cycle = NoCycle::default();
    record_sync(&sync_runs, at(DAY, 4, 8), Ok(())).await;
    // (the maintenance fire's hour and minute, the pages of the 05:14 check, of the 06:14 check)
    let days: [Day; 4] = [
        (
            (5, 13),
            vec![Reason::new("maintenance_drift").with("skew_min", 45)],
            vec![],
        ),
        ((4, 28), vec![], vec![]),
        ((4, 57), vec![], vec![]),
        (
            (5, 0),
            vec![Reason::new("maintenance_drift").with("skew_min", 32)],
            vec![],
        ),
    ];
    for (day, ((hour, minute), first, second)) in (0..).zip(days) {
        // A sync each day keeps the dead-man watch quiet.
        record_sync(&sync_runs, at(DAY + day, 4, 8), Ok(())).await;
        clock.set(at(DAY + day, hour, minute));
        let fired = runner
            .run_job("maintenance", &cycle, &db)
            .await
            .expect("the maintenance runs");
        assert_eq!(fired.exit_code(), 0, "day {day}: {fired:?}");
        clock.set(at(DAY + day, 5, 14));
        let checked = runner
            .run_job("liveness", &cycle, &db)
            .await
            .expect("the check runs");
        assert_eq!(
            checked.pages, first,
            "day {day}: the first check after the fire"
        );
        clock.set(at(DAY + day, 6, 14));
        let again = runner
            .run_job("liveness", &cycle, &db)
            .await
            .expect("the check runs again");
        assert_eq!(
            again.pages, second,
            "day {day}: a later check of the same fire"
        );
    }
    println!("examined 4 maintenance fire(s)");
}
#[test]
fn the_watch_pages_just_past_each_of_its_boundaries() {
    let rule = StudyDayRule::default();
    let later = |instant: UtcMillis, millis: i64| {
        UtcMillis::from_epoch_millis(instant.epoch_millis() + millis)
    };
    let dead = |silent_secs: i64| Reason::new("sync_dead").with("silent_secs", silent_secs);
    let quiet: Vec<Reason> = Vec::new();

    // One success: dead from the end of the window, and not a millisecond before.
    let success = at(DAY, 4, 8);
    let history = SyncHistory {
        last_success_at: Some(success),
        first_attempt_at: Some(success),
    };
    let dead_after = later(success, DEAD_MAN_WINDOW_SECS * 1000);
    assert_eq!(history.dead_after(), Some(dead_after));
    assert_eq!(check(dead_after, None, history, None, rule), quiet);
    assert_eq!(
        check(later(dead_after, 1), None, history, None, rule),
        [dead(DEAD_MAN_WINDOW_SECS)]
    );
    // The previous check ran at the very instant it died, so it had not found it dead: this one is
    // the first. One that ran a millisecond later had, so this one only repeats.
    assert_eq!(
        check(later(dead_after, 2), Some(dead_after), history, None, rule),
        [dead(DEAD_MAN_WINDOW_SECS)]
    );
    assert_eq!(
        check(
            later(dead_after, 2),
            Some(later(dead_after, 1)),
            history,
            None,
            rule
        ),
        quiet
    );

    // Never a success: the grace counts from the first attempt.
    let never = SyncHistory {
        last_success_at: None,
        first_attempt_at: Some(success),
    };
    let grace_end = later(success, LIVENESS_BOOT_GRACE_SECS * 1000);
    assert_eq!(never.dead_after(), Some(grace_end));
    assert_eq!(check(grace_end, None, never, None, rule), quiet);
    assert_eq!(
        check(later(grace_end, 1000), None, never, None, rule),
        [Reason::new("sync_never_succeeded").with("since_first_attempt_secs", 901)]
    );
    assert_eq!(
        SyncHistory::default().dead_after(),
        None,
        "nothing attempted, nothing dead"
    );

    // The drift: 30 minutes off the 04:28 slot is within the tolerance, 31 is not, late or early;
    // and only the first check after the fire pages on it.
    let alive = SyncHistory {
        last_success_at: Some(success),
        first_attempt_at: Some(success),
    };
    let now = at(DAY, 5, 14);
    let drift = |skew_min: i64| Reason::new("maintenance_drift").with("skew_min", skew_min);
    let off_slot = [
        (at(DAY, 4, 58), vec![]),
        (at(DAY, 4, 59), vec![drift(31)]),
        (at(DAY, 3, 58), vec![]),
        (at(DAY, 3, 57), vec![drift(-31)]),
    ];
    for (fired, expected) in &off_slot {
        assert_eq!(check(now, None, alive, Some(*fired), rule), *expected);
    }
    println!(
        "examined {} maintenance fire(s) against the tolerance",
        off_slot.len()
    );
    let fired = at(DAY, 4, 59);
    assert_eq!(check(now, Some(fired), alive, Some(fired), rule), quiet);
    assert_eq!(
        check(now, Some(later(fired, -1)), alive, Some(fired), rule),
        [drift(31)]
    );
}
