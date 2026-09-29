//! The ladder's values in the policy file (SPEC-084 A12, R7): the tier maps, the rare floor, the
//! budgets, the retry cap, the cooldown, the streak-break cap and the reveal's pause equal the
//! goldens, and the ladder reads the reaction's age and the near-miss bounds from the file, so the
//! goldens of the reaction's freshness and the near-miss gate hold through the compiled policy.
//! The policy also names every bot delivery call the transport port declares, the reveal among
//! them (ADR-084), so the one-router check polices each.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use deck_streak_kernel::UtcMillis;
use deck_streak_notifications::ladder::{self, REVEAL_PAUSE};
use deck_streak_notifications::{Policy, Tier};
use serde_json::{Value, json};
use support::ladder::tier;

/// The compiled policy, written back as the file's JSON.
fn policy_json(policy: &Policy) -> Value {
    serde_json::to_value(policy).expect("the policy writes back")
}

/// A golden tier map as the policy spells it: each name to its tier's text.
fn tier_map(map: &Value) -> Value {
    Value::Object(
        map.as_object()
            .expect("a tier map")
            .iter()
            .map(|(name, rank)| {
                (
                    name.clone(),
                    json!(tier(rank.as_u64().expect("a tier")).as_str()),
                )
            })
            .collect(),
    )
}

#[test]
fn the_policy_ladder_values_equal_the_parity_goldens() {
    let policy = Policy::compiled().expect("the compiled policy parses");
    let file = policy_json(&policy);
    let mut constants = BTreeMap::new();
    let examined = golden::each_case("ladder.constants", |case| {
        let name = case.input["name"].as_str().expect("a name").to_owned();
        constants.insert(name, case.output.clone());
    });
    assert!(examined.count > 0);
    let constant = |suffix: &str| {
        constants
            .iter()
            .find(|(name, _)| name.ends_with(suffix))
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| panic!("the golden holds {suffix}"))
    };

    assert_eq!(
        file["ladder"]["rarity"],
        tier_map(&constant("._RARITY_TIER"))
    );
    assert_eq!(
        file["ladder"]["events"],
        tier_map(&constant("._EVENT_TIER"))
    );
    let floor = json!(tier(constant("._RARE_FLOOR_TIER").as_u64().expect("a tier")).as_str());
    assert_eq!(
        file["ladder"]["rarity_floor"],
        json!({"epic": floor, "legendary": floor}),
        "an Epic and a Legendary keep the rare floor"
    );
    let budgets: BTreeMap<String, Value> = constant(".CELEBRATION_BUDGETS")
        .as_object()
        .expect("the budgets")
        .iter()
        .map(|(intensity, pair)| (intensity.clone(), json!({"T4": pair[0], "T5": pair[1]})))
        .collect();
    assert_eq!(
        file["celebration_budgets"]["intensities"],
        json!(budgets),
        "the weekly budgets by intensity"
    );
    assert_eq!(
        file["send_failure"]["retry_max"],
        constant(".CELEBRATION_SEND_RETRY_MAX")
    );
    assert_eq!(
        file["send_failure"]["outage_cooldown_ms"],
        constant(".CELEBRATION_OUTAGE_COOLDOWN_MS")
    );
    assert!(
        (REVEAL_PAUSE.as_secs_f64()
            - constant("._REVEAL_SUSPENSE_SECS")
                .as_f64()
                .expect("seconds"))
        .abs()
            < f64::EPSILON,
        "the reveal's pause"
    );
    assert_eq!(
        file["streak_break"]["cap"],
        json!(ladder::outcome_cap(&policy, true).as_str()),
        "the streak-break cap is the ladder's cap on a day the streak broke"
    );
    assert_eq!(ladder::outcome_cap(&policy, true), Tier::T1);
    assert_eq!(ladder::outcome_cap(&policy, false), Tier::T5);

    // The reaction's age and the near-miss bounds are read from the file: the goldens hold
    // through the compiled policy.
    let examined = golden::each_case("reaction_freshness", |case| {
        let (Some(age), false) = (
            case.input["owner"]["age_minutes"].as_i64(),
            case.input["reaction_open"].as_bool() == Some(true),
        ) else {
            return;
        };
        let now = UtcMillis::from_epoch_millis(1_728_000_000_000);
        let arrived = UtcMillis::from_epoch_millis(now.epoch_millis() - age * 60_000);
        assert_eq!(
            ladder::reaction_fresh(&policy, arrived, now),
            !case.output["calls"].as_array().expect("calls").is_empty(),
            "a message {age} minute(s) old"
        );
    });
    assert!(examined.count > 0);
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

/// The bot delivery calls the transport port declares, in its order: every `fn push_` of it.
fn port_calls() -> Vec<String> {
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/transport.rs"))
        .expect("the port's source");
    source
        .split("fn ")
        .skip(1)
        .filter_map(|rest| {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            name.starts_with("push_").then_some(name)
        })
        .collect()
}

#[test]
fn the_policy_polices_the_reveal_beside_the_other_delivery_calls() {
    let policy = Policy::compiled().expect("the compiled policy parses");
    let transport = &policy_json(&policy)["router"]["transport"];
    let bot: Vec<String> =
        serde_json::from_value(transport["bot"].clone()).expect("the bot's calls");
    assert_eq!(
        bot,
        port_calls(),
        "the policy names every bot delivery call the port declares, in its order"
    );
    assert_eq!(
        bot,
        [
            "push_message",
            "push_reveal",
            "push_dice",
            "push_reaction",
            "push_pin"
        ],
        "the reveal is its own delivery call, beside the other bot calls"
    );
    assert_eq!(transport["mini-app"], json!(["push_in_app"]));
}
