# SPEC-637: the bench test module ends with its main guard, so a script run reaches every class

- **Issue:** none; a script run of the bench test module stops before its last class.
  **Context(s):** `scripts/tests/test_perf_benches.py` and its new guard test; no crate, bench,
  bound, workflow or gate stage changes.
- **Decided by:** ADR-653 (where the guard stands, and the test and row that hold it there).
- **Schematic:** none; moving a guard changes no component, data flow or state.
- **Status:** delivered by the pull request that adds this file, with its test and
  `docs/red-first/SPEC-637.md`. **Mutation band:** S63700-S63799, one row.

## 1. The problem, measured

- **The guard stands above the last class.** At the development branch's commit `36efa13`,
  `scripts/tests/test_perf_benches.py` holds 29 top-level statements, and the 21st is its
  `if __name__ == "__main__":` guard, which calls `unittest.main()`. Eight statements follow it:
  the bound table, five module constants, the bound helper `required_bound` and the class
  `EachWebBenchPassesItsOwnBound`, which holds SPEC-618's bound criterion,
  `test_each_web_bench_passes_its_own_measured_bound`.
- **A script run stops at the guard.** Run as a script, the module reaches the guard with only
  the class `TheBenchesAreInTheTree` defined: `unittest.main()` runs its five tests and exits, so
  the class below the guard is never defined and its test never runs. Filtered by `-k` to that
  test, such a run finds no test, prints `NO TESTS RAN` and exits with status 5.
- **Discovery runs every class.** `python3 -m unittest discover` imports the module whole, so the
  guard does not fire and every class is collected. The `hygiene` job of CI and SPEC-618's
  acceptance line run the module that way, so the bound criterion's record stands.
- **Nothing cites a line of the module.** No mutation row targets the module, and no document of
  the tree cites one of its lines by number, so moving the guard moves no row find and no citation.
- **Four other modules hold a class after their guard:** `test_ci_workflows.py`,
  `test_coverage_report.py`, `test_deploy_templates.py` and `test_mutation_python_shard_binding.py`,
  all in `scripts/tests`. They stay as they are (section 5).

## 2. Requirements

- **R1. The guard closes the module.** `scripts/tests/test_perf_benches.py` holds exactly one
  `if __name__ == "__main__":` guard, and it is the module's last top-level statement: the guard's
  two lines, and the two blank lines below them, leave their place above the bound table, and the
  same two lines close the module. No other line of the module's tests, tables, constants or bounds
  changes.
- **R2. The guard test.** The module gains `import ast`, the path constant `THIS_MODULE`, the
  helper `is_main_guard` and one test, `test_the_main_guard_is_the_last_statement_after_every_class`
  in the class `TheScriptRunReachesEveryClass`. The test parses the committed module with `ast`,
  refuses a count of zero guards or zero classes, and asserts that no class follows the first guard
  and that the module's one guard is its last statement. It reads one file and starts no process,
  names no dunder but `__name__` and no builtin outside the census's benign list, and loads no
  code, so the census registers need no line.
- **R3. A script run reaches every class.** Run as a script, the module runs every test it
  defines, the bound criterion's test included.
- **R4. Mutation.** One row, S63700, in `scripts/mutation-rows.d/S63700-S63799.json`, puts a guard
  above the bound table again, and A1 kills it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the module's one main guard is its last statement, after every class (R1, R2, R3) | `test_perf_benches.py` `test_the_main_guard_is_the_last_statement_after_every_class` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_perf_benches.py -k test_the_main_guard_is_the_last_statement_after_every_class
```

A1 reads red at the red commit, which adds the test below the guard and leaves the guard where the
development branch holds it: the assertion fails on the two classes below the guard, an assertion
and not an error. It reads green once the guard closes the module, with the test unchanged. R3 is
decided by A1, which holds the guard last, and by one script run of the module at the delivery's
head, `python3 scripts/tests/test_perf_benches.py -v -k
test_each_web_bench_passes_its_own_measured_bound`, which names that test, reads `ok` and exits 0;
at the red commit the same run finds no test, prints `NO TESTS RAN` and exits with status 5. R4 is
decided by the mutation rows job of CI, which proves row S63700 against A1.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_perf_benches.py` | bench tests | changed: the guard closes the module (R1), and the guard test (R2) |
| `scripts/mutation-rows.d/S63700-S63799.json` | bench tests | added: one mutation row (R4) |
| `docs/specs/SPEC-637-the-bench-test-module-ends-with-its-main-guard-so-a-script-run-reaches-every-class.md` | docs | added: this delivery's SPEC |
| `docs/decisions/ADR-653-the-main-guard-is-the-last-statement-of-the-bench-test-module-and-a-test-holds-it.md` | docs | added: where the guard stands, and what holds it there |
| `docs/red-first/SPEC-637.md` | docs | added: the red-first record, with its red and green lines |
| `changelog.d/perf-bench-main-guard-637.md` | docs | added: the changelog fragment |

## 5. What this does NOT do

- It changes no test, assertion, table, constant or bound the module already holds, and no bench
  file.
- It moves no other module's guard: the four modules section 1 names each hold a class after their
  guard, and each move is a change of its own.
- It adds no census over every module of `scripts/tests`; such a census would read red on those
  four modules until they move (ADR-653, option (c)).
- It changes no workflow, gate stage or test runner: discovery still runs every module.

## 6. Risks

- **A class appended below the guard again.** A later change that appends a class below the guard
  reads red at A1, so the guard stays last.
- **A second guard.** A guard added above a class reads red at A1; row S63700 is that mutant.
- **A renamed module.** A1 parses the committed text by its path constant; a rename that leaves the
  constant behind reads an error at A1, never a pass.

## 7. Mutation rows

| row | file | mutant | killer |
|---|---|---|---|
| `S63700` | `scripts/tests/test_perf_benches.py` | a guard stands above the bound table again | A1 |
