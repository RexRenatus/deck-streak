//! The study day equals the predecessor's for every golden case, across the rollover, and renders
//! as its ISO date and parses back (SPEC-020 A1, A2); an hour of the day stops at 23 (R2, #222).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_kernel::settings::{DIGEST_HOUR, ROLLOVER_HOUR};
use deck_streak_kernel::{
    Environment, Hour, KernelSettings, Setting, SettingsError, StudyDay, StudyDayRule, UtcMillis,
    UtcOffset,
};

/// The integer `key` of a golden case's input.
fn integer(case: &golden::Case, key: &str) -> i64 {
    case.input[key]
        .as_i64()
        .unwrap_or_else(|| panic!("{key} is an integer in {}", case.input))
}

/// The rule a study-day case was computed under.
fn rule_of(case: &golden::Case) -> StudyDayRule {
    let hour = u8::try_from(integer(case, "rollover_hour"))
        .ok()
        .and_then(Hour::new)
        .expect("the golden's rollover hour is 0 to 23");
    let offset = i16::try_from(integer(case, "utc_offset_minutes"))
        .ok()
        .and_then(UtcOffset::from_minutes)
        .expect("the golden's offset is within the setting's bounds");
    StudyDayRule::new(hour, offset)
}

/// The day `text` names, one day later, rendered.
fn the_day_after(text: &str) -> String {
    let day: StudyDay = text.parse().expect("an ISO date");
    StudyDay::from_epoch_day(day.epoch_day() + 1).to_string()
}

#[test]
fn the_study_day_matches_the_predecessors_golden() {
    let mut classes = BTreeSet::new();
    golden::each_case("study_day", |case| {
        let instant = UtcMillis::from_epoch_millis(integer(case, "instant_ms"));
        let day = rule_of(case).study_day(instant);
        assert_eq!(
            Some(day.epoch_day()),
            case.output.as_i64(),
            "the study day of {}",
            case.input
        );
        classes.extend(case.class.clone());
    });
    // The boundaries a port gets wrong were examined, not only ordinary instants.
    for class in ["rollover", "negative", "offset"] {
        assert!(classes.contains(class), "no {class} case was examined");
    }
}

#[test]
fn every_hour_past_23_is_refused_and_23_is_admitted() {
    // A day's hours run from 0 to 23 (SPEC-020 R2): 23, the last of them, is an hour, as itself.
    assert_eq!(Hour::new(23).map(Hour::get), Some(23));
    // 24 is no hour, nor is any later value a byte holds: each is refused as none at all.
    for hour in 24..=u8::MAX {
        assert_eq!(Hour::new(hour), None, "{hour} was admitted as an hour");
    }
    // So an hour setting of 24 refuses start as malformed, by its name and an hour's shape.
    for setting in [ROLLOVER_HOUR, DIGEST_HOUR] {
        assert_eq!(
            KernelSettings::from_env(&Environment::from_vars([(setting, "24")])),
            Err(SettingsError::Malformed {
                setting,
                expected: Hour::SHAPE,
            }),
            "{setting} of 24"
        );
    }
}

#[test]
fn a_study_day_renders_as_its_iso_date_and_parses_back() {
    // Epoch day zero is the Unix epoch's own day, by definition, and the day before it closed 1969.
    assert_eq!(StudyDay::from_epoch_day(0).to_string(), "1970-01-01");
    assert_eq!(StudyDay::from_epoch_day(-1).to_string(), "1969-12-31");
    // Every day the predecessor's golden produced renders as a date and parses back to itself.
    golden::each_case("study_day", |case| {
        let day = StudyDay::from_epoch_day(case.output.as_i64().expect("an epoch day"));
        let text = day.to_string();
        assert_eq!(text.len(), 10, "{text} is YYYY-MM-DD");
        assert_eq!(text.parse::<StudyDay>(), Ok(day), "{text}");
    });
    // The calendar's rules, read from consecutive days: February has a 29th every fourth year,
    // except in a century year, except in a year divisible by 400.
    assert_eq!(the_day_after("1996-02-28"), "1996-02-29");
    assert_eq!(the_day_after("1996-02-29"), "1996-03-01");
    assert_eq!(the_day_after("1999-02-28"), "1999-03-01");
    assert_eq!(the_day_after("1900-02-28"), "1900-03-01");
    assert_eq!(the_day_after("2000-02-28"), "2000-02-29");
    assert_eq!(the_day_after("1970-04-30"), "1970-05-01");
    assert_eq!(the_day_after("1999-12-31"), "2000-01-01");
    // Every day within a thousand of the epoch, and every 997th day of the eleven thousand years
    // either side of it, parses back from its date.
    for epoch_day in -4_000_000..4_000_000_i64 {
        if epoch_day % 997 == 0 || (-1_000..1_000).contains(&epoch_day) {
            let day = StudyDay::from_epoch_day(epoch_day);
            assert_eq!(day.to_string().parse::<StudyDay>(), Ok(day), "{epoch_day}");
        }
    }
    // A year outside 0000 to 9999 carries its sign, and still parses back.
    for epoch_day in [-800_000_i64, 3_000_000] {
        let day = StudyDay::from_epoch_day(epoch_day);
        let text = day.to_string();
        assert!(text.starts_with(['-', '+']), "{text} carries a sign");
        assert_eq!(text.parse::<StudyDay>(), Ok(day), "{text}");
    }
    // A date the calendar does not have, or text of another shape, is refused.
    for text in [
        "1900-02-29",
        "1999-02-29",
        "1970-13-01",
        "1970-00-10",
        "1970-04-31",
        "1970-01-00",
        "1970-1-01",
        "19700101",
        "70-01-01",
        "+1970-01-01",
        "1970-01-01T00",
        "",
    ] {
        assert!(
            text.parse::<StudyDay>().is_err(),
            "{text:?} was read as a date"
        );
    }
}
