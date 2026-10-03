//! The typed policy (SPEC-041 A1, A3; R2, R10, R14): it reads every key of `notifications-policy.json`,
//! it carries `reading_ready` as a recorded deviation, and it refuses start naming the key for an
//! unknown key, a malformed value, an undeclared kind or budget, and a missing withhold reason.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use deck_streak_notifications::occasion::{Class, DedupeScope, Tier};
use deck_streak_notifications::policy::{Deviation, POLICY_SCHEMA};
use deck_streak_notifications::{Policy, PolicyError};
use serde_json::{Value, json};

/// The repository's root.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The policy file, as JSON.
fn file() -> Value {
    let text = fs::read_to_string(root().join("notifications-policy.json")).expect("the file");
    serde_json::from_str(&text).expect("the file is JSON")
}

/// What parsing `value` refuses, or `None` when it parses.
fn refusal(value: &Value) -> Option<PolicyError> {
    Policy::parse(&value.to_string()).err()
}

#[test]
fn the_typed_policy_reads_every_key_of_the_file() {
    let policy = Policy::compiled().expect("the compiled policy parses");

    let written_back = serde_json::to_value(&policy).expect("the policy is written back");

    assert_eq!(
        written_back,
        file(),
        "every key of the file, and nothing else"
    );
}

#[test]
fn the_reading_ready_kind_is_a_recorded_deviation() {
    let policy = Policy::compiled().expect("the compiled policy parses");

    let kind = policy.kind("reading_ready");

    assert_eq!(
        kind.as_ref().map(|kind| (
            kind.class(),
            kind.tiers().to_vec(),
            kind.budget(),
            kind.dedupe(),
            kind.setting()
        )),
        Some((
            Class::Nudge,
            vec![Tier::T2],
            None,
            DedupeScope::PerStudyDay,
            Some("reading_ready_enabled")
        )),
        "reading_ready is a nudge of tier T2, per study day, behind its own switch"
    );
    let adr = "docs/decisions/ADR-041-notification-router-core.md";
    let photo_adr = "docs/decisions/ADR-135-images-are-drawn-through-a-port-with-no-provider-wired-by-a-sending-job-capped-cached-and-gated.md";
    assert_eq!(
        policy.deviations(),
        [
            Deviation {
                key: "kinds.reading_ready".to_owned(),
                adr: adr.to_owned()
            },
            Deviation {
                key: "withhold.reasons".to_owned(),
                adr: photo_adr.to_owned()
            }
        ]
    );
    let photo_record =
        fs::read_to_string(root().join(photo_adr)).expect("the photo deviation's ADR exists");
    assert!(
        photo_record.contains("withhold.reasons"),
        "the ADR names the key it decided"
    );
    let record = fs::read_to_string(root().join(adr)).expect("the deviation's ADR exists");
    assert!(
        record.contains("kinds.reading_ready"),
        "the ADR names the key it decided"
    );
}

#[test]
fn an_undeclared_kind_is_no_kind() {
    let policy = Policy::compiled().expect("the compiled policy parses");

    assert_eq!(policy.kind("unknown_blast"), None);
    assert_eq!(
        policy
            .kind("alert")
            .map(|kind| kind.setting().map(str::to_owned)),
        Some(None),
        "an alert has no switch"
    );
}

#[test]
fn the_policy_refuses_an_unknown_or_missing_key_by_name() {
    let mut unknown = file();
    unknown["quiet_windows"] = json!({});
    let mut missing = file();
    missing
        .as_object_mut()
        .expect("an object")
        .remove("deferral");

    assert_eq!(
        refusal(&unknown),
        Some(PolicyError::UnknownKey {
            key: "quiet_windows".to_owned()
        })
    );
    assert_eq!(
        refusal(&missing),
        Some(PolicyError::Missing { key: "deferral" })
    );
}

#[test]
fn the_policy_refuses_a_malformed_value_naming_its_key() {
    let mut section = file();
    section["deferral"]["flush_max"] = json!("two");
    let mut kind = file();
    kind["kinds"]["habit"]["cadence"] = json!("daily");
    let mut clock = file();
    clock["quiet_hours"]["start"] = json!("25:00");
    let mut schema = file();
    schema["schema"] = json!("phx.notifications.policy.v2");

    let keys: Vec<Option<String>> = [section, kind, clock, schema]
        .iter()
        .map(|value| match refusal(value) {
            Some(PolicyError::Malformed { key, .. }) => Some(key),
            _ => None,
        })
        .collect();

    assert_eq!(
        keys,
        [
            Some("deferral".to_owned()),
            Some("kinds.habit".to_owned()),
            Some("quiet_hours".to_owned()),
            Some("schema".to_owned())
        ]
    );
    assert_eq!(POLICY_SCHEMA, "phx.notifications.policy.v1");
}

#[test]
fn the_policy_refuses_an_undeclared_kind_or_budget() {
    let mut holdout = file();
    holdout["holdout"]["kinds"] = json!(["morning", "evening_blast"]);
    let mut budget = file();
    budget["kinds"]["habit"]["budget"] = json!("weekend");
    let mut declared = file();
    declared["kinds"]["habit"]["budget"] = json!("evening");

    assert_eq!(
        refusal(&holdout),
        Some(PolicyError::UndeclaredKind {
            key: "holdout.kinds",
            kind: "evening_blast".to_owned()
        })
    );
    assert_eq!(
        refusal(&budget),
        Some(PolicyError::UndeclaredBudget {
            key: "kinds.habit.budget".to_owned(),
            budget: "weekend".to_owned()
        })
    );
    assert_eq!(
        refusal(&declared),
        None,
        "a declared nudge budget is admitted"
    );
}

#[test]
fn the_policy_refuses_withhold_reasons_that_lack_one_the_router_records() {
    for reason in [
        "nudges_disabled",
        "already_recorded",
        "lapse",
        "quiet_hours",
        "budget_spent",
        "no_notifier",
        "photo_unsupported",
    ] {
        let mut lacking = file();
        let reasons = lacking["withhold"]["reasons"]
            .as_array_mut()
            .expect("the reasons");
        reasons.retain(|listed| listed != reason);

        assert!(
            matches!(
                refusal(&lacking),
                Some(PolicyError::Malformed { ref key, ref reason })
                    if key == "withhold.reasons" && reason.contains("lacks")
            ),
            "the policy without {reason} is refused"
        );
    }
}

#[test]
fn the_policy_refuses_text_that_is_not_an_object() {
    for text in ["[]", "{", "\"policy\""] {
        assert!(
            matches!(Policy::parse(text), Err(PolicyError::NotAnObject { .. })),
            "{text} is refused"
        );
    }
}
