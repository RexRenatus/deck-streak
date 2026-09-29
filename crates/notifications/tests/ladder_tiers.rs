//! The ladder's arithmetic (SPEC-084 A1 to A4, A11): the requested tier, the weekly budget, the
//! step down over budget with its rare floor, the streak-break rule and the near-miss gate, each
//! equal to the golden of the predecessor's own function, read from the compiled policy.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use deck_streak_kernel::StudyDay;
use deck_streak_notifications::occasion::StreakFacts;
use deck_streak_notifications::{Policy, ladder};
use serde_json::Value;
use support::ladder::tier;

fn policy() -> Policy {
    Policy::compiled().expect("the compiled policy parses")
}

fn text<'a>(case: &'a golden::Case, name: &str) -> &'a str {
    case.input[name].as_str().expect("a text input")
}

fn number(value: &Value) -> u64 {
    value.as_u64().expect("a whole number")
}

fn count(case: &golden::Case, name: &str) -> u32 {
    u32::try_from(number(&case.input[name])).expect("a count")
}

#[test]
fn the_requested_tier_matches_the_parity_golden() {
    let policy = policy();
    let examined = golden::each_case("requested_tier", |case| {
        let (event, rarity) = (text(case, "event_type"), text(case, "rarity"));
        assert_eq!(
            ladder::requested_tier(&policy, event, Some(rarity)),
            tier(number(&case.output)),
            "the requested tier of {event:?} with rarity {rarity:?}"
        );
    });
    assert!(examined.count > 0);
}

#[test]
fn the_weekly_budget_matches_the_parity_golden() {
    let policy = policy();
    let examined = golden::each_case("weekly_budget", |case| {
        let intensity = text(case, "intensity");
        let pair = case.output.as_array().expect("a pair");
        let wanted = (
            u32::try_from(number(&pair[0])).expect("a count"),
            u32::try_from(number(&pair[1])).expect("a count"),
        );
        assert_eq!(
            ladder::weekly_budget(&policy, intensity),
            wanted,
            "the weekly budget of {intensity:?}"
        );
    });
    assert!(examined.count > 0);
}

#[test]
fn an_over_budget_tier_steps_down_as_the_parity_golden_says() {
    let policy = policy();
    let floor = ladder::rare_floor(&policy, Some("epic"));
    assert_eq!(
        floor,
        ladder::rare_floor(&policy, Some("legendary")),
        "an Epic and a Legendary share the rare floor"
    );
    assert_eq!(
        ladder::rare_floor(&policy, Some("rare")),
        None,
        "a Rare has no floor"
    );
    assert!(
        ladder::budget_exempt(&policy, "band_up"),
        "a band-up is exempt"
    );
    assert!(
        !ladder::budget_exempt(&policy, "badge"),
        "a badge is not exempt"
    );
    let examined = golden::each_case("apply_budget", |case| {
        let budget = case.input["budget"].as_array().expect("a budget pair");
        let budget = (
            u32::try_from(number(&budget[0])).expect("a count"),
            u32::try_from(number(&budget[1])).expect("a count"),
        );
        let rare = case.input["rare"].as_bool().expect("a flag");
        let exempt = case.input["exempt"].as_bool().expect("a flag");
        let requested = tier(number(&case.input["requested"]));
        let used = (count(case, "t4_used"), count(case, "t5_used"));
        assert_eq!(
            ladder::apply_budget(
                &policy,
                requested,
                used,
                budget,
                exempt,
                floor.filter(|_| rare)
            ),
            tier(number(&case.output)),
            "{requested:?} with {used:?} used of {budget:?}, exempt {exempt}, rare {rare}"
        );
    });
    assert!(examined.count > 0);
}

#[test]
fn the_streak_broke_on_a_day_as_the_parity_golden_says() {
    let examined = golden::each_case("streak_broke_today", |case| {
        let today = case.input["today"].as_i64().expect("a day");
        let facts = StreakFacts {
            last_study_day: case.input["last_study_day"]
                .as_i64()
                .map(StudyDay::from_epoch_day),
            current: count(case, "current"),
            longest: count(case, "longest"),
        };
        assert_eq!(
            ladder::streak_broke_on(Some(&facts), StudyDay::from_epoch_day(today)),
            case.output.as_bool().expect("a flag"),
            "{facts:?} on {today}"
        );
    });
    assert!(examined.count > 0);
    assert!(
        !ladder::streak_broke_on(None, StudyDay::from_epoch_day(20_000)),
        "an occasion without the streak's facts reads no break"
    );
}

#[test]
fn the_near_miss_gate_matches_the_parity_golden() {
    let policy = policy();
    let examined = golden::each_case("near_miss_ok", |case| {
        let remaining = case.input["remaining"].as_f64().expect("a gap");
        let target = case.input["target"].as_f64().expect("a target");
        assert_eq!(
            ladder::near_miss_ok(&policy, remaining, target),
            case.output.as_bool().expect("a flag"),
            "a gap of {remaining} toward {target}"
        );
    });
    assert!(examined.count > 0);
}
