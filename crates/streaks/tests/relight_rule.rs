//! The relight rule (SPEC-076 A16; R16): a return day with at least three reviews earns the XP
//! once, and its celebration carries the predecessor's event name.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::relight::relight;

#[test]
fn the_relight_rule_matches_the_parity_golden() {
    let examined = golden::each_case("relight", |case| {
        let today = StudyDay::from_epoch_day(case.input["today"].as_i64().expect("today"));
        let reviews = case.input["reviews"]
            .as_i64()
            .map(|n| u32::try_from(n).expect("reviews"));
        let granted =
            u32::try_from(case.input["granted"].as_i64().expect("granted")).expect("granted");
        let got = relight(today, reviews, granted);
        assert_eq!(
            got.as_ref().map(|r| i64::from(r.amount)),
            case.output["amount"].as_i64(),
            "class {:?}, input {}",
            case.class,
            case.input
        );
        assert_eq!(
            got.as_ref().map(|r| r.event_type),
            case.output["event_type"].as_str(),
            "class {:?}",
            case.class
        );
        assert_eq!(
            got.as_ref().map(|r| r.event_key.as_str()),
            case.output["event_key"].as_str(),
            "class {:?}",
            case.class
        );
        assert_eq!(
            got.as_ref().map(|r| r.source.as_str()),
            case.output["source"].as_str(),
            "class {:?}",
            case.class
        );
    });
    println!("{examined}");
}
