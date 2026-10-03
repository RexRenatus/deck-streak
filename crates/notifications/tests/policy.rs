//! The typed policy (SPEC-041 A1, A3; R2, R10, R14): it reads every key of `notifications-policy.json`,
//! it carries `reading_ready` as a recorded deviation, and it refuses start naming the key for an
//! unknown key, a malformed value, an undeclared kind or budget, and a missing withhold reason.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

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

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Every `*.msg.json` under `dir`, as its path from the root and its JSON.
fn collect_goldens(root: &Path, dir: &Path, found: &mut Vec<(String, Value)>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("the directory reads")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !["target", "node_modules", ".git"].contains(&name) {
                collect_goldens(root, &path, found);
            }
        } else if name.ends_with(".msg.json") {
            let text = fs::read_to_string(&path).expect("the golden reads");
            let golden = serde_json::from_str(&text).expect("the golden is JSON");
            let shown = path.strip_prefix(root).unwrap_or(&path);
            found.push((shown.display().to_string(), golden));
        }
    }
}

/// What is wrong with the selection of command replies: a kind-less golden whose duty the policy
/// does not declare a reply, and a declared reply that is the duty of no kind-less golden.
fn selection_faults(replies: &[String], goldens: &[(String, Value)]) -> Vec<String> {
    let mut faults = Vec::new();
    for (path, golden) in goldens {
        let kindless = golden.get("kind").is_none();
        let duty = golden.get("duty").and_then(Value::as_str).unwrap_or("");
        if kindless && !replies.iter().any(|reply| reply == duty) {
            faults.push(format!("{path}: duty {duty} is not a declared reply"));
        }
    }
    for reply in replies {
        let carried = goldens.iter().any(|(_, golden)| {
            golden.get("kind").is_none()
                && golden.get("duty").and_then(Value::as_str) == Some(reply.as_str())
        });
        if !carried {
            faults.push(format!(
                "replies entry {reply} is the duty of no kind-less golden"
            ));
        }
    }
    faults
}

#[test]
fn every_golden_is_a_declared_notification_or_a_declared_reply() {
    let root = root();
    let mut found = Vec::new();
    collect_goldens(&root, &root, &mut found);
    let goldens = examined("golden message(s)", found);
    let replies: Vec<String> = file()["replies"]
        .as_array()
        .expect("the replies list")
        .iter()
        .map(|reply| reply.as_str().expect("a duty name").to_owned())
        .collect();

    assert_eq!(
        selection_faults(&replies, &goldens),
        Vec::<String>::new(),
        "every committed golden is a notification or a declared reply"
    );

    let mut planted = goldens.clone();
    planted.push((
        "planted/digest.msg.json".to_owned(),
        json!({"duty": "daily-digest", "schema": "phx.duty.message.v1"}),
    ));
    assert_eq!(
        selection_faults(&replies, &planted),
        ["planted/digest.msg.json: duty daily-digest is not a declared reply"],
        "a kind-less golden of an undeclared duty is refused by name"
    );
    let mut ghost = replies.clone();
    ghost.push("ghost-duty".to_owned());
    assert_eq!(
        selection_faults(&ghost, &goldens),
        ["replies entry ghost-duty is the duty of no kind-less golden"],
        "a declared reply no golden carries is refused by name"
    );
    let mut null_kind = goldens;
    null_kind.push((
        "planted/null.msg.json".to_owned(),
        json!({"duty": "bot-commands", "kind": null}),
    ));
    assert_eq!(
        selection_faults(&replies, &null_kind),
        Vec::<String>::new(),
        "a golden that carries a kind key, even null, is a notification for the probe, not a reply"
    );
}

#[test]
fn a_reply_that_names_a_declared_kind_is_refused_at_start() {
    let mut named = file();
    named["replies"] = json!(["bot-commands", "alert"]);
    let mut missing = file();
    missing
        .as_object_mut()
        .expect("an object")
        .remove("replies");
    let mut declared = file();
    declared["replies"] = json!(["bot-commands"]);

    assert!(
        matches!(
            refusal(&named),
            Some(PolicyError::Malformed { ref key, ref reason })
                if key == "replies" && reason.contains("alert")
        ),
        "a reply that names a declared kind is refused"
    );
    assert_eq!(
        refusal(&missing),
        Some(PolicyError::Missing { key: "replies" })
    );
    assert_eq!(
        refusal(&declared),
        None,
        "a duty that is no kind is admitted"
    );
}
