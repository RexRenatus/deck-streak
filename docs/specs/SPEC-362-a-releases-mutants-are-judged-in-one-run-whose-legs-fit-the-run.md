# SPEC-362: a release's mutants are judged in one run whose legs fit the run

- **Issue:** the release pull request #691, refused at its plan; the scheduled battery's re-sizing,
  #597. **Context(s):** CI (`.github/workflows/`) and the mutation scripts (`scripts/`); no crate
  changes.
- **Decided by:** ADR-373 (one run, one matrix of round-robin legs sized to half the hosted job's
  limit, bounded by the run's job budget, with the per-mutant timeout covering the census).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-362.md`. **Mutation band:** S36200-S36299. **Model:**
  `formal/tla/EveryLegCounted`. Amends SPEC-039 R18, SPEC-129 R4 and SPEC-327 R2, R7 and R8, each by
  an amendment line in its own file; it retires SPEC-129 A2 and SPEC-087 A14, each in SPEC-056 R14's shape. The per-mutant budget and the leg's 360 minutes are the
  owner's signed ruling, `docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2200.md`.

## 1. The problem, measured

A release pull request into `main` is judged on its own merge diff (SPEC-039 R3). Its plan refuses
before any mutant runs, so `ci` cannot pass and `main` cannot move:

- **The refusal.** Run 37410215011's `mutation-plan` job (release pull request #691) read 2890
  changed paths and listed 8367 mutants in 26 packages: every mutant the merged tree holds, since
  `main` holds nothing past its first tag. `shards` projected them at 1,023,555 s serially and
  refused: more than 256 shards within 3600 s each. `mutation-verdict` failed after it, and `ci`
  needs both. Read with `gh run view 37410215011 --log --job 112096748975`.
- **The bound is the cause, not the matrix.** A shard is bounded by half of its job's
  `timeout-minutes: 120` (R18). A hosted job runs for up to 360 minutes, so the bound is a sixth of
  what a leg may run. At 10800 s the same listing takes 200 legs, with the baseline and the
  census re-measured (below). Recomputed from the plan artifact's `listed.json` by `shards`' own
  round-robin projection, which charges each census package's term once more in every leg's
  baseline (SPEC-327 R4); a projection that charges the census only inside the measured baseline
  reads 144.
- **256 is per run, and the run has other jobs.** A workflow run generates at most 256 jobs from
  its matrices, and GitHub Actions' limits page states it per run. `ci.yml` runs 16 jobs, of which
  every job but `mutation-rust` can generate 47 at most (`mutation-python` 32, `engine` 2, and 13
  single jobs), so `mutation-rust` can hold 209 legs. `mutation-weekly.yml`'s other jobs can
  generate 22, so its legs can number 234. `MAX_SHARDS = 256` names neither. Counted from each
  workflow file's jobs and matrices at `dev` 25db4f6.
- **The baseline and the census are stale.** `BASELINE_SECONDS = 371` is the mean of the battery's
  baselines over 7 packages. A leg over the whole listing tests every package, and the `rust` job's
  nextest run of the workspace took 1717 s (run 37410215011, job `rust`, `Summary [1717.322s]
  1585 tests run`). Its slowest test, `deck-streak-progression::xp_census
  only_progression_writes_xp_settlement_and_only_coordination_settles`, took 1144.5 s there and
  1378.5 s in `dev`'s push run 37392351782, against `CENSUS_SECONDS = 788`.
- **Readings at this delivery's base.** The push run at the base commit, 37445796358, read that
  census test at 1441.022 s and the workspace's nextest run at `Summary [2057.103s]` over 1588
  tests. They are readings, not constants: `BASELINE_SECONDS` and `CENSUS_SECONDS` keep the figures
  the signed ruling names (R5, R6), and R7 holds each leg to its own slowest baseline test.
- **A census-only kill reads as a timeout.** The per-mutant `--timeout` is 1200 s (SPEC-327 R1), so a
  progression mutant that only the census catches, and one that no test catches, both run past it
  when the census takes 1378 s. cargo-mutants reports `Timeout`, which the verdict counts as
  examined and never as failing: a surviving mutant can pass the gate.
- **The battery cannot judge the tree.** `mutation-weekly.yml` deals the whole tree into 32 legs
  (`WHOLE_SHARDS`, SPEC-129 R4, SPEC-327 R8) of `timeout-minutes: 120`. At 8367 mutants each leg is
  projected at about 46,000 s at the re-measured prices (33,000 s at today's), against the leg's
  7200, so each is cut at its timeout, uploads a partial report or none, and
  the battery names 32 legs VOID: the runner time is spent and nothing is judged.
- **Prices.** 19 of the 26 packages have no measured price and are charged the table's highest,
  126 s. Five of them are measured since, from 125 pull-request shard reports: `ffi` 17.1 s,
  `push` 5.5, `engine-core` 3.8, `web-engine` 2.2 and `fsrs7` 1.6 per mutant (means).

## 2. Requirements

R1. **One run, one matrix.** The release pull request's Rust mutants run in `ci.yml`'s one
    `mutation-rust` matrix, as every pull request's do: `mutation-plan` sizes it, every leg's
    report goes to `mutation-verdict`, and `ci` needs the verdict. No release case defers a
    mutant, a row or a leg to any other run or to the pull requests that `dev` merged.
R2. **The bound is half of a hosted job's limit.** Each `mutation-rust` leg, in `ci.yml` and in
    `mutation-weekly.yml`, declares `timeout-minutes: 360`. `SHARD_BOUND_SECONDS` is 10800, half of
    it, as R18 sets the bound at half the leg's timeout. A test reads both workflow files and holds
    the constant equal to half of each leg job's `timeout-minutes`, in seconds.
R3. **The ceiling is the run's job budget.** `LEG_CEILING` names, for each workflow that runs
    legs, 256 less the most jobs every other job of that workflow can generate. A test derives each
    from the workflow file's jobs and matrices and holds the constants equal to it. `shards` and
    `size` take the fewest legs whose slowest is projected within the bound; when that is more
    than the workflow's ceiling, or one mutant alone exceeds the bound, they refuse with the count,
    the ceiling and the projection, and never cap, sample or drop a mutant.
R4. **The plan states its headroom.** Every sizing prints `legs N of ceiling C` and the serial and
    per-leg projections, so a range that approaches its ceiling is seen before it is refused.
R5. **The baseline is the workspace's.** `BASELINE_SECONDS` is the `rust` job's measured nextest
    run of the workspace plus its start, rounded up, held with the run id it came from.
R6. **The census and the per-mutant timeout are re-derived.** `CENSUS_SECONDS` is the slowest
    measured census test plus its start, rounded up. Every cargo-mutants command's `--timeout` is
    `CENSUS_SECONDS` times 1.5, rounded up to the hundred, by SPEC-327 R2's own rule. The test that
    holds the timeout over the census reads the new need.
R7. **A leg proves its own timeout covers its tests.** `mutation-verdict` reads each leg's
    baseline log and names the leg VOID when 1.5 times its slowest single test exceeds the
    per-mutant timeout: a mutant that only that test kills could otherwise read as a timeout.
R8. **The listing, not the plan, is the population.** `mutation-verdict` reads the tool's own
    listing (`listed.json`, uploaded by the plan) and names the run VOID unless the plan's legs
    hold each listed mutant exactly once. The leg loop counts every planned leg from 0 to N-1, as
    R18 states; a leg with no report, or a partial one, stays VOID by name.
R9. **A caught mutant stops at its first failure.** `.config/nextest.toml` gains a `mutants`
    profile with `fail-fast = { max-fail = 1, terminate = "immediate" }`, and every cargo-mutants
    command runs nextest under it. A mutant that a test kills is caught as before; one that no test
    kills runs every test as before. No mutant is skipped, and no price is lowered until a run's
    reports measure it.
R10. **New prices from measurement.** The price table gains `ffi` 18, `push` 6, `engine-core` 4,
    `web-engine` 3 and `fsrs7` 2, each the rounded-up mean of the named pull-request runs' shard
    reports. A package the table does not name is still charged the highest.
R11. **The battery is sized like any diff.** `mutation-weekly.yml`'s `size` job lists the whole
    tree and sizes it with the same function at the battery's ceiling. `WHOLE_SHARDS` is retired;
    the battery's legs, its count check and its verdict read the plan's count. A tree beyond the
    ceiling is refused by name, never capped.
R12. **The release runbook names what a release run needs.** `RELEASING.md` states that a push to
    `dev` cancels the release pull request's run, so `dev` merges nothing while it runs; that the
    plan prints its headroom; and that a release is cut before its range's plan reaches the ceiling.
R13. **The counting surface is modelled.** A TLA+ entry, `formal/tla/EveryLegCounted/`, models legs
    that finish whole, partial or not at all, re-runs of failed legs, and the verdict. Its
    properties `PassMeansEveryLegCounted` and `PassMeansThePartition` hold at its model config, and
    each of its two witnesses is caught.
R14. **The Python matrix is sized to the hosted job.** `PYTHON_SHARD_MUTANTS` is 20, so
    `python_shards` sizes the diff's Python matrix at the ceiling of listed over 20, clamped to 1 to
    32 shards (`PYTHON_MAX_SHARDS` stays 32). Measured: one leg of 34 mutants took 54 minutes 42
    seconds and its sibling was cancelled at the 60-minute job cap, so a full shard of 40 needs about
    64 minutes against the 60-minute job. Every listed mutant is still examined and every shard still
    counted; the same population only spreads over more legs. It retires SPEC-087 A14.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the release's listing at #691 sizes within the ci ceiling at the new bound | `python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_release_listing_fits_one_run` |
| A2 | the bound is half of every leg job's timeout, in both workflows | `python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_bound_is_half_of_every_legs_timeout` |
| A3 | each workflow's ceiling equals 256 less what its other jobs can generate | `python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_ceiling_is_the_runs_job_budget` |
| A4 | a listing beyond the ceiling is refused with its count, never capped | `python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_listing_beyond_the_ceiling_is_refused_whole` |
| A5 | the per-mutant timeout covers the re-derived census with its margin | `python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_gates_timeout_covers_the_census_with_its_margin` |
| A6 | a leg whose slowest baseline test outgrows the timeout is VOID by name | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_leg_whose_slowest_test_outgrows_the_timeout_is_void` |
| A7 | a plan whose legs do not partition the tool's listing is VOID by name | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_plan_that_does_not_partition_the_listing_is_void` |
| A8 | a planned leg with no report is VOID even when every other leg passed | `python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_planned_leg_with_no_report_is_void` |
| A9 | every cargo-mutants command runs nextest's `mutants` profile, which stops at the first failure | `python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_every_mutation_command_runs_the_mutants_profile` |
| A10 | the battery is sized from its listing, at its ceiling, and no longer fixed | `python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_whole_tree_is_sized_from_its_listing` |
| A11 | the plan prints its headroom on every sizing | `python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_sizing_prints_its_headroom` |
| A12 | the Python matrix is at most 20 listed mutants a shard, clamped to 1 to 32: 20 listed read 1 shard, 21 read 2, 68 read 4, and 32 x 20 + 1 read 32 | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_matrix_takes_twenty_listed_mutants_a_shard_clamped_to_thirty_two` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_release_listing_fits_one_run
A2: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_bound_is_half_of_every_legs_timeout
A3: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_ceiling_is_the_runs_job_budget
A4: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_listing_beyond_the_ceiling_is_refused_whole
A5: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_gates_timeout_covers_the_census_with_its_margin
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_leg_whose_slowest_test_outgrows_the_timeout_is_void
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_plan_that_does_not_partition_the_listing_is_void
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_planned_leg_with_no_report_is_void
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_every_mutation_command_runs_the_mutants_profile
A10: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_whole_tree_is_sized_from_its_listing
A11: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_sizing_prints_its_headroom
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_matrix_takes_twenty_listed_mutants_a_shard_clamped_to_thirty_two
```

R13 is decided by the formal check of `formal/tla/EveryLegCounted` at review, not by a unittest,
so it has no line in the fence.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/mutation-verdict.py` | CI scripts | changed: Python shard size 20, bound, ceiling, baseline, census, prices, headroom line, R7 and R8 checks, `size` through `shards`' sizing |
| `.github/workflows/ci.yml` | CI | changed: `mutation-rust` timeout, `--timeout`, nextest profile |
| `.github/workflows/mutation-weekly.yml` | CI | changed: legs sized by the plan, timeout, `--timeout`, nextest profile |
| `.config/nextest.toml` | CI | added: the `mutants` profile |
| `.cargo/mutants.toml` | CI | changed: runs nextest under the `mutants` profile |
| `scripts/tests/test_dispatch_shards.py` | CI scripts | changed: A1 to A5, A10, A11; `TheWholeTreeKeepsThirtyTwo` retired |
| `scripts/tests/test_mutation_verdict.py` | CI scripts | changed: A6 to A8; `cargo_report`'s Baseline names its log, which R7 reads |
| `scripts/tests/test_mutation_workflows.py` | CI scripts | changed: A9 |
| `scripts/tests/test_mutation_python_shard_binding.py` | CI scripts | changed: R14's fixture sizes by a literal 20; the pin test A12 |
| `scripts/tests/test_mutation_python_verdict.py` | CI scripts | changed: its sizing cases and a three-shard fixture follow the 20-mutant rule (the retired A14's test; the cases scale by the same factor) |
| `docs/specs/SPEC-087-the-guard-scripts-and-the-parity-oracle-prove-their-tests-kill-generated-mutants.md` | docs | amended: A14 retired (section 12) |
| `docs/red-first/SPEC-087.md` | docs | changed: A14's lines in a retired fence |
| `scripts/tests/_mutants_finder.py` | CI scripts | changed: the bounds it reads |
| `scripts/tests/test_ci_workflows.py` | CI scripts | changed: the census entries for the new tests' reads |
| `scripts/tests/test_memory_scope.py` | CI scripts | changed: its byte pins read `--timeout 2200` |
| `scripts/tests/test_memory_cap_verdict.py` | CI scripts | changed: its counts derived from the bound (143, 142); assertions unchanged |
| `scripts/tests/fixtures/not-started-legs/empty/mutation-plan/listed.json` | CI scripts | added: an empty listing |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-plan/listed.json` | CI scripts | added: the plan's five mutants, listed |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-rust-shard-0/mutants.out/outcomes.json` | CI scripts | changed: the Baseline names its log |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-rust-shard-0/mutants.out/log/baseline.log` | CI scripts | added: a baseline log that times its test |
| `scripts/mutation-rows.d/S36200-S36299.json` | CI scripts | added: the rows that pin each new constant |
| `scripts/mutation-rows.d/S03900-S03999.json` | CI scripts | changed: S03913's mutant caps at the `ci` ceiling |
| `scripts/mutation-rows.d/S12900-S12999.json` | CI scripts | changed: S12901's mutant caps at the ceiling; S12903 re-anchored to the battery's ceiling; S12904 to S12909 to 2200 |
| `scripts/mutation-rows.d/S32700-S32799.json` | CI scripts | changed: S32700 and S32705 re-anchored to 1430 and 2200; S32706 to the ceiling |
| `docs/red-first/SPEC-362.md` | docs | added: the red-first record |
| `formal/tla/EveryLegCounted/EveryLegCounted.tla` | formal | added |
| `formal/tla/EveryLegCounted/MCEveryLegCounted.cfg` | formal | added |
| `formal/tla/EveryLegCounted/witness/PassMeansEveryLegCounted.cfg` | formal | added |
| `formal/tla/EveryLegCounted/witness/PassMeansThePartition.cfg` | formal | added |
| `RELEASING.md` | release | changed: R12 |
| `docs/schematics/mutation-testing.md` | docs | changed: the plan, legs, verdict and `ci` as drawn in this SPEC's schematic |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | docs | amended: R18's bound and ceiling |
| `docs/specs/SPEC-056-every-pack-is-judged-on-the-box-and-nothing-of-the-hub-is-published.md` | docs | amended: section 7's rows of A2 and of SPEC-087's A14, section 10 |
| `docs/red-first/SPEC-129.md` | docs | changed: A2's lines in a retired fence |
| `docs/specs/SPEC-129-a-package-dispatch-is-sharded-by-its-projected-weight.md` | docs | amended: R4; A2 retired (section 14) |
| `docs/specs/SPEC-327-the-mutation-budget-covers-the-settle-census.md` | docs | amended: R2's need, R7, R8 |
| `changelog.d/release-legs-362.md` | release | added |

## 5. What this does NOT do

- It does not speed the census tests themselves. The progression census is the largest single
  cost a progression mutant carries, and making it cheaper without skipping it is its own work,
  #692.
- It does not shard `mutation-web`. The release's StrykerJS job took 39 of its 60 minutes in run
  37410215011; its growth is tracked by #693.
- It does not add a verdict across runs, or a second matrix: a run's 256 jobs bound one run, and
  ADR-373 records why the release stays in one, #691.
- It does not change which head reaches `main`: `base-is-dev` and ADR-034's model stand, #691.
- It does not lower any price on the strength of R9 before a run's reports measure it, #597.
- It does not triage the tree's surviving mutants: the first whole-tree run names them, and each
  is fixed on `dev` as any survivor is, #597.

## 6. Risks

- **The tree outgrows one run.** At today's prices the whole tree needs 200 legs of a ceiling of
  209 in `ci.yml` and 234 in the battery. The count is set by progression's 399 mutants, each
  charged its census, two to a leg, so a range with a few more progression mutants needs more
  than `ci.yml` holds; the plan's headroom line (R4) shows the approach, and the refusal names it.
  Detected by the headroom line on every plan and by the battery's own sizing.
- **A leg outruns its projection.** The prices are means; a new slow test in a priced package (the
  `ingest` suite holds one of 575 s, against a price of 126 s per mutant) makes its legs slower. The
  bound is half the leg's limit, and a leg cut at its timeout is VOID by name, never green.
- **A red `dev` voids every leg.** cargo-mutants refuses to test mutants when its baseline fails,
  so a release cut from a `dev` whose suite is red reads every leg VOID. Detected by the `rust` job
  of the same run.
- **The first whole-tree run reads the tree's survivors.** Code merged before per-diff gating was
  never judged as a whole; its survivors fail the first release and the first battery by name.
  That is the gate measuring, and it is fixed on `dev`.
- **A runner can be shut down mid-leg.** The leg uploads a partial report or none, the verdict
  names it VOID, and "Re-run failed jobs" runs that leg and the verdict again (SPEC-039).
- **Wall-clock follows the account's concurrent-job limit.** Legs run as many at a time as that limit
  allows, so the whole tree's first run is long; the matrix sets no `max-parallel`, and the first
  battery's wall-clock decides whether one is owed.
- **The timeout raise needs a signed ruling.** `--timeout` 2200 and the leg's 360 minutes raise a bound
  that SPEC-327 set under a signed ruling. The owner signed the raise:
  `docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2200.md`, on `dev` before this delivery.
- **The ceiling reading.** The ceiling counts every job the run can generate against 256 (all jobs count). If the
  limit counts matrix jobs only, the ceiling is conservative, never too large.

## Amendment: the per-mutant budget is 2300 seconds

Insert-only; every earlier byte is kept. The owner's signed ruling
`docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2300.md` supersedes three figures of the 2200
ruling: `--timeout` becomes 2300 at the ten places that hold the bound equal (`.github/workflows/ci.yml`,
`.github/workflows/mutation-weekly.yml` and `BOUNDS` in `scripts/tests/_mutants_finder.py`),
`CENSUS_SECONDS` becomes 1521 and `BASELINE_SECONDS` 2141, each held with run 37438835490. R6 and R7
read those figures, so R7's margin is the same 1.5: a census whose 1.5 times passes 2300 (a test over
1533 s) still reads VOID by name, never a pass. `--build-timeout 600`, each Rust leg's
`timeout-minutes` of 360 and `SHARD_BOUND_SECONDS` of 10800 are unchanged. The pins and rows that quote
the figures follow them (`test_memory_scope.py`, `test_dispatch_shards.py`, `test_mutation_verdict.py`,
`test_memory_cap_verdict.py`, `test_ci_workflows.py`, and the bands S12900, S32700 and S36200), and a
row's id keeps the figure it was named for.

## Amendment: the web legs' bound is 100 minutes (SPEC-379)

Insert-only; every earlier byte is kept. Section 5 records the release's StrykerJS job inside its
60 minutes and leaves the web sweep's growth to #693. The weekly battery's whole sweep has since
been cut at that bound (job 113826350088), so SPEC-379 raises both web legs' `timeout-minutes` to
100 by measurement, held inside 100 to 120 (ADR-390). The web sweep is still not sharded, and its
StrykerJS settings are unchanged: sharding stays #693.
