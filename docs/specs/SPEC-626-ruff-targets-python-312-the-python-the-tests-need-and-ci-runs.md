# SPEC-626: ruff targets Python 3.12, the Python the tests need and CI runs

- **Issue:** none; ruff's target lags the Python the tests need.
  **Context(s):** `ruff.toml` and its test (`scripts/tests/`); no crate, workflow or gate stage
  changes.
- **Decided by:** ADR-648 (the target, where it is set, and the test and row that hold it).
- **Schematic:** none; the target changes no component, data flow or state.
- **Status:** delivered by the pull request that adds this file, with its test and
  `docs/red-first/SPEC-626.md`. **Mutation band:** S62600-S62699, one row.

## 1. The problem, measured

- **ruff reads the tree as Python 3.11 code.** At the development branch's commit `7bd6923`,
  `ruff.toml` line 9 reads `target-version = "py311"`, the file's only target line. Its `[lint]`
  table sets only `unfixable`, so ruff applies its default rule set.
- **The tests need Python 3.12.** The test-module census in `scripts/tests/test_ci_workflows.py`,
  in its function `bindings()`, reads `ast.TypeVar`, `ast.ParamSpec` and `ast.TypeVarTuple`,
  syntax-tree classes that Python 3.12 added and that the `ast` module of Python 3.11 lacks.
- **CI runs Python 3.12.** `git grep -c -i -E 'setup-python|python-version' 7bd6923 -- .github`
  finds no file: no workflow sets up a Python or names its version, so the jobs of the `ci`
  workflow run the runner image's own `python3`, which is Python 3.12. The census above runs in
  that workflow's `hygiene` job. This claim covers the jobs of the `ci` workflow alone.
- **The new target changes nothing ruff reports.** ruff examines 176 files of the tree at
  `7bd6923`: `ruff.toml` excludes 34 tracked Python files, and two more are excluded by name when
  the tree is walked. Over those files, each command run once with `--target-version py311` and
  once with `--target-version py312`:
  - `ruff check --no-cache`, with the tree's own settings, reads `All checks passed!` at both
    targets;
  - `ruff check --no-cache --select ALL` reads `Found 27488 errors.` at both targets, and the two
    outputs are byte-identical;
  - `ruff format --no-cache --diff` gives byte-identical output at both targets, each naming the
    same 2 files that would be reformatted and 174 already formatted, neither of the 2 a file this
    delivery touches.

  So the new target raises no finding and changes no formatting, and the cure is the one line.

## 2. Requirements

- **R1. Target.** `ruff.toml` line 9 reads `target-version = "py312"`; no other byte of the file
  changes.
- **R2. The test.** `scripts/tests/test_ruff_target.py` holds one test,
  `test_ruff_targets_python_312` in the class `RuffTarget`, which parses the committed `ruff.toml`
  with `tomllib` and asserts that its top-level `target-version` equals `py312`. It reads one file
  and starts no process, names no dunder but `__name__` and no builtin, and loads no code, so the
  census registers need no line.
- **R3. No lint.** ruff with the tree's own settings reports no finding at the new target, and
  the target changes neither ruff's findings under every rule nor its formatting of any file it
  examines: section 1's commands, run again at the delivery's head, give the same results.
- **R4. Mutation.** One row, S62600, in `scripts/mutation-rows.d/S62600-S62699.json`, sets the
  target line of `ruff.toml` back to py311, and A1 kills it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the committed `ruff.toml` targets `py312` (R1, R2) | `test_ruff_target.py` `test_ruff_targets_python_312` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ruff_target.py -k test_ruff_targets_python_312
```

A1 reads red at the red commit, which adds the test and leaves `ruff.toml` as the development
branch holds it: the assertion fails with `AssertionError: 'py311' != 'py312'`, an assertion and
not an error. It reads green once line 9 names py312, with the test unchanged. R3 is decided by
section 1's commands at the delivery's head, and R4 by the mutation rows job of CI, which proves
row S62600 against A1.

## 4. File manifest

| path | part | change |
|---|---|---|
| `ruff.toml` | lint settings | changed: line 9 alone, the target from py311 to py312 (R1) |
| `scripts/tests/test_ruff_target.py` | lint settings tests | added: the target test (R2) |
| `scripts/mutation-rows.d/S62600-S62699.json` | lint settings tests | added: one mutation row (R4) |
| `docs/specs/SPEC-626-ruff-targets-python-312-the-python-the-tests-need-and-ci-runs.md` | docs | added: this delivery's SPEC |
| `docs/decisions/ADR-648-ruff-targets-python-312-and-a-test-holds-the-target.md` | docs | added: the target, where it is set, and what holds it |
| `docs/red-first/SPEC-626.md` | docs | added: the red-first record, with its red and green lines |
| `changelog.d/ruff-target-py312-626.md` | docs | added: the changelog fragment |

## 5. What this does NOT do

- It does not change the gate's Python floor: `scripts/check.sh`, in its `need_python` step, asks
  for Python 3.11 or later, and the README's prerequisite line names 3.11, though the census needs
  3.12. Raising the floor is a separate change, since the floor is the gate's own refusal.
- It adds no ruff run to CI or to the gate; that is a separate change.
- It selects no rule and changes no exclude of `ruff.toml`.
- It changes no workflow and pins no Python in CI.

## 6. Risks

- **A later rule selection that reads the target.** ruff's version-gated rules read the target, so
  a rule selection added later can raise findings at py312 that it would not raise at py311; the
  ruff run of the change that adds the selection reads them.
- **A runner whose Python is older than 3.12.** ruff would then read the tree as newer code than
  the runner runs; the census already fails on such a runner, so the `hygiene` job reads red.
- **A developer interpreter at 3.11.** The gate's floor still admits it (section 5); the census
  reads syntax-tree classes that 3.11 lacks, so the gap shows at the first local run of
  `scripts/tests` under it.

## 7. Mutation rows

| row | file | mutant | killer |
|---|---|---|---|
| `S62600` | `ruff.toml` | the target line names py311 again | A1 |
