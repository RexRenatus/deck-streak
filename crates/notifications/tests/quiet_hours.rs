//! The quiet window (SPEC-041 A8; R9): the port of the predecessor's `quiet_hours.py:in_quiet_hours`
//! equals its golden, wrapping, same-day and disabled windows included, with minutes outside the
//! day brought back into it; the local minute is read at the configured offset; and the policy's
//! clock times are `HH:MM` on a 24-hour clock.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_kernel::{UtcMillis, UtcOffset};
use deck_streak_notifications::quiet::{ClockTime, in_quiet_hours, local_minute};

/// The integer `name` of a case's input.
fn input(case: &golden::Case, name: &str) -> i64 {
    case.input[name]
        .as_i64()
        .unwrap_or_else(|| panic!("the input {name} is an integer: {}", case.input))
}

#[test]
fn the_quiet_window_matches_the_parity_golden() {
    let mut classes = std::collections::BTreeSet::new();
    let examined = golden::each_case("in_quiet_hours", |case| {
        let port = in_quiet_hours(
            input(case, "local_minutes"),
            input(case, "start_min"),
            input(case, "end_min"),
        );
        assert_eq!(
            serde_json::Value::Bool(port),
            case.output,
            "in_quiet_hours({})",
            case.input
        );
        if let Some(class) = &case.class {
            classes.insert(class.clone());
        }
    });
    assert_eq!(examined.function, "quiet_hours.in_quiet_hours");
    assert_eq!(
        classes.into_iter().collect::<Vec<_>>(),
        ["disabled", "normalized", "same-day", "wrap"],
        "every class of boundary was examined"
    );
}

#[test]
fn the_local_minute_is_read_at_the_configured_offset() {
    let offset = |minutes: i16| UtcOffset::from_minutes(minutes).expect("an offset in bounds");
    // 23:30 UTC on the epoch's second day, and a minute before the epoch.
    let late = UtcMillis::from_epoch_millis(86_400_000 + (23 * 60 + 30) * 60_000);
    let before_epoch = UtcMillis::from_epoch_millis(-60_000);

    assert_eq!(local_minute(late, UtcOffset::UTC), 23 * 60 + 30);
    assert_eq!(
        local_minute(late, offset(60)),
        30,
        "past midnight an hour east"
    );
    assert_eq!(
        local_minute(late, offset(-120)),
        21 * 60 + 30,
        "two hours west"
    );
    assert_eq!(local_minute(before_epoch, UtcOffset::UTC), 23 * 60 + 59);
    assert_eq!(
        local_minute(UtcMillis::from_epoch_millis(59_999), UtcOffset::UTC),
        0,
        "a minute is whole"
    );
}

#[test]
fn a_clock_time_is_hh_mm_on_a_24_hour_clock() {
    let accepted: Vec<(&str, i64)> = ["00:00", "07:30", "23:00", "23:59", "09:05"]
        .into_iter()
        .filter_map(|text| ClockTime::parse(text).map(|time| (text, time.minutes())))
        .collect();
    let refused: Vec<&str> = [
        "24:00", "23:60", "7:30", "07:3", "0730", "07-30", "", ":", "07:30:00", "ab:cd", "+7:30",
        "07:+3",
    ]
    .into_iter()
    .filter(|text| ClockTime::parse(text).is_some())
    .collect();

    assert_eq!(
        accepted,
        [
            ("00:00", 0),
            ("07:30", 450),
            ("23:00", 1380),
            ("23:59", 1439),
            ("09:05", 545)
        ]
    );
    assert!(refused.is_empty(), "accepted: {refused:?}");
}
