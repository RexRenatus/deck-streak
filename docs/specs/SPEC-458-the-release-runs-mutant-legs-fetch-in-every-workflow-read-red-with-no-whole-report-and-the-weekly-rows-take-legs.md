# SPEC-458: the release run's mutant legs fetch the locked graph in every workflow, a Rust leg with no whole report reads red, and the weekly rows take legs

Path:
`docs/specs/SPEC-458-the-release-runs-mutant-legs-fetch-in-every-workflow-read-red-with-no-whole-report-and-the-weekly-rows-take-legs.md`.

- **Issue:** #827 and #830, which record the release run's red baselines and its cut rows leg and
  leave the weekly battery's legs, the warm-up's save, the Rust leg's colour and the weekly rows
  open. This delivery refers to both and closes neither.
- **Context(s):** CI (`.github/workflows/`) and its tests (`scripts/tests/`); in the next pull
  request also the rows runner and the verdict (`scripts/`). No crate, no `web/` file and no test
  under `crates/` changes.
- **Decided by:** ADR-475 (D1 to D9). It stands beside ADR-462 D4, unlanded (ADR-475 D3).
- **Schematic:** `docs/schematics/release-mutant-legs.md`, new.
- **Status:** delivered in two pull requests. The one that adds this file delivers sections 2 to
  9 with its tests, its rows and `docs/red-first/SPEC-458.md`; section 10 is delivered by the next,
  after the owner's signed ruling for its ceiling is on `dev`. **Mutation band:** `S45800-S45899`
  (section 7). **Model:** none (section 9). Amends SPEC-416 and SPEC-191, each by an amendment
  section appended to its own file.

## 1. The problem, measured

Read from the release pull request's run at `dev` 83846ea3, its jobs by
`gh api --paginate 'repos/RexRenatus/deck-streak/actions/runs/38976943303/jobs?per_page=100'` and
each leg's log by `gh api --allow-escape-sequences repos/RexRenatus/deck-streak/actions/jobs/<job>/logs`,
while 49 of its 192 `mutation-rust` legs were still running or queued:

- **No baseline failed on a download.** All 143 completed `mutation-rust` legs of run 38976943303 read `Cache not found`, and none read a baseline `FAILED` on a missing package; the fetch step took 38 s in job 117228734746.
- **76 legs read a red baseline and concluded `success`.** Each read `TIMEOUT  Unmutated baseline` at 2300 s of test in run 38976943303 (job 117228740687: 305 s build + 2300 s test), exited 4, and concluded `success`.
- **67 legs passed their baseline.** Their baseline tests took 1451 s (job 117228740771) to 2298 s (job 117228734414); 35 exited 0, 22 exited 2 and 10 exited 3.
- **The rows legs were dealt to fit.** The plan job 117228556587 read `legs: 17 leg(s) for 2653 row(s), the largest 157 row(s), projected at 2669 s`.
- **Every rows leg finished inside its bound.** All 17 legs of run 38976943303 concluded `success` in 1641 s (job 117228729317) to 2968 s (job 117228729301), 1994 s on the mean: the slowest at 1.49 times the mean leg and 0.55 of its 5400 s bound.
- **The release's rows leave 33 of the legs' room.** 17 legs of at most 158 rows hold 2686; the plan job 117228556587 selected 2653, projected at 2669 s.

Read from the tree at `dev` 83846ea3 by `git show 83846ea3:<path>`:

- **The weekly `rust` leg fetches nothing.** `.github/workflows/mutation-weekly.yml` restores the
  Rust cache (`:135-145`), installs its tools (`:146-153`) and runs cargo-mutants (`:163-178`) with
  no fetch; `rehearsal` restores (`:419-429`), runs cargo-mutants (`:438-445`) and only then
  fetches (`:446-447`). SPEC-416 section 5 left both open under #827.
- **A scheduled run restores the default branch's entry.** A run restores only from its own ref,
  its pull request's base and the default branch (`.github/workflows/rust-cache.yml:3-6`), and a
  scheduled run's ref is the default branch; the battery checks out `dev`
  (`mutation-weekly.yml:123`) but restores what the default branch's scope holds.
- **The warm-up saves an entry with no fetch.** `rust-cache.yml`'s `warm` restores by prefix
  (`:68-80`), builds (`:81-86`), cleans (`:87-89`) and saves (`:90-99`). The only other Rust save,
  `ci.yml`'s `rust` job, fetches (`ci.yml:65-66`) before it saves (`:74-83`).
- **The Rust leg never returns its exit.** `ci.yml:524-535` records `cargo-mutants.exit` and echoes
  it; the verdict reads an exit outside `WHOLE_EXITS = frozenset({0, 2, 3})`
  (`scripts/mutation-verdict.py:187`) as a partial leg (`:1574-1577`) and fails closed.
- **The judge already takes an anchor.** `refusals(file, job, job_steps, anchor)` in
  `scripts/tests/test_rows_jobs_fetch_graph.py:90-117` orders a fetch after the last restore and
  before the anchor step; `baseline_jobs()` (`:214-221`) reads `ci.yml` alone.
- **The weekly rows are one job.** `mutation-weekly.yml:255-299` runs `prove --all` in one job of
  120 minutes; `battery` reads one `rows/rows.json` and asks only that it prove a row
  (`scripts/mutation-verdict.py:2593-2600`); `legs` needs `--rows-from`
  (`scripts/mutation_rows.py:1478-1479`).
- **The battery's ceiling counts the weekly's other jobs at 22.** `LEG_CEILING = {"ci": 190,
  "battery": 234}` (`scripts/mutation-verdict.py:959-961`), derived by
  `scripts/tests/test_dispatch_shards.py:324-339`.

## 2. Requirements

- **R1.** Every job, in every workflow file under `.github/workflows/`, that runs a cargo-mutants
  baseline, meaning a step whose code runs `cargo mutants` without `--list`, runs
  `cargo fetch --locked` in a step after its last cache-restore step and before its first such
  step, swallowing no failure (no `||`, no `set +e`, no `continue-on-error`), under that step's
  `if:`. This is SPEC-416 R1 to R3 with its population read from every workflow.
- **R2.** In `mutation-weekly.yml`, `rust` gains, right after its restore step, a step named
  `the locked dependency graph, every target's packages (the census reads it offline at the baseline)`
  that runs `cargo fetch --locked --manifest-path Cargo.toml`; `rehearsal`'s fetch step moves from
  after its cargo-mutants step to right after its restore step, named
  `the locked dependency graph, every target's packages (the rehearsal's baseline reads it as a shard's does)`
  and running `cargo fetch --locked --manifest-path Cargo.toml`. Neither has an `if:`, as no step of
  either job has. The `rows` job's fetch step and every row's anchor on it are unchanged.
- **R3.** Every job, in every workflow, that holds a step saving the Rust cache's paths (an
  `actions/cache/save` step whose paths hold `~/.cargo/registry/cache/`) runs `cargo fetch --locked`
  in a step after its last cache-restore step and before that save, swallowing no failure.
- **R4.** `rust-cache.yml`'s `warm` job gains, between its restore by prefix (`id: warm`) and its
  clippy build, a step named
  `the locked dependency graph, every target's packages (the saved entry holds them for every job that restores it)`
  with `if: ${{ steps.lookup.outputs.cache-hit != 'true' }}` and
  `run: cargo fetch --locked --manifest-path Cargo.toml`. SPEC-191's rules stand: the job runs no
  test, cleans right before its save, and every step after its lookup stops on a hit.
- **R5.** The judge is SPEC-332's `refusals()`. `baseline_jobs()` takes the workflow files it
  reads, `ci.yml` alone by default, so SPEC-416 A1 reads what it read; a new `saving_jobs()` reads
  every workflow; the save anchor is `Anchor("Rust cache save", saves_the_rust_cache, False)`.
  SPEC-332's and SPEC-416's tests pass unchanged.
- **R6.** `ci.yml`'s `mutation-rust` cargo-mutants step keeps every line it has and ends on this
  line, after the line that echoes the exit:
  `case "$rc" in 0|2|3) ;; *) echo "mutation-rust: shard $SHARD left no whole report (cargo-mutants exited $rc); the verdict reads it VOID"; exit "$rc" ;; esac`.
  The step's name and the upload step after it, under `if: ${{ always() }}`, are unchanged.
- **R7.** The line's exits equal `WHOLE_EXITS`, which the test reads from
  `scripts/mutation-verdict.py`'s text and never imports; no line of that file changes.
- **R8.** No job's `timeout-minutes`, no `--timeout` or `--build-timeout`, no price, bound or
  ceiling, and no cache step's key, paths or condition changes, and no job saves a cache it did not
  save before.
- **R9.** Rows `S45800` to `S45811` (section 7), each killed by the test of the criterion that
  observes its line.

## 3. Acceptance criteria of SPEC-458

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | every job of every workflow that runs a cargo-mutants baseline fetches the locked graph after its last cache restore and before cargo-mutants, swallowing nothing, under that step's `if:`; the population names `ci.yml`'s `mutation-rust` and `mutation-weekly.yml`'s `rust` and `rehearsal`, and reports its examined count (R1, R2) | red at the base: `rust` has no fetch step, and `rehearsal`'s fetch is not before its baseline | `test_rows_jobs_fetch_graph.AllWorkflowsBaselineJobsFetchTheLockedGraph.test_each_baseline_job_of_every_workflow_fetches_after_its_cache_restore_and_before_cargo_mutants` |
| A2 | every job that saves the Rust cache fetches the locked graph after its last cache restore and before its save, swallowing nothing; the population names `ci.yml`'s `rust` and `rust-cache.yml`'s `warm` and reports its examined count (R3, R4) | red at the base: `warm` has no fetch step | `test_rows_jobs_fetch_graph.EveryRustCacheSaveFetchesTheLockedGraph.test_each_job_that_saves_the_rust_cache_fetches_after_its_restore_and_before_its_save` |
| A3 | the judge refuses, by name, each planted shape of a saving job: no fetch, a fetch before the restore, a fetch after the save, `\|\| true` and `continue-on-error`; the fixed shape passes (R3, R5) | not red: it plants its own shapes | `test_rows_jobs_fetch_graph.TheJudgeRefusesEachPlantedSaveShape.test_the_judge_refuses_each_planted_shape_on_a_saving_job` |
| A4 | the Rust leg's cargo-mutants step ends on R6's line, after the line that records the exit; the exits it passes equal `WHOLE_EXITS` read from the verdict's text, its other arm ends `exit "$rc"`, and the upload after it runs under `always()` (R6, R7) | red at the base: the step has no such line | `test_mutation_workflows.TheMutantLegReadsRedWithNoWholeReport.test_the_rust_leg_fails_its_job_on_every_exit_outside_the_whole_exits` |
| A5 | the line's reader refuses, by name, each planted shape: no line, an exit 4 added, an exit 2 dropped, an arm ending `true`, the line before the exit is recorded; the fixed shape passes (R6) | not red: it plants its own shapes | `test_mutation_workflows.TheMutantLegReadsRedWithNoWholeReport.test_the_colour_reader_refuses_each_planted_shape` |
| A6 | SPEC-332's and SPEC-416's tests and `test_rust_cache_workflow.py` pass unchanged (R4, R5, R8) | not red: they pin what the base already did, and guard the change | the four SPEC-332 and SPEC-416 classes, and `test_rust_cache_workflow`, unchanged |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_each_baseline_job_of_every_workflow_fetches_after_its_cache_restore_and_before_cargo_mutants
A2: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_each_job_that_saves_the_rust_cache_fetches_after_its_restore_and_before_its_save
A3: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_the_judge_refuses_each_planted_shape_on_a_saving_job
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_rust_leg_fails_its_job_on_every_exit_outside_the_whole_exits
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_colour_reader_refuses_each_planted_shape
A6: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k EveryRowsJobFetchesTheLockedGraph -k TheJudgeRefusesEachPlantedShape -k EveryMutantsJobFetchesTheLockedGraph -k TheJudgeRefusesEachPlantedBaselineShape
A6: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py
```

Every criterion is read red and green from CI's `hygiene` job, which runs `scripts/tests`. Each
module reads a workflow file, so none runs off CI for this delivery.

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/mutation-weekly.yml` | CI | changed: a fetch step in `rust` right after its restore; `rehearsal`'s fetch step moved to right after its restore (R2) |
| `.github/workflows/rust-cache.yml` | CI | changed: a fetch step in `warm` between its restore by prefix and its clippy build (R4) |
| `.github/workflows/ci.yml` | CI | changed: one line ending `mutation-rust`'s cargo-mutants step (R6) |
| `scripts/tests/test_rows_jobs_fetch_graph.py` | CI tests | changed: `baseline_jobs()`'s files parameter, `saves_the_rust_cache`, `saving_jobs()`, the save anchor, A1 to A3 |
| `scripts/tests/test_mutation_workflows.py` | CI tests | changed: `verdict_constant()`, the line's reader, A4 and A5 |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: one register entry for `verdict_constant`'s read of the verdict's text, shaped as `rows_constant`'s (`:5173`) |
| `scripts/mutation-rows.d/S45800-S45899.json` | CI scripts | added: rows S45800 to S45811 (section 7) |
| `docs/specs/SPEC-458-the-release-runs-mutant-legs-fetch-in-every-workflow-read-red-with-no-whole-report-and-the-weekly-rows-take-legs.md` | docs | added: this file |
| `docs/decisions/ADR-475-the-release-runs-mutant-legs-fetch-in-every-workflow-read-red-with-no-whole-report-and-the-weekly-rows-take-legs.md` | docs | added |
| `docs/schematics/release-mutant-legs.md` | docs | added |
| `docs/specs/SPEC-416-every-mutant-leg-fetches-the-locked-graph-before-its-baseline.md` | docs | changed: an amendment section appended, naming SPEC-458 |
| `docs/specs/SPEC-191-a-scheduled-job-saves-the-rust-cache-in-the-default-branchs-scope.md` | docs | changed: an amendment section appended, naming SPEC-458 |
| `docs/red-first/SPEC-458.md` | docs | added: A1 to A6 |
| `changelog.d/release-mutant-legs-458.md` | docs | added: the changelog fragment |

## 5. What this does NOT cover

- The 76 legs of the release's run whose baseline timed out: their census compiles every pinned
  package cold inside the per-mutant budget, which the census store's own delivery cures (#919);
  this delivery only makes each such leg read red.
- ADR-462 D4's pnpm and Playwright keys and the retirement of `dev`'s Rust save, which are that
  decision's own delivery beside this one (#827, ADR-475 D3).
- The rows legs' colour: the rows runner's exit stays recorded and the verdict judges each leg's
  report by id, and no rows leg read green over a missing report in the release's run (#830).
- A per-row price table and a deal by cost, which wait on more than one reading of each row
  (#933, ADR-475 D6).
- A raise of `ROWS_MAX_LEGS` for a release that selects more than 2686 rows, which is read on that
  release's final union and signed then (#830).
- The weekly `rust` leg's `exit "$rc"`, which fails it on survivors as well, unchanged (#827).
- Any re-run, dispatch or cancel of a run, and any change to who starts a release (#822).
- The weekly rows' legs and the battery's ceiling, which section 10 delivers in the next pull
  request (#830).

## 6. Risks

- **A registry outage fails a fetch.** The job then reads red at the fetch and uploads no report;
  the verdict names the leg VOID, and a re-run after the outage is the remedy. Detected by the
  failed step's name.
- **A red leg is re-run before its cause is cured.** A leg whose baseline timed out repeats the
  timeout on a re-run until #919 lands. Detected by the leg's own line, which names its exit.
- **The warm-up's entry grows.** It gains the packages a host build does not compile, the set the
  `rust` job's saved entries already hold; eviction by last access trims the store. Detected by the
  repository's cache list.
- **A scheduled workflow's change waits for a release.** `rust-cache.yml` and the weekly battery
  run the default branch's copy, so R2 and R4 take effect when a release carries them to `main`.
  Detected by the first scheduled run's legs after that release, read by job id.
- **ADR-462 D3 moves the Rust leg's report path.** R6's line reads `$rc`, not a path, so it holds
  whichever lands first; the lines around it move. Detected by the anchor census at the cut.
- **A new whole exit.** If cargo-mutants adds one, `WHOLE_EXITS` and R6's line change together,
  and A4 refuses either alone.

## 7. Mutation rows

`SCRIPT_MUTATIONS` rows in `scripts/mutation-rows.d/S45800-S45899.json`, six cells each (id, path,
find, replace, description, killer). Each find occurs once in its file. The weekly finds are each
new step's name line and its `run:` line; the warm-up's are its `if:` and `run:` lines; the Rust
leg's are the spans of R6's line named below.

| stem | file | mutant | killer |
|---|---|---|---|
| `S45800-THE-WEEKLY-RUST-LEG-FETCHES-NOTHING` | `.github/workflows/mutation-weekly.yml` | the `rust` fetch step's `run:` becomes `echo skipped` | A1 |
| `S45801-THE-WEEKLY-RUST-LEG-SWALLOWS-ITS-FETCH` | `.github/workflows/mutation-weekly.yml` | that `run:` gains `\|\| true` | A1 |
| `S45802-THE-REHEARSAL-FETCHES-NOTHING` | `.github/workflows/mutation-weekly.yml` | the `rehearsal` fetch step's `run:` becomes `echo skipped` | A1 |
| `S45803-THE-REHEARSAL-SWALLOWS-ITS-FETCH` | `.github/workflows/mutation-weekly.yml` | that `run:` gains `\|\| true` | A1 |
| `S45804-THE-WARM-UP-FETCHES-NOTHING` | `.github/workflows/rust-cache.yml` | the `warm` fetch step's `run:` becomes `echo skipped` | A2 |
| `S45805-THE-WARM-UP-SWALLOWS-ITS-FETCH` | `.github/workflows/rust-cache.yml` | that `run:` gains `\|\| true` | A2 |
| `S45806-THE-WARM-UP-FETCHES-ON-A-HIT` | `.github/workflows/rust-cache.yml` | the fetch step loses its `if:` line | `test_rust_cache_workflow.TheRustCacheWorkflow.test_it_looks_the_exact_key_up_and_every_later_step_stops_on_a_hit` |
| `S45807-THE-RUST-LEG-PASSES-A-RED-BASELINE` | `.github/workflows/ci.yml` | `case "$rc" in 0\|2\|3) ;;` becomes `case "$rc" in 0\|2\|3\|4) ;;` | A4 |
| `S45808-THE-RUST-LEG-FAILS-ON-ITS-SURVIVORS` | `.github/workflows/ci.yml` | it becomes `case "$rc" in 0\|3) ;;` | A4 |
| `S45809-THE-RUST-LEG-FAILS-ON-ITS-TIMEOUTS` | `.github/workflows/ci.yml` | it becomes `case "$rc" in 0\|2) ;;` | A4 |
| `S45810-THE-RUST-LEG-FAILS-WHEN-ALL-ARE-CAUGHT` | `.github/workflows/ci.yml` | it becomes `case "$rc" in 2\|3) ;;` | A4 |
| `S45811-THE-RUST-LEG-NEVER-FAILS` | `.github/workflows/ci.yml` | `the verdict reads it VOID"; exit "$rc" ;; esac` becomes `the verdict reads it VOID"; true ;; esac` | A4 |

A1, A2 and A4 name the killer tests of section 3. Every killer reads a workflow, so each row is
NOT MEASURED LOCALLY and CI's `mutation-rows` legs decide it by id.

## 8. What only CI proves

A1 to A6's reds and greens, and the rows' census and proof, are CI's, read by name in the `hygiene`
and `mutation-rows` jobs. The release's next run shows, by job id, each `mutation-rust` leg that
exits outside 0, 2 and 3 concluding `failure` and each other concluding `success`. R2's and R4's
effect is read on the first scheduled runs after a release carries them to `main`.

## 9. Formal model

None (ADR-475 D8). Each changed step runs inside one job's own runner and writes no object another
job reads before every leg has ended; R6's line decides a job's colour, not the gate's verdict,
which already fails closed on the same exits. No `@phx covers` names a workflow file or
`scripts/tests/`, and no covered span of `scripts/mutation-verdict.py` (`shards`, `whole_reports`,
`partition`, `examined_sum`, `population_gaps`, `python_by_cost`) is edited.

## 10. Delivered by the next pull request: the weekly rows' legs

Built after the owner's signed ruling for R15's ceiling is on `dev` (ADR-475 D5, D9), under this
SPEC's number and band, with its own red-first lines appended to `docs/red-first/SPEC-458.md`.

- **R10.** `scripts/mutation_rows.py legs` takes `--all` beside `--rows-from`, one of them required,
  and with `--all` counts every row; nothing else in `legs`, `prove` or `merge` changes.
- **R11.** The weekly `size` job runs, before its Rust steps, a step with `id: rows` and
  `if: ${{ !inputs.package }}` that runs `python3 scripts/mutation_rows.py legs --all`, and outputs
  `rows_legs` and `rows_matrix` from it.
- **R12.** The weekly `rows` job needs `size`, runs `strategy: fail-fast: false` over
  `leg: ${{ fromJSON(needs.size.outputs.rows_matrix) }}`, keeps `timeout-minutes: 120` and its step
  named `every hand-proved row`, which proves `prove --all --leg "$LEG/$LEGS"` into
  `$RUNNER_TEMP/mutation/rows.json`, and uploads it as `rows-leg-${{ matrix.leg }}`.
- **R13.** `mutation-verdict.py battery` takes `--rows-legs N`: with it, the rows are promised as N
  reports `rows-leg-<k>/rows.json`, `k` from 0 to N-1, each named `MISSING rows-leg-<k>` or
  `PARTIAL rows-leg-<k>` when absent or proving no row, and a `rows-leg-<k>` at or past N is
  `FOREIGN`; without it the battery reads `rows/rows.json` as today, which the rehearsal lays.
- **R14.** The survivors job's battery step passes `${ROWS_LEGS:+--rows-legs "$ROWS_LEGS"}` from
  `needs.size.outputs.rows_legs`; its drafting step, which reads every `rows.json` under the
  reports, is unchanged.
- **R15.** `LEG_CEILING["battery"]` is 218 and its comment names 38 other jobs of
  `mutation-weekly.yml`; `test_dispatch_shards.py`'s `generated()` counts the weekly rows axis at
  `ROWS_MAX_LEGS`; every test pin of 234, or of a figure derived from it, follows and keeps its
  relation: the refused and sized pair 15913 and 15912 becomes 14825 and 14824 at 68 ingest mutants
  a leg, the register entries naming those calls follow, and 14213 still sizes at 210.
- **R16.** Rows from `S45820` on each new workflow and script line R10 to R15 hold, each killed by
  its criterion's test.

| id | criterion | delivered by |
|---|---|---|
| A7 | `legs --all` writes `rows_legs` and `rows_matrix` for every row, and refuses past `ROWS_MAX_LEGS` by name (R10) | `test_mutation_rows`, a test named for it in the next pull request |
| A8 | the weekly `size` and `rows` jobs read as R11 and R12 state, the rows matrix on `needs.size.outputs.rows_matrix`, each leg uploading `rows-leg-<k>` (R11, R12) | `test_mutation_workflows`, a test named for it in the next pull request |
| A9 | `battery --rows-legs 3` over a planted store names a missing leg, a leg proving no row and a foreign leg, and counts three whole legs whole (R13) | `test_mutation_workflows`, a test named for it in the next pull request |
| A10 | the survivors step passes `--rows-legs` from the size job's output (R14) | `test_mutation_workflows`, a test named for it in the next pull request |
| A11 | `LEG_CEILING` equals the derivation, `{"ci": 190, "battery": 218}`, and every pin of R15 follows (R15) | `test_dispatch_shards` and `test_mutation_verdict`, their ceiling tests |
| A12 | each row of R16 is killed by its criterion's test (R16) | CI's `mutation-rows` legs, by id |
