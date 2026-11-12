# SPEC-610: each mutation runner test helper writes its report into a directory of its own

- **Issue:** #833, the two test helpers that run the Python mutation runner leave a `costs.jsonl`
  in the shared system temporary directory; refers, closes nothing (it closes at the release,
  never by this pull request).
  **Context(s):** the tests of the mutation scripts (`scripts/tests/`), with no script, workflow
  or crate change.
- **Decided by:** ADR-632 (a report helper takes a temporary directory per run, removed with its
  test).
- **Schematic:** none: no component, data flow or state machine changes; the change is where two
  test helpers put a scratch file.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-610.md`. **Mutation band:** S61000-S61099. **Model:** none (section 8).
  Builds on SPEC-418 R6 and SPEC-087, and amends neither.

## 1. The problem, measured

At the dev this change starts from, two test helpers run the Python mutation runner and write its
report into the parent of the scenario tree, which is the shared system temporary directory:

- `scripts/tests/test_mutation_python.py` line 265, in `Tree.report`:
  `target = self.root.parent / f"{self.root.name}-report.json"`.
- `scripts/tests/test_mutation_python_cli_kills.py` line 136, in
  `TheRunPrintsAndWritesItsReport.report_of`:
  `target = tree.root.parent / f"{tree.root.name}-cli.json"`.

The runner opens a `costs.jsonl` beside every report and appends to it (`scripts/mutation_python.py`
line 958, SPEC-418 R6). Each helper removes only its report once it has read it (line 269 of the
first module, line 139 of the second), so the `costs.jsonl` beside the report stays: every run of
either helper appends to one shared file that outlives the run, and runs at the same time append to
the same file.

`grep -n -F 'root.parent' scripts/tests/test_mutation_python.py scripts/tests/test_mutation_python_cli_kills.py`
reads three lines: the two helper lines above, and line 144 of the second module, the plan file of
its clean-run test, which section 5 leaves out.

## 2. Requirements

- **R1.** `Tree.report` gives each run a temporary directory of its own, made per call and registered for cleanup with the test that made the tree (`Tree` keeps that test), and writes the report there: never into the parent of the scenario tree, and never inside the scenario tree.
- **R2.** `TheRunPrintsAndWritesItsReport.report_of` gives each run a temporary directory of its own, made per call and registered with its own test's cleanup, and writes the report there, with the same two exclusions.
- **R3.** Each helper returns what it returned before, in the same order, and every existing caller is unchanged.
- **R4.** `scripts/mutation_python.py` is unchanged and still writes `costs.jsonl` beside `--report` (SPEC-418 R6).

R1 and R2 move only where each helper's report goes; R3 keeps every caller as it is; R4 keeps the
runner as it is, so its costs record still sits beside whatever `--report` names, and now goes
with the helper's directory.

## 3. Acceptance criteria of SPEC-610

| id | criterion | decided by |
|---|---|---|
| A1 | `Tree.report` (R1) writes its report, and the costs record the runner puts beside it, into a directory that is neither the parent of the scenario tree nor inside it, and that directory is gone after the test's cleanup | `python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_the_tree_report_helper_writes_its_report_and_costs_into_a_directory_of_its_own` |
| A2 | `TheRunPrintsAndWritesItsReport.report_of` (R2) does the same, and still returns the exit, the output, the error output and the report text | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_cli_kills.py -k test_the_cli_report_helper_writes_its_report_and_costs_into_a_directory_of_its_own` |
| A3 | the runner (R4) still records each file's control cost beside the report before its mutants, SPEC-418 A8's existing test | `python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_files_control_cost_is_recorded_beside_the_report_before_its_mutants` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_the_tree_report_helper_writes_its_report_and_costs_into_a_directory_of_its_own
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_cli_kills.py -k test_the_cli_report_helper_writes_its_report_and_costs_into_a_directory_of_its_own
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_files_control_cost_is_recorded_beside_the_report_before_its_mutants
```

R3 is held by the existing tests of the two modules, run whole before the push and by CI on every
push, and A3 pins a behaviour the base already has, so it is recorded `not red` with its reason.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_mutation_python.py` | tests | changed: `Tree` keeps the test that made it, `Tree.report` writes into a directory of its own, and the A1 test |
| `scripts/tests/test_mutation_python_cli_kills.py` | tests | changed: `import tempfile`, `report_of` writes into a directory of its own, and the A2 test |
| `scripts/tests/test_ci_workflows.py` | tests | changed: one dynamic-site listing for the A2 test's patch of its own module |
| `scripts/mutation-rows.d/S61000-S61099.json` | mutation rows | added: S61000 and S61001 |
| `scripts/mutation_python.py` | mutation scripts | unchanged: it still writes `costs.jsonl` beside `--report` |
| `docs/specs/SPEC-610-each-mutation-runner-test-helper-writes-its-report-into-a-directory-of-its-own.md` | records | added: this SPEC |
| `docs/decisions/ADR-632-a-report-helper-takes-a-temporary-directory-per-run-removed-with-its-test.md` | records | added: its decision |
| `docs/red-first/SPEC-610.md` | records | added: the red-first record |
| `changelog.d/mutation-report-scratch-610.md` | records | added: the changelog fragment |

## 5. What this does NOT cover

- The plan file of the clean-run test: `test_a_clean_run_is_exit_zero_and_prints_the_counts_and_the_selection_it_ran`
  in `scripts/tests/test_mutation_python_cli_kills.py` also writes into the parent of the scenario
  tree, but it is a test body, not a report helper, and #833 names the two report helpers only; its
  line is unchanged.
- The runner's costs record and its readers: where `scripts/mutation_python.py` writes
  `costs.jsonl`, and what reads it, stay as SPEC-418 delivered them for #829; this SPEC changes
  neither.

## 6. Risks

- A `Tree` made by something other than a test case would have no cleanup to register the report
  directory with. Answer: every caller passes one (each `Tree` call in both modules passes the
  test case it runs in, or the test its own helper was given), the constructor already registers
  the scenario tree's own directory with that test, and the only importer of
  `scripts/tests/test_mutation_python.py` is `scripts/tests/test_mutation_python_cli_kills.py`.
- A helper's directory removed before its caller reads the report. Answer: the removal runs at the
  test's cleanup, after the helper returns, and each helper reads its report before it returns.
- The mutation runner's cost. Answer: none; both new tests fake the runner and never run
  `scripts/mutation_python.py`, and every existing caller runs it exactly as before.

## 7. What only CI proves

Rows S61000 and S61001 are proved by the `mutation-rows` job: each puts one helper's report back
into the parent of the scenario tree, and its killer must fail on that mutant.

## 8. Formal model

None. The change has no protocol, concurrency or state machine: two test helpers choose where a
scratch file goes. The formal covers name `scripts/mutation_python.py`, which is unchanged, and
neither test module.

## 9. Mutation rows

| id | target | mutant | killer |
|---|---|---|---|
| S61000 | `scripts/tests/test_mutation_python.py` | `Tree.report` keeps its own directory and its cleanup, but writes the report into the parent of the scenario tree again | `test_mutation_python.TheTreeReportHelperWritesIntoItsOwnDirectory.test_the_tree_report_helper_writes_its_report_and_costs_into_a_directory_of_its_own` (A1) |
| S61001 | `scripts/tests/test_mutation_python_cli_kills.py` | `report_of` keeps its own directory and its cleanup, but writes the report into the parent of the scenario tree again | `test_mutation_python_cli_kills.TheRunPrintsAndWritesItsReport.test_the_cli_report_helper_writes_its_report_and_costs_into_a_directory_of_its_own` (A2) |

## 10. Amendments

A later change closes a gap the first A1 and A2 left open. Each called its helper once, so a helper
that kept one directory for its tree, or for its test, and wrote every report there would have
passed them, though R1 and R2 say the directory is made per call. A1 now calls `Tree.report` twice
and A2 calls `report_of` twice, and each asserts that the two reports went into two different
directories, both gone at the test's cleanup. The helpers already made a directory per call, so no
helper changes, and the acceptance block above is unchanged: the criteria keep their names and
their commands.

The change touched these files beyond the rows above:
- `scripts/tests/test_mutation_python.py`: the A1 test calls `Tree.report` twice.
- `scripts/tests/test_mutation_python_cli_kills.py`: the A2 test calls `report_of` twice.
- `scripts/mutation-rows.d/S61000-S61099.json`: rows S61002 and S61003.
- `docs/decisions/ADR-632-a-report-helper-takes-a-temporary-directory-per-run-removed-with-its-test.md`: an amendment that weighs a with-block directory inside each helper.
- `docs/red-first/SPEC-610.md`: a paragraph on how the new assertions are shown able to fail.
- `changelog.d/report-helper-per-call-610.md`: the changelog fragment of the change.

Rows the change leaves unchanged:
- `scripts/mutation_python.py` is unchanged.

| id | target | mutant | killer |
|---|---|---|---|
| S61002 | `scripts/tests/test_mutation_python.py` | `Tree.report` keeps the directory of its first call and writes every later report there | `test_mutation_python.TheTreeReportHelperWritesIntoItsOwnDirectory.test_the_tree_report_helper_writes_its_report_and_costs_into_a_directory_of_its_own` |
| S61003 | `scripts/tests/test_mutation_python_cli_kills.py` | `report_of` keeps the directory of its first call and writes every later report there | `test_mutation_python_cli_kills.TheRunPrintsAndWritesItsReport.test_the_cli_report_helper_writes_its_report_and_costs_into_a_directory_of_its_own` |

The `mutation-rows` job proves rows S61002 and S61003, as section 7 says of S61000 and S61001.
