# Red-first record: SPEC-330

The SPEC, ADR-330 and the two tests were committed first (d91cc808), with the gate, `clippy.toml`
and CI untouched. At that commit the module fails by assertion: `examined 0 disallowed macros`,
`'test-release' not found in [fmt, clippy, ..., secrets]` and `Lists differ: [] != ['release']`
(`FAILED (failures=3)`).

```red-first
A1: red at d91cc808: AssertionError: 'test-release' not found in ['fmt', 'clippy', 'test', 'doctest', 'audit-rust', 'test-engine', 'web', 'audit-web', 'python', 'scrub', 'secrets']
A2: red at d91cc808: AssertionError: examined 0 disallowed macros
A1: green at 1134ef11
A2: green at 1134ef11
```

The behaviour the tests stand for, measured with plants that were never committed, in `deck-streak-kernel`
with the stage's own flags (`cargo nextest run --locked --no-fail-fast --release`, `-j 1`):

```text
A1 planted: `if cfg!(debug_assertions) { 1 } else { 2 }` with a test asserting 1
  --release : FAIL the_planted_outcome_is_one (left: 2, right: 1); Summary 1 test run: 0 passed, 1 failed
  dev profile (the existing test stage's profile): PASS; Summary 1 test run: 1 passed
A1 unplanted: the release stage over the workspace passes (the full release run: 1323 of 1324 pass; the one failure is the finding in the pull request)
A2 planted `cfg!(debug_assertions)`: error: use of a disallowed macro `std::cfg` (crates/kernel/src/lib.rs:54:8)
A2 planted `option_env!("PLANT")`: error: use of a disallowed macro `std::option_env`
```

## Amendment: the logging test states the build under test (ruling 150)

The full release run found one red, `each_json_line_opens_with_its_journal_priority`: a pinned
dependency's `release_max_level_debug` compiles `trace!` out of the shipped build (ADR-330). The
test now reads `tracing::level_filters::STATIC_MAX_LEVEL` and expects exactly the probe levels at
or below it, with the TRACE line absent when the build compiles it out. Measured at 30943a65 with
the stage's own flags (`cargo nextest run --locked --no-fail-fast --release`, `-j 1`, the kernel
and ingest crates so the workspace's feature unification applies):

```text
before, --release : FAIL each_json_line_opens_with_its_journal_priority
  left: ["ERROR", "WARN", "INFO", "DEBUG"], right: ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"]
before, dev       : PASS (1 test run: 1 passed)
after,  --release : PASS (1 test run: 1 passed)
after,  dev       : PASS (1 test run: 1 passed)
plant (the literal five-level list restored, never committed), --release : FAIL, same left and right
kernel crate, all tests: --release 84 passed, 5 skipped; dev 84 passed, 5 skipped
```

Correction (ruling 154): CI run 37153665162 at the prep merge e7013b7d read mutation-verdict FAILURE, 18 rows (S19041 to S19058) VOID because their killer, test_ci_workflows.WorkflowFilesAreReadAsBytes.test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read, was red on the unmutated head: the census refused `test_ship_profile: <module>: import yaml: 1 dynamic site(s), 0 listed`. The fix commit e5c590df reads ci.yml, check.sh and clippy.toml in test_ship_profile.py through the loader and the checker's reader, with no yaml import; A1 and A2 stay green. CI by name decides the killer.
