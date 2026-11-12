//! Every scheduled flush step sits outside the quiet window it serves (SPEC-041 R7, amended for
//! #291): a flush that fires inside the window finds it closed and delivers nothing, so the job
//! table's calendar and the deploy template's calendar of each flush step are both checked against
//! the window the router reads from the policy. Every value is synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::fs;

use deck_streak_coordination::jobs::{Job, TABLE};
use deck_streak_kernel::Hour;
use deck_streak_notifications::quiet::{ClockTime, in_quiet_hours};

/// The prefix every flush step's job id carries.
const FLUSH_PREFIX: &str = "held_flush";

/// The configured quiet window, as local minutes of the day, read from the policy file the router
/// compiles in.
fn window() -> (i64, i64) {
    let text = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../notifications-policy.json"
    ))
    .expect("the policy file");
    let policy: serde_json::Value = serde_json::from_str(&text).expect("the policy parses");
    let bound = |name: &str| {
        let text = policy["quiet_hours"][name]
            .as_str()
            .expect("a window bound");
        ClockTime::parse(text).expect("a clock time").minutes()
    };
    (bound("start"), bound("end"))
}

/// The flush steps of the job table.
fn flush_jobs() -> Vec<Job> {
    TABLE
        .into_iter()
        .filter(|job| job.id.starts_with(FLUSH_PREFIX))
        .collect()
}

/// Every local minute of the day `job` fires at, the rollover hour being `rollover`.
fn minutes_of_day(job: &Job, rollover: Hour) -> Vec<i64> {
    match job.schedule.daily_slot(rollover) {
        Some((hour, minute)) => vec![i64::from(hour) * 60 + i64::from(minute)],
        None => (0..24)
            .map(|hour| hour * 60 + i64::from(job.schedule.minute()))
            .collect(),
    }
}

/// The minutes of the day a timer's `OnCalendar` fires at: `*-*-* HH:MM:SS UTC`, or `*-*-* *:MM:SS
/// UTC` for every hour.
fn timer_minutes(calendar: &str) -> Vec<i64> {
    let clock = calendar
        .strip_prefix("*-*-* ")
        .and_then(|rest| rest.strip_suffix(" UTC"))
        .expect("a daily calendar in UTC");
    let mut parts = clock.split(':');
    let (hour, minute) = (
        parts.next().expect("an hour field"),
        parts
            .next()
            .expect("a minute field")
            .parse::<i64>()
            .expect("a minute"),
    );
    if hour == "*" {
        (0..24).map(|hour| hour * 60 + minute).collect()
    } else {
        vec![hour.parse::<i64>().expect("an hour") * 60 + minute]
    }
}

#[test]
fn the_check_tells_a_slot_inside_the_window_from_one_outside_it() {
    let (start, end) = window();
    assert!(
        in_quiet_hours(start, start, end),
        "the window's start is inside it"
    );
    assert!(
        !in_quiet_hours(end, start, end),
        "the window's end is outside it"
    );
}

#[test]
fn every_flush_step_of_the_job_table_fires_outside_the_quiet_window() {
    let (start, end) = window();
    let jobs = flush_jobs();
    assert!(!jobs.is_empty(), "the job table has a scheduled flush step");
    let mut examined = 0_usize;
    for job in &jobs {
        for hour in 0..24 {
            let rollover = Hour::new(hour).expect("an hour");
            for minute in minutes_of_day(job, rollover) {
                assert!(
                    !in_quiet_hours(minute, start, end),
                    "{} fires at minute {minute} of the day, inside the window {start}..{end}, \
                     at rollover {hour}",
                    job.id
                );
                examined += 1;
            }
        }
    }
    println!("examined {examined} flush slot(s)");
    assert!(
        examined >= jobs.len() * 24,
        "every rollover hour of every flush step"
    );
}

#[test]
fn every_flush_step_of_the_deploy_templates_fires_outside_the_quiet_window() {
    let (start, end) = window();
    let jobs = flush_jobs();
    assert!(!jobs.is_empty(), "the job table has a scheduled flush step");
    let mut examined = 0_usize;
    for job in &jobs {
        let path = format!(
            "{}/../../deploy/systemd/deck-streak-job@{}.timer",
            env!("CARGO_MANIFEST_DIR"),
            job.id
        );
        let text = fs::read_to_string(&path).expect("the flush step's timer template");
        let calendars: Vec<&str> = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("OnCalendar="))
            .collect();
        assert!(!calendars.is_empty(), "{} has an OnCalendar line", job.id);
        for calendar in calendars {
            for minute in timer_minutes(calendar) {
                assert!(
                    !in_quiet_hours(minute, start, end),
                    "{} fires at minute {minute} of the day by its template, inside the window \
                     {start}..{end}",
                    job.id
                );
                examined += 1;
            }
        }
    }
    println!("examined {examined} flush template slot(s)");
    assert!(
        examined >= jobs.len(),
        "every flush step's template was read"
    );
}
