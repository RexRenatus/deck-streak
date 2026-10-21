# Red-first record: SPEC-480

SPEC-480 (R1 to R9, A1 to A8). The SPEC, ADR-497 and section 11 of the Python schematic were
committed first, in aaf5dd0c. The tests of A1 to A7 were committed next and alone, in 54e30834,
over the base's production code unchanged: `rust_tokens` held no progress guard, and `leniently`
looped `while at < bytes.len()`. A1's red is therefore the stand-in's own sentinel, an assertion
bounded in memory, never a missing name or an empty selection. The guard followed in 390a800d,
the pass bound in 84bcb25e, the rows in c0b0c340, and the Lean entry with A8 in 18f3893b, its
covers stamped in f9a8fb93 and its vectors written in d8609712.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_reading_that_does_not_advance_is_refused_at_its_start
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_punctuation_character_is_one_token
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_is_one_token_to_its_end
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_word_that_opens_with_an_underscore_is_one_token
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_each_token_of_the_word_text_meets_the_guard_once
A6: python3 -m unittest discover -s scripts/tests -p test_rust_tokens_identity.py -k test_every_rust_source_reads_the_same_tokens_with_the_guard
A7: cargo test -p deck-streak-ingest --test memory_state_lenient_bound -- --exact the_bounded_reader_parses_every_seeded_text_as_the_base_reader_does
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_tokens_agree_with_every_vector_of_their_proof
```

```red-first
A1: red at 54e308341cc8136dfd44af18aa31e3ce0191956a: FAIL: test_a_reading_that_does_not_advance_is_refused_at_its_start (test_mutation_verdict_python_kills.RustTokensEndOnEveryInput.test_a_reading_that_does_not_advance_is_refused_at_its_start) AssertionError: rust_tokens read position 1 64 times: its loop does not end
A1: green at 390a800d0cdc09ab1eee51b964dabc54fe9d077e
A2: not red: pins the token list the base already reads; its reds are rows S48001 and S48002, killed by A2
A3: not red: pins the token list the base already reads; its reds are rows S48003 and S48004 and the guard's own listed mutant, killed by A3
A4: not red: pins the token list the base already reads; its reds are rows S48005 and S48006, killed by A4
A5: not red: A3's non-vacuity control: its text meets the guard once per token, each reading past its start
A6: not red: the identity census, equal at the base by construction; it holds the guard to the release population
A7: not red: the identity of the pass bound, equal at the base by construction; its red is row S48007, killed by memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field
A8: not red: holds rust_tokens to the vectors of lean/TokenProgress
```

Each line above was read by running its one test by name, in a checkout of the commit it names,
before the pull request's one push; the pull request's own checks read every criterion again at
its head. A1 ran `Ran 1 test` at 54e30834 and failed with the stand-in's assertion above, after
one reading at position 1 had repeated until the stand-in's limit of 64 readings: the base's loop
appended the token `(1, 1)` and moved to 1 on each pass. At 390a800d it ran `Ran 1 test` and passed:
the guard raised `RuntimeError(1)` after one reading. A2, A3, A4 and A5 each ran by name at
54e30834, `Ran 1 test`, and passed; at 390a800d, 84bcb25e, c0b0c340 and 18f3893b they passed
inside the whole run of their module's classes. A6 and A7 passed at each of 54e30834, 390a800d,
84bcb25e, c0b0c340 and 18f3893b, and A8 passed at 18f3893b and at d8609712.

The counts each census printed (round protocol P8):

- A5, the non-vacuity control of A3's kill: a spy wrapping the real `rust_literal` counted 2
  readings over `"ab c1"`, and both tokens end past their start.
- A6: the positive control first, a guarded side that drops one token, read the one fixture path
  as differing over 6 base tokens; then the census printed `examined 332 rust sources` and
  `compared 360180 tokens over 332 sources` at 84bcb25e, c0b0c340 and 18f3893b, with no source
  differing (`360164` at 54e30834 and 390a800d, before the pass bound added its lines). That run
  passed four exclude pathspecs at its invocation (`added pathspecs 4`), which left 332 of the 340
  sources the `rust` class reads; under `CI` the census refuses any pathspec, so CI reads all 340.
- A7: `examined 4096 texts, 410 parsed to a state`, with no text whose state differs from the
  base reader's.
- A8: `examined 12 token progress vectors`, each text's tokens equal to its vector's and no more
  of them than the text's length.

Row S48007 was proved by hand at 84bcb25e, the head that holds the pass bound, its target's
sha256 recorded first. `at *= width;` was installed once by script in
`crates/ingest/src/memory_state.rs`, in which `at += width;` occurs once. The killer,
`memory_state_boundaries::a_bare_word_outside_a_string_drops_only_its_field`, then ran
`running 1 test` and failed at its `expect`, which found no state (`a state`, at
`crates/ingest/tests/memory_state_boundaries.rs:137:80`): the reading ended after its text's
length of passes with a wrong text, and `parse` answered none. A7 ran `running 1 test` under the
same mutant and failed too, by a panic in the mutated reader, which sliced inside a multibyte
character (`crates/ingest/src/memory_state.rs:71:30`) before the comparison ran. Both runs ended.
The target was restored by writing its saved bytes back, its sha256 compared equal, the file was
touched and rebuilt, and both targets then passed, 14 tests and 1. Rows S48001 to S48006 mutate
`scripts/mutation-verdict.py`, whose mutants are never installed outside CI: their proof is the
anchor census, each find occurring once in its target, the band's duplicate-key check, and CI's
`mutation-rows` line for each, read by its stem.

A1's green names the commit that adds the guard, and HEAD agrees with it for A1:
`git diff --quiet 390a800d HEAD -- scripts/mutation-verdict.py` exits 0, and
`git diff --numstat 390a800d HEAD -- scripts/tests/test_mutation_verdict_python_kills.py` reads
`20 0`, the 20 added lines being A8 alone. No commit between the red and the green changes a test.

## Section 9: each Rust leg's own bounded baseline (A9 to A22)

SPEC-480 section 9 (R10 to R17) adds A9 to A22. Section 9, ADR-509, section 12 of the mutation
testing schematic and the changelog fragment were committed first. The red commit holds the tests
of A9 to A12, A14 to A17 and A19 to A21 alone on top of them, with a stub of each new script that
keeps every input and runs the base's test command unbounded, in the default environment, with no
deal, and prints no package, over a `scripts/mutation-verdict.py` that still reads each report's own
`Baseline` outcome and a `.github/workflows/ci.yml` that holds no baseline step and no Timeouts
step. The test modules run only in CI, so every red is quoted from the pull request's checks over
the red commit, from the job that runs `scripts/tests`, and each red and green line joins this
section in the round after that push. A13, A18 and A22 are recorded `not red`, each with its reason.

```text
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

```red-first
A13: not red: pins three figures the build writes together; its red is row S48008, and the verdict's own constant mutants are killed by A9
A18: not red: pins the committed derive, whose code lines are proved equal to those the budget was sized by; its red is row S48028
A22: not red: pins the charge the base already holds
```

The reds were read in job 117884587210, the job that runs `scripts/tests` over the red commit 16b8763c: 11 failures, each a
test of A9 to A12, A14 to A17 or A19 to A21, and no other. The commits between the red and the greens that change a test file
are fb01db13 (SPEC-362 A6's two report-outcome cases, the old log copy of `shard_reports` and A13's class) and e6f02d15
(`CI_LEG`): neither changes an assertion of a criterion recorded red here. Each green names the commit that completes its
criterion's code, and CI reads every green over this push's tip, the one head it runs. `git diff --quiet fb01db13 HEAD --
scripts/tests/test_mutation_verdict.py scripts/mutation-verdict.py` and `git diff --quiet e6f02d15 HEAD --
scripts/tests/test_mutation_workflows.py .github/workflows/ci.yml` each exit 0 at that tip, for A9 to A12 and for A19 and A21.
For A14 to A17 and A20 the same command cannot, since fb01db13 and daa6eabd add A13's and A18's classes, imports and constants
to `scripts/tests/test_mutation_baseline.py`. There each top-level statement of that module at 7b5d68d5 and at 1be56629, its
docstring alone excepted, is byte-equal at that tip, the classes `ALegRunsItsOwnBoundedBaseline` and
`EachPackagesTimeoutsArePrinted` among them, and `git diff --quiet 7b5d68d5 HEAD -- scripts/mutation-baseline.sh` and `git
diff --quiet 1be56629 HEAD -- scripts/mutation-timeouts.sh` each exit 0.

```red-first
A9: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: 'VOID mutation-rust-shard-1: its unmutated baseline reached its 3845 s budget, so no mutant of it was judged' not found in 'mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)\nmutation: rust: VOID mutation-rust-shard-1: no report: its outcomes.json holds no cargo-mutants outcomes\nmutation: rust: cargo-mutants examined 91 (caught 91, missed 0, timeout 0), unviable 0, of 91 on the diff\nmutation: rust: missed 0: equivalent 0, unexplained 0\nmutation: rust: examined 91 by cargo-mutants and 0 by rows\nmutation: rust: verdict: VOID: 1 measurement(s) the class needs are missing\nexamined 91\n'
A9: green at fb01db13ecf2958ddcd7e1dfd22655637abe9e8b
A10: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: 'VOID mutation-rust-shard-1: its unmutated baseline failed (exit 100), so no mutant of it was judged' not found in 'mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)\nmutation: rust: VOID mutation-rust-shard-1: no report: its outcomes.json holds no cargo-mutants outcomes\nmutation: rust: cargo-mutants examined 91 (caught 91, missed 0, timeout 0), unviable 0, of 91 on the diff\nmutation: rust: missed 0: equivalent 0, unexplained 0\nmutation: rust: examined 91 by cargo-mutants and 0 by rows\nmutation: rust: verdict: VOID: 1 measurement(s) the class needs are missing\nexamined 91\n'
A10: green at fb01db13ecf2958ddcd7e1dfd22655637abe9e8b
A11: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: 'VOID mutation-rust-shard-1: it left no record of its unmutated baseline, so no mutant of it was judged' not found in 'mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)\nmutation: rust: VOID mutation-rust-shard-1: no report: its outcomes.json holds no cargo-mutants outcomes\nmutation: rust: cargo-mutants examined 91 (caught 91, missed 0, timeout 0), unviable 0, of 91 on the diff\nmutation: rust: missed 0: equivalent 0, unexplained 0\nmutation: rust: examined 91 by cargo-mutants and 0 by rows\nmutation: rust: verdict: VOID: 1 measurement(s) the class needs are missing\nexamined 91\n'
A11: green at fb01db13ecf2958ddcd7e1dfd22655637abe9e8b
A12: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: 3 != 0 : mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)
A12: green at fb01db13ecf2958ddcd7e1dfd22655637abe9e8b
A14: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: Lists differ: ['nex[26 chars] '--profile', 'mutants', 'INSTA_UPDATE=always [15 chars]S=1'] != ['nex[26 chars] '--package=a', '--package=c', '--profile', 'm[41 chars]S=0']
A14: green at 7b5d68d58c2b83efec480c92ee11b9b791472466
A15: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: None != '        PASS [   0.004s] (1/1) a tests::one\n'
A15: green at 7b5d68d58c2b83efec480c92ee11b9b791472466
A16: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: 0 != 100 :         PASS [   0.004s] (1/1) a tests::one
A16: green at 7b5d68d58c2b83efec480c92ee11b9b791472466
A17: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: 0 != 124 : ends at the signal:         PASS [   0.004s] (1/1) a tests::one
A17: green at 7b5d68d58c2b83efec480c92ee11b9b791472466
A20: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: 'Time[17 chars]over the leg reports under /tmp/tmpuxl68345/reports\n' != 'Time[17 chars]over 3 leg report(s) and 3 green baseline log([941 chars] s\n'
A20: green at 1be5662979d00fe001a6e827493293a2d8c43211
A19: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: unexpectedly None : mutation-rust has no step whose id is baseline
A19: green at e6f02d150a54915bbdc4feb36dbd6d85b7924d2d
A21: red at 16b8763c4845b6462d9f449c1c43e305deffefc2: AssertionError: 'run: bash scripts/mutation-timeouts.sh "$RUNNER_TEMP/reports"' not found in '      - name: the verdict, from every shard\'s report and the rows\' (the plan names a not-applicable case)\n        env:\n          RUST_LEG: ${{ needs.mutation-rust.result }}\n          ROWS_LEG: ${{ needs.mutation-rows.result }}\n          PYTHON_RELEASE_LEG: ${{ needs.mutation-python-release.result }}\n        run: |\n          reports="$RUNNER_TEMP/reports"\n          status=0\n          python3 scripts/mutation-verdict.py judge --plan "$reports/mutation-plan/plan.json" --class rust --shard-reports "$reports" --rows "$reports/mutation-rows/rows.json" --whole "$reports/mutation-plan/whole.json" || status=$?\n          oracle=0\n          python3 scripts/mutation-verdict.py judge --plan "$reports/mutation-plan/plan.json" --class oracle --python "$reports" --rows "$reports/mutation-rows/rows.json" --python-whole "$reports/mutation-plan/python-whole.json" || oracle=$?\n          if [ "$status" -eq 0 ]; then status=$oracle; fi\n          scripts=0\n          python3 scripts/mutation-verdict.py judge --plan "$reports/mutation-plan/plan.json" --class scripts --python "$reports" --rows "$reports/mutation-rows/rows.json" --python-whole "$reports/mutation-plan/python-whole.json" || scripts=$?\n          if [ "$status" -eq 0 ]; then status=$scripts; fi\n          legs=0\n          python3 scripts/mutation-verdict.py legs --plan "$reports/mutation-plan/plan.json" --rust-leg "$RUST_LEG" --rows-leg "$ROWS_LEG" --python-release-leg "$PYTHON_RELEASE_LEG" || legs=$?\n          if [ "$status" -eq 0 ]; then status=$legs; fi\n          exit "$status"\n\n  # StrykerJS over every web production file the diff changes, whole, so a survivor already on a\n  # touched file is the pull request\'s own (SPEC-039 R3, ADR-057 D3), dealt into three shards\n  # (SPEC-419, ADR-433). Each shard plans as the leg always has, asks StrykerJS\'s own mutation\n  # server how many mutants each planned file holds, takes its share of the files dealt whole by\n  # that count, largest first onto the lightest shard, and runs StrykerJS over its share. It keeps\n  # its record and its report under its own index whatever its outcome, and one shard\'s failure\n  # cancels no other. It restores the pnpm store and saves none.\n'
A21: green at e6f02d150a54915bbdc4feb36dbd6d85b7924d2d
```

SPEC-416 A1 after this delivery's baseline step. At push 2's tip `027905e9`, CI job 117988929169
read SPEC-416 A1 (`test_each_baseline_job_fetches_after_its_cache_restore_and_before_cargo_mutants`)
red, with the finding that in `ci.yml` job `mutation-rust` the fetch step's `if:` is not the
cargo-mutants baseline step's `if:`. Its judge, `runs_a_baseline`, took the first `cargo mutants`
step that does more than list as the leg's baseline. Since ADR-509 that is the step that skips its
own baseline, under the added condition that the leg's own baseline read green, while the fetch and
the leg's own baseline step share the plain condition.

The commit `test(ci): SPEC-416 A2 plants the leg's own baseline shapes, red` adds two planted shapes
to SPEC-416 A2 (the ADR-509 shape, and a fetch after the leg's own baseline) and one control (the
leg's own baseline script is a baseline). Each reads red against that judge, beside A1. The commit
that carries this note adds the leg's own baseline script to the steps `runs_a_baseline` counts as a
baseline, so A1 judges the fetch against the step that runs the baseline. It removes no test,
planted case or member and adds no filter. Its greens are read in CI by name at this push's tip.
