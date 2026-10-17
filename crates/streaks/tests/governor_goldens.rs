//! Strength and the governor (SPEC-076 A12, A13, A15, A23; R12 to R15, R25): every case of the
//! goldens is the predecessor's own answer, and the floats are compared bit for bit.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::governor::{NoticeInput, assess, silence_walk, standby_notice};
use deck_streak_streaks::lapse::anchor_beyond_the_walk;
use deck_streak_streaks::strength::{advance, fold};
use serde_json::Value;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

fn days(value: &Value) -> BTreeSet<StudyDay> {
    value
        .as_array()
        .expect("days")
        .iter()
        .map(|n| day(int(n)))
        .collect()
}

fn bits(value: &Value) -> u64 {
    value.as_f64().expect("a float").to_bits()
}

#[test]
fn strength_and_the_verdict_match_the_parity_goldens() {
    let advanced = golden::each_case("strength_advance", |case| {
        let got = advance(
            case.input["prev"].as_f64().expect("prev"),
            case.input["studied"].as_bool().expect("studied"),
        );
        assert_eq!(got.to_bits(), bits(&case.output), "input {}", case.input);
    });
    let verdicts = golden::each_case("governor_assess", |case| {
        let got = assess(
            case.input["strength"].as_f64().expect("strength"),
            u32::try_from(int(&case.input["silent_days"])).expect("silent days"),
        );
        assert_eq!(got.strength.to_bits(), bits(&case.output["strength"]));
        assert_eq!(
            Some(got.standby),
            case.output["standby"].as_bool(),
            "{}",
            case.input
        );
        assert_eq!(
            Some(got.lapse),
            case.output["lapse"].as_bool(),
            "{}",
            case.input
        );
        assert_eq!(
            Some(got.armed()),
            case.output["armed"].as_bool(),
            "{}",
            case.input
        );
    });
    // The fold from the first study day is the same recurrence, oldest first.
    let studied: BTreeSet<StudyDay> = [day(100), day(101), day(103)].into_iter().collect();
    let series = fold(&studied, day(105));
    assert_eq!(
        series.len(),
        6,
        "one value a day from the first study day to today"
    );
    let mut want = 0.0_f64;
    for (index, (when, got)) in series.iter().enumerate() {
        want = advance(want, studied.contains(when));
        assert_eq!(got.to_bits(), want.to_bits(), "day index {index}");
    }
    println!("{advanced} {verdicts}");
}

fn anchor_case(case: &golden::Case) -> Option<i64> {
    anchor_beyond_the_walk(
        day(int(&case.input["today"])),
        &days(&case.input["study_days"]),
        &days(&case.input["skip_days"]),
        case.input["stored_anchor"].as_i64().map(day),
    )
    .map(StudyDay::epoch_day)
}

#[test]
fn the_anchor_beyond_the_walk_matches_the_parity_golden() {
    let examined = golden::each_case("lapse_anchor_beyond_the_walk", |case| {
        if case
            .class
            .as_deref()
            .is_some_and(|c| c.starts_with("empty-history-"))
        {
            return;
        }
        assert_eq!(
            anchor_case(case),
            case.output.as_i64(),
            "class {:?}, input {}",
            case.class,
            case.input
        );
    });
    println!("{examined}");
}

#[test]
fn an_empty_history_is_anchored_as_the_predecessor_anchors_it() {
    let mut seen = 0_usize;
    let examined = golden::each_case("lapse_anchor_beyond_the_walk", |case| {
        if !case
            .class
            .as_deref()
            .is_some_and(|c| c.starts_with("empty-history-"))
        {
            return;
        }
        seen += 1;
        assert_eq!(
            anchor_case(case),
            case.output.as_i64(),
            "class {:?}, input {}",
            case.class,
            case.input
        );
    });
    assert!(
        seen >= 6,
        "the golden holds its empty-history cases: {seen}"
    );
    println!("{examined}");
}

#[test]
fn the_standby_notice_rule_matches_the_parity_golden() {
    let examined = golden::each_case("standby_notice", |case| {
        let today = day(int(&case.input["today"]));
        let studied = days(&case.input["study_days"]);
        let skipped = days(&case.input["skip_days"]);
        let series = fold(&studied, today);
        let strength = series.last().map_or(0.0, |(_, value)| *value);
        let silence = silence_walk(today, &studied, &skipped);
        let verdict = assess(strength, silence.silent_days);
        let decision = standby_notice(&NoticeInput {
            verdict,
            was_standby: case.input["standby"].as_bool().expect("standby"),
            notified_day: case.input["notified_day"].as_i64().map(day),
            quiet_hours: case.input["quiet"].as_bool().expect("quiet"),
            notifier: case.input["notifier"].as_bool().expect("notifier"),
            today,
        });
        assert_eq!(
            Some(decision.sent),
            case.output["sent"].as_bool(),
            "class {:?}, input {}",
            case.class,
            case.input
        );
        assert_eq!(
            decision.notified_day.map(StudyDay::epoch_day),
            case.output["notified_day"].as_i64(),
            "class {:?}, input {}",
            case.class,
            case.input
        );
        assert_eq!(Some(verdict.standby), case.output["standby"].as_bool());
        assert_eq!(Some(verdict.lapse), case.output["lapse"].as_bool());
    });
    println!("{examined}");
}
