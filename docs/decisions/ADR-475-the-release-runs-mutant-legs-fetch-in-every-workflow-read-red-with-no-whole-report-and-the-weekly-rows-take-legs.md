---
status: "accepted"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-475: the release run's mutant legs fetch the locked graph in every workflow, a Rust leg with no whole report reads red, and the weekly rows are dealt into legs

Path:
`docs/decisions/ADR-475-the-release-runs-mutant-legs-fetch-in-every-workflow-read-red-with-no-whole-report-and-the-weekly-rows-take-legs.md`.

- **Status:** accepted. D1 to D4 and D6 to D8 are built by SPEC-458's first pull request. D5 changes
  `LEG_CEILING["battery"]`, a mutation bound, so it is built by SPEC-458's next pull request, and
  only after the owner's signed ruling for it is on `dev` (D9).
- **Decides:** SPEC-458.
- **Refers to:** #827 and #830. This decision closes neither; each issue closes citing the merge
  that delivers it.
- **Stands beside:** ADR-462 D4 (the caches, per workflow and job), unlanded (D3).
- **Schematic:** `docs/schematics/release-mutant-legs.md`, new.

## Context and Problem Statement

SPEC-416 put a fetch of the locked graph before each `ci.yml` mutant leg's baseline, and SPEC-417
dealt the pull request's rows into legs within the rows job's kept bound. The release pull request's
run at `dev` 83846ea3 is the first to carry both. Read from its jobs by
`gh api --paginate 'repos/RexRenatus/deck-streak/actions/runs/38976943303/jobs?per_page=100'` and
from each leg's log by `gh api --allow-escape-sequences repos/RexRenatus/deck-streak/actions/jobs/<job>/logs`:

- **The fetch holds the release's baselines past the download.** Of the 143 `mutation-rust` legs completed at the reading of run 38976943303, 143 read `Cache not found` and none read a baseline that failed on a download; the fetch step took 38 s in job 117228734746.
- **A red baseline still reads green.** 76 of those 143 legs read `TIMEOUT  Unmutated baseline` at 2300 s of test (job 117228740687: 305 s build + 2300 s test) and exited 4, and every one of the 76 concluded `success`.
- **The other 67 passed their baseline.** Their baseline tests ran from 1451 s (job 117228740771) to 2298 s (job 117228734414), and they exited 0 (35), 2 (22) or 3 (10).
- **The rows legs fit their bound.** The plan job 117228556587 dealt 2653 rows into 17 legs, "the largest 157 row(s), projected at 2669 s".
- **Every rows leg finished inside its bound.** All 17 legs of run 38976943303 concluded `success`, from 1641 s (job 117228729317) to 2968 s (job 117228729301), 1994 s on the mean, so the slowest took 1.49 times the mean leg and 0.55 of its 5400 s bound.
- **The release's rows are near the most the legs hold.** 17 legs of at most 158 rows hold 2686 rows: the run's 2653 leave 33 (plan job 117228556587, projected at 2669 s).

Read from the tree at `dev` 83846ea3:

- **The weekly battery's `rust` leg fetches nothing.** `mutation-weekly.yml`'s `rust` restores the
  Rust cache (`:135-145`) and runs cargo-mutants (`:163-178`) with no fetch between, and its
  `rehearsal` fetches (`:446-447`) only after its cargo-mutants step (`:438-445`). SPEC-416 section
  5 left both open under #827, reading that both restore `dev`'s cache.
- **A scheduled run restores the default branch's entry, not `dev`'s.** A run restores a cache
  only from its own ref, its pull request's base branch and the default branch, and a scheduled
  run's ref is the default branch (`rust-cache.yml:3-6` states the restore rule). So the weekly battery,
  which checks out `dev` (`mutation-weekly.yml:123`), restores what the default branch's scope
  holds: the warm-up's entry, or a push to `main`'s.
- **The warm-up saves an entry built with no fetch.** `rust-cache.yml`'s `warm` job restores by
  prefix (`:68-80`), builds what `ci.yml`'s `rust` job builds (`:81-86`), cleans (`:87-89`) and
  saves (`:90-99`); a build downloads only the packages it compiles, and the census asks for every
  target's, offline (`crates/progression/tests/xp_census.rs`). `ci.yml`'s `rust` job, the only other
  Rust save, fetches after its restore (`ci.yml:65-66`) before it saves (`:74-83`).
- **The Rust leg records its exit and never returns it.** `ci.yml:524-535` writes
  `cargo-mutants.exit` and echoes it; the verdict reads any exit outside `WHOLE_EXITS`
  (`scripts/mutation-verdict.py:187`) as a partial leg and fails closed, so a red baseline cannot
  pass the gate, but nothing marks the leg itself.
- **The weekly rows job is one job of 120 minutes.** `mutation-weekly.yml:255-299` proves every row
  with `prove --all` in one job; SPEC-417 section 5 left it open under #830.
- **The battery's ceiling counts the weekly's other jobs.** `LEG_CEILING = {"ci": 190, "battery":
  234}` (`scripts/mutation-verdict.py:959-961`): 256 jobs less the 22 other jobs
  `mutation-weekly.yml` can generate, the rows job among them as one
  (`scripts/tests/test_dispatch_shards.py:324-339`).
- **The cache decision beside this one.** ADR-462 D4, unlanded, gives `rust-cache.yml`'s scheduled
  run a pnpm and a Playwright step in a second delivery with no SPEC yet, and keeps every key inside
  the repository's configured cache limit.

The question is how each leg a release, a pull request and the weekly battery run starts from a
graph its offline census can read, reads red when it leaves no whole report, and fits its bound,
with no bound moved without the owner.

## Decision Drivers

- A pass still means every listed mutant and every selected row was examined, whatever the cache
  held.
- A leg that leaves the verdict nothing whole is visible as a red job, so the maintainer's targeted
  re-run of failed jobs finds it.
- No timeout, per-mutant budget, price or leg ceiling moves without the owner's signature; a
  tightening is signed as a raise is.
- One rule holds one population: the fetch rule of SPEC-332 and SPEC-416 is widened, never copied.
- A scheduled workflow's change takes effect only when a release carries it to the default branch,
  so each decision is judged by the tests on the pull request that lands it.

## Decision Outcome

### D1. Every job, in every workflow, that runs a cargo-mutants baseline fetches the locked graph after its last cache restore and before its baseline

The population SPEC-416 judges in `ci.yml` alone is read from every workflow file. The weekly
`rust` job gains a fetch step right after its restore, and `rehearsal`'s fetch step moves from
after its cargo-mutants step to right after its restore. Each new step runs
`cargo fetch --locked --manifest-path Cargo.toml`, the mutant legs' spelling, under a name of its own,
so the rows jobs' `run: cargo fetch --locked` lines, which rows S33200 to S33202 anchor on, keep
their places. The judge is SPEC-332's `refusals()` with SPEC-416's baseline anchor, unchanged.

### D2. Every job that saves the Rust cache fetches the locked graph after its last restore and before its save

`rust-cache.yml`'s `warm` job gains one step between its restore by prefix and its first build:
`cargo fetch --locked --manifest-path Cargo.toml`, under the lookup's
`if: ${{ steps.lookup.outputs.cache-hit != 'true' }}` that every later step of the job carries. So the
default branch's Rust entry, which a release pull request and every scheduled run restore, holds
every package the lock pins for every target, as the entries `ci.yml`'s `rust` job saves already
do. The same `refusals()` judges it, with an anchor that is the step saving the Rust cache's paths;
the population is every job, in every workflow, holding such a save: `ci.yml`'s `rust` and
`rust-cache.yml`'s `warm` at `dev` 83846ea3.

### D3. The warm-up's fetch stands beside ADR-462 D4's cache delivery, and lands on its own

SPEC-458 carries D2's one step, its test and its rows; ADR-462 D4's delivery keeps its pnpm and
Playwright keys, its own SPEC and its own band. The two share `rust-cache.yml` and
`scripts/tests/test_rust_cache_workflow.py`, on different steps; whichever lands second rebases
onto the first and re-reads the warm job's step list at its cut. D2 adds no cache key and no entry:
the key set and the count of entries stay as they are, and the one entry the warm-up saves gains
only the packages a host build does not compile. Raising the repository's cache limit stays the
owner's setting, which neither delivery touches.

### D4. A pull request's Rust mutant leg fails its job, after its upload, when cargo-mutants' exit leaves no whole report

`ci.yml`'s `mutation-rust` step keeps recording the exit and echoing it, and then ends on one line:

```
case "$rc" in 0|2|3) ;; *) echo "mutation-rust: shard $SHARD left no whole report (cargo-mutants exited $rc); the verdict reads it VOID"; exit "$rc" ;; esac
```

`0|2|3` is `WHOLE_EXITS`, read by the test from the verdict's own text, so the two cannot drift. A
leg whose exit is whole stays green and the verdict judges it, survivors included; a red or
timed-out baseline, a usage error, a crash or a signal fails the leg. The upload step after it runs
under `if: ${{ always() }}`, so the leg's report is still kept, the verdict still reads the leg
VOID by name, and `ci` fails as it did; what a run admits does not change.

### D5. The weekly rows are dealt into legs by the rows runner's own deal, and the battery's ceiling follows

Built by SPEC-458's next pull request, after the owner's signed ruling. The weekly `size` job runs
`mutation_rows.py legs --all`, which `legs` accepts beside `--rows-from`, and outputs `rows_legs` and
`rows_matrix`; `rows` needs `size` and runs one leg per entry, each proving `prove --all --leg k/n`
into its own `rows.json` under the artifact `rows-leg-<k>`, keeping its 120 minutes; the survivors
job's `battery` counts the legs by number, each MISSING or PARTIAL by name, and its drafting reads
every leg's `rows.json` as it reads one today. `LEG_CEILING["battery"]` falls from 234 to 218: 256
jobs less 38, the rows axis counted at `ROWS_MAX_LEGS`, as `ci`'s already is. That is a tightening of
a mutation bound, so it is the owner's to sign.

### D6. The rows keep one price, one bound and seventeen legs

`ROW_SECONDS = 17`, `ROWS_BOUND_SECONDS = 2700` and `ROWS_MAX_LEGS = 17` stay. Every rows leg of the
release's run fit its bound (the facts above), so pricing every row alike cut no leg. The release's
2653 rows leave 33 of the 2686 the legs hold, so a later release that selects more is refused whole
by name on its plan job, never cut, and its most legs is then read on its final union and signed, as
the release's Python and Rust ceilings were.

### D7. The tests

D1 and D2 are two populations of `scripts/tests/test_rows_jobs_fetch_graph.py`'s one judge: its
`baseline_jobs()` gains a reading of every workflow file beside its `ci.yml` default, and a new
`saving_jobs()` finds every job holding a step that saves the Rust cache's paths. D4's test sits in
`scripts/tests/test_mutation_workflows.py` beside the other `mutation-rust` step tests, reading the
whole exits from the verdict's text the way `rows_constant` reads the rows runner's. Every test reads
a workflow, so each is red and green in CI's `hygiene` job, never on the box.

### D8. Rows and formal surface

Rows come from the band `S45800-S45899`, `SCRIPT_MUTATIONS` rows on each new workflow line a
criterion holds, each killed by that criterion's test. No formal model: each step runs inside one
job's own runner, writes no shared object and decides no verdict, the battery's per-leg count is a
total function over the reports present after every leg has ended, and no span a `@phx covers`
names is edited (`formal/tla/EveryLegCounted` covers `shards`, `whole_reports`, `partition`,
`examined_sum`, `population_gaps` and `python_by_cost`).

### D9. Shape: one SPEC in two pull requests

The first pull request builds D1, D2 and D4 with their tests, rows and documents; the next builds
D5 once the owner's ruling is on `dev`. D1 to D4 depend on no signature, so the release's next run
gains them however the owner answers.

### Consequences

- A release pull request restores a default-branch entry that already holds every package, once a
  release has carried D2 to `main`; each leg's fetch then downloads nothing it lacks.
- The release run's Rust legs that leave no whole report read red, so its non-success legs are the
  legs the verdict names VOID, and a re-run of failed jobs reaches them.
- The weekly battery's Rust legs read the census offline whatever entry the default branch holds.
- After D5, the weekly battery proves every row within its legs' bound, and the battery's Rust legs
  hold at most 218.

### Confirmation

SPEC-458's acceptance criteria, read by name in CI's `hygiene` job. The release's next run, read by
job id: each Rust leg that exits outside 0, 2 and 3 concludes `failure`, and each that exits inside
concludes `success`.

## What it was chosen against

D1 was chosen against:

- **D1, leave the weekly legs to `dev`'s cache**: a scheduled run's ref is the default branch, so it never restores `dev`'s entries and reads the warm-up's, which holds no fetched package.
- **D1, fetch inside the cargo-mutants step (`cargo fetch && cargo mutants`)**: a failed fetch would then read as cargo-mutants' own exit, and SPEC-416's judge, which reads steps, would hold two rules for one.
- **D1, a judge of its own for the weekly file**: one rule then has two populations that can drift, where widening the one population holds both.

D2 was chosen against:

- **D2, rely on each consumer's fetch alone**: every restoring job then downloads the same packages on every run, and a job added later without a fetch reads a short registry offline.
- **D2, fetch after the builds, before the clean**: the clippy and nextest builds would then run without the packages a build script may read, and the order would differ from `ci.yml`'s `rust` job, whose fetch comes right after its restore.
- **D2, save a second entry holding only the registry**: it adds a key against the repository's cache limit, and every job would restore two entries.

D3 was chosen against:

- **D3, join ADR-462 D4's delivery**: that delivery has no SPEC and no band yet and changes keys in jobs this one never touches, so #827 would wait on it.
- **D3, carry ADR-462 D4's pnpm and Playwright keys here**: they add keys against the cache limit, which is the owner's setting, and lie outside #827 and #830.

D4 was chosen against:

- **D4, keep the leg green and let the verdict alone fail closed**: 76 of 143 legs of run 38976943303 read a red baseline and concluded `success`, so the maintainer's re-run of failed jobs would pass every one of them by.
- **D4, return cargo-mutants' exit (`exit "$rc"`)**: a leg with survivors (exit 2) or timeouts (exit 3) would read red too, where its report is whole and the verdict is the one judge of it.
- **D4, a step after the upload that reads `cargo-mutants.exit` back**: it ties the colour to the report's path, which ADR-462 D3's attempt directories move, where `$rc` is in hand inside the step.
- **D4, a verb in `scripts/mutation-verdict.py` that prints the whole exits**: it adds code the Python legs must mutate for one comparison a test of the constant's text holds.

D5 was chosen against:

- **D5, keep the weekly rows in one job of 120 minutes**: 2653 rows took 17 legs of 1641 to 2968 s each in run 38976943303, so one job is cut at its bound every week and the battery reads its rows MISSING.
- **D5, read the run's job limit per matrix and keep 234**: it rewrites SPEC-362 R3's derivation of every ceiling, a wider change than the weekly rows need, and is the owner's in its own right.
- **D5, a weekly rows bound of its own (half of 120 minutes)**: two more constants for four fewer legs, and the battery's ceiling still falls.
- **D5, merge the legs into one `rows/rows.json` before the battery**: `merge` exits 0 on an absent leg, so the battery's one-report check would read a week with a missing leg whole.
- **D5, a workflow of its own for the weekly rows**: the battery's count of the rows would leave the run that files the survivors.

D6 was chosen against:

- **D6, raise `ROWS_MAX_LEGS` now with headroom**: it admits legs no measurement read, the owner's own ruling rejects a ceiling with headroom above the measured figures, and every rows leg costs a Rust leg of `ci`'s ceiling.
- **D6, re-price `ROW_SECONDS` from run 38976943303's mean leg**: the slowest leg, job 117228729301, ran 2968 s for 156 rows, 19 s a row against the price's 17, so a lower price moves the slow legs past their projection.
- **D6, a per-row price table dealt by cost now**: each row was proved once, in one leg, with the leg's cold build in its first row's time, so the table would price most rows from one noisy reading.

D7 was chosen against:

- **D7, a new test module for the weekly and the warm-up**: the rule already lives in `test_rows_jobs_fetch_graph.py`, whose `refusals()` takes its anchor as a parameter for this.
- **D7, run the workflow tests on the box**: every touched module reads a workflow file, so their reds and greens are read in CI by name.

D8 was chosen against:

- **D8, a TLA+ model of the legs and the battery**: the legs share no written object and the battery reads their reports only after every leg has ended, so there is no interleaving to model.

D9 was chosen against:

- **D9, one pull request holding D5 too**: D1 to D4 would then wait on a signature they do not need.
- **D9, a second SPEC for D5**: D5 is #830's weekly half of the same rule, and a second SPEC splits one decision's criteria across two files.

## What would make this wrong

- A scheduled run is measured restoring `dev`'s entries: D1's reason for the weekly legs then
  weakens, and the fetch stays as the rule's population.
- cargo-mutants changes which exits leave a whole report: `WHOLE_EXITS` and D4's line then change
  together, and the test refuses either alone.
- A rows leg of a later release runs past two thirds of its 5400 s bound: D6's one price then needs
  the per-row table, priced from more than one reading of each row.
- The documented cache scoping or the repository's cache limit changes.

## More Information

SPEC-458, SPEC-416, SPEC-417, SPEC-332, SPEC-191, SPEC-362 R3, ADR-430, ADR-431, ADR-462 D3 and
D4, `docs/schematics/release-mutant-legs.md`, `docs/schematics/ci-jobs-and-caches.md`,
`docs/schematics/mutation-testing.md`, #827, #830.
