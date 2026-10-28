//! The parity oracle's consumer can fail (SPEC-498 R8, ADR-516): a fixture golden with one
//! deliberately wrong output makes the consumer report a mismatch, so a consumer that passes
//! has compared something. The goldens of the second old system are read by Rust tests through
//! `golden::mismatches` and `golden::assert_parity`; this target proves that pair on a fixture
//! before any of those goldens exists.

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::panic;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The fixture: six cases of whole days from a creation time, the fourth recorded one day off.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/parity-oracle/anki/fixtures/consumer_can_fail.json")
}

/// The fixture's port: whole days from the creation time to the instant, both in seconds,
/// floored, so an instant before the creation time is day -1.
fn whole_days(input: &Value) -> Value {
    let crt = input["crt"].as_i64().unwrap_or_default();
    let now = input["now"].as_i64().unwrap_or_default();
    json!((now - crt).div_euclid(86_400))
}

#[test]
fn golden_consumer_can_fail() {
    let fixture = golden::read(&fixture()).expect("the fixture is a strict golden");
    let found = golden::mismatches(&fixture, whole_days);
    println!(
        "examined {} case(s) of {}",
        fixture.cases.len(),
        fixture.function
    );
    assert_eq!(fixture.cases.len(), 6);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].index, 3);
    assert_eq!(found[0].expected, json!(3));
    assert_eq!(found[0].found, json!(2));
    assert_eq!(
        found[0].input,
        json!({"crt": 1_000_000_000, "now": 1_000_172_805})
    );
}

#[test]
fn the_consumer_panics_naming_the_wrong_case() {
    let outcome = panic::catch_unwind(|| golden::assert_parity(&fixture(), whole_days));
    let payload = outcome.expect_err("a golden with a wrong output must fail its consumer");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();
    assert!(
        message.contains("1 of 6 case(s) differ from the port"),
        "{message}"
    );
    assert!(
        message.contains("case 3 with input"),
        "the message names the case: {message}"
    );
}

#[test]
fn the_corrected_fixture_passes_its_consumer() {
    let text = std::fs::read_to_string(fixture()).expect("the fixture reads");
    let mut value: Value = serde_json::from_str(&text).expect("the fixture is JSON");
    value["cases"][3]["output"] = json!(2);
    let corrected =
        golden::parse(&fixture(), &value.to_string()).expect("the corrected fixture is a golden");
    let found = golden::mismatches(&corrected, whole_days);
    println!(
        "examined {} case(s), {} mismatch(es)",
        corrected.cases.len(),
        found.len()
    );
    assert!(found.is_empty(), "{found:?}");
}
