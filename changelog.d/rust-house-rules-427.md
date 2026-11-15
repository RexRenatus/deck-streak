### Added

- One guard, `scripts/tests/test_rust_house_rules.py`, holds four Rust house rules at zero over
  every shipped `crates/*/src/**/*.rs` file: SQLite is opened only in `crates/kernel/src/db.rs`,
  no `format!` reaches a query, no secret-named name is passed to a `tracing` or `log` macro, and
  randomness is drawn only through `getrandom`. Each rule prints the count it examined and fails
  on an empty population, and CI's `hygiene` job runs it with the other guard tests (SPEC-427).
