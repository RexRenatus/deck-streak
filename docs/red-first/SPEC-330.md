# Red-first record: SPEC-330

The SPEC, ADR-330 and the two tests were committed first (d91cc808), with the gate, `clippy.toml`
and CI untouched. At that commit the module fails by assertion: `examined 0 disallowed macros`,
`'test-release' not found in [fmt, clippy, ..., secrets]` and `Lists differ: [] != ['release']`
(`FAILED (failures=3)`).

```red-first
A1: red at d91cc808: AssertionError: 'test-release' not found in ['fmt', 'clippy', 'test', 'doctest', 'audit-rust', 'test-engine', 'web', 'audit-web', 'python', 'scrub', 'secrets']
A2: red at d91cc808: AssertionError: examined 0 disallowed macros
A1: green at GREEN
A2: green at GREEN
```

The behaviour the tests stand for, measured with plants that were never committed, in `deck-streak-kernel`
with the stage's own flags (`cargo nextest run --locked --no-fail-fast --release`, `-j 1`):

```text
A1 planted: `if cfg!(debug_assertions) { 1 } else { 2 }` with a test asserting 1
  --release : FAIL the_planted_outcome_is_one (left: 2, right: 1); Summary 1 test run: 0 passed, 1 failed
  dev profile (the existing test stage's profile): PASS; Summary 1 test run: 1 passed
A1 unplanted: the release stage over the workspace passes (RELEASE_RUN below)
A2 planted `cfg!(debug_assertions)`: error: use of a disallowed macro `std::cfg` (crates/kernel/src/lib.rs:54:8)
A2 planted `option_env!("PLANT")`: error: use of a disallowed macro `std::option_env`
```
