//! The parity oracle's one golden reader, proved in the first crate that compiles it (SPEC-029 A12
//! to A14). The reader is `tools/parity-oracle/golden.rs`, included by path (ADR-029), so this
//! crate proves the same file every later proving crate compiles.

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::path::{Path, PathBuf};

/// A planted golden under this crate's test fixtures.
fn planted(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/goldens")
        .join(name)
}

#[test]
fn the_reader_reports_how_many_cases_it_examined() {
    let mut visited = 0_usize;
    let examined = golden::each_case("study_day", |_| visited += 1);
    // The study-day golden's builder draws 24 rollover, 6 negative, 4 offset and 16 other cases.
    assert_eq!(visited, 50);
    assert_eq!(
        examined.to_string(),
        "examined 50 case(s) of analytics.study_day"
    );
}

#[test]
fn a_golden_with_no_case_is_refused() {
    let refused = golden::read(&planted("empty_cases.json"));
    assert!(
        matches!(refused, Err(golden::GoldenError::NoCase { .. })),
        "a golden with no case was not refused as one: {refused:?}"
    );
}

#[test]
fn a_golden_of_another_schema_is_refused() {
    let refused = golden::read(&planted("wrong_schema.json"));
    assert!(
        matches!(
            &refused,
            Err(golden::GoldenError::Schema { found, .. }) if found == "\"phx.parity-golden.v0\""
        ),
        "a golden of another schema was not refused as one: {refused:?}"
    );
}

#[test]
fn a_golden_missing_a_field_is_refused() {
    // A whole golden but for its function.
    let text = r#"{
        "cases": [{"input": {"instant_ms": 0}, "output": 0}],
        "generator": "tools/parity-oracle/generate.py",
        "generator_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
        "inputs": "synthetic",
        "kind": "function",
        "registry": "tools/parity-oracle/registry/spec_000.py",
        "registry_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
        "schema": "phx.parity-golden.v1",
        "seed": 20,
        "source_commit": "0000000000000000000000000000000000000000"
    }"#;
    let refused = golden::parse(Path::new("inline.json"), text);
    assert!(
        matches!(
            &refused,
            Err(golden::GoldenError::Malformed { reason, .. }) if reason.contains("`function`")
        ),
        "a golden missing its function was not refused as malformed: {refused:?}"
    );
}
