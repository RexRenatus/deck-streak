# SPEC-416: every mutant leg fetches the locked graph before its baseline

- **Issue:** #827, which records the release run's red baselines and the weekly legs' same
  gap; this delivery refers to it and closes nothing. **Context(s):** CI (`.github/workflows/`)
  and its tests (`scripts/tests/`); no crate, no test under `crates/` and no verdict code changes.
- **Decided by:** ADR-430 (one fetch step in each job that runs a cargo-mutants baseline, judged
  by SPEC-332's judge; the verdict already reads a red baseline as VOID and is unchanged).
- **Schematic:** `docs/schematics/ci-jobs-and-caches.md`, a section appended for SPEC-416: the
  leg's restore, fetch, baseline and report, before and after, and how a red baseline reaches the
  verdict.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-416.md`. **Mutation band:** `S41600-S41699` (section 9). **Model:** none
  (section 8). Amends SPEC-332 (its section 5's exclusion of a job that runs an offline cargo call
  without proving rows) by an amendment section in its own file.

## 1. The problem, measured

Every number below was read from one CI job's log through `gh api` on the named job id, during
the release pull request #822's run, which was then cancelled.

- **The legs examined nothing.** Job 114529706626 (`mutation-rust`, shard 0) read `Cache not
  found` (log line 252), listed 49 mutants, and then `FAILED   Unmutated baseline in 345s build +
  935s test` (line 387). Job 114529706626's step took 1283 s and ended `cargo-mutants exited 4`
  (line 6137); the job concluded success. Every completed `mutation-rust` leg of the run read the
  same baseline failure.
- **The failing test is the progression census.** It runs `cargo metadata --format-version 1
  --locked --offline --all-features` (`crates/progression/tests/xp_census.rs:714-716`) and, in
  that job, failed in 3.548 s with `failed to download` a package (`accessory`, line 6120); fail-
  fast then stopped three other tests.
- **The jobs that fetch first passed on the same miss.** Job 114529617445 (`rust`) also read a
  cache miss (line 287), and its `cargo fetch --locked --manifest-path Cargo.toml` step
  (job 114529617445) ran 36 s and downloaded that package (line 339); the census passed. Job
  114529617364 (`release`) fetched in 39 s and was green.
- **Why.** A pull request into `main` restores no cache written on `dev`. A build downloads only
  the packages it compiles; the census asks for every target's, offline. `ci.yml`'s `rust`
  (`:65-66`), `release` (`:187-188`) and `mutation-rows` (`:622-624`) jobs fetch after their
  restore; `mutation-rust` (`:447-524`) restores at `:485-496` and runs cargo-mutants at
  `:507-518` with no fetch between.
- **The verdict already fails closed.** cargo-mutants' exit 4 is a red baseline
  (`scripts/mutation-verdict.py:134-140`); `partial_reason` refuses every exit outside
  `WHOLE_EXITS = {0, 2, 3}` (`:133`, `:1203-1206`); `whole_reports` marks the leg VOID (`:1260`);
  `Verdict.close` returns `EXIT_VOID` (`:1134`); the judge step exits with it (`ci.yml:689`,
  `:699`), and the aggregate `ci` job fails (`:904`). The leg's green colour comes from its step
  recording the exit instead of returning it (`ci.yml:515-518`); that is noted, not changed.

## 2. Requirements

- **R1.** Every job in `.github/workflows/ci.yml` that runs a cargo-mutants baseline, meaning a
  step whose code runs `cargo mutants` without `--list`, runs `cargo fetch --locked` in a step
  after its last cache-restore step and before its first such step.
- **R2.** That fetch step swallows no failure: no `||`, no `set +e` and no `continue-on-error`.
- **R3.** The fetch step's `if:` is the `if:` of the job's cargo-mutants step, so a plan with no
  Rust work skips both and a plan with Rust work runs both.
- **R4.** The fetch step's `run:` line is `cargo fetch --locked --manifest-path Cargo.toml`, the
  `rust` and `release` jobs' own, so the rows job's `run: cargo fetch --locked` line, which
  S33200 and S33201 anchor on, stays the only one of its kind in the file.
- **R5.** The census keeps `--locked --offline --all-features`, and no test is skipped, filtered
  or changed under `crates/`.
- **R6.** The verdict's reading of a red baseline is unchanged: no edit to
  `scripts/mutation-verdict.py`.
- **R7.** The judge is SPEC-332's: `refusals()` in `scripts/tests/test_rows_jobs_fetch_graph.py`
  takes the anchor step it orders the fetch before, and with its default it returns SPEC-332's
  messages byte for byte.

## 3. Acceptance criteria of SPEC-416

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | every `ci.yml` job that runs a cargo-mutants baseline fetches the locked graph after its last cache restore and before cargo-mutants, swallowing nothing, under the cargo-mutants step's `if:`; the population names `mutation-rust` and reports its examined count (R1, R2, R3) | red at the base, where `mutation-rust` holds no fetch: `ci.yml:mutation-rust: no step runs cargo fetch --locked` | `EveryMutantsJobFetchesTheLockedGraph.test_each_baseline_job_fetches_after_its_cache_restore_and_before_cargo_mutants` |
| A2 | the judge refuses, by name, each planted shape of a baseline job: no fetch, a fetch before the restore, a fetch after cargo-mutants, `\|\| true`, `set +e`, `continue-on-error`, and an `if:` that differs; the fixed shape passes (R1, R2, R3) | not red: it plants its own shapes into copies of the job, so it pins the judge, not the tree | `TheJudgeRefusesEachPlantedBaselineShape.test_the_judge_refuses_each_planted_shape_on_a_baseline_job` |
| A3 | SPEC-332's tests pass unchanged with the judge's anchor as a parameter (R7) | not red: they pin the behaviour the base already had, and guard the refactor | `EveryRowsJobFetchesTheLockedGraph` and `TheJudgeRefusesEachPlantedShape`, unchanged |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_each_baseline_job_fetches_after_its_cache_restore_and_before_cargo_mutants
A2: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_the_judge_refuses_each_planted_shape_on_a_baseline_job
A3: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k EveryRowsJobFetchesTheLockedGraph -k TheJudgeRefusesEachPlantedShape
```

All three are read red and green from CI's `hygiene` job, which runs `scripts/tests` through
`scripts/check.sh python`. No test module runs off CI for this delivery.

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/ci.yml` | CI | changed: one step in `mutation-rust`, right after its cache restore (ADR-430, Decision Outcome) |
| `scripts/tests/test_rows_jobs_fetch_graph.py` | CI tests | changed: the baseline-job population, `refusals()`'s anchor parameter, A1 and A2 |
| `scripts/mutation-rows.d/S41600-S41699.json` | CI scripts | added: rows S41600 to S41602 (section 9) |
| `docs/specs/SPEC-416-every-mutant-leg-fetches-the-locked-graph-before-its-baseline.md` | docs | added: this file |
| `docs/decisions/ADR-430-every-mutant-leg-fetches-the-locked-graph-in-its-own-step.md` | docs | added |
| `docs/specs/SPEC-332-every-job-that-proves-mutation-rows-fetches-the-locked-graph-first.md` | docs | changed: an amendment section appended, naming SPEC-416 |
| `docs/schematics/ci-jobs-and-caches.md` | docs | changed: a section appended for SPEC-416 |
| `docs/red-first/SPEC-416.md` | docs | added: A1 to A3 |
| `changelog.d/mutation-baseline-416.md` | docs | added: the changelog fragment |

## 5. What this does NOT cover

- The weekly battery's `rust` leg (`.github/workflows/mutation-weekly.yml`) runs cargo-mutants
  with no fetch step, and its rehearsal fetches only after its cargo-mutants run. Both run on
  `dev`, whose saved cache the `rust` job filled after its own fetch, and neither is part of the
  release's re-run; both stay open under #827.
- It does not size or deal the rows leg, which was cut at its bound in the same run (#822); that
  is SPEC-417's.
- It does not change the leg's colour: the step records cargo-mutants' exit and the verdict reads
  it (section 1); a leg that turns red on its own exit is not asked for by #822 or #827.
- It adds no fetch to `rust-cache.yml`'s warm-up, whose entry a pull request into `main` cannot
  restore (#827).

## 6. Risks

- **A registry outage fails the fetch.** The leg then uploads no report, which the verdict reads
  as VOID by name; a re-run after the outage is the remedy. Detected by the leg's failed step.
- **The leg grows by its fetch.** About 40 s a leg (jobs 114529617445 and 114529617364), inside
  the half-bound margin each leg's projection keeps (SPEC-362 R2). Detected by the next release
  run's leg times, read by job id.
- **A test needs a package the lock does not hold.** Then the fetch passes and the baseline still
  fails on a download; the verdict reads VOID, and the fix is in `Cargo.lock`.
- **Another job gains a cargo-mutants baseline.** A1's population is found by what a job runs, so
  the new job is held to the rule on the pull request that adds it.

## 7. What only CI proves

A1's red and green, A2's and A3's passes, and the rows' proof are CI's, read by name in the
`hygiene` and `mutation-rows` jobs. The release's re-run then shows each `mutation-rust` leg past
its baseline, examining its mutants, read by job id.

## 8. Formal model

None. One step inside one job's own runner adds no actor, no shared state and no write path: the
leg saves no cache, and its report is its own. No `@phx covers` names `ci.yml` or
`test_rows_jobs_fetch_graph.py`, and `scripts/mutation-verdict.py`, whose spans
`formal/tla/EveryLegCounted` covers, is not edited.

## 9. Mutation rows

Each row's find is the fetch step's last two lines,
`        if: ${{ needs.mutation-plan.outputs.rust == 'true' }}\n        run: cargo fetch --locked --manifest-path Cargo.toml\n`,
which occur once in `ci.yml` (the `rust` and `release` jobs' fetch lines follow a `- name:` line).

| stem | file | mutant | killer |
|---|---|---|---|
| `S41600-THE-MUTANT-LEG-FETCHES-NOTHING` | `.github/workflows/ci.yml` | the step's `run:` becomes `echo skipped` | `test_rows_jobs_fetch_graph.EveryMutantsJobFetchesTheLockedGraph.test_each_baseline_job_fetches_after_its_cache_restore_and_before_cargo_mutants` |
| `S41601-THE-MUTANT-LEG-SWALLOWS-ITS-FETCH` | `.github/workflows/ci.yml` | the step's `run:` gains `\|\| true` | the same |
| `S41602-THE-MUTANT-LEG-FETCHES-UNDER-ANOTHER-IF` | `.github/workflows/ci.yml` | the step's `if:` reads `== 'false'` | the same |
