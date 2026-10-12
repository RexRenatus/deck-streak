# SPEC-420: the Python share is one value capped at 128, its prices are measured, and each mutant runs only the tests that execute its line

- **Issue:** #829, the Python mutation legs sized by what each file's mutants cost. This SPEC
  refers to it and closes nothing. It also refers to #825, the Python legs' fit, and #822, the
  release.
- **Context(s):** the mutation scripts (`scripts/`), their tests and fixtures, and the release
  runbook. No workflow changes and no crate changes.
- **Decided by:** ADR-434 (the share is one capped value, the price table is derived from a recorded
  run, and each mutant runs the tests its line's trace names). ADR-432 is amended insert-only.
- **Schematic:** `docs/schematics/mutation-testing-python.md` section 7, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-420.md`.
- **Mutation band:** S42000-S42099.
- **Models:** `formal/tla/EveryLegCounted`, unchanged, and a new `formal/tla/TracedSetCovers`.
- **Builds on:** SPEC-418 and SPEC-415, whose cost deal and count rule it keeps.

## 1. The problem, measured

Every figure below was read at dev `e64a778e`, or from battery run 38242806596, whose sixteen
Python legs each ran to the job's 120-minute bound.

- `git show e64a778e:scripts/mutation-verdict.py | sed -n 122p` reads
  `PYTHON_SHARD_MUTANTS, PYTHON_MAX_SHARDS = 20, 32`. The share shares its line with the count
  rule, and nothing caps it.
- `grep -n` finds the share's number spelled outside the code three times: `RELEASING.md:66-67`,
  pinned by `scripts/tests/test_rulesets.py:165-166`, and the joint lines that
  `scripts/tests/test_mutation_python_shard_binding.py:444-455` and `:670-677` pin (`python 2 of 32`,
  `python 3 of 32`, `of 225`, `of 35700`).
- `test_dispatch_shards.py`'s `test_the_ceiling_is_the_runs_job_budget` derives
  `LEG_CEILING["ci"]` from the share. Any share above 32 lowers the derived Rust ceiling below the
  pinned 193.
- `git show e64a778e:scripts/mutation-verdict.py | sed -n 129p` reads
  `PYTHON_CONTROL_SECONDS: dict[str, int] = {}`. The price table is empty.
- `mutation-verdict.py prices --costs <the run's twelve kept leg artifacts>` reads
  `examined 1017 record(s) in 12 file(s); torn 0` and prints seven figures. The largest is
  `mutation-verdict.py` at 224 s (leg 13's control, 223.66 s, job 114789331343, cancelled at its
  120-minute bound). Legs 0, 5, 6 and 14 kept no records.
- `shards` over the release listing (main...dev, 5670 mutants in 14 files) with the table filled
  refuses at 443 priced legs and 114 unpriced legs: 557, which rounds to 560. With the table empty it
  refuses at 284.
- Eight listed files no recorded leg reached hold 2272 mutants: `mutation_rows.py` 1018,
  `mutation_python.py` 599, `testflight_age.py` 236, `generate.py` 143, `threat_model.py` 116,
  `public-scrub.py` 104, `web-engine-size.py` 53 and `write-back-issues.py` 3.
- Each mutant runs every test module its file maps to, less the byte readers
  (`scripts/mutation_python.py`, `Judge.file`). `mutation-verdict.py`'s 224 s per mutant is spread
  over five modules: 38.3, 24.7, 20.3, 16.5 and 0.2 per cent, as read by a local per-test timing of
  the control.
- A local trace of the sentinel run, scoping each mutant to the tests that execute its line, prices
  the release at 274 legs (280 rounded). Counted per file, with a whole control and sentinel in every
  leg, it prices them at 304.
- That trace sampled 55 mutants across every listed file it could run, and all 55 read the same
  outcome scoped and whole.
- The Python legs run no control mutant.

## 2. Requirements

R1. `scripts/mutation-verdict.py` spells the share once, on its own line, as
`PYTHON_MAX_SHARDS = 32`. The count rule is `PYTHON_SHARD_MUTANTS = 20`, on a line of its own. The
share's value is unchanged.

R2. `PYTHON_SHARE_CAP = 128` is a module constant. `python_share_refused()` returns, and `main`'s
`shards` arm prints before any deal, then returns `EXIT_FAIL` on:

`mutation: shards: REFUSED: the python share PYTHON_MAX_SHARDS = <n> is outside 1..<cap> (PYTHON_SHARE_CAP); a share above the cap is never dealt`

It does so whenever the share is not a whole number from 1 to the cap. The whole-plan refusal by
name above the share is unchanged.

R3. No reader spells the share's number:
- the shard-binding tests that pin the plan's lines patch the share and `LEG_CEILING` to literals
  of their own;
- section 2 of `RELEASING.md` names `PYTHON_MAX_SHARDS` and `PYTHON_SHARE_CAP` and spells neither
  number, and the SPEC-415 runbook test's phrases follow;
- `.github/workflows/ci.yml` is unchanged, since its Python matrix is the plan's output.

A test pins the cap at 128, and pins the share as a multiple of 8 from 8 to the cap.

R4. `PYTHON_CONTROL_SECONDS` holds exactly the lines `mutation-verdict.py prices --costs <dir>` prints
over the records of battery run 38242806596. Those records are committed under
`scripts/tests/fixtures/mutation-prices-38242806596/` in the layout the run's artifacts download in.
A test re-derives the table from them byte for byte.

R5. A listed file the table does not price keeps the count rule of 20 a leg, counted against the
share (SPEC-418, unchanged). It is never dealt as free.

R6. In every `run`, each file of the leg has its control, bound and sentinel taken, and its control
record written, before any mutant of the leg runs. In `--plan` mode, its trace is taken in the same
pass.

R7. In a `run --plan` leg, the runner traces each file's sentinel run, in-process through
`sys.monitoring` `LINE` events reset at each test's start, and in child processes through a scratch
`sitecustomize`. Each process counts the processes it starts from the interpreter's
process-creation audit events and writes one record when it exits.

A mutant runs only the tests whose processes executed its line, in the suite's order. It runs the
whole suite instead in each of these cases:
- its name says `in <module>`;
- its line ran at import or outside any test;
- no traced test executed its line;
- the file's trace failed or is partial;
- its scoped run ran fewer tests than its set names.

A test joins every scoped set of its file in each of these cases:
- a process it started left no record;
- it replaced a process image with `os.exec`;
- it left a thread running past its end.

A scoped pass is confirmed by the whole suite before the mutant reads SURVIVED. Byte readers stay
excluded from both the scoped and the whole runs.

R8. `run --all`, the scheduled battery's mode, runs every mutant on the whole suite and takes no
trace.

R9. Every `run` whose root holds `scripts/tests/fixtures/mutation-python-control/` runs its three
named mutants first. Before the leg's files, it expects them to read KILLED, KILLED and SURVIVED. Any
other outcome writes `control: <mutant> read <outcome>, expected <outcome>` as the `void` of every
file entry of the report.

The control writes no cost record. A root without the fixture records `control: absent`. The report
carries `control` either way.

R10. Each mutant line of `costs.jsonl` carries:
- `scope`, either `lines` or `whole`;
- `why`, when the scope is whole: one of `module`, `import`, `unreached`, `trace`, `ran`,
  `survivor` or `all`;
- `tests`, the number of tests it ran.

`prices` over records without those fields prints what it printed before.

R11. These stay unchanged:
- the 1800 s legs;
- `LEG_CEILING` (`{"ci": 193, "battery": 234}`, the Rust share);
- every listed mutant examined;
- the whole-plan refusal by name above the share;
- no leg-time raise;
- `ci.yml` and `mutation-weekly.yml`;
- the spans `formal/tla/EveryLegCounted` covers;
- `mutation-verdict.py`'s report readers.

## 3. Acceptance criteria of SPEC-420

| id | criterion | decided by |
|---|---|---|
| A1 | `PYTHON_SHARE_CAP` is 128, and `PYTHON_MAX_SHARDS` is a multiple of 8 from 8 to the cap; the test never names the share's number | `test_mutation_python_shard_binding.py` `test_the_share_is_one_value_capped_at_128_in_the_tree` |
| A2 | `shards` refuses before any deal: with the share patched to 136, and again to 0, it prints R2's line and returns `EXIT_FAIL`; with the share patched to 128, the same fixture plan is dealt | `test_mutation_python_shard_binding.py` `test_a_share_outside_the_cap_refuses_the_plan_whole_by_name` |
| A3 | the share reaches every printed line: with the share patched to 40 and `LEG_CEILING["ci"]` to 193, the fixture prints `python legs 2 of ceiling 40` and `run legs 3 of 233: python 2 of 40, rust 1 of 193; projected 93 runner-minute(s) of 35940` | `test_mutation_python_shard_binding.py` `test_every_printed_share_reads_the_one_value` |
| A4 | section 2 of `RELEASING.md` names `PYTHON_MAX_SHARDS` and `PYTHON_SHARE_CAP`, and holds neither `python legs N of ceiling <digits>` nor `more than <digits> Python legs` | `test_rulesets.py` `test_the_runbook_names_the_python_share_by_its_name_never_its_number` |
| A5 | `prices --costs` over the committed records prints exactly the seven `PYTHON_CONTROL_SECONDS` lines of `scripts/mutation-verdict.py`, byte for byte, then `examined 1017 record(s) in 12 file(s); torn 0` last | `test_mutation_python_shard_binding.py` `test_the_price_table_is_the_prices_verbs_output_over_the_recorded_run` |
| A6 | over the filled table, 41 mutants in a path the table does not price (`scripts/web_mutation_shards.py`), listed beside a priced file, are dealt as 3 unpriced legs of at most 20 mutants, counted against the share | `test_mutation_python_shard_binding.py` `test_a_file_the_filled_table_does_not_price_keeps_the_count_rule` |
| A7 | a two-file leg's `costs.jsonl` holds both files' control lines before its first mutant line | `test_mutation_python.py` `test_every_files_control_is_recorded_before_any_mutant_of_the_leg` |
| A8 | over the scope fixture, each mutant's record names exactly the tests the fixture's own table defines as executing its line | `test_mutation_python.py` `test_each_mutant_runs_only_the_tests_that_execute_its_line` |
| A9 | a mutant named `in <module>`, one on a class body's line and one on a `setUpClass` line each run the whole suite, recorded `why` `module`, `import` and `import` | `test_mutation_python.py` `test_a_line_run_at_import_or_outside_a_test_runs_the_whole_suite` |
| A10 | a mutant on a line no test executes runs the whole suite, recorded `why: unreached`, and is never skipped | `test_mutation_python.py` `test_a_line_no_traced_test_executes_runs_the_whole_suite` |
| A11 | a mutant on a line only a Python child process executes is killed, with the child's test in its scoped set by the child's record | `test_mutation_python.py` `test_a_line_only_a_child_process_executes_keeps_that_test` |
| A12 | a test whose process left no record (through `sh -c`, or a child ended by `os._exit`), a test whose child calls `os.execv`, and a test that leaves a thread running each join every scoped set of the file | `test_mutation_python.py` `test_a_test_whose_process_left_no_record_joins_every_scoped_set`, `test_a_test_whose_child_execs_joins_every_scoped_set`, `test_a_test_that_leaves_a_thread_running_joins_every_scoped_set` |
| A13 | when the fixture's checks take the monitoring tool id first, every mutant of the file runs whole, recorded `why: trace`, and none is skipped | `test_mutation_python.py` `test_a_failed_or_partial_trace_runs_every_mutant_of_its_file_whole` |
| A14 | a scoped set naming a test the runner cannot load runs whole, recorded `why: ran` | `test_mutation_python.py` `test_a_scoped_run_that_ran_fewer_tests_than_its_set_reruns_whole` |
| A15 | a mutant that only a test reading the target's text kills, on a line another test executes, reads KILLED, recorded `why: survivor` | `test_mutation_python.py` `test_a_scoped_survivor_is_confirmed_by_the_whole_suite` |
| A16 | `run --all` over the scope fixture runs every mutant whole and takes no trace, recorded `why: all` | `test_mutation_python.py` `test_the_battery_runs_every_mutant_against_the_whole_suite` |
| A17 | a leg whose control mutant reads otherwise than expected writes R9's `void` into every file entry, which the verdict reads by name; the repository holds the control fixture, whose listing is exactly its three named mutants | `test_mutation_python.py` `test_a_leg_whose_control_reads_wrong_voids_every_file_by_name`, `test_the_repository_holds_the_leg_control_and_its_three_mutants` |
| A18 | each mutant line of `costs.jsonl` carries `scope`, `why` when whole, and `tests`; `prices` over A5's records is unchanged | `test_mutation_python.py` `test_each_mutant_record_names_its_scope_and_its_tests` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_share_is_one_value_capped_at_128_in_the_tree
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_share_outside_the_cap_refuses_the_plan_whole_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_every_printed_share_reads_the_one_value
A4: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_python_share_by_its_name_never_its_number
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_price_table_is_the_prices_verbs_output_over_the_recorded_run
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_file_the_filled_table_does_not_price_keeps_the_count_rule
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_every_files_control_is_recorded_before_any_mutant_of_the_leg
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_mutant_runs_only_the_tests_that_execute_its_line
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_line_run_at_import_or_outside_a_test_runs_the_whole_suite
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_line_no_traced_test_executes_runs_the_whole_suite
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_line_only_a_child_process_executes_keeps_that_test
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_test_whose_process_left_no_record_joins_every_scoped_set
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_test_whose_child_execs_joins_every_scoped_set
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_test_that_leaves_a_thread_running_joins_every_scoped_set
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_failed_or_partial_trace_runs_every_mutant_of_its_file_whole
A14: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_scoped_run_that_ran_fewer_tests_than_its_set_reruns_whole
A15: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_scoped_survivor_is_confirmed_by_the_whole_suite
A16: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_the_battery_runs_every_mutant_against_the_whole_suite
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_leg_whose_control_reads_wrong_voids_every_file_by_name
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_the_repository_holds_the_leg_control_and_its_three_mutants
A18: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_mutant_record_names_its_scope_and_its_tests
```

A3, A6 and A16 are not red first, because the base already reads the share at call time, keeps the
count rule, and runs every mutant whole. `docs/red-first/SPEC-420.md` records each as `not red`,
with that reason. Every other criterion is red at the red commit, on an assertion of its behaviour.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | mutation scripts | changed: line 122 split into `PYTHON_SHARD_MUTANTS = 20` and `PYTHON_MAX_SHARDS = 32`; added `PYTHON_SHARE_CAP = 128` and `python_share_refused()`; `main`'s `shards` arm refuses first; `PYTHON_CONTROL_SECONDS` filled by the `prices` verb |
| `scripts/mutation_python.py` | mutation scripts | changed: control first in `run`; the sentinel's trace, the scoped sets and their fallbacks in `Judge`; the copied runner's tracer and its selection of named tests; `child_trace()` and the scratch `sitecustomize`; the leg control; the mutant record's `scope`, `why` and `tests` |
| `scripts/tests/test_mutation_python_shard_binding.py` | mutation tests | changed: A1, A2, A3, A5 and A6; the two pinned-line tests patch the share and `LEG_CEILING` |
| `scripts/tests/test_mutation_python.py` | mutation tests | changed: A7 to A18 |
| `scripts/tests/test_rulesets.py` | runbook tests | changed: A4; SPEC-415's runbook test names the share instead of its number |
| `scripts/tests/test_ci_workflows.py` | read census | changed: `DYNAMIC_IMPORTS` names the two import sites the fixture checks add, each with its count; the new tests add no `NOT_WORKFLOW_READS` read |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-1/costs.jsonl` | fixture | added: leg 1's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-2/costs.jsonl` | fixture | added: leg 2's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-3/costs.jsonl` | fixture | added: leg 3's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-4/costs.jsonl` | fixture | added: leg 4's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-7/costs.jsonl` | fixture | added: leg 7's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-8/costs.jsonl` | fixture | added: leg 8's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-9/costs.jsonl` | fixture | added: leg 9's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-10/costs.jsonl` | fixture | added: leg 10's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-11/costs.jsonl` | fixture | added: leg 11's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-12/costs.jsonl` | fixture | added: leg 12's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-13/costs.jsonl` | fixture | added: leg 13's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-prices-38242806596/mutation-python-shard-15/costs.jsonl` | fixture | added: leg 15's kept `costs.jsonl`, in the downloaded layout |
| `scripts/tests/fixtures/mutation-python-scope/scope_target.py` | fixture | added: a target whose lines are executed in-process, in a child, at import, in a fixture, by no test, and read as text |
| `scripts/tests/fixtures/mutation-python-scope/scope_checks.py` | fixture | added: its checks, with the table of which test executes which line |
| `scripts/tests/fixtures/mutation-python-control/control_target.py` | fixture | added: the leg control's target |
| `scripts/tests/fixtures/mutation-python-control/control_checks.py` | fixture | added: the leg control's checks |
| `RELEASING.md` | release runbook | changed: section 2 names `PYTHON_MAX_SHARDS` and its cap rather than the number, and names the scoped test set and the leg control |
| `formal/tla/TracedSetCovers/TracedSetCovers.tla` | formal model | added: the trace records' model |
| `formal/tla/TracedSetCovers/MCTracedSetCovers.cfg` | formal model | added |
| `formal/tla/TracedSetCovers/witness/NoSpawnCount.cfg` | formal model | added |
| `scripts/mutation-rows.d/S42000-S42099.json` | mutation rows | added: S42000-S42028 |
| `scripts/mutation-rows.d/S08700-S08799.json` | mutation rows | changed: `S08704-A-BYTE-READER-IS-NEVER-A-KILLER` alone is re-anchored, on the line where `Judge.judge` reads the byte readers; its killer is unchanged (RULED 1421) |
| `scripts/mutation-rows.d/S41500-S41599.json` | mutation rows | changed: `S41506-THE-RUNBOOK-NAMES-THE-LEG-SIZE-AS-THE-CEILING` alone is re-anchored, on section 2's `python legs N of ceiling S`; its killer is unchanged (RULED 1421) |
| `docs/specs/SPEC-420-the-python-share-is-one-value-capped-at-128-its-prices-are-measured-and-each-mutant-runs-only-the-tests-that-execute-its-line.md` | spec | added |
| `docs/decisions/ADR-434-the-python-share-is-one-capped-value-the-price-table-is-derived-from-a-recorded-run-and-each-mutant-runs-the-tests-its-line-traced.md` | decision | added |
| `docs/decisions/ADR-432-the-python-mutation-legs-are-dealt-by-each-files-measured-control-cost-under-the-runs-fixed-leg-shares.md` | decision | changed: one insert-only amendment |
| `docs/schematics/mutation-testing-python.md` | schematic | changed: section 7, insert-only |
| `docs/red-first/SPEC-420.md` | red-first record | added |
| `changelog.d/python-scope-420.md` | changelog | added |

## 5. What this does NOT cover

- It raises no share. The owner signs the value, and the run's job budget at a share above 32 waits
  on the Rust share or a per-matrix reading of the job limit (#825).
- It adds no second, scoped price figure. The plan prices each mutant at its file's control until a
  scoped run is recorded and a second figure is decided (#825).
- It does not plan the release. The release's Python legs read 557, and the plan refuses them by
  name at any share up to the cap (#822).
- It re-runs no battery leg for costs, and it prices no file the recorded run did not reach (#829).
- It changes no leg time, no Rust share, no workflow and no battery order. The scheduled battery's
  legs still stop at their bound (#825).

## 6. Risks

- **A test whose outcome depends on which tests ran before it.** A scoped kill could differ from the
  whole suite's. Detected by the battery, which runs every mutant whole and would show that mutant
  surviving.
- **A process started by a route the audit events do not see, such as a native extension calling
  `fork`.** If it executes a line and leaves no record, no count sees it. Detected by the battery's
  whole-suite runs. The leg control proves the counted routes.
- **A test that inspects `sys.monitoring`, the `sitecustomize`, or the environment.** It may behave
  differently under the trace. A trace that fails or is partial runs the file whole (A13), and a
  changed outcome shows as a sentinel failure, which already excludes the test as a byte reader.
- **This delivery's own Python listing could exceed the in-tree share.** Its own plan would then
  refuse it. Detected by the plan line of its run, which the build reads before it asks to land.
- **A refresh replaces the fixture's records.** The test re-derives the table from whichever records
  are committed, so a refresh that changes one without the other goes red.
- **A scoped run's fixed start-up cost, about half a second, exceeds a tiny file's whole suite.** The
  cost is time only, never a verdict.

## 7. What only CI proves

- Every red and green of A1-A18. No test module of this delivery runs outside CI.
- Each leg's control outcomes, in the report's `control`.
- The plan line of this delivery's own run, within the share at 32.
- The `formal/tla/TracedSetCovers` check, and `formal/tla/EveryLegCounted` reading no `STALE` cover.
- The release's plan refusal, read when #822 re-runs. This delivery does not run it.

## 8. Formal model

**`formal/tla/EveryLegCounted`: unchanged.** It covers `shards`, `whole_reports`, `partition`,
`examined_sum`, `population_gaps` and `python_by_cost` in `scripts/mutation-verdict.py`, and
`planned_leg` in `scripts/mutation_python.py`. This delivery edits none of those spans:
- the cap's refusal sits in `main`;
- the table and the constants are module constants;
- `planned_leg` is untouched.

So no digest moves. CI's formal check reading no `STALE` line for the entry confirms it.

**`formal/tla/TracedSetCovers`: new, because the trace records are an interleaving surface.**
- Actors: the runner's process, each test's child and grandchild processes, and the judge.
- Shared state: the scratch record directory.
- Check then act: the judge reads the directory once, after the runner exits, while a process that
  outlived it may still write.

The model has one test, two target lines, a root and up to three descendants. They spawn, execute a
line, write a record and exit in any order, and the judge reads once.

Property `NoExecutorDropped`: for each line, every test any of whose processes executed it is in
that line's scoped set, or the line runs whole.

The entry covers `scripts/mutation_python.py` anchors `child_trace` (the record's writer) and
`scoped_sets` (its reader). Its witness, `witness/NoSpawnCount.cfg`, switches off the per-parent
count. TLC must then find a child that executed a line and exited with no record while its test was
dropped. A `Finished` step is enabled only once the judge has read.

## 9. Mutation rows

Each row's find is written by the build on the line it lands, and occurs exactly once at its
target.

| row | behaviour | mutant | killer |
|---|---|---|---|
| S42000 | the cap is 128 | `PYTHON_SHARE_CAP = 128` to `= 136` | A1 |
| S42001 | the share is a multiple of 8 | `PYTHON_MAX_SHARDS = 32` to `= 36` | A1 |
| S42002 | the count rule's 20 is its own value | `PYTHON_SHARD_MUTANTS = 20` to `= 21` | the SPEC-415 count-rule test the build names |
| S42003 | a share above the cap is refused | the cap comparison widened by 8 | A2 |
| S42004 | a share below 1 is refused | the lower bound `< 1` to `< 0` | A2 |
| S42005 | the refusal fails the plan | the arm's `EXIT_FAIL` to `EXIT_OK` | A2 |
| S42006 | `main` asks before dealing | the call to `python_share_refused()` replaced by `None` | A2 |
| S42007-S42013 | each of the seven table figures | the figure plus 1, one row per line | A5 |
| S42014 | every control comes before any mutant | the mutant pass moved into the control pass | A7 |
| S42015 | a module-level mutant runs whole | its name test disabled | A9 |
| S42016 | a line run at import or outside a test runs whole | the import-line rule disabled | A9 |
| S42017 | an unreached line runs whole | the empty-set rule disabled | A10 |
| S42018 | child records are read | the child records left out of the merge | A11 |
| S42019 | an unrecorded process joins every set | the per-parent count off by one | A12 |
| S42020 | an exec joins every set | the exec rule disabled | A12 |
| S42021 | a running thread joins every set | the thread rule disabled | A12 |
| S42022 | a failed trace runs the file whole | the trace-failure rule disabled | A13 |
| S42023 | a short scoped run reruns whole | the ran-count comparison disabled | A14 |
| S42024 | a scoped survivor is confirmed whole | the confirmation skipped | A15 |
| S42025 | the battery stays whole | `--all` given the scoped path | A16 |
| S42026 | a wrong control voids every file | the void left unwritten | A17 |
| S42027 | the control's survivor arm | the expected SURVIVED read as KILLED | A17 |
| S42028 | the record names its scope | the `scope` field dropped | A18 |
