---
status: "accepted"
---

# ADR-443: a price row is stale when its file's suite holds more tests than its price run ran, and a verb of the plan's own names it in the plan job without failing it

Decides SPEC-429.

## Context

Every `path:line` below was read at dev `ac7ebd75`.

**What a price row rests on.** The Python plan deals each listed file's mutants by a price row.
`PYTHON_CONTROL_SECONDS` (`scripts/mutation-verdict.py:143-151`, seven rows) holds whole-suite
prices from the battery run whose records are committed under
`scripts/tests/fixtures/mutation-prices-<run>/` (SPEC-420 R4). `PYTHON_SCOPED_SECONDS`
(`scripts/mutation-verdict.py:157-174`, sixteen rows) holds the (control, sentinel, mutant)
triples of the dispatched price run whose records are committed under
`scripts/tests/fixtures/mutation-scoped-prices-<run>/` (SPEC-421 R12). Every row's comment names
the run its figures came from, and a test re-derives each table from its records byte for byte.
When the legs trace, `shards` prices a file by its scoped row, else by its whole row
(`scripts/mutation-verdict.py:1287-1288`).

**What the price run recorded of the suite.** Each file's control record in those `costs.jsonl`
files carries `ran`: the number of tests the control ran, which is the whole suite the runner
builds from the file's `modules` in `scripts/mutation-python.json` (`scripts/mutation_python.py:749-756`,
`loader.discover` per module, then `flatten`, with no test skipped in the control,
`scripts/mutation_python.py:1116`). A file with no modules has an `uncovered` record instead.

**What the per-line selection chooses from.** In a `run --plan` leg each mutant runs the tests whose
processes executed its line, drawn from that same suite in its order (`scope_of`,
`scripts/mutation_python.py:886-905`), and the control and the traced sentinel each run the suite
whole. So every figure of a row grows when the suite grows: the control and the sentinel by every
added test, the mean mutant by every added test that executes a mutated line.

**One row reads low at dev.** `docs/rulings/OWNER-RULING-2026-10-16-release-legs-python-305-rust-192.md`,
its paragraph "One price is known to read low", records that `scripts/mutation-verdict.py`'s row
came from a run that predates tests added for that file, and that two of its legs recorded 1834 s
and 2077 s against the 1800 s projection bound. At dev, the file's mapped modules define 157 tests;
its scoped row's run ran 138, and its whole row's run ran 117. No other row's suite differs from
what its run ran.

**Where the plan runs.** The `mutation-plan` job checks out the merge ref with `fetch-depth: 2`
(`.github/workflows/ci.yml:394-398`), so no commit a price run ran at is in its checkout. Its
`shards` step (`.github/workflows/ci.yml:439-441`) prints the deal; `shards` is a span
`formal/tla/EveryLegCounted` covers by digest, and tests in the plan's test modules pin its
whole output byte for byte.

## Decision Drivers

- The warning prints and never blocks: the plan's exit code, its deal, its outputs and the verdict
  are unchanged.
- Every count it prints is a count of what it examined, and a run that examined nothing says VOID
  by name.
- It reads only the tree: no test module is imported or run, and no price is written.
- It names its fix, a price refresh by `mutation-prices.yml`, and when the fix is owed: before the
  release run.

## Decisions

**D1. A row is stale when its file's suite holds more tests now than the run it names ran.** The
suite now is the tests the file's mapped modules define, counted from each module's syntax tree
by the runner's own loader rules: each class at the module's top level whose bases reach a name
ending in `TestCase` through classes of the same module, and each `test`-prefixed method of it and
of its same-module ancestors, once per class. The suite then is the largest `ran` of the file's
`control` records under the row's run's record directory, or 0 when its only records are
`uncovered`. Stale is now > then; equal or fewer reads fresh. A row the tree cannot read is
unread, by name, and is not examined: its line names no run; its run keeps no record directory;
the records hold no control or uncovered record of its file; `scripts/mutation-python.json` does
not map the file; or a mapped module is absent, does not parse, or holds a shape the syntax tree
cannot count (a top-level `load_tests`, a star import, a test class with a base that is neither a
class of the module nor named `*TestCase`, a class under a top-level compound statement, or a
name imported from a module of its own directory that is a test class there). The tree settles a
row that names no run: SPEC-420 R4's and SPEC-421 R12's tests hold every line to its verb's output,
each naming its run, so such a row reaches dev only if those tests stop holding, and reading it
unread keeps the other rows examined.

**D2. A verb of its own, `stale`, prints in the plan job and exits 0 on every path.**
`python3 scripts/mutation-verdict.py stale` prints, in path order, one line per file whose pricing
row is stale:

`mutation: stale: <path>: its suite holds <now> test(s), <then> when run <run> priced it (<TABLE>); refresh its price by mutation-prices.yml before the release run`

then one line per unread row, `mutation: stale: unread: <path> (<TABLE>): <reason>`, then
`mutation: stale: <s> file(s) stale of <f> priced; <h> stale row(s) the plan does not price by; <u> row(s) unread`,
and last `examined <N>`, the rows it read. With N = 0 it prints
`mutation: stale: VOID: no price row was read, so this warning examined nothing` before
`examined 0`. It returns `EXIT_OK` on every path. The plan job runs it as one step after the rows
legs step: `python3 scripts/mutation-verdict.py stale || echo "mutation: stale: VOID: the check exited $?"`,
so a crash prints a VOID line and the job's result is unchanged. No `shards` line, no output and
no artifact changes.

**D3. Each line names the refresh and its time.** The fix is SPEC-421 R12's refresh: one dispatch
of `mutation-prices.yml` and a delivery that replaces the record directory and the table together.
It is owed before the release run, whose listing reaches every file. The refresh gives every
mapped file a scoped row, and the plan prices by the scoped row while the legs trace, so one route
clears a stale whole row too. `RELEASING.md` section 2 says to read the plan job's
`mutation: stale:` lines before the release pull request and to refresh first when any names a
file.

**D4. The population is every row of both tables.** At dev that is 23 rows over 16 files, all read:
two rows are stale, both `scripts/mutation-verdict.py`'s, and one file is named, by its scoped
row; its whole row is counted as a stale row the plan does not price by. A row is never re-priced
here.

**D5. The code sits beside the price verbs, outside every covered span.** The verb is one function
after `prices`, one entry in `main`'s verb list after `"prices"`, one arm after the `prices` arm,
and one module constant after `PYTHON_POPULATION`, naming the two record directory templates. No
line of `shards`, `python_by_cost`, `whole_reports`, `partition`, `examined_sum` or
`population_gaps` changes, so every digest `formal/tla/EveryLegCounted` records holds.

**D6. Kill criteria.**
- **Live precision.** Over the last 30 merged PRs, a warned file is judged in a run when one of its
  Python legs held the file, and it is a hit when, in such a leg, the file's recorded control
  seconds exceed its pricing row's control figure. Precision is hits divided by judged. Below 0.8
  with at least 5 judged, or 2 or more misses among the first 5 judged, propose a demotion.
- **Promotion.** A blocking level is a later owner call on measured precision; this ADR sets none.
- **Demotion.** Removing the warning, narrowing its population or silencing its VOID is a
  weakening, and goes to the owner as a signed ruling.
- **Tooling.** `examined 0`, the step's VOID line, or a tool that is missing or off its pin is VOID:
  that run's warning measured nothing. It is never a pass and never a hit, and it is counted apart.
  2 or more VOID runs among the last 30 merged PRs is a defect to fix, not a demotion.

**D7. Not in this delivery.** No price, no `PYTHON_LEG_BOUND_SECONDS`, no ceiling, no signed
release-leg ruling and no blocking level changes.

**D8. Red first, then rows, in two pushes.** Each criterion's test is committed first over a stub
of the verb that keeps every input and prints only `examined 0`, so each red is its criterion's
assertion. The fixture that makes the first red is a root whose row's run ran 2 tests and whose
module now defines 3: structural, with no clock. Rows are S42900 onward. Push 1 is the red commit
alone, and push 2 carries the green, the rows and the red-first record.

**D9. One delivery under SPEC-429 and this ADR.** SPEC-421 is amended insert-only, because its
R12's record directory gains a reader whose figure is the control record's `ran`.
`docs/schematics/mutation-testing-python.md` gains section 10, insert-only.

## What it was chosen against

### D1

- A trace of each file at plan time: rejected because it is a price run's sentinel per file on every pull request, 158 s for `scripts/mutation-verdict.py` alone by its own row.
- The runner's own loader at plan time: rejected because it imports every mapped test module in the plan job, so an import side effect or failure there becomes a new way for the plan to fail, and a test of it imports modules that never run here.
- A derivation at the run's commit: rejected because the plan job's checkout is two commits deep, and the run's commit is not in it.
- A new record of the suite's test ids kept by the price run: rejected because it changes the runner's records and the price verbs, every committed record lacks it, and the `ran` they already keep is the figure the warning needs.
- The scoped sets in the mutant records: rejected because they are the sampled mutants' sets, not the suite.
- A freshness bound by commit distance: rejected because age is not cost, and a row with no new test owes no refresh.
- A comparison of test-id sets: rejected because no committed record holds the ids of the suite a run ran.
- A row that names no run read as stale: rejected because its line could name no run to refresh against.
- A row that names no run read as VOID for the whole check: rejected because one row would blind the other 22.

### D2

- Printing inside `shards`: rejected because `shards` is a covered span whose digest would move, its whole output is pinned by many tests that would all change, and every call would pay the parse.
- A non-zero exit on a stale row: rejected because that is a blocking level.
- `continue-on-error` on the step: rejected because a crash would then read as a green step with no line.
- A workflow annotation: rejected because the documentation queried returned the error command's form only, and the warning needs no spelling nobody has verified.
- A record written into the plan's artifact: rejected because nothing in this delivery reads it, and the plan job's lines and the legs' reports already carry what the precision measure needs.

### D3

- Naming the weekly battery for a stale whole row: rejected because the plan prices by the scoped row while the legs trace, and a battery run re-prices only the fallback.
- Naming no route: rejected because a warning without its fix asks a reader to find it.
- Refreshing from the plan: rejected because a price is a figure a CI run recorded, never one the plan writes.

### D4

- Only the files the pull request lists: rejected because the population would move with every diff, and a stale row no pull request touches would first be read in the release run.
- Only the rows the plan prices by: rejected because a row dropped from the count is unexamined; the summary keeps it counted without a line.

### D5

- A separate script: rejected because it is a second reader of the tables and the records, with its own population entry, price and test module to keep.
- A check written only in the workflow: rejected because it would be untested and unmutated, and the workflow's own tests do not run here.
- Inside `shards`: rejected because it edits a span the model covers, for no change to the deal.

### D6

- A window of days: rejected because a kill window reads the last N merged PRs.
- Promotion on a streak alone: rejected because a block is the owner's call on measured precision.

### D7

- Re-pricing `scripts/mutation-verdict.py` here: rejected because a refresh is its own dispatch and delivery.

### D8

- A wall-clock fixture: rejected because a timing on a shared runner is not a criterion.
- One push: rejected because each red is read in CI over the commit that holds the tests alone.

### D9

- A new schematic: rejected because the Python plan's schematic already holds each earlier price section.
- An amendment to SPEC-420: rejected because the warning names the scoped refresh as its fix, and SPEC-420's whole table stays the fallback it reads as committed.

## Consequences

- Every plan prints the stale lines, so a suite that grows is named on the pull request that grows
  it, long before the release run.
- At dev the plan names one file, `scripts/mutation-verdict.py`, until its price is refreshed.
- A future price record that drops `ran` reads each of its rows unread, by name, and the summary
  counts them.
- A suite that swaps a test for another keeps its size and reads fresh.

## Confirmation

- SPEC-429's acceptance criteria, each red first in CI.
- The plan job's own lines on this delivery's pull request, read in CI.
- The kill criteria of D6, measured over the last 30 merged PRs.

## What would make this wrong

- A mapped module whose tests the loader finds and the syntax tree cannot see without reading it
  unread: A5's population would then miss a shape, and the count would read low.
- A leg that runs past its row with no test added: the warning would stay silent, and a slower
  test is not what it measures.
- Precision below D6's floor: the suite's size would not be what makes a row read low.

## More Information

SPEC-420 and SPEC-421 define the records and the per-line selection this reads. SPEC-418 deals the
legs by the rows. `RELEASING.md` section 2 holds the refresh.
