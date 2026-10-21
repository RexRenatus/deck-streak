//! The kernel's constants equal the predecessor's, read from its modules by the parity oracle and
//! never retyped from a reading of them (SPEC-020 A8; CHARTER 8).

// An integration test is test code: the golden reader prints the examined count on purpose.
#![allow(clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::db::DB_BUSY_TIMEOUT_MS;
use deck_streak_kernel::offload::{OFFLOAD_MAX_WORKERS, SLOW_OFFLOAD_MS};
use deck_streak_kernel::redact::{
    MIN_SECRET_LEN, REDACTED, TELEGRAM_TOKEN_PATTERN, TOKEN_ID_MIN_DIGITS, TOKEN_SECRET_MIN_CHARS,
};
use deck_streak_kernel::settings::DEFAULT_DIGEST_HOUR;
use deck_streak_kernel::study_day::DEFAULT_ROLLOVER_HOUR;
use serde_json::{Value, json};

#[test]
fn the_kernel_constants_equal_the_predecessors() {
    // The token pattern the redactor matches by hand, written from the bounds it matches with.
    let pattern =
        format!(r"\d{{{TOKEN_ID_MIN_DIGITS},}}:[A-Za-z0-9_-]{{{TOKEN_SECRET_MIN_CHARS},}}");
    let port: BTreeMap<&str, Value> = BTreeMap::from([
        (
            "constants.DEFAULT_ROLLOVER_HOUR",
            json!(DEFAULT_ROLLOVER_HOUR),
        ),
        ("constants.DEFAULT_DIGEST_HOUR", json!(DEFAULT_DIGEST_HOUR)),
        ("logging_redact._REDACTED", json!(REDACTED)),
        ("logging_redact._TELEGRAM_TOKEN_RE.pattern", json!(pattern)),
        ("logging_redact._MIN_SECRET_LEN", json!(MIN_SECRET_LEN)),
        ("offload.OFFLOAD_MAX_WORKERS", json!(OFFLOAD_MAX_WORKERS)),
        ("offload.SLOW_OFFLOAD_MS", json!(SLOW_OFFLOAD_MS)),
        ("database.DB_BUSY_TIMEOUT_MS", json!(DB_BUSY_TIMEOUT_MS)),
    ]);
    let mut proved = BTreeSet::new();
    golden::each_case("kernel.constants", |case| {
        let name = case.input["name"].as_str().unwrap_or_default();
        let value = port
            .get(name)
            .unwrap_or_else(|| panic!("the golden names {name}, which the kernel does not hold"));
        assert_eq!(value, &case.output, "{name}");
        proved.insert(name.to_owned());
    });
    assert_eq!(
        proved,
        port.keys().map(|&name| name.to_owned()).collect(),
        "every constant the kernel ports was proved"
    );
    // The pattern the kernel documents is the one it was proved to match.
    assert_eq!(TELEGRAM_TOKEN_PATTERN, pattern);
}
