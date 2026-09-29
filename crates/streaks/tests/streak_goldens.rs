//! The language streak's machine equals the predecessor's (SPEC-076 A1 to A4; R1 to R6): every case
//! of the goldens is the predecessor's own answer over synthetic epoch days.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::constants::{
    FREEZE_DROP_MONTHLY_CAP, RELIGHT_CARDS, RELIGHT_XP, STREAK_COMEBACK_MIN,
    STREAK_DAYS_PER_FREEZE, STREAK_FREEZE_CAP, STREAK_HEAT, STREAK_START_FREEZES,
    STRENGTH_ARM_THRESHOLD, STRENGTH_HALF_LIFE_DAYS,
};
use deck_streak_streaks::lapse::LAPSE_AFTER_SILENT_DAYS;
use deck_streak_streaks::streak::{
    StreakState, Transition, classify_gap, comeback_view, decay_on_lapse, heat_for, real_misses,
    update_on_study,
};
use serde_json::Value;

const ECONOMY: &str = include_str!("../../../economy.json");

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

fn count(value: &Value) -> u32 {
    u32::try_from(int(value)).expect("a count")
}

fn skips(value: &Value) -> BTreeSet<StudyDay> {
    value
        .as_array()
        .expect("the skip days")
        .iter()
        .map(|number| day(int(number)))
        .collect()
}

fn state(value: &Value) -> StreakState {
    StreakState {
        current: count(&value["current"]),
        longest: count(&value["longest"]),
        freezes: count(&value["freezes"]),
        last_study_day: value["last_study_day"].as_i64().map(day),
        comeback_armed: value["comeback_armed"].as_bool().expect("a flag"),
    }
}

fn transition_json(out: &Transition) -> Value {
    serde_json::json!({
        "current": out.state.current,
        "longest": out.state.longest,
        "freezes": out.state.freezes,
        "last_study_day": out.state.last_study_day.map(StudyDay::epoch_day),
        "comeback_armed": out.state.comeback_armed,
        "heat": heat_for(out.state.current),
        "froze_today": out.froze_today,
        "broke_today": out.broke_today,
    })
}

#[test]
fn the_language_streak_transitions_match_the_parity_goldens() {
    let gaps = golden::each_case("classify_gap", |case| {
        let got = classify_gap(
            &state(&case.input["state"]),
            day(int(&case.input["today"])),
            &skips(&case.input["skip_days"]),
        );
        assert_eq!(
            Some(got.as_str()),
            case.output.as_str(),
            "class {:?}, input {}",
            case.class,
            case.input
        );
    });
    let studies = golden::each_case("update_on_study", |case| {
        let got = update_on_study(
            &state(&case.input["state"]),
            day(int(&case.input["today"])),
            count(&case.input["observed_streak"]),
            &skips(&case.input["skip_days"]),
        );
        assert_eq!(
            transition_json(&got),
            case.output,
            "class {:?}, input {}",
            case.class,
            case.input
        );
    });
    println!("{gaps} {studies}");
}

#[test]
fn a_day_without_study_breaks_only_an_unsavable_streak() {
    let examined = golden::each_case("decay_on_lapse", |case| {
        let got = decay_on_lapse(
            &state(&case.input["state"]),
            day(int(&case.input["today"])),
            &skips(&case.input["skip_days"]),
        );
        assert_eq!(
            transition_json(&got),
            case.output,
            "class {:?}, input {}",
            case.class,
            case.input
        );
    });
    println!("{examined}");
    // One real miss is still rescuable by a freeze bought today: held, never broken.
    let live = StreakState {
        current: 9,
        longest: 9,
        freezes: 0,
        last_study_day: Some(day(19_998)),
        comeback_armed: false,
    };
    let one_miss = decay_on_lapse(&live, day(20_000), &BTreeSet::new());
    assert!(!one_miss.broke_today, "one real miss holds the streak");
    assert_eq!(one_miss.state.current, 9);
    // Two real misses break it, arm the comeback, and leave the day and the freezes alone.
    let two_misses = decay_on_lapse(&live, day(20_001), &BTreeSet::new());
    assert!(two_misses.broke_today, "two real misses break the streak");
    assert_eq!(two_misses.state.current, 0);
    assert!(two_misses.state.comeback_armed);
    assert_eq!(two_misses.state.last_study_day, live.last_study_day);
    assert_eq!(two_misses.state.freezes, live.freezes);
    assert_eq!(two_misses.state.longest, 9);
}

#[test]
fn the_heat_and_the_comeback_view_match_the_parity_goldens() {
    let heat = golden::each_case("heat_for", |case| {
        assert_eq!(
            Some(heat_for(count(&case.input["days"]))),
            case.output.as_str(),
            "input {}",
            case.input
        );
    });
    let badge = golden::each_case("badge_view", |case| {
        assert_eq!(
            Some(comeback_view(&state(&case.input["state"]))),
            case.output.as_bool(),
            "input {}",
            case.input
        );
    });
    let misses = golden::each_case("real_misses", |case| {
        let got = real_misses(
            day(int(&case.input["last_study"])),
            day(int(&case.input["today"])),
            &skips(&case.input["skip_days"]),
        );
        assert_eq!(
            Some(i64::from(got)),
            case.output.as_i64(),
            "class {:?}, input {}",
            case.class,
            case.input
        );
    });
    println!("{heat} {badge} {misses}");
}

#[test]
fn the_streak_constants_and_economy_json_match_the_predecessors() {
    let economy: Value = serde_json::from_str(ECONOMY).expect("economy.json");
    let streak = &economy["streak"];
    let governor = &economy["governor"];
    let relight = &economy["xp"]["bonuses"]["relight"];
    assert_eq!(
        int(&streak["start_freezes"]),
        i64::from(STREAK_START_FREEZES)
    );
    assert_eq!(int(&streak["freeze_cap"]), i64::from(STREAK_FREEZE_CAP));
    assert_eq!(
        int(&streak["days_per_freeze"]),
        i64::from(STREAK_DAYS_PER_FREEZE)
    );
    assert_eq!(
        int(&streak["comeback_min_days"]),
        i64::from(STREAK_COMEBACK_MIN)
    );
    assert_eq!(
        int(&streak["freeze_drops_per_month"]),
        i64::from(FREEZE_DROP_MONTHLY_CAP)
    );
    assert_eq!(
        int(&governor["strength_half_life_days"]),
        i64::from(STRENGTH_HALF_LIFE_DAYS)
    );
    assert_eq!(
        int(&governor["lapse_after_silent_days"]),
        i64::from(LAPSE_AFTER_SILENT_DAYS)
    );
    assert_eq!(
        governor["arm_threshold"].as_f64().map(f64::to_bits),
        Some(STRENGTH_ARM_THRESHOLD.to_bits())
    );
    assert_eq!(int(&relight["xp"]), i64::from(RELIGHT_XP));
    assert_eq!(int(&relight["min_reviews"]), i64::from(RELIGHT_CARDS));
    let examined = golden::each_case("streaks.constants", |case| {
        let name = case.input["name"].as_str().expect("a name");
        match name {
            "constants.STREAK_START_FREEZES" => {
                assert_eq!(case.output.as_i64(), Some(i64::from(STREAK_START_FREEZES)));
            }
            "constants.STREAK_FREEZE_CAP" => {
                assert_eq!(case.output.as_i64(), Some(i64::from(STREAK_FREEZE_CAP)));
            }
            "constants.STREAK_DAYS_PER_FREEZE" => {
                assert_eq!(
                    case.output.as_i64(),
                    Some(i64::from(STREAK_DAYS_PER_FREEZE))
                );
            }
            "constants.STREAK_COMEBACK_MIN" => {
                assert_eq!(case.output.as_i64(), Some(i64::from(STREAK_COMEBACK_MIN)));
            }
            "constants.STREAK_HEAT" => {
                let tiers: Vec<Value> = STREAK_HEAT
                    .iter()
                    .map(|(days, emoji)| serde_json::json!([days, emoji]))
                    .collect();
                assert_eq!(case.output, Value::Array(tiers));
            }
            "constants.FREEZE_DROP_MONTHLY_CAP" => {
                assert_eq!(
                    case.output.as_i64(),
                    Some(i64::from(FREEZE_DROP_MONTHLY_CAP))
                );
            }
            "constants.STRENGTH_HALF_LIFE_DAYS" => {
                assert_eq!(
                    case.output.as_i64(),
                    Some(i64::from(STRENGTH_HALF_LIFE_DAYS))
                );
            }
            "constants.STRENGTH_ARM_THRESHOLD" => {
                assert_eq!(
                    case.output.as_f64().map(f64::to_bits),
                    Some(STRENGTH_ARM_THRESHOLD.to_bits())
                );
            }
            "constants.LAPSE_AFTER_SILENT_DAYS" => {
                assert_eq!(
                    case.output.as_i64(),
                    Some(i64::from(LAPSE_AFTER_SILENT_DAYS))
                );
            }
            "constants.RELIGHT_CARDS" => {
                assert_eq!(case.output.as_i64(), Some(i64::from(RELIGHT_CARDS)));
            }
            "constants.RELIGHT_XP" => {
                assert_eq!(case.output.as_i64(), Some(i64::from(RELIGHT_XP)));
            }
            "gamification.strength.STRENGTH_DECAY" => {
                assert_eq!(
                    case.output.as_f64().map(f64::to_bits),
                    Some(deck_streak_streaks::strength::decay().to_bits())
                );
            }
            "pipeline_layers.governor._SILENCE_WALK_CAP_DAYS" => {
                assert_eq!(
                    case.output.as_i64(),
                    Some(deck_streak_streaks::constants::SILENCE_WALK_CAP_DAYS)
                );
            }
            "pipeline_layers.governor._STRENGTH_PERSIST_DAYS" => {
                assert_eq!(
                    case.output.as_i64(),
                    Some(deck_streak_streaks::constants::STRENGTH_PERSIST_DAYS)
                );
            }
            other => panic!("a constant the test does not name: {other}"),
        }
    });
    assert_eq!(examined.count, 14, "every registered constant is compared");
    println!("{examined}");
}
