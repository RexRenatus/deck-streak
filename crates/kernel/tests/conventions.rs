//! The owner's note conventions are private configuration read once at start (SPEC-094 A1 to A3;
//! R1, R2; ADR-096): the neutral example loads into one value, every malformed file refuses start
//! naming the setting and never a value, and a forbidden direction label refuses start.
//!
//! Every name here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::path::{Path, PathBuf};

use deck_streak_kernel::conventions::{CONVENTIONS_FILE, CONVENTIONS_SCHEMA, Direction};
use deck_streak_kernel::{Conventions, ConventionsError, Environment};
use serde_json::{Value, json};
use tempfile::TempDir;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn example() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/config/conventions.example.json");
    let text = std::fs::read_to_string(path).expect("the example is readable");
    serde_json::from_str(&text).expect("the example is JSON")
}

/// The example with `edit` applied to its JSON.
fn edited(edit: impl FnOnce(&mut Value)) -> String {
    let mut value = example();
    edit(&mut value);
    value.to_string()
}

fn load_text(text: &str) -> Result<Conventions, ConventionsError> {
    let dir = TempDir::new().expect("a scratch directory");
    let path: PathBuf = dir.path().join("conventions.json");
    std::fs::write(&path, text).expect("the file is written");
    Conventions::load(&Environment::from_vars([(
        CONVENTIONS_FILE,
        path.as_os_str(),
    )]))
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// Every string value of the file: a refusal may quote none of them.
fn values(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::String(text) => into.push(text.clone()),
        Value::Array(items) => items.iter().for_each(|item| values(item, into)),
        Value::Object(map) => map.values().for_each(|item| values(item, into)),
        _ => {}
    }
}

#[test]
fn the_conventions_file_loads_into_one_value() {
    let loaded = load_text(&example().to_string()).expect("the neutral example loads");
    let direction = &loaded.direction;
    let pairs = examined("pair rules", direction.pair_rules.clone());
    assert_eq!(pairs[0].front, "Example front");
    assert_eq!(pairs[0].back, "Example back");
    assert_eq!(pairs[0].direction, Direction::CuedRecall);
    let templates = examined("template rules", direction.template_rules.clone());
    assert_eq!(templates.len(), 2);
    assert_eq!(templates[0].template, "Example forward");
    assert_eq!(templates[0].direction, Direction::Production);
    assert_eq!(templates[1].template, "Example reverse");
    assert_eq!(templates[1].direction, Direction::Recognition);
    let types = examined("note type rules", direction.note_type_rules.clone());
    assert_eq!(types[0].contains, "Example type");
    assert_eq!(direction.exempt_tokens, strings(&["Example exempt"]));
    assert_eq!(
        loaded.transfer.excluded_fields,
        strings(&["Example Sort", "Example Audio"])
    );
    assert_eq!(loaded.transfer.choice_prefix, "Pick_");
    assert_eq!(loaded.transfer.rank_field, "Example Rank");
    assert_eq!(
        loaded.transfer.meaning_fields,
        strings(&["Example Gloss", "Example Sense"])
    );
    assert_eq!(loaded.can_do_field, "Example Statement");
    // The setting unset is the empty value, not a refusal.
    assert_eq!(
        Conventions::load(&Environment::from_vars::<[(&str, &str); 0], _, _>([]))
            .expect("unset loads"),
        Conventions::default()
    );
}

#[test]
fn a_malformed_conventions_file_refuses_start() {
    let malformed = |expected: &'static str| ConventionsError::Malformed {
        setting: CONVENTIONS_FILE,
        expected,
    };
    let planted: Vec<(&str, String)> = vec![
        ("not JSON", "not json at all".to_owned()),
        ("not an object", "[1]".to_owned()),
        (
            "a wrong schema",
            edited(|v| v["schema"] = json!("deckstreak.conventions.v0")),
        ),
        (
            "no direction",
            edited(|v| {
                v.as_object_mut().expect("object").remove("direction");
            }),
        ),
        (
            "a template rule with no direction",
            edited(|v| {
                v["direction"]["template_rules"][0]
                    .as_object_mut()
                    .expect("object")
                    .remove("direction");
            }),
        ),
        (
            "an unknown direction bucket",
            edited(|v| v["direction"]["template_rules"][0]["direction"] = json!("sideways")),
        ),
        (
            "a blank rank field",
            edited(|v| v["transfer"]["rank_field"] = json!("  ")),
        ),
        (
            "a meaning field that is no text",
            edited(|v| v["transfer"]["meaning_fields"] = json!([1])),
        ),
        (
            "no can-do field",
            edited(|v| {
                v.as_object_mut().expect("object").remove("can_do_field");
            }),
        ),
    ];
    let planted = examined("malformed conventions files", planted);
    let mut quoted = Vec::new();
    values(&example(), &mut quoted);
    quoted.retain(|value| value.len() > 1 && value != CONVENTIONS_SCHEMA);
    for (what, text) in planted {
        let refusal = load_text(&text).expect_err(what);
        assert!(
            matches!(refusal, ConventionsError::Malformed { setting, .. } if setting == CONVENTIONS_FILE),
            "{what}: {refusal:?}"
        );
        let message = refusal.to_string();
        assert!(message.contains(CONVENTIONS_FILE), "{what}: {message}");
        for value in &quoted {
            assert!(
                !message.contains(value.as_str()),
                "{what}: the refusal quotes a value of the file"
            );
        }
    }
    // A path that cannot be read is named by the setting alone.
    let unreadable = Conventions::load(&Environment::from_vars([(
        CONVENTIONS_FILE,
        "/nonexistent/conventions.json",
    )]))
    .expect_err("an unreadable file refuses");
    assert_eq!(
        unreadable,
        ConventionsError::Unreadable {
            setting: CONVENTIONS_FILE
        }
    );
    // A relative path is malformed as a setting.
    let relative = Conventions::load(&Environment::from_vars([(
        CONVENTIONS_FILE,
        "relative.json",
    )]))
    .expect_err("a relative path refuses");
    assert_eq!(relative, malformed("an absolute file path"));
}

#[test]
fn a_forbidden_direction_label_refuses_start() {
    let forbidden = ConventionsError::ForbiddenLabel {
        setting: CONVENTIONS_FILE,
    };
    let planted: Vec<(&str, String)> = ["recall", "Recognition", "to RECOGNIZE"]
        .into_iter()
        .flat_map(|token| {
            [
                (
                    "template rule",
                    edited(|v| {
                        v["direction"]["template_rules"][0]["template"] =
                            json!(format!("Example {token} card"));
                    }),
                ),
                (
                    "note type rule",
                    edited(|v| {
                        v["direction"]["note_type_rules"][0]["contains"] =
                            json!(format!("Example {token}"));
                    }),
                ),
                (
                    "pair rule",
                    edited(|v| {
                        v["direction"]["pair_rules"][0]["front"] = json!(format!("{token} front"));
                    }),
                ),
            ]
        })
        .collect();
    for (what, text) in examined("forbidden labels", planted) {
        assert_eq!(load_text(&text).expect_err(what), forbidden, "{what}");
    }
    // A label the exempt list names is allowed: the exemption is the owner's explicit word.
    let exempt = edited(|v| {
        v["direction"]["template_rules"][0]["template"] = json!("Example recall card");
        v["direction"]["exempt_tokens"] = json!(["Example recall card"]);
    });
    let loaded = load_text(&exempt).expect("an exempt label loads");
    assert_eq!(
        loaded.direction.template_rules[0].template,
        "Example recall card"
    );
}
