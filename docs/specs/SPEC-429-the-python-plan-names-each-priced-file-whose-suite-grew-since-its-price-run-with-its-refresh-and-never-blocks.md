# SPEC-429: the Python plan names each priced file whose suite grew since its price run, with its refresh, and never blocks

- **Issue:** none; this SPEC closes nothing and files nothing. It refers to #837, the Python legs
  priced at what a scoped leg runs, to #825, the Python legs' fit, and to #822, the release.
- **Context(s):** the mutation scripts (`scripts/`), their tests, the plan job of `ci.yml` and the
  release runbook. No crate, no price, no fixture and no other workflow changes.
- **Decided by:** ADR-443 (a price row is stale when its file's suite holds more tests than its
  price run ran, and a verb of the plan's own names it in the plan job without failing it).
  SPEC-421 is amended insert-only.
- **Schematic:** `docs/schematics/mutation-testing-python.md` section 10, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-429.md`.
- **Mutation band:** S42900-S42999.
- **Models:** none edited. `formal/tla/EveryLegCounted` and `formal/tla/TracedSetCovers` are read by
  their digests at the base and the head, and every covered span is unchanged.
- **Builds on:** SPEC-420 and SPEC-421, whose committed records and per-line test selection it
  reads, and SPEC-418, whose deal prices by the rows it judges.

## 1. The problem, measured

Every figure below was read at dev `ac7ebd75`.

- **A price row can read low.** `docs/rulings/OWNER-RULING-2026-10-16-release-legs-python-305-rust-192.md`,
  its paragraph "One price is known to read low", records that `scripts/mutation-verdict.py`'s row
  came from a run that predates tests added for that file, and that two legs recorded 1834 s and
  2077 s against the 1800 s projection bound.
- **The rows and what their runs ran.** `PYTHON_CONTROL_SECONDS` holds 7 rows and
  `PYTHON_SCOPED_SECONDS` 16 (`scripts/mutation-verdict.py:143-174`), 23 rows over the 16 files of
  `scripts/mutation-python.json`. Each row's run kept a `control` record per file carrying `ran`, or
  an `uncovered` record for a file with no modules, under
  `scripts/tests/fixtures/mutation-prices-<run>/` (the whole rows) and
  `scripts/tests/fixtures/mutation-scoped-prices-<run>/` (the scoped rows), `<run>` being the run the
  row's line names.
- **One file's suite has grown.** The seven modules `scripts/mutation-python.json` maps to
  `scripts/mutation-verdict.py` define 157 test methods: `git grep -c -E '^    (async )?def test'
  ac7ebd75 -- . ':(exclude,icase)*drill*' ':(exclude,icase)*readings_tree*'
  ':(exclude,icase)*_standin_checks*' ':(exclude,icase)*test_deploy_standin*'`, summed over those
  seven modules. Its scoped row's run ran 138 and its whole row's run ran 117, by those runs'
  control records of the file. Every other row's file defines exactly the tests its run ran.
- **Nothing says so.** No line of the plan compares a row with its file's suite, so a row that reads
  low is first seen when a leg runs past it, in the release run whose listing reaches every file.
- **Where a reading can run.** The `mutation-plan` job checks out two commits
  (`.github/workflows/ci.yml:394-398`), so no commit a price run ran at is in it. Its `shards` step
  (`.github/workflows/ci.yml:439-441`) runs `shards`, a span `formal/tla/EveryLegCounted` covers.

## 2. Requirements

R1. `scripts/mutation-verdict.py` gains the verb `stale`: `python3 scripts/mutation-verdict.py stale
[--root R]`. It reads, under the root: the lines of the two price tables in
`scripts/mutation-verdict.py`, `scripts/mutation-python.json`, each mapped test module, the modules
of the same directory those import from, and each named run's record directory. It writes nothing
but its standard output, and it imports or runs no test module.

R2. Every line of `PYTHON_CONTROL_SECONDS` and of `PYTHON_SCOPED_SECONDS`, from each table's opening
line to its closing `}`, is one row: its table, its path, and each run its comment names as
`run <n>`. Every row is examined or named unread; none is dropped.

R3. A row's suite now is the count of tests its path's mapped modules define (`dir` and `modules` in
`scripts/mutation-python.json`), read from each module's syntax tree: each class at the module's
top level whose bases reach a name ending in `TestCase` through classes of the same module, and
each `def` or `async def` named with the prefix `test` in that class or in a same-module class it
inherits from, each name once per class. A path whose `modules` is empty counts 0. A class defined
inside a function or a class is not counted, as the loader does not see it.

R4. A row's suite then is the largest `ran` of the path's `control` records in every `costs.jsonl`
under its run's record directory, `scripts/tests/fixtures/mutation-prices-<run>/` for a
`PYTHON_CONTROL_SECONDS` row and `scripts/tests/fixtures/mutation-scoped-prices-<run>/` for a
`PYTHON_SCOPED_SECONDS` row; or 0 when the path's only records there are `uncovered`. A row naming
several runs reads the largest over them. Those two templates are one module constant,
`PYTHON_PRICE_RECORDS`, on its own lines after `PYTHON_POPULATION`.

R5. A row is stale when its suite now is larger than its suite then. Equal or smaller reads fresh.

R6. A file's pricing row is its `PYTHON_SCOPED_SECONDS` row when `"plan"` is in
`mutation_python.SCOPED_SELECTIONS` and that table holds the file, and its `PYTHON_CONTROL_SECONDS`
row otherwise: the row `shards` prices it by.

R7. The verb prints, in path order, one line for each file whose pricing row is stale:

`mutation: stale: <path>: its suite holds <now> test(s), <then> when run <run> priced it (<TABLE>); refresh its price by mutation-prices.yml before the release run`

where `<run>` is the run whose records gave `<then>`, then one line for each unread row, in path
order with the `PYTHON_CONTROL_SECONDS` row first:

`mutation: stale: unread: <path> (<TABLE>): <reason>`

then the summary, where `<s>` counts the files named above, `<f>` the files whose pricing row was
read, `<h>` the stale rows that are not a pricing row and `<u>` the unread rows:

`mutation: stale: <s> file(s) stale of <f> priced; <h> stale row(s) the plan does not price by; <u> row(s) unread`

and last `examined <N>`, where N counts the rows read, fresh or stale.

R8. Each unread reason reads exactly one of:
- `its line names no run`;
- `run <r> keeps no records under <directory>/`;
- `run <r>'s records hold no control or uncovered record of it`;
- `run <r>'s control record of it carries no ran`;
- `scripts/mutation-python.json does not map it`;
- `<dir>/<module>.py is absent`;
- `<dir>/<module>.py does not parse`;
- `<dir>/<module>.py holds a test shape the syntax tree cannot count: <shape>`, the shape being
  `load_tests`, `a star import`, `class <C> has base <B>, which the module does not define`,
  `class <C> under a top-level <keyword>` or `it imports the test class <name> from <module>`.
A test class's base that is neither a class of the module nor a name ending in `TestCase` is the
third shape; a class that reaches no `TestCase` is not a test class and reads nothing.

R9. With N = 0 the verb prints `mutation: stale: VOID: no price row was read, so this warning
examined nothing` before `examined 0`. On every path it returns `EXIT_OK`.

R10. The `mutation-plan` job of `.github/workflows/ci.yml` gains one step after the rows legs step,
named `the price rows whose suite grew since their run, a warning that never fails the plan`, whose
`run` is
`python3 scripts/mutation-verdict.py stale || echo "mutation: stale: VOID: the check exited $?"`.
No other step, output or artifact of any job changes.

R11. `RELEASING.md` section 2 gains one paragraph after SPEC-421's: before the release pull request,
read the plan job's `mutation: stale:` lines from `mutation-verdict.py stale`, and when any names a
file, refresh its price by `mutation-prices.yml` first.

R12. These stay unchanged: every price and both tables' lines; `PYTHON_LEG_BOUND_SECONDS` 1800; the
share `PYTHON_MAX_SHARDS` and `PYTHON_SHARE_CAP`; `PYTHON_RELEASE_CEILING` and
`RUST_RELEASE_CEILING`; every record under `scripts/tests/fixtures/`; `scripts/mutation-python.json`;
`shards`, `python_by_cost`, `whole_reports`, `partition`, `examined_sum`, `population_gaps`,
`prices` and `scoped_prices`; and every refusal's text. No mutant leaves the plan, and no
skipped-mutant record and no exemption exists.

R13. Insert-only amendment sections, every earlier byte kept: SPEC-421 (its R12's records gain this
reader). `docs/schematics/mutation-testing-python.md` gains section 10.

## 3. Acceptance criteria of SPEC-429

| id | criterion | decided by |
|---|---|---|
| A1 | over a root whose scoped row's run ran 2 tests and whose module now defines 3, the verb prints that file's stale line naming 3, 2, the run, `PYTHON_SCOPED_SECONDS` and `mutation-prices.yml`, then the summary and `examined 1` | `test_mutation_python_shard_binding.py` `test_a_row_whose_suite_grew_since_its_run_is_named_with_its_fix` |
| A2 | the same root with a module of 2 tests, and of 1, prints no stale line and a summary of 0 stale | `test_mutation_python_shard_binding.py` `test_a_suite_no_larger_than_its_run_ran_reads_fresh` |
| A3 | a file with a stale whole row and a fresh scoped row prints no line and counts 1 stale row the plan does not price by; with `SCOPED_SELECTIONS` empty its whole row prices it and its line names `PYTHON_CONTROL_SECONDS`; a file with only a whole row is judged by it either way | `test_mutation_python_shard_binding.py` `test_a_file_is_judged_by_the_row_the_plan_prices_it_by` |
| A4 | the suite then is the largest `ran` of the file's control records, 0 for a file whose only record is `uncovered`, and the larger over a row naming two runs | `test_mutation_python_shard_binding.py` `test_the_suite_then_is_the_largest_ran_its_runs_records_hold` |
| A5 | over a population of modules (a plain test class, a same-module base, a same-module mixin, an async test, a skipped test, a helper method, a test class inside a function, a helper with a standard-library base, two classes sharing a base), the syntax-tree count of each equals what the runner's loader loads from it, `loader.discover` then `flatten` | `test_mutation_python_shard_binding.py` `test_the_syntax_tree_counts_what_the_runners_loader_loads` |
| A6 | each reason of R8 prints its unread line, is not examined and is counted in the summary | `test_mutation_python_shard_binding.py` `test_each_unread_reason_is_named_and_not_examined` |
| A7 | a root whose tables hold no row read prints the VOID line, then `examined 0` | `test_mutation_python_shard_binding.py` `test_no_row_read_prints_void_and_examined_zero` |
| A8 | over every root of A1 to A7 the verb returns 0, its last line is `examined <N>` for the rows that root reads, and the root's bytes are unchanged | `test_mutation_python_shard_binding.py` `test_every_reading_exits_zero_and_leaves_the_root_unchanged` |
| A9 | over the repository's own tree, the verb's rows are exactly the keys of `PYTHON_CONTROL_SECONDS` and of `PYTHON_SCOPED_SECONDS`, each examined or named unread | `test_mutation_python_shard_binding.py` `test_the_trees_rows_are_every_row_of_both_tables` |
| A10 | section 2 of `RELEASING.md` names `mutation-verdict.py stale`, the `mutation: stale:` lines and `mutation-prices.yml`, before the release pull request | `test_rulesets.py` `test_the_runbook_reads_the_stale_lines_before_the_release` |
| A11 | the `mutation-plan` job holds one step running the verb after the rows legs step, whose command ends with the VOID echo, and its `shards` step's command is unchanged | `test_mutation_python_workflows.py` `test_the_plan_job_runs_the_stale_verb_in_a_step_that_cannot_fail` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_row_whose_suite_grew_since_its_run_is_named_with_its_fix
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_suite_no_larger_than_its_run_ran_reads_fresh
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_file_is_judged_by_the_row_the_plan_prices_it_by
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_suite_then_is_the_largest_ran_its_runs_records_hold
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_syntax_tree_counts_what_the_runners_loader_loads
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_each_unread_reason_is_named_and_not_examined
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_no_row_read_prints_void_and_examined_zero
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_every_reading_exits_zero_and_leaves_the_root_unchanged
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_trees_rows_are_every_row_of_both_tables
A10: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_reads_the_stale_lines_before_the_release
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_python_workflows.py -k test_the_plan_job_runs_the_stale_verb_in_a_step_that_cannot_fail
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | mutation scripts | changed: `PYTHON_PRICE_RECORDS` after `PYTHON_POPULATION`; the `stale` verb's function after `prices`; `"stale"` after `"prices"` in `main`'s verb list; its arm after the `prices` arm |
| `.github/workflows/ci.yml` | CI | changed: one step in `mutation-plan` after the rows legs step |
| `RELEASING.md` | release runbook | changed: one paragraph in section 2, after SPEC-421's |
| `scripts/tests/test_mutation_python_shard_binding.py` | mutation tests | changed: A1-A9, one class at the module's end |
| `scripts/tests/test_mutation_python_workflows.py` | workflow tests | changed: A11 |
| `scripts/tests/test_rulesets.py` | runbook tests | changed: A10 |
| `scripts/tests/test_ci_workflows.py` | workflow tests | changed: the dynamic-import register's entries for the sites A5's loader oracle adds |
| `scripts/mutation-rows.d/S42900-S42999.json` | mutation rows | added: section 9's rows |
| `docs/specs/SPEC-421-each-python-file-is-priced-at-its-scoped-test-cost-measured-by-a-dispatched-run-and-the-plan-deals-scoped-legs-by-it.md` | records | changed: an insert-only amendment section |
| `docs/schematics/mutation-testing-python.md` | records | changed: section 10, insert-only |
| `docs/specs/SPEC-429-the-python-plan-names-each-priced-file-whose-suite-grew-since-its-price-run-with-its-refresh-and-never-blocks.md` | records | added: this file |
| `docs/decisions/ADR-443-a-price-row-is-stale-when-its-files-suite-holds-more-tests-than-its-price-run-ran-and-the-plan-job-names-it-without-failing.md` | records | added |
| `docs/red-first/SPEC-429.md` | records | added |
| `changelog.d/stale-price-row-429.md` | changelog | added |

## 5. What this does NOT cover

- It changes no price. `scripts/mutation-verdict.py`'s rows stay as committed, and a refresh is a
  dispatch and a delivery of its own (#837).
- It moves no leg bound: `PYTHON_LEG_BOUND_SECONDS` stays 1800 and the share stays as it is (#825).
- It changes no release ceiling and no signed release-leg ruling (#822).
- It sets no blocking level. The warning prints and never fails a job; a block is a later call on
  measured precision (#868).

## 6. Risks

- **A swap of equal size.** A test removed and another added keep the suite's size, so the row reads
  fresh. Detected by a leg past its projected seconds, which the plan prints per shard, and counted
  by ADR-443 D6's precision.
- **A slower test with none added.** The warning measures the suite's size, not its time. Detected
  the same way.
- **A loader shape the syntax tree misreads.** A5's population holds the counter to the runner's
  loader on every shape the mapped modules use; a shape outside it reads unread by R8 when the
  counter can see it, and A9 shows every row examined or named.
- **A crash of the verb.** The step's echo prints a VOID line and the job's result is unchanged; ADR-443
  D6 counts such runs apart.
- **A record without `ran`.** Its row reads unread by name (R8), and SPEC-421's amendment asks a
  refresh to keep it.

## 7. What only CI proves

- Every red and green of A1-A11. No test module of this delivery runs outside CI.
- The plan job's own lines on this delivery's pull request: the verb's reading of the head, with
  `scripts/mutation-verdict.py` named while its price stands.
- The mutation verdict of the verb's own code, every listed mutant examined.

## 8. Formal model

**Not applicable by surface.** The verb is one actor in one step. It reads committed files, the
two tables' lines, `scripts/mutation-python.json`, the mapped modules and the record directories,
which no other actor writes during the job, and it writes only its own standard output: no file,
step output or artifact a later job reads. The verb's count is a pure function that decides no
verdict, ledger, release or price; its reference semantics is the runner's own loader, which A5
holds it to over a hostile population. No span `formal/tla/EveryLegCounted` or
`formal/tla/TracedSetCovers` covers is edited, and the build compares each of their nine digests
by the digest rule at the base and at the head.

## 9. Mutation rows

Each row's find is written by the build on the line it lands, and occurs exactly once at its
target.

| row | behaviour | mutant | killer |
|---|---|---|---|
| S42900 | stale is a suite larger than its run ran | `>` to `>=` in the comparison | A2 |
| S42901 | the pricing row follows the legs' tracing | the `"plan" in mutation_python.SCOPED_SELECTIONS` read made `False` | A3 |
| S42902 | a scoped row prices a file only when its table holds it | the table-holds-the-file test dropped | A3 |
| S42903 | the suite then is the largest `ran` | `max` to `min` over the control records | A4 |
| S42904 | an `uncovered` record reads 0 | the uncovered arm read as no record | A4 |
| S42905 | a same-module ancestor lends its tests | the ancestor walk dropped | A5 |
| S42906 | a test class reaches a name ending in `TestCase` | the suffix test given another suffix | A5 |
| S42907 | an unread row is not examined | the unread arm counted as examined | A6 |
| S42908 | a top-level `load_tests` reads unread | that check disabled | A6 |
| S42909 | a test class's foreign base reads unread | that check disabled | A6 |
| S42910 | no row read prints VOID | the zero test made never true | A7 |
| S42911 | the verb returns `EXIT_OK` | its return given `EXIT_FAIL` when a file is stale | A8 |
| S42912 | the scoped record directory's template | its name changed | A1 |
| S42913 | the plan step cannot fail | the command's or-list made an and-list | A11 |
| S42914 | the runbook names the verb | the verb's name changed in the paragraph | A10 |
