# Red-first record: SPEC-497

The SPEC and ADR-515 were installed first (d03994e1). The crate's tests came next, whole, over a stub that keeps every
shape but the behaviour (113b4c7a): the stub crate holds no `study_day.rs`, its study day is 0, it renders and parses
nothing, it admits every hour, its instant reads 0 and its date error's text is empty, and the kernel still defines all
six types itself. ed017445 turns them green: it moves `study_day.rs` into the crate with only its two import lines
re-pointed, moves `UtcMillis` and `IsoDateError` beside it, re-exports all three from the kernel, keeps `SystemClock` in
the kernel's clock, and draws the kernel's new edge in the context map and its schematic. Each red below is quoted from
a run of the fence's tests on the tree of 113b4c7a, each green from a run on the tree of ed017445.

```red-first
A1: red at 113b4c7a: crates/study-day/tests/crate_shape.rs:260: assertion `left == right` failed: the crate and the kernel (left: ["Hour: still defined in crates/kernel/src/study_day.rs", "IsoDateError: still defined in crates/kernel/src/error.rs", "StudyDay: still defined in crates/kernel/src/study_day.rs", "StudyDayRule: still defined in crates/kernel/src/study_day.rs", "UtcMillis: still defined in crates/kernel/src/clock.rs", "UtcOffset: still defined in crates/kernel/src/study_day.rs", "crates/study-day/src/study_day.rs is missing"], right: [])
A1: green at ed017445
A2: red at 113b4c7a: crates/kernel/tests/study_day_crate.rs:209: assertion `left == right` failed: each kernel path names the study day crate's own type (left: all 12 kernel paths examined, from "deck_streak_kernel::UtcMillis" to "deck_streak_kernel::study_day::StudyDay", right: [])
A2: green at ed017445
A3: red at 113b4c7a: crates/study-day/tests/study_day.rs:48: assertion `left == right` failed: the study day of {"instant_ms":856180799999,"rollover_hour":0,"utc_offset_minutes":-720} (left: Some(0), right: Some(9908)); study_day.rs:70: 24 was admitted as an hour (left: Some(Hour(24)), right: None); study_day.rs:77 (left: "", right: "1970-01-01"); study_day.rs:137 (left: Err(IsoDateError), right: Ok("-0001-01-01")); crates/study-day/tests/instant.rs:12 (left: 0, right: 1234); instant.rs:20 (left: 0, right: -1234); instant.rs:26 (left: "", right: "not an ISO calendar date (YYYY-MM-DD)")
A3: green at ed017445
A4: not red: the crate's builds for wasm32-unknown-unknown and aarch64-apple-ios run only in CI's web-engine job (SPEC-497 section 7); this delivery's local runs build the host target alone, so the criterion is read by name on CI at the head
A5: red at 113b4c7a: test_context_map_schematic.py: test_every_arrow_is_a_manifest_edge_the_fence_names: AssertionError: findings: 3 (kernel --> study-day: a manifest edge no arrow draws; kernel: the manifest names study-day, the fence does not; study-day: no fence row); test_every_crate_directory_is_one_node_named_by_it: AssertionError: findings: 1 (study-day: no node)
A5: green at ed017445
A6: red at 113b4c7a: crates/kernel/tests/study_day_crate.rs:239: assertion `left == right` failed: the kernel's clock and the crate (left: ["crates/kernel/src/clock.rs does not take UtcMillis from the crate", "crates/kernel/src/clock.rs still defines UtcMillis"], right: [])
A6: green at ed017445
```

## What each pair disclosed

- **ed017445 changes no fence test.** `crates/study-day/tests/` and `crates/kernel/tests/` are byte-identical at
  113b4c7a and ed017445, and no file under `scripts/` changes on this branch.
- **ed017445 edits one existing test,** `crates/coordination/tests/relight_order.rs`. Its route-state census reads one
  more source tree, `crates/study-day/src` (4 trees to 5), since the router stamps the crate's instant, and its pin of
  `ManualClock`'s changed line reads `now: AtomicI64::new(start.epoch_millis()),`. No criterion of this SPEC rests on
  it, and it was not run at 113b4c7a.
- **The ten Rust fence tests are RED-FIRST:** each failed by name at 113b4c7a (4 binaries, 0 passed, 10 failed) and
  passed at ed017445.
- **A5's red is the schematic census's.** Both census files are the suite's own, unchanged. At 113b4c7a
  `test_context_map_schematic.py` read 4 run, 2 failed, and `test_context_map_register.py` read 4 run, OK; at
  ed017445 both read 4 run, OK.
- **A2's workspace run is green at ed017445:** 447 test binaries, 1933 passed, 0 failed, 10 ignored, against the
  base's 441 and 1923 with the same 10 ignored. The six new binaries are the crate's unit and doc tests, its three
  test files and the kernel's `study_day_crate`; the ten new tests are the fence's. `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` each exit 0 on the same tree. The green run's memory line
  read `oom_kill_delta=1` from a concurrent run on the shared memory slice, and the run itself shows no kill: every
  step exited 0, no test died by signal 9, and its own journal names no out-of-memory kill.
- **Every line of `crates/` at the head equals ed017445's;** the later commit adds only this record and the
  changelog fragment.

## Fix round 2: three refusals the parse already made

A mutation run over the crate's parse found three refusal arms that no test held: the date's part count, the signed
year's width and the day's width. Widening any one of them (to admit a fourth part, a signed year of one digit or a day
of one digit) passed the crate's suite and the kernel's. Each was inherited: the kernel's tests lacked the same inputs
before the move. This round adds one case per arm, in the test that owns the arm, and changes no production line:

- SPEC-497 A3, a fourth part after the day: `2024-01-15-01` is refused, in
  `a_study_day_renders_as_its_iso_date_and_parses_back`.
- SPEC-497 A3, a day of one digit: `2024-01-5` is refused, in the same test.
- SPEC-497 A3, a signed year of one digit: `-1-01-01` is refused, in
  `a_signed_year_is_written_only_where_four_digits_cannot_hold_it`.

The kernel's `crates/kernel/tests/study_day.rs` gains the same three cases in its tests of the same names, each named
for SPEC-020 A2, the criterion that file holds. R3 and R7 keep the kernel's tests as they were; this round adds three
cases to them and changes none of their lines.

Each case passes from the start, because the parse already refused its input, so none is red here and A3's red quote
above stands. Each is proved by its arm instead: on a copy of this commit with that one arm widened, the case's test
fails at the new case's assertion.

From this round on, `crates/` differs from ed017445's in these two test files alone, by 32 added lines; the note
above that every line of `crates/` equals ed017445's held for the head it named.

## Fix: three mutation rows follow the moved file

ed017445 changed no file under `scripts/`, so three hand-proved rows in `scripts/mutation-rows.d/S02000-S02099.json`
still named `crates/kernel/src/study_day.rs` after it moved. At the integration head before this fix,
`mutation_rows.py census` read `census: S02001-ROLLOVER-HOUR-IS-FOUR: its target crates/kernel/src/study_day.rs does
not exist`, and the same for S02002-AN-HOUR-RUNS-TO-23 and S02005-THE-DAY-HAS-NO-HOUR-24, and `mutation_rows.py prove
--row S02001-ROLLOVER-HOUR-IS-FOUR` stopped on `FileNotFoundError` for that path. The fix changes three cells and no
line of `crates/`:

- S02002 and S02005 run in the crate `study-day` (target `src/study_day.rs`); their killers keep their names, since
  `crates/study-day/tests/study_day.rs` holds both tests.
- S02001 stays in the crate `kernel`, because its killer is the kernel's `constants` test of the predecessor's
  goldens, which reads the constant through the kernel's re-export; its target becomes `../study-day/src/study_day.rs`,
  the crate-relative form other rows already use for a file outside their killer's crate.
