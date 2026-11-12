//! The law streak (SPEC-076 A9, A11; R10, R11; ADR-076): the predecessor's bridged streak, and a
//! row that never spends or receives a freeze.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::law::{bridged_streak, law_state};
use deck_streak_streaks::streak::StreakState;
use serde_json::Value;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn days(value: &Value) -> BTreeSet<StudyDay> {
    value
        .as_array()
        .expect("days")
        .iter()
        .map(|n| day(n.as_i64().expect("an epoch day")))
        .collect()
}

#[test]
fn the_law_streak_matches_the_parity_golden() {
    let examined = golden::each_case("bridged_streak", |case| {
        let got = bridged_streak(
            &days(&case.input["days"]),
            day(case.input["today"].as_i64().expect("today")),
            &days(&case.input["skip_days"]),
        );
        assert_eq!(
            Some(i64::from(got)),
            case.output.as_i64(),
            "class {:?}, input {}",
            case.class,
            case.input
        );
    });
    println!("{examined}");
}

#[test]
fn the_law_row_never_spends_or_receives_a_freeze() {
    let start = StreakState {
        current: 6,
        longest: 6,
        freezes: 1,
        last_study_day: Some(day(19_999)),
        comeback_armed: false,
    };
    let run: BTreeSet<StudyDay> = (19_990..=19_999).map(day).collect();
    // A ten day run crosses the seven day multiple: the language track would earn a freeze.
    let held = law_state(&start, &run, day(19_999), &BTreeSet::new());
    assert_eq!(held.current, 10);
    assert_eq!(held.freezes, 1, "the law row keeps its start freezes");
    // A break neither spends the freeze nor arms a comeback.
    let broken = law_state(&held, &run, day(20_005), &BTreeSet::new());
    assert_eq!(broken.current, 0);
    assert_eq!(broken.freezes, 1);
    assert!(!broken.comeback_armed);
    assert_eq!(broken.longest, 10);
}
