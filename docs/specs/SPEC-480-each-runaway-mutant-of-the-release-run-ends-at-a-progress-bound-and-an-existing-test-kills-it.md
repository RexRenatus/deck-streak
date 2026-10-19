# SPEC-480: each runaway mutant of the release run ends at a progress bound and an existing test kills it

- **Issue:** #822, the release whose run measured six Python legs dead with no report and one Rust
  mutant stopped by the memory scope; refers, closes nothing. Refers #322 (every survivor killed or
  recorded equivalent) and #825 (the Python legs' fit).
  **Context(s):** the mutation scripts (`scripts/`), the ingest crate's memory-state reader
  (`crates/ingest`) and `formal/`; section 9 adds the Rust legs' own bounded baseline, which
  changes the `mutation-rust` and `mutation-verdict` jobs of `.github/workflows/ci.yml`, adds two
  shell scripts and `tools/rust-baseline-budget/`. No file of the mutation harness changes.
- **Decided by:** ADR-497 (a runaway mutant of the release run is killed by a progress bound that is
  the identity on every admitted input); ADR-509 for section 9 (a Rust leg runs its own unmutated
  baseline under a time budget, and cargo-mutants skips its own).
- **Schematic:** `docs/schematics/mutation-testing-python.md` section 11, added insert-only;
  section 9 is drawn in `docs/schematics/mutation-testing.md` section 12, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-480.md`; sections 9 to 11 by the pull request that amends this file and
  that record. **Mutation band:** S48000-S48099. **Models:** the new Lean entry
  `lean/TokenProgress`.

## 1. The problem, measured

Every figure below was read from run 39033655190 (its six dead legs ran 755 s to 1327 s), its jobs' logs and artifacts,
or from `git show 36c4783a:<path>` at #822's head `36c4783a29c44d2ffafc987803ccb1742a2c2cfc`.

- **Six Python legs left no report.** `mutation-python (150)`, `(152)`, `(154)`, `(156)`, `(157)`
  and `(158)` each died in the step that runs `scripts/mutation_python.py run ... --failfast`, the
  log ending with the runner's shutdown signal and exit 143 and no line after the step's start, in
  both attempts. Attempt 1's jobs 117399178683, 117399178676, 117399178643, 117399178597, 117399178680, 117399178691 ran 781 s, 822 s, 817 s, 896 s, 1271 s and 1025 s;
  attempt 2's jobs 117533719194, 117533719226, 117533719211, 117533719241, 117533719224, 117533719259 ran 846 s, 837 s, 788 s, 755 s, 1327 s and 1280 s (each the run step's seconds, from its log).
- **Each leg's third dealt mutant never advances its loop.** All 78 mutants dealt to the six are in
  `scripts/mutation-verdict.py`, 13 per leg. The third of each is in `rust_tokens` (:513-539):
  `532:25` `and` to `or`, `532:53` `==` to `!=`, `534:24` `<` to `>=`, `534:62` `or` to `and`,
  `534:76` `==` to `!=`, `534:79` `"_"` to `""`. Under each, a token start (punctuation for line
  532, a word character for `534:24` and `534:62`, a word opening with `_` for `534:76` and
  `534:79`) leaves `stop == at`, and `tokens.append((at, stop)); at = stop` (:537-538) appends
  without end.
- **The harness bounds time, not memory.** `scripts/mutation_python.py` runs each mutant in a child
  of its own session (:1039-1046) under a wall-clock bound derived from its control run (:1138),
  and kills the child's process group when it passes (:1047-1077). Nothing bounds the child's
  memory. The sibling legs 151, 153, 155 and 159 of run 39033655190 had bounds of 1138 s, 800 s, 932 s and 763 s.
  Their own `rust_tokens` mutants advance and reported, the slowest whole in 302.2 s (run 39033655190, leg 153);
  by their pace the third mutant of a dead leg started near 440 s to 880 s, before any bound could pass (run 39033655190).
- **The verdict hid the six under the class FAIL.** The run's mutation verdict read the scripts
  class FAIL with 804 unexplained survivors of 6617 examined, the oracle class VOID for the six
  missing reports, and six scripts VOID lines below the FAIL: a class FAIL outranks its VOID lines
  (`Verdict.close`, mutation-verdict.py:1495-1505).
- **One Rust mutant ran until the memory scope stopped it.** Rust shard 140 (job 117399177077) read `crates/ingest/src/memory_state.rs:85:16`, `+=` to `*=` in `leniently`, stopped 754.018 s in;
  the class read it as a memory finding, neither caught nor timed out. In `leniently` (:41-89),
  `at *= width` with `at` at 0 on a text that opens with `{` never advances, and `out` grows.
- **The plan has room for one mutant.** The release plan dealt `scripts/mutation-verdict.py`'s
  2843 mutants over 219 runs of 13 (215) and 12 (4), so four more fit before a run is added; it
  dealt 304 Python legs against `PYTHON_RELEASE_CEILING` (mutation-verdict.py:131), 305.
- **The Rust population the plan tokenizes.** At `36c4783a`, 340 paths match the `rust` class's
  pattern (`RUST`, mutation-verdict.py:115); the plan tokenizes each changed one whole through
  `rust_test_lines` (:633-637), called at :866.

## 2. Requirements

R1. `rust_tokens` refuses a reading that does not advance. Right after
`stop = at + 1 if stop is None else stop` it holds `if stop <= at:` followed by
`raise RuntimeError(at)`, so every pass of its loop moves `at` forward by at least one character or
raises naming the position. Nothing else in `rust_tokens` changes.

R2. The guard is the identity on the release population. For every path of the tree that the
verdict's own `classify` reads as `rust` (the pattern `RUST`, `crates/<crate>/src/**/*.rs`), read
whole at the head under test, the JSON text of the guarded function's token list, encoded as UTF-8,
is byte-equal to that of a verbatim copy of the base function (`36c4783a`, mutation-verdict.py:513-539)
held in the test module. At #822's head the population is the 340 sources of section 1, which hold
every Rust file the release plan tokenizes. The test prints its examined count and refuses 0. A
local run may add exclude pathspecs at its invocation through `RUST_CENSUS_PATHSPECS`, one per line;
the test prints how many it applied, and refuses any when `CI` is set, so CI reads the population
whole.

R3. Each of the six runaway mutants is killed by a test that reads a literal text holding its token
class and asserts the whole token list: a punctuation text for S48001 and S48002, a word text for
S48003 and S48004, a word opening with `_` for S48005 and S48006. Under each mutant the guard
raises at the stalled token and the test fails.

R4. The guard's own listed mutant, `<=` to `>`, is killed by the word-text test of R3, which then
raises at its first token, and a non-vacuity control shows that test's text meets the guard once per
token: a spy wrapping the real `rust_literal` counts one reading per token, and every token
advances, so each reading reaches the guard with its stop past its start. A bounded stand-in reader
that returns its start shows the guard refuses at the position where a reading stays.

R5. `leniently` in `crates/ingest/src/memory_state.rs` runs at most `bytes.len()` passes: its
`while at < bytes.len() {` becomes `for _ in 0..bytes.len() {` whose body opens with
`if at >= bytes.len() { break; }`, the old body unchanged after it. The bound is the identity on
every admitted input: over 4096 texts drawn from a fixed seed, `parse` returns exactly what a
verbatim copy of the base reader returns (`36c4783a`, memory_state.rs:41-124), and the test prints
its examined count and the count that parsed to a state, refusing 0 for either.

R6. Row S48007 mutates `at += width;` to `at *= width;` in `src/memory_state.rs`, and the existing
`memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field` kills it: with the
bound the reading ends with a wrong text and `parse` answers none.

R7. The Lean entry `lean/TokenProgress` (`formal/lean/Formal/TokenProgress.lean`) covers
`rust_tokens` and `block_end` and proves that, when a block comment's end lies past its start, the
guarded loop ends within the text's length plus one passes and returns at most the text's length of
tokens, whatever the literal reader answers. Its witnesses are the base loop under row S48001's
mutant on `{` and under row S48003's on `a`, and its vectors hold `rust_tokens` to the proof.

R8. No file of the mutation harness changes: `scripts/mutation_python.py` and
`scripts/mutation-python.json` stay as at the base, and `PYTHON_RELEASE_CEILING` stays 305. The
census of R2 lives in a module outside the verdict file's map, so no mutant run executes it; the
verdict file's dealt figure becomes 2844, within its 219 runs.

R9. After the land, the next push-triggered run of #822 reads each of the six legs with a report
and its examined count (section 7). This is the delivery's measure after the land, decided there,
and no criterion of the pull request that lands it.

## 3. Acceptance criteria of SPEC-480

| id | criterion | decided by |
|---|---|---|
| A1 | a reading that does not advance is refused at its start: over `" {"` with a stand-in `rust_literal` that returns its start, `rust_tokens` raises `RuntimeError(1)` after one reading (R1, R4) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_reading_that_does_not_advance_is_refused_at_its_start` |
| A2 | a punctuation character is one token: `"{};"` reads `[(0, 1), (1, 2), (2, 3)]` (R3; kills S48001, S48002) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_punctuation_character_is_one_token` |
| A3 | a word is one token to its end: `"ab c1"` reads `[(0, 2), (3, 5)]` (R3, R4; kills S48003, S48004 and the guard's own mutant) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_is_one_token_to_its_end` |
| A4 | a word that opens with an underscore is one token: `"_a b_"` reads `[(0, 2), (3, 5)]` (R3; kills S48005, S48006) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_that_opens_with_an_underscore_is_one_token` |
| A5 | A3's text meets the guard once per token: a spy wrapping the real `rust_literal` counts 2 readings, and both tokens advance (R4, the non-vacuity control of A3's kill) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_each_token_of_the_word_text_meets_the_guard_once` |
| A6 | every Rust source of the release population reads byte-equal tokens with the guard and with the base copy, its examined count printed and 0 refused (R2) | `python3 -m unittest discover -s scripts/tests -p test_rust_tokens_identity.py -k test_every_rust_source_reads_the_same_tokens_with_the_guard` |
| A7 | the bounded reader parses every seeded text as the base reader does, its examined and parsed counts printed and 0 refused (R5) | `cargo test -p deck-streak-ingest --test memory_state_lenient_bound -- --exact the_bounded_reader_parses_every_seeded_text_as_the_base_reader_does` |
| A8 | `rust_tokens` agrees with every vector of `lean/TokenProgress`, each vector's tokens no more than its text's length (R7) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_tokens_agree_with_every_vector_of_their_proof` |
| A9 | a dealt leg whose baseline record reads exit 124 is VOID by name, "its unmutated baseline reached its 3845 s budget, so no mutant of it was judged", the verdict exits 3, and a green sibling leg reads no VOID (R13) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_leg_whose_baseline_reached_its_budget_is_void` |
| A10 | a dealt leg whose baseline exited 100 is VOID by name, "its unmutated baseline failed (exit 100), so no mutant of it was judged" (R13) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_leg_whose_baseline_failed_is_void_with_its_exit` |
| A11 | a dealt leg that left no `baseline/baseline.exit` is VOID by name, "it left no record of its unmutated baseline, so no mutant of it was judged", and a leg the plan deals no mutant is not read for one (R13) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_dealt_leg_that_left_no_baseline_record_is_void` |
| A12 | legs whose reports hold no `Baseline` outcome, as `--baseline=skip` writes them, each with a green baseline record and a log that times its slowest test at 12.345 s, are read whole: the verdict exits 0 with no VOID line (R12, R13) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_legs_with_a_green_bounded_baseline_and_no_baseline_outcome_are_whole` |
| A13 | the budget is one figure: section 9's `BUDGET 3845 s` line, the baseline script's `budget=3845` and the verdict's `BASELINE_BUDGET_SECONDS` each read 3845; and the Timeouts script's `timeout=2300` and `margin=1.5` equal the verdict's `MUTANT_TIMEOUT_SECONDS` and `TIMEOUT_MARGIN` (R10, R16) | `python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_the_budget_and_the_timeout_are_one_figure_each` |
| A14 | over a seven-mutant listing dealt to three legs, the baseline script runs `cargo nextest run --verbose --package=a --package=c --profile mutants` for leg 0, `--package=b` for leg 1 and `--package=a --package=d` for leg 2, each under `timeout --kill-after=60 3845` with `INSTA_UPDATE=no` and `INSTA_FORCE_PASS=0`, and a leg dealt no package exits 2 with no test run (R11) | `python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_the_packages_are_dealt_as_cargo_mutants_deals_them` |
| A15 | a green baseline exits 0, keeps its run's output in its log, and prints one line naming exit 0, its whole seconds and its 3845 s budget (R11) | `python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_a_green_baseline_exits_0_and_keeps_its_log` |
| A16 | a baseline whose run exits 100 before its budget exits 100 (R11) | `python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_a_failed_baseline_exits_its_own_code` |
| A17 | a copy of the baseline script at a 1 s budget and a 1 s kill delay exits 124 both when its run ends at the budget's signal and when it must be killed after it (R11) | `python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_a_baseline_that_reaches_its_budget_exits_124` |
| A18 | over a fixed four-shard fixture the committed derive prints its five lines exactly, the last `BUDGET 1951 s = ceil(1300.300 x 1.5)` (R15) | `python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_the_derive_prints_its_lines_over_a_fixed_fixture` |
| A19 | the `mutation-rust` job's step `id: baseline` comes before the cargo-mutants step, wraps `bash scripts/mutation-baseline.sh` in the memory scope, records `baseline/baseline.exit` and sets `green=true` only on exit 0; the cargo-mutants step's `if:` requires that output, and its command's words are the base's with `--baseline=skip` before `--sharding` (R11, R12) | `python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_bounded_baseline_runs_before_cargo_mutants_which_skips_its_own` |
| A20 | over a fixed three-leg fixture the Timeouts script prints its lines exactly: the package over the margin and the untimed package READ BY NAME, each Timeout by leg and name, and the package under the margin and the packages with no Timeout by their counts alone (R16) | `python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_each_package_prints_its_timeouts_and_an_over_margin_one_is_read_by_name` |
| A21 | the `mutation-verdict` job runs `bash scripts/mutation-timeouts.sh "$RUNNER_TEMP/reports"` in a step after the verdict's, under `if: ${{ always() }}` (R16) | `python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_verdict_job_prints_the_timeouts_by_package` |
| A22 | the plan's charge stays the measured one: `BASELINE_SECONDS` 2141, `CENSUS_SECONDS`, `SHARD_BOUND_SECONDS` 10800 and the timeout pair, as at the base (R14) | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_sizing_constants_are_the_measured_ones` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_reading_that_does_not_advance_is_refused_at_its_start
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_punctuation_character_is_one_token
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_is_one_token_to_its_end
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_that_opens_with_an_underscore_is_one_token
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_each_token_of_the_word_text_meets_the_guard_once
A6: python3 -m unittest discover -s scripts/tests -p test_rust_tokens_identity.py -k test_every_rust_source_reads_the_same_tokens_with_the_guard
A7: cargo test -p deck-streak-ingest --test memory_state_lenient_bound -- --exact the_bounded_reader_parses_every_seeded_text_as_the_base_reader_does
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_tokens_agree_with_every_vector_of_their_proof
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_leg_whose_baseline_reached_its_budget_is_void
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_leg_whose_baseline_failed_is_void_with_its_exit
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_dealt_leg_that_left_no_baseline_record_is_void
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_legs_with_a_green_bounded_baseline_and_no_baseline_outcome_are_whole
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_the_budget_and_the_timeout_are_one_figure_each
A14: python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_the_packages_are_dealt_as_cargo_mutants_deals_them
A15: python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_a_green_baseline_exits_0_and_keeps_its_log
A16: python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_a_failed_baseline_exits_its_own_code
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_a_baseline_that_reaches_its_budget_exits_124
A18: python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_the_derive_prints_its_lines_over_a_fixed_fixture
A19: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_bounded_baseline_runs_before_cargo_mutants_which_skips_its_own
A20: python3 -m unittest discover -s scripts/tests -p test_mutation_baseline.py -k test_each_package_prints_its_timeouts_and_an_over_margin_one_is_read_by_name
A21: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_verdict_job_prints_the_timeouts_by_package
A22: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_sizing_constants_are_the_measured_ones
```

A1 is red first: at the base `rust_tokens` holds no guard, so the stand-in's reading at position 1
repeats until the stand-in's own call limit fails the test with its sentinel, an assertion bounded
in memory, never a missing name or an empty selection. The other seven pin behaviour the base
already has and are recorded `not red`, each with its reason: A2, A3 and A4 are the killers of rows
S48001 to S48006 (A3 also of the guard's own mutant), whose mutants the base cannot run to an end;
A5 is A3's non-vacuity control; A6 and A7 are the identity proofs, equal at the base by
construction, the second with row S48007 as its red; and A8 holds the code to the Lean entry's
vectors. A6, A7 and A8 enumerate what they read, print the count they examined and refuse zero.

A9 to A12, A19 and A21 are red first at the base, each by assertion: the base's verdict reads
the report's own `Baseline` outcome, so A9 to A11 find no line naming a budget, a failure or a
missing record and A12 reads its legs VOID; and the base's workflow holds no baseline step and
no Timeouts step. A14 to A17 and A20 are red over a stub of each new script, committed with
them, that keeps every input and runs the base's test command unbounded, in the default
environment, with no deal, and prints no package: each fails by assertion, never by a missing
file. A13, A18 and A22 are recorded `not red`, each with its reason: A13 pins three figures the
build writes together, its red row S48008 and the verdict's own constant mutants killed by A9;
A18 pins the committed derive, whose code lines are proved equal to those the budget was sized
by, its red row S48028; and A22 pins the charge the base already holds. Every criterion of
section 9 is read in the pull request's checks, and the reds on its red push.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | the verdict | the progress guard in `rust_tokens`: two lines after `stop = at + 1 if stop is None else stop` (R1) |
| `scripts/tests/test_mutation_verdict_python_kills.py` | tests | class `RustTokensEndOnEveryInput`: A1 to A5 and A8 |
| `scripts/tests/test_rust_tokens_identity.py` | tests | new: A6, the census over the release population, outside the map |
| `crates/ingest/src/memory_state.rs` | ingest | the pass bound in `leniently` (R5) |
| `crates/ingest/tests/memory_state_lenient_bound.rs` | tests | new: A7, with the verbatim base reader and the seeded population |
| `scripts/mutation-rows.d/S48000-S48099.json` | rows | new: S48001 to S48006 in `SCRIPT_MUTATIONS`, S48007 in `MUTATIONS` |
| `formal/lean/Formal/TokenProgress.lean` | formal | new: the entry, its claim, its theorem and its two witnesses |
| `formal/lean/Formal/TokenProgressVectors.lean` | formal | new: the entry's vectors writer |
| `formal/lean/Formal/Vectors.lean` | formal | one import and one arm for the writer |
| `formal/vectors/token-progress.jsonl` | formal | new: the vectors, written by the writer |
| `docs/specs/SPEC-480-each-runaway-mutant-of-the-release-run-ends-at-a-progress-bound-and-an-existing-test-kills-it.md` | docs | new: this SPEC |
| `docs/decisions/ADR-497-a-runaway-mutant-of-the-release-run-is-killed-by-a-progress-bound-that-is-the-identity-on-every-admitted-input.md` | docs | new: the decision |
| `docs/schematics/mutation-testing-python.md` | docs | section 11 appended, insert-only |
| `docs/red-first/SPEC-480.md` | docs | new: the red-first record |
| `changelog.d/release-mutation-legs-480.md` | docs | new: the changelog fragment |
| `.github/workflows/ci.yml` | CI | section 9: in `mutation-rust`, the step `id: baseline` before cargo-mutants, the cargo-mutants step's `if:` gate and `--baseline=skip` (R11, R12); in `mutation-verdict`, the Timeouts step after the verdict's (R16) |
| `scripts/mutation-baseline.sh` | CI scripts | new: the leg's own bounded baseline (R11) |
| `scripts/mutation-timeouts.sh` | CI scripts | new: the Timeouts by package (R16) |
| `scripts/mutation-verdict.py` | the verdict | section 9: `BASELINE_BUDGET_SECONDS` and `BASELINE_BUDGET_EXIT`, and `baseline_void` over the leg's own record, called for every dealt leg in `judge_rust` (R13) |
| `scripts/tests/test_mutation_verdict.py` | tests | section 9: class `ALegIsJudgedAgainstItsOwnBoundedBaseline` (A9 to A12); `shard_reports` writes each leg's baseline record; SPEC-362 A6's test reads the leg's own log, its two report-outcome cases removed |
| `scripts/tests/test_memory_cap_verdict.py` | tests | its shard fixtures write a green baseline record |
| `scripts/tests/test_not_started_legs.py` | tests | its listed legs read their baseline record |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-rust-shard-0/baseline/baseline.exit` | tests | new: the listed leg's green baseline exit |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-rust-shard-0/baseline/baseline.log` | tests | new: the listed leg's baseline log |
| `scripts/tests/test_release_rust_ceiling.py` | tests | its shard fixtures write a green baseline record |
| `scripts/tests/test_verdict_download.py` | tests | its shard fixtures write a green baseline record |
| `scripts/tests/test_mutation_workflows.py` | tests | class `TheRustLegRunsItsBoundedBaselineFirst` (A19, A21) |
| `scripts/tests/test_mutation_baseline.py` | tests | new: A13 to A18 and A20, with stand-in `cargo` and `timeout` first on `PATH` |
| `scripts/tests/test_memory_scope.py` | tests | `CI_LEG` gains `--baseline=skip` |
| `tools/rust-baseline-budget/size_budget.py` | tools | new: the committed derive (R15) |
| `scripts/mutation-rows.d/S48000-S48099.json` | rows | S48008 to S48038 in `SCRIPT_MUTATIONS` |
| `docs/schematics/mutation-testing.md` | docs | section 12 appended, insert-only |
| `docs/decisions/ADR-509-a-rust-leg-runs-its-own-unmutated-baseline-under-a-time-budget-and-cargo-mutants-skips-its-own.md` | docs | new: the decision of section 9 |
| `docs/red-first/SPEC-480.md` | docs | section 9's criteria, A9 to A22 |
| `changelog.d/rust-baseline-budget-822.md` | docs | new: section 9's changelog fragment |

## 5. What this does NOT cover

- The per-mutant memory bound in `scripts/mutation_python.py` (ADR-497 D7), held until the release pull request merges (#822).
- The scripts class's unexplained survivors: ADR-451's kill parts 2 to 6 (SPEC-438 to SPEC-442), each re-derived from the cure's measure after this land and never from an earlier part's build (#822).
- The 107 survivors in `scripts/mutation_rows.py`, which an open change to that file and its test may move before part 2 reads them (#954).
- The rust shards' red baselines, which section 9 now bounds by each leg's own baseline budget
  and reads VOID with their reasons; its closing run is read in section 11 (#822).
- A package the closing run's Timeouts lines mark READ BY NAME, which becomes a follow-up item
  read by its Timeouts' names, never a change to section 9 (#822).
- A skipped baseline that no leg runs, and a raised per-mutant timeout: each narrows the gate,
  and the owner's ruling refuses both (#822).
- The open change to the verdict script and the Python schematic, which shares two of this manifest's files; whichever lands second re-reads the other's lines (#873).
- Every survivor killed or recorded equivalent, which is the closing run's (#322), and the Python legs' fit to their job (#825).

## 6. Risks

- A6 reads every Rust source of the tree, so a source added later in which a reading stays would
  raise in the guarded function where the base copy loops; no base reading stays on any text,
  since each branch advances, and A6 runs in the job that runs `scripts/tests`, which reads it.
- The guard moves every later line of the verdict file down two lines: rows anchor by text, and the
  pull request's own mutation verdict reads any record or row it moves as STALE.
- The stride deal places mutants on legs by listing position, so the six may sit on other legs
  after the cure; section 7 reads the six named legs, and any Python leg with no report reads VOID
  in the same verdict.
- A census over the test modules may refuse a module outside the verdict file's map; the job that
  runs `scripts/tests` reads it.
- `lean/TokenProgress` covers `rust_tokens` and `block_end`, so an edit of either, #873's included,
  reads the entry STALE until it is re-ported and stamped.
- Section 9's baseline deals its packages from the plan's listing as cargo-mutants deals its
  shards; a pinned cargo-mutants that dealt otherwise would test a mutant whose package no
  baseline ran. A14 pins the deal, and the tool's pin in the workflow moves only by a change
  that reads A14 again.
- A nextest status line of another shape times no test: R13 then reads the leg VOID and R16
  reads its packages untimed, READ BY NAME, so a format change fails closed.
- A leg's whole baseline may now take its 3845 s budget where the plan charges 3662 s (run 39151539183's plan),
  and the release deal's slowest leg then projects at 10690 s of 10800 (run 39151539183's plan); the
  verdict's legs line reads any leg the job's bound stops.
- The two shell scripts are outside the Python mutant population, so rows S48008 to S48038 are
  their only mutants, each mutant checked by `bash -n` before its row ships.

## 7. The cure's measure after the land

Who: the next push-triggered run of #822 after this pull request lands, the run ADR-497 D5 reads
as the cure's measure, read by the legs' names. When: once that run completes. Record: each of `mutation-python (150)`, `(152)`,
`(154)`, `(156)`, `(157)` and `(158)` uploads its report, and the run's mutation verdict prints each
leg's examined count with no `no report` line for any of them. It is no acceptance criterion: the
pull request's own checks cannot run it, and it is read after the land.

## 8. Mutation rows S48001 to S48007

Each find occurs once in its target at `36c4783a`, and each killer is one test.

| id | table | target | find | replace | killer |
|---|---|---|---|---|---|
| S48001 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `if stop is None and (char.isalnum() or char == "_"):` | `if stop is None or (char.isalnum() or char == "_"):` | A2 |
| S48002 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `if stop is None and (char.isalnum() or char == "_"):` | `if stop is None and (char.isalnum() or char != "_"):` | A2 |
| S48003 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):` | `while stop >= len(text) and (text[stop].isalnum() or text[stop] == "_"):` | A3 |
| S48004 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):` | `while stop < len(text) and (text[stop].isalnum() and text[stop] == "_"):` | A3 |
| S48005 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):` | `while stop < len(text) and (text[stop].isalnum() or text[stop] != "_"):` | A4 |
| S48006 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):` | `while stop < len(text) and (text[stop].isalnum() or text[stop] == ""):` | A4 |
| S48007 | `MUTATIONS` | `ingest`, `src/memory_state.rs` | `at += width;` | `at *= width;` | `memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field` |

The six Python rows mirror the release run's six runaway mutants. Under the guard each ends at its
first stalled token; without it each runs until memory ends it, so none of them is installed where
the guard is absent. S48007 is installed only where the bound of R5 holds.

## 9. Requirements of the Rust legs' baseline budget

Run 39033655190 of #822 measured why its rust class read VOID. cargo-mutants has one `--timeout`,
which bounds a baseline's tests as well as each mutant's (ADR-328), and 107 of the run's 192 Rust
legs' baselines read Timeout at 2300.0 s to 2300.1 s of test (run 39033655190), so no mutant of those
legs was judged. The owner's ruling bounds each leg's own unmutated baseline by a time budget sized
from that run's whole legs, and keeps every mutant at the 2300 s timeout (ADR-509).

R10. The budget is 3845 s, the largest whole baseline of run 39033655190 (build and test together)
times SPEC-362 R7's margin 1.5, rounded up:
`BUDGET 3845 s = ceil(2562.744 x 1.5)` (run 39033655190).
The derive of R15 examined 192 shards of run 39033655190: 85 whole, 107 whose baseline read Timeout, 0 other.
Its largest whole baseline was mutation-rust-shard-121 of run 39033655190: build 300.8 s, test 2261.9 s.
The figure is written three times, each equal: the `BUDGET` line above, `budget=3845` in
`scripts/mutation-baseline.sh`, and `BASELINE_BUDGET_SECONDS` in `scripts/mutation-verdict.py`.

R11. Each `mutation-rust` leg runs its own unmutated baseline before cargo-mutants, in a step
`id: baseline` of `.github/workflows/ci.yml` that runs `bash scripts/mutation-baseline.sh` inside
the memory scope (SPEC-196). The script deals the packages of the leg's mutants from the plan's
listing, `listed.json`, as cargo-mutants deals its round-robin shards (mutant `i` of the listing
to leg `i mod n`), sorted and unique, and runs one
`cargo nextest run --verbose --package=<each> --profile mutants` with `INSTA_UPDATE=no` and
`INSTA_FORCE_PASS=0`, as cargo-mutants' own baseline does, under `timeout --kill-after=60 3845`.
It exits 124 when its run ended at or past the budget, whether `timeout` exited 124 or 137, else
its run's own exit, and 2, with no test run, when the leg is dealt no package or the listing
cannot be read; it prints one line naming its exit, its whole seconds and its budget. The step
records that exit in `baseline/baseline.exit` and the run's output in `baseline/baseline.log`
under the leg's report directory, which the leg's artifact uploads, and sets its output `green`
only on exit 0. The step itself always succeeds.

R12. The cargo-mutants step runs only when the baseline step's `green` output is `true`, and its
command is the base's with `--baseline=skip` before `--sharding`. Every other word,
`--timeout 2300 --build-timeout 600` included, is unchanged, so each mutant keeps the per-mutant
timeout of SPEC-327 R2, and no mutant is filtered or skipped. A leg whose baseline is not green
runs no mutant.

R13. The verdict's rust class judges each leg the plan deals a mutant against that leg's own
baseline record: `baseline_void` is called for every dealt leg, not only for one with a whole
report. A leg with no `baseline/baseline.exit` reads VOID "it left no record of its unmutated
baseline, so no mutant of it was judged"; exit 124 reads VOID "its unmutated baseline reached
its 3845 s budget, so no mutant of it was judged"; any other non-zero exit reads VOID "its
unmutated baseline failed (exit <code>), so no mutant of it was judged". A green baseline is then
held to SPEC-362 R7 over `baseline/baseline.log`: an unreadable log, a log that times no test,
and a slowest test whose time times 1.5 passes 2300 s each read VOID, as before. A VOID is never
a pass, and a report that holds no `Baseline` outcome, as `--baseline=skip` writes it, is read
whole.

R14. The plan's charge does not change. `BASELINE_SECONDS` 2141, plus the census term of each
listed census package (`CENSUS_SECONDS`, 1521 for `deck-streak-progression`),
`SHARD_BOUND_SECONDS` 10800 and `RUST_RELEASE_CEILING` 192 stay as at the base.
Over the release plan's Rust listing of 9383 mutants, that charge, 3662 s, deals 192 legs of 192, the slowest projected at 10507 s of 10800 (run 39151539183's plan).
The same deal with each leg's baseline at its whole 3845 s budget projects its slowest leg at 10690 s of 10800 (run 39151539183's plan).
A charge that dealt more than 192 Rust legs would refuse the plan whole, as at the base.

R15. The derive is committed. `tools/rust-baseline-budget/size_budget.py` holds the derive the
owner's ruling sized the budget by, its code lines byte for byte under a header that names its
inputs and its command. From a checkout of the repository, its derive command is
`gh run download 39033655190 -p 'mutation-rust-shard-*' -D <dir>`, then
`python3 -I -B tools/rust-baseline-budget/size_budget.py <dir>`, and over run 39033655190's
artifacts it prints the figures of R10,
its last line `BUDGET 3845 s = ceil(2562.744 x 1.5)` (run 39033655190).
Over a fixed four-shard fixture it prints the lines A18 states.

R16. The verdict job prints each Rust package's Timeouts, and adds no Python leg: a step of the
`mutation-verdict` job, after the verdict's and under `if: ${{ always() }}`, runs
`bash scripts/mutation-timeouts.sh` over the job's reports directory. For each package a leg's
`outcomes.json` names, it prints one line: the package, how many of its mutants read Timeout, how
many it has, and its baseline test time, the sum of the seconds nextest gives that package's
tests in a leg's green `baseline/baseline.log`, the largest over the legs whose baseline tested
it, or `untimed` when no log times it. A package with a Timeout whose baseline test time, times
1.5, passes 2300 s (a time over 2300/1.5 s), or that is untimed, is marked READ BY NAME, and each
of its Timeouts is printed by leg and mutant name, so that the closing run's reader reads those
Timeouts by name before ruling the run complete and none counts silently as a kill. The script
first prints how many leg reports and green baseline logs it read, and exits 0 unless a report
cannot be read, which fails the job.

R17. After the land, the next push-triggered run of #822 is the closing run, read by name
(section 11). It is no criterion of the pull request that lands section 9.

## 10. Mutation rows S48008 to S48038

Each find occurs once in its target at the head that lands section 9: `\n` in a find is its
line break, and `\|` in a cell is its `|`. Each killer is the one test its criterion names.
Each shell row's mutant is checked by `bash -n` before the row ships, since the rows' viability
check parses no shell.

| id | table | target | find | replace | killer |
|---|---|---|---|---|---|
| S48008 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `budget=3845` | `budget=3844` | A13 |
| S48009 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `select(.key % $n == $k)` | `select(.key % $n != $k)` | A14 |
| S48010 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `\| unique \| .[] \|` | `\| .[] \|` | A14 |
| S48011 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `--profile mutants >` | `--profile default >` | A14 |
| S48012 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `INSTA_UPDATE=no INSTA_FORCE_PASS=0` | `INSTA_UPDATE=always INSTA_FORCE_PASS=0` | A14 |
| S48013 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `dealt no package by $listed"\n    exit 2` | `dealt no package by $listed"\n    exit 0` | A14 |
| S48014 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `--kill-after=60 "$budget"` | `--kill-after=60 "$((budget + 1))"` | A14 |
| S48015 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `--kill-after=60` | `--kill-after=600` | A14 |
| S48016 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `[ "$rc" -ne 0 ] && [ "$elapsed"` | `[ "$rc" -eq 0 ] && [ "$elapsed"` | A17 |
| S48017 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `-ge "$budget" ]; then rc=124` | `-lt "$budget" ]; then rc=124` | A16 |
| S48018 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `then rc=124; fi` | `then rc=125; fi` | A17 |
| S48019 | `SCRIPT_MUTATIONS` | `scripts/mutation-baseline.sh` | `exit "$rc"` | `exit 0` | A16 |
| S48020 | `SCRIPT_MUTATIONS` | `.github/workflows/ci.yml` | `steps.baseline.outputs.green == 'true'` | `steps.baseline.outputs.green != 'true'` | A19 |
| S48021 | `SCRIPT_MUTATIONS` | `.github/workflows/ci.yml` | `git.diff" --baseline=skip --sharding` | `git.diff" --sharding` | A19 |
| S48022 | `SCRIPT_MUTATIONS` | `.github/workflows/ci.yml` | `if [ "$rc" -eq 0 ]; then echo "green=true"` | `if [ "$rc" -ne 0 ]; then echo "green=true"` | A19 |
| S48023 | `SCRIPT_MUTATIONS` | `.github/workflows/ci.yml` | `echo "$rc" > "$out/baseline/baseline.exit"` | `echo "$rc" > "$out/baseline.exit"` | A19 |
| S48024 | `SCRIPT_MUTATIONS` | `.github/workflows/ci.yml` | `each over-margin package by name\n        if: ${{ always() }}` | `each over-margin package by name\n        if: ${{ success() }}` | A21 |
| S48025 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `reached its {BASELINE_BUDGET_SECONDS} s budget` | `reached its {MUTANT_TIMEOUT_SECONDS} s budget` | A9 |
| S48026 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `failed (exit {code})` | `failed (exit {BASELINE_BUDGET_EXIT})` | A10 |
| S48027 | `SCRIPT_MUTATIONS` | `scripts/mutation-verdict.py` | `it left no record of its unmutated baseline` | `it left a record of its unmutated baseline` | A11 |
| S48028 | `SCRIPT_MUTATIONS` | `tools/rust-baseline-budget/size_budget.py` | `MARGIN = 1.5` | `MARGIN = 1.4` | A18 |
| S48029 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `$3 == "Timeout"` | `$3 != "Timeout"` | A20 |
| S48030 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `timeouts[package] > 0 &&` | `timeouts[package] >= 0 &&` | A20 |
| S48031 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `(!(package in most) \|\| most[package] * margin > timeout)` | `(most[package] * margin > timeout)` | A20 |
| S48032 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `most[package] * margin > timeout)` | `most[package] * margin < timeout)` | A20 |
| S48033 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `timeout=2300` | `timeout=2200` | A20 |
| S48034 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `margin=1.5` | `margin=1.4` | A20 |
| S48035 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `sum[package] += head` | `sum[package] = head` | A20 |
| S48036 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `$3 + 0 > most[$2]` | `$3 + 0 < most[$2]` | A20 |
| S48037 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `sub(/::.*/, "", package)` | `sub(/::$/, "", package)` | A20 |
| S48038 | `SCRIPT_MUTATIONS` | `scripts/mutation-timeouts.sh` | `sub(/^\([[:space:]]*[0-9]+\/[0-9]+\)[[:space:]]+/, "", rest)` | `sub(/^\(\)/, "", rest)` | A20 |

The verdict rows S48025 to S48027 sit in f-strings, which the Python mutation run does not
mutate; the verdict's other new lines are its own listed mutants, killed by A9 to A12. A row
on a figure (S48008, S48028, S48033, S48034) pins the value its criterion reads.

## 11. The closing run after the land

Who: the next push-triggered run of #822 after the pull request that lands section 9, read by
name by the release's reviewer. When: once that run completes. Record: each Rust leg's baseline
line, the rust class's VOID lines by their reasons, and the Timeouts-by-package lines of the
verdict job. Each package those lines mark READ BY NAME has its Timeouts read by name before the
run is ruled complete, and none counts silently as a kill; such a package becomes a follow-up
item, never a change to section 9. It is no acceptance criterion: the pull request's own checks
cannot run it, and it is read after the land.
