# SPEC-087: the guard scripts and the parity oracle prove their tests kill generated mutants

- **Wave:** none of its own: #218 and #219 carry no wave label, and the campaign that clears the
  whole population's standing survivors is #322 (wave 3). **Issue:** #218 and #219; the delivery
  closes both. **Context(s):** `repo` (`scripts/`, `tools/parity-oracle/`, `.github/`, `docs/`).
  **Mutation band:** `S08700-S08799`.
- **Decided by:** ADR-073 (this SPEC's: the repository's Python is mutated by a runner of its own,
  restored by digest), ADR-057 (mutation testing on the diff and weekly; its D1 amended by ADR-073),
  ADR-070 (the equivalence record excuses exactly a recorded mutant), ADR-016 (planned SPECs),
  ADR-034 (`dev` and `main`) and ADR-059 (public text).
- **Prerequisites:** SPEC-039 (the mutation jobs, the verdict, the rows and their census) and the
  machinery SPEC-057's first delivery built (the equivalence record, its census, `table`, and the
  weekly battery's `package` input).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-087.md` (ADR-016).

## 1. The problem, measured

SPEC-039 generates mutants for the Rust crates (cargo-mutants) and the Mini App (StrykerJS), and
none for Python. Two Python populations are judged by rows alone:

- `scripts/<name>.py`, the gate's own guard scripts. At `dev` 26263de they are six files:
  `mutation-verdict.py`, `mutation_rows.py`, `public-scrub.py`, `create-issues.py`,
  `audit-web-verdict.py` and `write-back-issues.py`. They decide verdicts: the scrub refuses a
  leak, the web audit's verdict grades the audit, and the mutation runner and verdict decide what
  every other mutant means. SPEC-039 R2 keeps them outside production code (#218).
- `tools/parity-oracle/generate.py`, production code under SPEC-039 R2, whose invariants are rows
  only because mutmut 3.8.0 did not fit it (SPEC-039 section 1, R1; #219).

A row is one mutant a person chose (ADR-057 D1, SPEC-039 R8). Section 1.2 measures what that
leaves out.

### 1.1 Three tools on four targets

Measured on `dev` 26263de in a scratch export of the tree, with `PYTHONDONTWRITEBYTECODE=1`, each
tool at a pinned version: mutmut 3.8.0 (the current release), cosmic-ray 8.7.0, and a prototype of
the runner ADR-073 chooses (an `ast` operator set, installed in place, restored and checked by
sha256, the file's tests run in a child that names every failing test). The four targets span the
shapes the tests load: `scripts/public-scrub.py` (loaded by path under a module name of the test's
own, and run as a child), `scripts/audit-web-verdict.py` (a command with `main()`, run only as a
child), `scripts/mutation_rows.py` (imported by name, run as a child, and read as text by its own
rows' census) and `tools/parity-oracle/generate.py` (loaded by path as `parity_generate`).

| tool | target | runs as the tree stands | listed | killed | survived | timed out | neither |
|---|---|---|---|---|---|---|---|
| mutmut | `public-scrub.py` | no: stops, the test loads the file under a name mutmut does not key | 684 | 61 | 31 | 0 | 592 "no tests", with the test changed to load the file under mutmut's name |
| mutmut | `audit-web-verdict.py` | no: its clean run fails, because the child `python3` imports the rewritten copy and cannot import mutmut | 0 run | | | | |
| mutmut | `mutation_rows.py` | no: its clean run fails, because the rewritten copy repeats every function and the rows census finds an anchor 31 times | 0 run | | | | |
| mutmut | `generate.py` | no: with `test_goldens.py` its clean run fails on the generator's digest; with `test_generate.py` alone it stops on the module name | 0 run | | | | |
| cosmic-ray | `public-scrub.py` | yes | 545 | 403 | 141 | 0 | 1 errored |
| cosmic-ray | `audit-web-verdict.py` | yes | 89 | 59 | 30 | 0 | 0 |
| cosmic-ray | `mutation_rows.py`, #288's diff | yes | 1,105, of which `cr-filter-git` left 60 | 34 | 26 | 0 | 0 |
| cosmic-ray | `generate.py`, both test modules | yes | 279 | 279 | 0 | 0 | 0 |
| cosmic-ray | `generate.py`, `test_generate.py` alone | yes | 279 | 227 | 52 | 0 | 0 |
| prototype | `public-scrub.py` | yes | 286 | 216 | 70 | 0 | 0 |
| prototype | `audit-web-verdict.py` | yes | 55 | 53 | 2 | 0 | 0 |
| prototype | `mutation_rows.py`, #288's diff | yes | 32 | 28 | 4 | 0 | 0 |
| prototype | `generate.py`, both test modules | yes | 171 | 165 | 0 | 0 | 3 unviable, 3 void |
| prototype | `generate.py`, `test_generate.py` alone | yes | 171 | 139 | 26 | 0 | 3 unviable, 3 void |

The prototype's scrub run used `test_public_scrub.py` alone. Its 3 unviable and 3 void mutants on
the generator are two faults of the prototype, not of the design: its removal of a `not` before a
parenthesised operand dropped the opening parenthesis, and its child loaded each test module as
`unittest.TestLoader.loadTestsFromName` does, which lets an import that raises anything but
`ImportError` crash the run, where `unittest discover` reports a failed test named for the module.
R3 and R6 require the fix of each.

| tool | mutates | a module the test loads by path | diff scope | a run that examined nothing | a mutant whose tests hang | the killer |
|---|---|---|---|---|---|---|
| mutmut | a copy under `mutants/`; the tree is untouched | not mutated where the test loads it: the test must import mutmut's module name | by file (`mutmut run` names, `only_mutate`), never by line | exits 0 reporting "no tests" | its own status | not recorded |
| cosmic-ray | in place; sha256 equal after every run, the tree clean | mutated | by line, `cr-filter-git` | reads `complete: 23 (100.00%)`, `surviving mutants: 0 (0.00%)`, a survival rate of 0.00 and exit 0 on a looping fixture whose filter left nothing; only its session database says `skipped` | recorded `killed`, output `timeout` (12 of that fixture's 23) | not recorded: only the test command's output |
| prototype | in place; sha256 equal after every run, the tree clean | mutated | by line, from the changed lines | listed and examined printed apart, so examined 0 is read by name | reported `timeout`, apart from every kill (1 of a looping fixture's 5) | every failing test's id, down to the sub-test |

Each tool's run exits 0 with survivors except the prototype's, which exits 1. cosmic-ray's
operators mutate type annotations, which never run under `from __future__ import annotations`
(each of the four targets holds it): 22 of its 30 survivors on the web audit's verdict were such,
and 22 of its 26 on #288's diff. Its report divides survivors by every job, the filtered ones
included, so that diff's 26 survivors of 60 run read `surviving mutants: 26 (2.35%)`.

### 1.2 What rows alone miss, measured on #288's diff

#288's delivery changed `scripts/mutation_rows.py` (the shell parse check) and carried rows of its
own. Over that diff's 48 changed lines the prototype listed 32 mutants: 28 were killed and 4
survived every test of `test_mutation_rows.py`:

- `PARSE_SECONDS = 60` replaced with 61;
- `split(b"\n", 1)` replaced with `split(b"\n", 2)`, whose first element is the same, so no test
  can tell it apart: an equivalent mutant that only a record with its argument can explain;
- two `errors="replace"` replaced with `errors=""`, which differ only on bytes that are not UTF-8.

Each is either killable by a test the diff did not write or equivalent, and either way is a
decision nobody wrote down. The census test that reads the rows' anchors co-killed many of the 28
and was never the only killer of any.

### 1.3 A test that reads the file's bytes

Appending one comment line to a file changes its bytes and never its behaviour. On the generator it
failed exactly two tests of `test_goldens.py`,
`test_a_golden_whose_generator_digest_differs_is_refused` and
`test_every_committed_golden_is_current_and_well_formed`, which compare each committed golden's
`generator_sha256` with the generator's digest; on the three scripts it failed none of the 55 tests
of `test_public_scrub.py`, `test_audit_web.py` and `test_mutation_rows.py`. Those two tests kill
every mutant of the generator: with them both tools read no survivor (279 of 279, 165 of 165);
with `test_generate.py` alone, 52 and 26 survived, and each of the prototype's 26 was killed by
those two tests and by no other. A run that counts them is green having proved nothing about the
generator's behaviour.

### 1.4 The population

The prototype's listing at `dev` 26263de, with docstrings, annotations and every part of an
f-string left alone (R3), and the test modules that load or run each file:

| file | mutants | test modules that load or run it |
|---|---|---|
| `scripts/mutation-verdict.py` | 1,850 | `test_mutation_verdict.py`, `test_mutation_equivalent.py` |
| `scripts/mutation_rows.py` | 535 | `test_mutation_rows.py`; the verdict imports it, so the verdict's two also run it |
| `scripts/public-scrub.py` | 286 | `test_public_scrub.py`, `test_readings_taxonomy_scrub.py`, `test_rail_contract.py`, `test_deploy_templates.py` |
| `tools/parity-oracle/generate.py` | 171 | `test_generate.py`, `test_goldens.py` |
| `scripts/create-issues.py` | 152 | none |
| `scripts/audit-web-verdict.py` | 55 | `test_audit_web.py` |
| `scripts/write-back-issues.py` | 21 | none |

3,070 mutants over seven files. Two files have no test module, so their 173 mutants read
`uncovered` (R4), and the first whole run will find survivors in files no pull request touches:
#322 is the campaign that clears both.

## 2. Requirements

**The population and its class**

R1. The population is every `scripts/<name>.py` (one path segment under `scripts/`, so nothing under
    `scripts/tests/` or deeper) and `tools/parity-oracle/generate.py`.
    `scripts/mutation-verdict.py classify` reads a path of the first shape as the class `scripts`,
    which stays outside production code (SPEC-039 R2), and `generate.py` as `oracle`, as today.
    `plan` names the `scripts` class beside `rust`, `web` and `oracle`, under the one scope
    decision every class shares (SPEC-039 R3), and reads a change of blank and comment lines only
    as `not-applicable` (SPEC-039 R4).

**The runner, `scripts/mutation_python.py`**

R2. A standard-library Python script with four verbs:
    - `list` selects mutants (R8), prints `mutation-python: listed N` and each mutant's name, writes
      the listing with `--out FILE`, and runs nothing;
    - `run` selects mutants the same way and judges each (R5), writing the report (R7);
    - `census` checks the map (R4) and prints `examined N`;
    - `tests`, the child `run` starts for each test run (R6).
R3. The operator set, exactly:
    - each comparison operator replaced by its negation (`==` and `!=`, `<` and `>=`, `>` and `<=`,
      `in` and `not in`, `is` and `is not`);
    - `+` and `-` swapped, `*` replaced by `/`, `/`, `//` and `%` replaced by `*`, `|` and `&`
      swapped;
    - `and` and `or` swapped, and a `not` removed with the whitespace after it, its operand and
      the operand's parentheses kept;
    - a `True` or `False` constant flipped, an integer constant replaced by itself plus one, and a
      one-line string constant replaced by `""`, or by `"XX"` when it is empty;
    - a `return` of a value other than `None` replaced by `return None`;
    - `break` and `continue` swapped.
    It never mutates a docstring, any part of a type annotation (an argument's, a return's or an
    annotated assignment's), or any part of an f-string. A mutant is named
    `<file>:<line>:<column>: replace <old> with <new> in <function>`, where `<function>` is the
    innermost enclosing function's name, `Class.method` inside a class, and `<module>` outside any;
    the text after the location is the record's `mutant` (R12). The same text always lists the same
    mutants in the same order, and no comment of any text changes a listing.
R4. `scripts/mutation-python.json` maps each population file to its test directory and the test
    modules that load or run it (section 1.4 is the starting map). A file whose module list is
    empty has no test: `run` reads each of its mutants `uncovered` and runs nothing for it. The
    census refuses, by name, a population file the map omits, a key outside the population, a
    module whose file does not exist in its directory, a module named twice for one file, and an
    entry with any key but `dir` and `modules`, so no entry can leave a mutant out; it
    prints `examined N`, and a test runs it over the committed tree, as the rows' census runs.
R5. `run` judges each selected file in order:
    - it first refuses a tree with a tracked change (`mutation_rows.py`'s `tracked_changes`),
      exit 2, before it mutates anything;
    - the control runs the file's tests unmutated: a control that fails, or that selects no test,
      makes that file VOID by name, and none of its mutants runs;
    - the sentinel appends one comment line to the file and runs its tests: each test that fails
      reads the file's bytes, is named in the report as a byte reader and is left out of every run
      of that file's mutants; a sentinel that leaves no test is VOID by name;
    - `judge_mutant(...)` judges one mutant's text (the parse, the run and the outcome below), `count(report)` counts each outcome and examined (R7), and `exit_code(report)` returns the run's exit once a report exists (0, 1 or 3; R7);
    - each mutant is installed in place and parsed with `ast.parse`: a mutant that does not parse
      is `unviable`, never a kill, and runs no test; else its tests run in the child (R6), bounded
      by `--test-seconds`, whose default is the larger of 60 and five times the control's seconds
      (`--control-seconds S` replaces the measured seconds in that bound, the seam by which a test
      tells the bound from a constant 60);
    - a run that outlives its bound kills the child's process group and reads `timeout`; a child
      that prints no report reads `void`; a failing test reads `killed`, naming its killers; else
      `survived`;
    - after every mutant, and after the sentinel, the file's bytes are written back and checked by
      sha256 against the bytes read before: a write that fails or a digest that differs stops the
      run at once, exit 4, naming the file;
    - every child runs with `PYTHONDONTWRITEBYTECODE=1` and `PYTHONPYCACHEPREFIX` in a temporary
      directory, so the run writes no bytecode into the tree.
R6. The child, `tests --dir DIR --module M ... [--skip ID ...] [--failfast]`, runs from a copy of
    `scripts/mutation_python.py` and of each module of `scripts/` it imports, taken outside the tree
    before the first mutant is installed, so no mutant of the runner or of `mutation_rows.py` runs as the child; before it loads any test module it drops from `sys.modules` every module it imported from that copy and removes the copy's directory from `sys.path`, so a test that imports `mutation_rows` by name loads the tree's file with the mutant installed; it loads each module as `unittest discover` does
    (`TestLoader.discover(DIR, pattern="M.py")`), so an import that fails in any way is a failed
    test named for its module, drops each test whose id is skipped, runs the rest, and prints one
    JSON line, `{"ran": N, "failed": [id, ...]}`, each id down to the sub-test.
R7. The report, `--report FILE`, schema `deckstreak.mutation-python.v1`, holds the selection
    (`plan`, `all` or `file`), the shard, whether it ran `--failfast`, and per file its test
    modules, the control's ran count, failures and seconds, the bound in seconds each of its
    mutants ran under, its byte readers, a VOID reason when it has one, and each mutant's name,
    file, line, end line, column, `mutant`, operator, outcome (`killed`, `survived`, `uncovered`,
    `unviable`, `timeout` or `void`) and killers. It counts each outcome and prints `examined N`,
    where examined is killed plus survived plus uncovered. `run` exits 0 when every examined mutant
    is killed, 1 on a survived or uncovered mutant, 2 on a usage error or a dirty tree, 3 (VOID) on
    a failed or empty control, an empty sentinel, a timeout or a void mutant, and 4 on a failed
    restore; VOID outranks a survivor. A selection of no mutant writes a report with examined 0 and
    exits 0: whether that is VOID is the verdict's to say (R11), as it is for cargo-mutants.
R8. The selection:
    - exactly one of `--plan`, `--all` and `--file` is given, and `--shard k/n` takes integers with
      0 <= k < n and n >= 1; anything else is a usage error (exit 2, R7), before any file is read;
    - `--plan PLAN` takes, from `plan.json`, the code lines of each changed population file whose
      class the plan says applies, and selects each mutant whose span, from its first line to its
      last, holds one of them;
    - `--all` selects every mutant of the population, and `--file PATH` every mutant of one file;
    - `--shard k/n` then keeps the k-th of every n in listing order (files in path order, mutants in
      source order, k from 0), round-robin as cargo-mutants shards, so the n shards partition the
      selection;
    - `--failfast` stops each mutant's tests at the first failure, which is then its one killer; a
      pull request runs with it, and the weekly battery without it, so the battery names every
      killer (ADR-073 D5).

**The pull request's jobs**

R9. `mutation-plan` lists the diff's Python mutants (`list --plan`) and the whole population's
    (`list --all`), and `mutation-verdict.py shards` sizes the Python matrix from the diff's listing
    (`--python-listed FILE`): the ceiling of listed over 40, clamped to 1 to 8, written into the
    plan beside each shard's mutants and as the outputs `python_shards` and `python_matrix`. A
    listing of no mutant is one shard.
R10. A `ci.yml` job `mutation-python` needs `mutation-plan`, runs one job per shard of the plan's
    matrix (`run --plan ... --shard k/n --failfast`), prints its event's case by name, is never
    skipped, bounds itself with `timeout-minutes`, uploads its report under `if: always()`, restores
    no cache and saves none, and is a need of `mutation-verdict` and of `ci`.
R11. `mutation-verdict` runs `judge --class scripts` and `judge --class oracle`, each with
    `--python DIR` (the downloaded reports) and `--rows`:
    - every shard the plan promised, from 0 to n-1, must report: one missing, unreadable or not of
      the schema is VOID by name;
    - examined is the reports' examined count for that class's files plus the rows the diff selects
      whose anchor overlaps a changed line (SPEC-039 R4, R10); a class that applies and examined 0
      is VOID by name, and one whose changed lines are all blank or comments reads `not-applicable`;
    - a survived or uncovered mutant that no record binds fails, by name;
    - a `timeout`, a `void` mutant, a VOID file (a failed or empty control, an empty sentinel) and a
      report whose exit is 4 are each VOID by name;
    - each unviable mutant and each byte reader is named, and changes no count;
    - the oracle's line reads `examined N: generated G, rows R`, and replaces "the oracle's Python
      has no generated mutants".

**The record, and how a survivor is resolved**

R12. An equivalent Python mutant is recorded in `scripts/mutation-equivalent.d/python.json`, the one
    fragment of the population, with SPEC-057 R5's fields: `file`, `mutant` (R3's text after the
    location), `anchor`, `span` when needed, `reason` (one line), `evidence` (the argument: the code
    fact that makes the mutant unobservable, where a reviewer can check it), `reached_by` (a test of
    the file's mapped modules that runs the mutated line, named as a row's killer is,
    `<module>.<Class>.<method>`, because the runner, like cargo-mutants, measures no coverage) and
    `issue`. The census (SPEC-057 R7) refuses a Python record lacking any of them, or whose
    `evidence` is empty or repeats its `reason`, or whose `reached_by` does not resolve to exactly
    one test, or whose file lies outside the population; binding (R6 there), STALE and AMBIGUOUS
    against the whole population's listing (R8 there), and REFUTED (killed), UNNEEDED (unviable)
    and UNCOVERED (a file with no test) hold as ADR-070 holds them for Rust. A record cannot excuse
    a timeout, which is VOID before any record is read (R11).
R13. A survivor is resolved in the pull request that meets it: by a test that kills it, committed
    red first against the mutant, or by a record that carries its argument (R12). There is no cap,
    no allowance and no skip: no comment, configuration key or map entry leaves a mutant out of a
    listing (SPEC-057 R11), and an uncovered mutant is resolved only by a test module in the map.

**The weekly battery**

R14. `mutation-weekly.yml` gains a `python` job of 16 shards, each `run --all --shard k/16`
    without `--failfast`, uploading its report under `if: always()`:
    - `listing` adds the whole population's listing, and `survivors` needs `python`;
    - `survivors` drafts one issue per file, titled as SPEC-039 R12 titles each file's
      (`mutation-verdict.py`'s `TITLE`, `Mutation survivors: <path>`), listing that file's
      unexplained mutants, unless an open issue holds that file's title, and none for a file with
      no unexplained mutant;
    - `battery` counts each of the 16 reports, VOID by name for one missing, and `table` prints
      `table: python: listed N, killed K, equivalent E, unexplained U, unviable V`, where
      unexplained is survived or uncovered with no record, and any timeout, void mutant or VOID
      file makes the table VOID by name;
    - the dispatch input `package` (SPEC-057 R14) also takes `python`, which sweeps only the
      population; a crate's or `miniapp`'s scope runs no Python shard and promises no Python report.

**Rows and documents**

R15. The rows of section 7 live in `scripts/mutation-rows.d/S08700-S08799.json`, table
    `SCRIPT_MUTATIONS`, each proved KILLED by `mutation_rows.py prove` with the killer section 7
    names.
R16. The delivery sets ADR-073 `accepted`; appends to SPEC-039 a dated amendment, insert-only under
    ruling (i) of SPEC-038 section 8, that records R1 and R2's change (the oracle judged by
    generated mutants beside its rows; the class `scripts`); appends to ADR-057 a dated note that
    points to ADR-073; teaches R13 in `docs/BUILDER-BRIEF.md` and the Python run in
    `docs/TESTING.md`; records `docs/red-first/SPEC-087.md`; adds a changelog fragment; and moves
    this SPEC to `docs/specs/` (ADR-016).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | over a planted file holding one site of each operator, a `not` before a parenthesised operand, and a docstring, an argument's, a return's and an annotated assignment's annotation and an f-string each holding operator sites of their own, and sites at module level, in a function and in a method, and, beside them, a bare `return`, a `return None`, an empty string, a multi-line string, a `True` and a `False`, `list` prints `mutation-python: listed N` with N the count of mutants and each one's name in source order, `list --out FILE` writes the same names, `list` leaves every byte as it was and runs no test, and it lists exactly R3's mutants (the bare `return`, the `return None` and the multi-line string listing none), each named `<file>:<line>:<column>: replace <old> with <new> in <function>`, each parsing, none inside the docstring, any of the three annotations or the f-string, a method's site named in `Class.method` and a module-level site in `<module>`, the same on a second listing, and a comment of any text on a line changes nothing | `test_mutation_python.py` (the runner mints the names) |
| A2 | the map census refuses, by name, a planted map that omits a population file, names a key outside the population, names a module whose file is absent, names a module twice, or carries a key but `dir` and `modules`; and it passes the committed map, printing `examined N` with N the number of files the map names | `test_mutation_python.py` (the runner's census) |
| A3 | `run` over a planted tree with a tracked change exits 2 and leaves every file's bytes as they were; `run` with no selection, with `--all` beside `--file`, and with `--shard` of `2/2`, `0/0` and `x` each exits 2 and writes no report | `test_mutation_python.py` |
| A4 | a file whose tests fail unmutated, and one whose tests select nothing, each read VOID by name with exit 3, even beside another file's survivor, and none of their mutants runs | `test_mutation_python.py` |
| A5 | a planted test that fails on the sentinel is named a byte reader, is never any mutant's killer, and a mutant only it would kill reads `survived`; a sentinel that leaves no test is VOID by name with exit 3 | `test_mutation_python.py` |
| A6 | a killed mutant names every failing test's id without `--failfast` and exactly one with it, a failing sub-test down to its sub-test, and a mutant that makes a test module's import raise is killed by a test named for that module; over a planted tree whose population holds the runner itself, the mutant that negates its `if __name__ == "__main__":` reads `killed`, never `void`, and a mutant of that tree's `mutation_rows.py` names among its killers a planted test that imports `mutation_rows` by name | `test_mutation_python.py` (the child mints the ids) |
| A7 | a mutant that makes its test loop reads `timeout` within `--test-seconds`, never `killed`, and no process of its child outlives the run; a mutant whose child exits without its report reads `void`; each makes the run exit 3 | `test_mutation_python.py` |
| A8 | a planted mutant whose text does not parse reads `unviable`, runs no test, is named, and is not examined, and over a report holding that mutant and no other `count` reads one `unviable` and examined 0 and `exit_code` returns 0 | `test_mutation_python.py` |
| A9 | after a run holding a killed, a survived and a timed-out mutant, every file's bytes equal the bytes before, the tree gains no file, and that run exits 3 with a killed, a survived and a timeout outcome in its report; a planted test that replaces the file with a directory while a mutant is installed ends the run with exit 4 naming the file, and so does one that replaces it with a symbolic link to `/dev/null`, and no later mutant runs | `test_mutation_python.py` |
| A10 | given a planted plan, `--plan` selects exactly the mutants whose span holds a changed code line of a population file, one whose span begins above that line included, one whose span ends on it, one whose first line is it and one whose span holds it between, and none whose span ends just above it or begins just below it, and none of any other file, and none of a population file the plan names whose class it says does not apply (`not-applicable`), the run reading that file as it reads a file the plan does not name; `--all` selects every mutant of the map's files and `--file` one file's; `--shard k/n` over n shards partitions the selection, disjoint and whole, with shard k holding the mutants at listing positions k, k+n, k+2n and so on from 0, and `0/1` the whole selection; and `run` over a plan whose changed lines hold no mutant writes a report with examined 0 and exits 0 | `test_mutation_python.py` |
| A11 | a file whose map entry names no module reads each mutant `uncovered`, counted in the report's `examined N`, runs no test, and the run exits 1 | `test_mutation_python.py` |
| A12 | the report carries R7's schema and each field it lists (the selection, the shard, whether it ran `--failfast`, each file's modules, control ran count, failures and seconds, bound, byte readers and VOID reason, and each mutant's name, file, line, end line, column, `mutant`, operator, outcome and killers), each read by name, examined equals killed plus survived plus uncovered, and the counts of `timeout` and `void` each equal the mutants that read it, over a run whose fixtures hold at least one of each (A7's two), the recorded bound, under `--control-seconds` of 5, 12 and 20, is 60, 60 and 100, unless `--test-seconds` sets it, and the run exits 0 when every examined mutant is killed and 1 on a survivor | `test_mutation_python.py` (the runner mints the report) |
| A13 | `classify` reads `scripts/<name>.py` as `scripts`, anything under `scripts/tests/` and deeper, and `scripts/x.sh`, as `other`, and the generator as `oracle`; `plan` over a diff that changes a script's code line names the `scripts` class as applying, and over a push naming the pull request it merges as `not-applicable` | `test_mutation_python_verdict.py` (the verdict mints the class) |
| A14 | `shards --python-listed` sizes the Python matrix at the ceiling of listed over 40, clamped to 1 to 8: one shard for no mutant and for 40, two for 41, eight for 320 and for 321, writes each shard's mutants into the plan, and writes `python_shards` and `python_matrix` | `test_mutation_python_verdict.py` |
| A15 | a class whose changed code lines hold no mutant and no row reads VOID by name, one that a selected row covers reads examined 1 from the row, one whose changed lines are all blank or comments reads `not-applicable`, a report holding one killed mutant of a script and one of the generator gives each class examined 1, and the oracle's line reads `examined N: generated G, rows R` with the words "has no generated mutants" gone | `test_mutation_python_verdict.py` |
| A16 | a survived or uncovered mutant with no record fails by name; a `timeout` (even with a record naming it), a `void` mutant, a VOID file and an exit-4 report are each VOID by name; an unviable mutant and a byte reader are named and change no count | `test_mutation_python_verdict.py` |
| A17 | a promised shard, the last included, whose report is missing, unreadable or not of the schema is VOID, naming its index | `test_mutation_python_verdict.py` |
| A18 | a `python.json` record binds exactly its survived mutant, which is counted equivalent and named; the census refuses a planted record lacking each of `file`, `mutant`, `anchor`, `reason`, `evidence`, `reached_by` and `issue` in turn, with an empty `evidence`, with `evidence` repeating its `reason`, with a `reached_by` that resolves to no test or to two, or with a file outside the population; a record that binds no mutant of the whole listing is STALE and one that binds two AMBIGUOUS, which a `span` resolves into one; and a record whose mutant was killed is REFUTED, unviable UNNEEDED and uncovered UNCOVERED | `test_mutation_python_verdict.py` (the verdict mints each word) |
| A19 | `battery` over a planted weekly layout names a missing Python shard VOID and, under a crate's or `miniapp`'s `package` scope, names none missing, `table` prints the `python` line whose listed equals its four counts summed and reads VOID on a timeout, on a void mutant and on a VOID file, and `survivors` drafts one issue per file titled `Mutation survivors: <path>` naming that file's unexplained mutants, none for a file with none, and none while an open issue holds a file's title | `test_mutation_python_verdict.py` |
| A20 | `ci.yml`'s `mutation-plan` runs `list --plan`, `list --all` and `shards --python-listed`; `mutation-python` needs it, runs its matrix with `--plan`, `--shard` and `--failfast`, prints the plan's case, has no job-level `if`, a `timeout-minutes`, an `if: always()` upload and no cache step, and is a need of `mutation-verdict` and `ci`, whose verdict step runs `judge --class scripts` and `--class oracle` each with `--python` and `--rows` | `test_mutation_python_workflows.py` (the workflow's text) |
| A21 | `mutation-weekly.yml`'s `python` job runs 16 shards with `--all` and no `--failfast`, uploads under `if: always()`, `listing` lists the whole population, `survivors` needs it, `battery` counts its 16 reports, the `package` input's crate and `miniapp` scopes run no Python shard, and its `python` scope runs no Rust shard and no Stryker sweep | `test_mutation_python_workflows.py` |
| A22 | `docs/BUILDER-BRIEF.md` teaches a survivor's two resolutions and names no comment or setting that skips a mutant; `docs/TESTING.md` names the Python run; SPEC-039 carries the dated amendment and ADR-057 the note, each naming ADR-073; and ADR-073 reads `accepted` | `test_mutation_python_workflows.py` (the documents' text) |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k the_runner_lists_exactly_the_operator_sets_mutants
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k the_map_census_refuses_an_omitted_file_a_missing_module_and_a_stranger
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k the_runner_refuses_a_dirty_tree_before_it_mutates
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k a_red_or_empty_control_voids_its_file
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k a_test_that_fails_on_the_sentinel_is_named_and_never_a_killer
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k a_killed_mutant_names_its_killers
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k a_mutant_whose_tests_hang_is_void_never_killed
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k a_mutant_that_does_not_parse_is_unviable_and_runs_no_test
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k every_mutant_is_restored_byte_for_byte_and_a_failed_restore_stops_the_run
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k the_plan_selects_the_changed_lines_mutants_and_the_shards_partition_them
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k a_file_with_no_test_module_reads_every_mutant_uncovered
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k the_report_counts_examined_and_the_exit_reads_its_outcomes
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k a_guard_script_is_its_own_class_and_its_tests_are_not
A14: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k the_plan_sizes_the_python_matrix_from_its_listing
A15: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k a_python_class_that_examined_nothing_is_void
A16: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k a_python_survivor_fails_and_a_timeout_is_void_by_name
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k a_missing_or_partial_python_shard_is_void
A18: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k a_python_record_excuses_exactly_its_survivor_and_carries_its_argument
A19: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k the_battery_the_table_and_the_survivors_read_the_python_reports
A20: python3 -m unittest discover -s scripts/tests -p test_mutation_python_workflows.py -k the_python_job_runs_the_plans_shards_and_the_verdict_reads_them
A21: python3 -m unittest discover -s scripts/tests -p test_mutation_python_workflows.py -k the_weekly_battery_sweeps_the_whole_python_population
A22: python3 -m unittest discover -s scripts/tests -p test_mutation_python_workflows.py -k the_builder_brief_and_the_amendments_teach_the_python_run
```

- **Where each asserted text is minted.** A1 to A12 assert text and reports that
  `scripts/mutation_python.py` prints or writes, and live in `test_mutation_python.py`; A13 to A19
  assert what `scripts/mutation-verdict.py` prints, and live in `test_mutation_python_verdict.py`;
  A20 to A22 read the workflows' and the documents' own text, and live in
  `test_mutation_python_workflows.py`. Each test plants its fixtures in a temporary directory (a
  git repository for A3, A4, A5, A7, A9 and A11, since `run` refuses a dirty tree) and runs the
  script as a child, as `test_mutation_rows.py` runs the rows' runner; A8 alone calls the runner's
  functions `judge_mutant`, `count` and `exit_code` on a planted mutant, since R3's operators never produce
  text that does not parse.
- **A7 is bounded.** Its fixture's mutant makes a test loop; the test passes `--test-seconds` of a
  few seconds and runs the runner under a subprocess timeout of its own, so a runner that never
  times the child out fails the test by its bound, never by hanging it.
- **Red first.** Each criterion is committed red against a stub that keeps each verb and does
  nothing (`list` lists nothing, `run` writes an empty report, `census` examines nothing, `judge`
  reads no `--python`), so each fails by assertion for its own reason, as SPEC-039 section 3 did;
  the criteria on existing text (A13, A15) fail because the class `scripts` does not exist yet.
- **The rows are not re-proved here.** R15's rows are proved by `mutation-rows` on the delivery's
  own pull request, whose diff selects each (their anchors are new lines), and the rows' census in
  `test_mutation_rows.py` holds their shape.

## 4. File manifest

| file | by | change |
|---|---|---|
| `docs/specs/planned/SPEC-087-the-guard-scripts-and-the-parity-oracle-prove-their-tests-kill-generated-mutants.md` | this plan | added; the delivery moves it to `docs/specs/` (R16) |
| `docs/decisions/ADR-073-the-repositorys-python-is-mutated-by-a-runner-of-its-own-restored-by-digest.md` | this plan | added, `proposed`; the delivery sets it `accepted` (R16) |
| `docs/schematics/mutation-testing-python.md` | this plan | added: where the Python run sits among the mutation jobs, on a pull request and in the weekly battery |
| `changelog.d/docs-python-mutants-087.md` | this plan | added: the plan's fragment |
| `scripts/mutation_python.py` | delivery | added: the runner (R2 to R8) |
| `scripts/mutation-python.json` | delivery | added: the map (R4) |
| `scripts/mutation-verdict.py` | delivery | changed: the class `scripts` (R1), the Python shards (R9), `judge --python` (R11), Python records (R12), and `battery`, `table` and `survivors` (R14) |
| `scripts/mutation-equivalent.d/python.json` | delivery | added: `{"records": [...]}`, holding the records the delivery's own survivors need, or none (R12) |
| `scripts/mutation-rows.d/S08700-S08799.json` | delivery | added: section 7's rows (R15) |
| `.github/workflows/ci.yml` | delivery | changed: `mutation-plan`'s Python listing and outputs (R9), the `mutation-python` job (R10), `mutation-verdict`'s needs and steps (R11), `ci`'s needs |
| `.github/workflows/mutation-weekly.yml` | delivery | changed: the `python` job, `listing`, `survivors`, `battery`, `table` and the `package` input (R14) |
| `scripts/tests/test_mutation_python.py` | delivery | added: A1 to A12 |
| `scripts/tests/test_mutation_python_verdict.py` | delivery | added: A13 to A19 |
| `scripts/tests/test_mutation_python_workflows.py` | delivery | added: A20 to A22 |
| `scripts/tests/test_mutation_verdict.py`, `scripts/tests/test_mutation_workflows.py` | delivery | changed only where they assert text this delivery changes (the plan's count line, `ci`'s and `mutation-verdict`'s needs), each change named in the red-first record |
| `docs/BUILDER-BRIEF.md`, `docs/TESTING.md` | delivery | changed: R13 and the Python run (R16) |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | delivery | changed: a dated amendment, insert-only (R16) |
| `docs/decisions/ADR-057-mutation-testing-runs-on-the-diff-in-ci-and-weekly-on-dev.md` | delivery | changed: a dated note pointing to ADR-073 (R16) |
| `docs/red-first/SPEC-087.md` | delivery | added: `A<n>: red at <sha>: <failure>` per criterion |
| `changelog.d/feat-python-mutants-087.md` | delivery | added: the delivery's fragment |

## 5. What this does NOT do

- It builds no killer map: the weekly battery records every killer (R8), and the map that refuses
  removing an ordinary test's last kill is #220.
- It sweeps no Mini App mutant and clears none of its survivors: the Mini App's sweep is #240.
- It clears no survivor that already stands in a file its own diff does not touch: the first whole
  run's survivors and the 173 uncovered mutants of the two files without a test module are #322's
  campaign.
- It generates no mutant of the parity oracle's registry modules
  (`tools/parity-oracle/registry/spec_NNN.py`, the case builders and glue the generator runs),
  which lie outside SPEC-039's `oracle` class as they lie outside this population. Each golden
  records its registry module's sha256, so every such mutant first fails `test_goldens.py`'s
  digest check, which the sentinel names a byte reader (R5); `test_generate.py` builds a synthetic
  registry of its own and never runs them, and the one test that runs a real registry module is
  `test_goldens.py`'s round trip of the `{day:N}` token through `spec_042.py`. #325 plans them.
- It records no equivalent Rust or Mini App mutant: those campaigns are #295 and #294.

## 6. Risks

- **The weekly battery's cost is unmeasured in CI.** The population is about 3,070 mutants, 1,850
  of them in `mutation-verdict.py`, whose tests are the slowest, and the battery runs every test
  of each mutant (R8). Sixteen shards is the plan's figure; the first battery measures the seconds
  per mutant on GitHub's runners, and a later delivery re-sizes the shards from that run, as
  SPEC-039 R18 sized the Rust shards, if its slowest shard passes half its job's timeout.
- **The operator set is the repository's own code.** It has blind spots a maintained tool would
  not (a call's arguments, a slice, a dictionary's keys), and it generates fewer mutants than
  cosmic-ray (55 against 89 on the web audit's verdict). Its rows (section 7) and its own
  generated mutants on every pull request that changes it are its guard; ADR-073 says what would
  make a maintained tool replace it.
- **The sentinel finds only a test that reads every byte.** A test that reads the file's text for
  one pattern, as the rows' census reads each anchor, is not named by a comment appended at the
  end, and can kill a mutant whose text it reads without observing behaviour. Section 1.2 measured
  such a test as a co-killer and never the only killer; a record or a narrower test is the answer
  when it is the only one.
- **A timeout is VOID, never a kill.** A mutant that makes a loop run forever makes its pull
  request VOID until a test with a bounded wait of its own kills it (ADR-073 D4). None of the
  prototype's runs on the four targets met one.
- **A changed line with no mutant is VOID without a row.** An import or a `pass` lists no mutant;
  like a Rust constant, such a line needs a row or reads VOID (SPEC-039 R4).
- **The plan's shard size is a count, not a projection.** Forty mutants per shard is a bound on a
  pull request's wait, not a measurement; a slow file's shard may run long. Every shard reports
  whatever it reached, and a job that outlives its `timeout-minutes` leaves a missing report,
  which is VOID by name (R11).

## 7. Mutation rows

| row | target | the mutant it plants | killer |
|---|---|---|---|
| `S08701-A-HUNG-MUTANT-IS-VOID-NEVER-KILLED` | `scripts/mutation_python.py` | a run that outlives its bound reads `killed` | `test_mutation_python.TheRunnerJudgesEachMutant.test_a_mutant_whose_tests_hang_is_void_never_killed` (bounded, A7) |
| `S08702-A-DIRTY-TREE-IS-REFUSED` | `scripts/mutation_python.py` | the tracked-change check never refuses | `test_mutation_python.TheRunnerJudgesEachMutant.test_the_runner_refuses_a_dirty_tree_before_it_mutates` |
| `S08703-AN-EMPTY-CONTROL-IS-VOID` | `scripts/mutation_python.py` | a control that ran no test passes | `test_mutation_python.TheRunnerJudgesEachMutant.test_a_red_or_empty_control_voids_its_file` |
| `S08704-A-BYTE-READER-IS-NEVER-A-KILLER` | `scripts/mutation_python.py` | a test that failed on the sentinel stays in the file's runs | `test_mutation_python.TheRunnerJudgesEachMutant.test_a_test_that_fails_on_the_sentinel_is_named_and_never_a_killer` |
| `S08705-A-SPAN-HOLDS-ITS-CHANGED-LINE` | `scripts/mutation_python.py` | a mutant is selected only when its first line changed | `test_mutation_python.TheRunnerListsItsMutants.test_the_plan_selects_the_changed_lines_mutants_and_the_shards_partition_them` |
| `S08706-AN-ANNOTATION-IS-NEVER-MUTATED` | `scripts/mutation_python.py` | the annotation skip is dropped | `test_mutation_python.TheRunnerListsItsMutants.test_the_runner_lists_exactly_the_operator_sets_mutants` |
| `S08707-THE-RESTORE-IS-CHECKED-BY-DIGEST` | `scripts/mutation_python.py` | the digest comparison after a restore is inverted | `test_mutation_python.TheRunnerJudgesEachMutant.test_every_mutant_is_restored_byte_for_byte_and_a_failed_restore_stops_the_run` |
| `S08708-AN-UNPARSABLE-MUTANT-IS-NEVER-A-KILL` | `scripts/mutation_python.py` | a mutant that does not parse reads `killed` | `test_mutation_python.TheRunnerJudgesEachMutant.test_a_mutant_that_does_not_parse_is_unviable_and_runs_no_test` |
| `S08709-AN-UNCOVERED-MUTANT-IS-EXAMINED` | `scripts/mutation_python.py` | an uncovered mutant is left out of examined | `test_mutation_python.TheRunnerJudgesEachMutant.test_a_file_with_no_test_module_reads_every_mutant_uncovered` |
| `S08710-A-GUARD-SCRIPT-IS-ITS-OWN-CLASS` | `scripts/mutation-verdict.py` | the `scripts` pattern admits a path with a further slash | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_a_guard_script_is_its_own_class_and_its_tests_are_not` |
| `S08711-A-PYTHON-CLASS-THAT-EXAMINED-NOTHING-IS-VOID` | `scripts/mutation-verdict.py` | a Python class that applies and examined 0 passes | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_a_python_class_that_examined_nothing_is_void` |
| `S08712-A-PYTHON-SURVIVOR-FAILS` | `scripts/mutation-verdict.py` | a survived mutant no record binds passes | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_a_python_survivor_fails_and_a_timeout_is_void_by_name` |
| `S08713-A-PYTHON-TIMEOUT-IS-VOID` | `scripts/mutation-verdict.py` | a `timeout` counts as killed | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_a_python_survivor_fails_and_a_timeout_is_void_by_name` |
| `S08714-EVERY-PROMISED-PYTHON-SHARD-REPORTS` | `scripts/mutation-verdict.py` | the promised shards are counted one short | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_a_missing_or_partial_python_shard_is_void` |
| `S08715-A-PYTHON-RECORD-CARRIES-ITS-ARGUMENT` | `scripts/mutation-verdict.py` | a Python record without `evidence` passes the census | `test_mutation_python_verdict.TheVerdictReadsThePythonReports.test_a_python_record_excuses_exactly_its_survivor_and_carries_its_argument` |

Each row's find text is written by the delivery, against the code it builds, and must occur once in
its target. Each mutant is killable by its killer's assertion, and none makes a test wait: S08701's
mutant changes an outcome's name, never the bound, so its killer ends within A7's own timeout.

## 8. References

SPEC-039 (section 1, R1 to R4, R8, R10, R18, section 5, section 12), SPEC-057 (R4 to R14),
SPEC-038 section 8 (insert-only amendments), ADR-057 (D1, D4), ADR-070, ADR-073,
`docs/schematics/mutation-testing.md`, `docs/schematics/mutation-equivalence-record.md`,
`docs/schematics/mutation-testing-python.md`; #218, #219, #220, #240, #294, #295, #322, #325.
