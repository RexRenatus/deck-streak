//! The job table is the one schedule: `sync` holds one daily slot claimed per study day, no job
//! shares a minute with the predecessor's schedule, and the scheduler's constants are the
//! predecessor's (SPEC-027 A9, A10, A15, A17; R1, R2).

// An integration test is test code: its helpers panic on a malformed golden, and the examined
// counts are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use deck_streak_coordination::jobs::{
    self, CATCHUP_MAX_LATE_MIN, FireDate, Job, SYNC_TICK_OFFSET_MIN, Schedule, TABLE,
    TICK_ALIGN_MIN_INTERVAL_MIN, tick_minutes,
};
use deck_streak_coordination::ledger::Outcome;
use deck_streak_coordination::liveness::{LIVENESS_BOOT_GRACE_SECS, ROLLOVER_DRIFT_TOLERANCE_MIN};
use deck_streak_coordination::maintenance::CRON_FIRES_RETENTION_DAYS;
use deck_streak_kernel::{Hour, StudyDayRule, UtcMillis, UtcOffset};
use serde_json::{Value, json};

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;

/// The reserved minutes (ADR-011): minutes of the hour at which no job of the table may fire.
const RESERVED_MINUTES: [i64; 3] = [0, 25, 39];

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The minutes of the hour a predecessor cron field names: a comma list of whole minutes.
fn minutes_of(field: &str) -> Vec<i64> {
    field
        .split(',')
        .map(|minute| minute.parse().expect("a whole minute"))
        .collect()
}

/// The predecessor's in-process schedule: its sync interval, and each job's id and minute field.
fn predecessor_schedule() -> (i64, Vec<(String, String)>) {
    let mut interval = None;
    let mut jobs = Vec::new();
    golden::each_case("predecessor_schedule", |case| {
        interval = case.output["sync_interval_min"].as_i64();
        for job in case.output["jobs"]
            .as_array()
            .expect("the golden lists jobs")
        {
            let id = job["id"].as_str().expect("a job's id").to_owned();
            let minute = job["fields"]["minute"]
                .as_str()
                .expect("every job names its minutes")
                .to_owned();
            jobs.push((id, minute));
        }
    });
    (interval.expect("the default sync interval"), jobs)
}

/// Every minute of the local day `job` fires at, the rollover hour being `rollover`.
fn minutes_of_day(job: &Job, rollover: Hour) -> Vec<i64> {
    match job.schedule.daily_slot(rollover) {
        Some((hour, minute)) => vec![i64::from(hour) * 60 + i64::from(minute)],
        None => (0..24)
            .map(|hour| hour * 60 + i64::from(job.schedule.minute()))
            .collect(),
    }
}

#[test]
fn the_scheduler_constants_equal_the_predecessors() {
    let port: [(&str, Value); 7] = [
        (
            "scheduler.CATCHUP_MAX_LATE_MIN",
            json!(CATCHUP_MAX_LATE_MIN),
        ),
        (
            "database.CRON_FIRES_RETENTION_DAYS",
            json!(CRON_FIRES_RETENTION_DAYS),
        ),
        (
            "database.CRON_OUTCOMES",
            json!(Outcome::ALL.map(Outcome::as_str)),
        ),
        (
            "constants.SYNC_TICK_OFFSET_MIN",
            json!(SYNC_TICK_OFFSET_MIN),
        ),
        (
            "constants.TICK_ALIGN_MIN_INTERVAL_MIN",
            json!(TICK_ALIGN_MIN_INTERVAL_MIN),
        ),
        (
            "constants.LIVENESS_BOOT_GRACE_SECS",
            json!(LIVENESS_BOOT_GRACE_SECS),
        ),
        (
            "constants.ROLLOVER_DRIFT_TOLERANCE_MIN",
            json!(ROLLOVER_DRIFT_TOLERANCE_MIN),
        ),
    ];
    let mut compared = BTreeSet::new();
    golden::each_case("scheduler.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        let (_, value) = port
            .iter()
            .find(|(held, _)| *held == name)
            .unwrap_or_else(|| panic!("the port holds no {name}"));
        assert_eq!(
            value, &case.output,
            "{name}: the port holds {value}, the predecessor {}",
            case.output
        );
        compared.insert(name.to_owned());
    });
    // Every constant the port holds was compared with the predecessor's.
    assert_eq!(compared.len(), port.len(), "compared {compared:?}");
}

#[test]
fn the_daily_sync_slot_keeps_off_the_predecessors_ticks_and_every_other_slot() {
    // The tick function is the predecessor's, over every case its golden draws.
    golden::each_case("tick_minutes", |case| {
        let interval = case.input["interval_min"].as_i64().expect("an interval");
        let offset = case.input["offset_min"].as_i64().expect("an offset");
        let expected: Vec<i64> = case
            .output
            .as_array()
            .expect("a list of minutes")
            .iter()
            .map(|minute| minute.as_i64().expect("a minute"))
            .collect();
        assert_eq!(
            tick_minutes(interval, offset),
            expected,
            "the ticks of {}",
            case.input
        );
    });

    // The predecessor's sync ticks: its tick function at its default interval and its offset, which
    // are the minutes its own schedule registers the sync at.
    let (interval, schedule) = predecessor_schedule();
    let ticks = tick_minutes(interval, SYNC_TICK_OFFSET_MIN);
    let registered = schedule
        .iter()
        .find(|(id, _)| id == "sync")
        .map(|(_, minute)| minutes_of(minute))
        .expect("the predecessor schedules its sync");
    assert_eq!(ticks, registered, "the ticks at interval {interval}");
    assert_eq!(
        ticks.len(),
        usize::try_from(60 / interval).expect("a count")
    );

    // The daily sync keeps off every tick.
    let sync_minute = i64::from(jobs::SYNC.schedule.minute());
    for tick in examined("predecessor sync tick(s)", ticks) {
        assert!(
            sync_minute != tick,
            "the daily sync fires at minute {sync_minute}, the predecessor's sync tick {tick}"
        );
    }

    // And no other job of the table fires at the sync's minute of the day, whatever the rollover.
    let mut compared = 0_usize;
    for hour in 0..24 {
        let rollover = Hour::new(hour).expect("an hour");
        let sync_slot = minutes_of_day(&jobs::SYNC, rollover);
        assert_eq!(sync_slot.len(), 1, "the sync has one slot a day");
        for job in TABLE.iter().filter(|job| job.id != jobs::SYNC.id) {
            for minute in minutes_of_day(job, rollover) {
                assert!(
                    minute != sync_slot[0],
                    "{} fires at minute {minute} of the day, the sync's slot at rollover {hour}",
                    job.id
                );
                compared += 1;
            }
        }
    }
    println!("examined {compared} slot(s) of the other jobs against the sync's");
    let slots_a_day: usize = TABLE
        .iter()
        .filter(|job| job.id != jobs::SYNC.id)
        .map(|job| if job.once_a_day() { 1 } else { 24 })
        .sum();
    assert_eq!(
        compared,
        24 * slots_a_day,
        "every other job's slots, at every rollover hour"
    );
}

#[test]
fn no_job_shares_a_minute_with_the_predecessors_schedule() {
    let (interval, schedule) = predecessor_schedule();
    let mut kept_off: Vec<(String, i64)> = Vec::new();
    for (id, field) in &schedule {
        for minute in minutes_of(field) {
            kept_off.push((format!("the predecessor's in-process job {id}"), minute));
        }
    }
    for tick in tick_minutes(interval, SYNC_TICK_OFFSET_MIN) {
        kept_off.push(("the predecessor's sync tick".to_owned(), tick));
    }
    for minute in RESERVED_MINUTES {
        kept_off.push(("a reserved minute (ADR-011)".to_owned(), minute));
    }
    let kept_off = examined("minute(s) the table keeps off", kept_off);

    let mut ours = BTreeSet::new();
    for job in examined("job(s) of the table", TABLE.to_vec()) {
        let minute = i64::from(job.schedule.minute());
        for (what, taken) in &kept_off {
            assert!(
                minute != *taken,
                "{} fires at minute {minute}, which {what} takes",
                job.id
            );
        }
        ours.insert(minute);
    }
    // No two jobs of the table share a minute either.
    assert_eq!(ours.len(), TABLE.len(), "the table's minutes: {ours:?}");
}

#[test]
fn the_job_table_holds_sync_to_one_daily_slot_claimed_per_study_day() {
    let syncs: Vec<Job> = TABLE
        .iter()
        .filter(|job| job.id == "sync")
        .copied()
        .collect();
    assert_eq!(syncs, [jobs::SYNC], "the table holds exactly one sync");
    let sync = syncs[0];
    assert_eq!(sync.schedule, Schedule::DailyAtRollover { minute: 7 });
    assert!(
        sync.once_a_day(),
        "the sync is claimed like every once-a-day job"
    );
    assert!(
        sync.catch_up,
        "a missed sync runs once, within the catch-up window"
    );
    let claimed: Vec<&str> = TABLE
        .iter()
        .filter(|job| job.once_a_day())
        .map(|job| job.id)
        .collect();
    assert_eq!(
        claimed,
        ["sync", "maintenance", "held_flush"],
        "the jobs that claim a fire date"
    );
    assert_eq!(jobs::job("sync"), Some(jobs::SYNC));

    // Whatever the rollover hour and the offset, each scheduled sync fires at the rollover hour,
    // minute 7, local; its fire date is the study day it runs in; and one day's sync follows the
    // last one's study day by exactly one.
    let offsets = [-720, -300, 0, 330, 840];
    let mut fires = 0_usize;
    for hour in 0..24 {
        for minutes in offsets {
            let offset = UtcOffset::from_minutes(minutes).expect("an offset");
            let rule = StudyDayRule::new(Hour::new(hour).expect("an hour"), offset);
            let mut previous = None;
            for day in 0..30 {
                let now = UtcMillis::from_epoch_millis(
                    (20_000 + day) * DAY_MS + 13 * HOUR_MS + 29 * MINUTE_MS,
                );
                let fire = sync.schedule.latest_at_or_before(now, rule);
                let local = fire.epoch_millis() + i64::from(minutes) * MINUTE_MS;
                assert_eq!(
                    local.rem_euclid(DAY_MS),
                    i64::from(hour) * HOUR_MS + 7 * MINUTE_MS,
                    "the sync fires at the rollover hour, minute 7, at rollover {hour}, offset \
                     {minutes}"
                );
                assert!(fire <= now, "the fire is at or before now");
                let study_day = rule.study_day(fire);
                assert_eq!(
                    FireDate::of(fire, offset).epoch_day(),
                    study_day.epoch_day(),
                    "the fire date is the study day at rollover {hour}, offset {minutes}"
                );
                if let Some(previous) = previous {
                    assert_eq!(
                        study_day.epoch_day(),
                        previous + 1,
                        "one sync per study day"
                    );
                }
                previous = Some(study_day.epoch_day());
                fires += 1;
            }
        }
    }
    println!("examined {fires} scheduled sync fire(s)");
    assert_eq!(fires, 24 * offsets.len() * 30);
}

/// The latest instant at or before `now` that `schedule` fires at, found the slow and obvious way:
/// a scan back from `now`, one minute at a time, to the first local hour and minute the schedule
/// names.
fn scanned(schedule: Schedule, now: UtcMillis, rule: StudyDayRule) -> UtcMillis {
    let offset = i64::from(rule.utc_offset().minutes()) * MINUTE_MS;
    let mut minute = now.epoch_millis().div_euclid(MINUTE_MS) * MINUTE_MS;
    loop {
        let local = minute + offset;
        let hour = local.rem_euclid(DAY_MS) / HOUR_MS;
        let of_hour = local.rem_euclid(HOUR_MS) / MINUTE_MS;
        let fires = match schedule.daily_slot(rule.rollover_hour()) {
            Some((slot_hour, slot_minute)) => {
                hour == i64::from(slot_hour) && of_hour == i64::from(slot_minute)
            }
            None => of_hour == i64::from(schedule.minute()),
        };
        if fires {
            return UtcMillis::from_epoch_millis(minute);
        }
        minute -= MINUTE_MS;
    }
}

#[test]
fn every_schedule_answers_its_latest_fire_at_every_offset() {
    let schedules = [
        jobs::SYNC.schedule,
        jobs::MAINTENANCE.schedule,
        jobs::LIVENESS.schedule,
        Schedule::DailyAt { hour: 0, minute: 0 },
        Schedule::DailyAt {
            hour: 23,
            minute: 59,
        },
    ];
    let mut answered = 0_usize;
    for schedule in schedules {
        for hour in [0, 4, 23] {
            for minutes in [-720, -300, 0, 330, 840] {
                let offset = UtcOffset::from_minutes(minutes).expect("an offset");
                let rule = StudyDayRule::new(Hour::new(hour).expect("an hour"), offset);
                // Instants on every side of a slot: the minute itself, a millisecond before and
                // after it, and the minutes around local midnight.
                let base = 20_000 * DAY_MS - i64::from(minutes) * MINUTE_MS;
                let mut instants = Vec::new();
                for local in [
                    i64::from(hour) * HOUR_MS + 7 * MINUTE_MS,
                    i64::from(hour) * HOUR_MS + 28 * MINUTE_MS,
                    14 * MINUTE_MS,
                    DAY_MS - MINUTE_MS,
                    DAY_MS + 30 * MINUTE_MS,
                    13 * HOUR_MS + 14 * MINUTE_MS,
                ] {
                    for nudge in [-1, 0, 1, 59_999] {
                        instants.push(UtcMillis::from_epoch_millis(base + local + nudge));
                    }
                }
                for now in instants {
                    let expected = scanned(schedule, now, rule);
                    assert_eq!(
                        schedule.latest_at_or_before(now, rule),
                        expected,
                        "{schedule:?} at {} with rollover {hour}, offset {minutes}",
                        now.epoch_millis()
                    );
                    let local_day = |instant: UtcMillis| {
                        (instant.epoch_millis() + i64::from(minutes) * MINUTE_MS).div_euclid(DAY_MS)
                    };
                    let today = local_day(expected) == local_day(now);
                    assert_eq!(
                        schedule.latest_elapsed_today(now, rule),
                        today.then_some(expected),
                        "{schedule:?} today at {} with rollover {hour}, offset {minutes}",
                        now.epoch_millis()
                    );
                    assert_eq!(
                        FireDate::of(expected, offset).epoch_day(),
                        local_day(expected)
                    );
                    answered += 1;
                }
            }
        }
    }
    println!("examined {answered} instant(s) of five schedules");
    assert_eq!(answered, 5 * 3 * 5 * 6 * 4);
}

/// The job template each job's timer starts an instance of (SPEC-032 R1). An instance's name is
/// built from it at run time and never written whole, because the public scrub reads that shape as
/// an email address (SPEC-032 R10).
const JOB_TEMPLATE: &str = "deck-streak-job";

/// The `[Timer]` section of a unit file: each key's values, in the order they appear.
fn timer_section(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut section = String::new();
    let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            name.clone_into(&mut section);
        } else if section == "Timer"
            && let Some((key, value)) = line.split_once('=')
        {
            values
                .entry(key.trim().to_owned())
                .or_default()
                .push(value.trim().to_owned());
        }
    }
    values
}

/// The calendar a job's timer carries in the templates: the job's slot in the table at `rule`'s
/// rollover hour, written in UTC, the neutral zone the private rail replaces (ADR-027).
fn neutral_calendar(schedule: Schedule, rule: StudyDayRule) -> String {
    if let Some((hour, minute)) = schedule.daily_slot(rule.rollover_hour()) {
        format!("*-*-* {hour:02}:{minute:02}:00 UTC")
    } else {
        let minute = schedule.minute();
        format!("*-*-* *:{minute:02}:00 UTC")
    }
}

#[test]
fn the_job_table_holds_the_drill_postback() {
    assert!(
        TABLE.contains(&jobs::DRILL_POSTBACK),
        "the table lists the drill post-back"
    );
    let job = jobs::job("drill_postback").unwrap_or(jobs::DRILL_POSTBACK);
    assert_eq!(
        jobs::job("drill_postback"),
        Some(job),
        "the table looks the job up by its id"
    );
    assert_eq!(job, jobs::DRILL_POSTBACK, "the table's entry");
    assert_eq!(
        job.schedule,
        Schedule::Hourly { minute: 19 },
        "hourly, at minute 19"
    );
    assert!(!job.catch_up, "a missed post-back is not run late");
    assert!(TABLE.contains(&job), "the table lists the job");
    assert!(
        !RESERVED_MINUTES.contains(&i64::from(job.schedule.minute())),
        "a reserved minute"
    );
    println!("examined {} job(s) of the table", TABLE.len());
}

#[test]
fn every_timer_calendar_equals_its_job_table_entry() {
    let systemd = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/systemd");
    let prefix = format!("{JOB_TEMPLATE}@");
    let mut found = Vec::new();
    for entry in fs::read_dir(&systemd).expect("deploy/systemd is readable") {
        let path = entry.expect("a directory entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        if let Some(id) = name
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(".timer"))
        {
            let text = fs::read_to_string(&path).expect("a readable timer");
            found.push((id.to_owned(), timer_section(&text)));
        }
    }
    let timers: BTreeMap<String, BTreeMap<String, Vec<String>>> =
        examined("job timer(s) under deploy/systemd", found)
            .into_iter()
            .collect();

    // One timer per job of the table, and none for a job the table does not hold.
    let ids: BTreeSet<&str> = TABLE.iter().map(|job| job.id).collect();
    let named: BTreeSet<&str> = timers.keys().map(String::as_str).collect();
    assert_eq!(
        named, ids,
        "the timers under deploy/systemd against the job table"
    );

    // The templates are the neutral example: the default rollover hour, at UTC.
    let rule = StudyDayRule::default();
    assert_eq!(
        rule.utc_offset(),
        UtcOffset::UTC,
        "the neutral example's offset"
    );
    for job in TABLE {
        let timer = &timers[job.id];
        assert_eq!(
            timer.get("OnCalendar"),
            Some(&vec![neutral_calendar(job.schedule, rule)]),
            "the {} job's timer against its slot in the table",
            job.id
        );
        // Only a catch-up job's timer replays a fire missed while the host was down (R4, ADR-027).
        let persistent = timer
            .get("Persistent")
            .is_some_and(|values| values == &["true"]);
        assert_eq!(
            persistent, job.catch_up,
            "the {} job's timer: Persistent= against the table's catch_up",
            job.id
        );
        // The timer starts the service of its own name, which is systemd's default (R10).
        assert!(
            !timer.contains_key("Unit"),
            "the {} job's timer names its unit",
            job.id
        );
    }
}
