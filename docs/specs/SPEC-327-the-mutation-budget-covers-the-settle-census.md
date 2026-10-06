# SPEC-327: the mutation budget covers the settle census

- **Issue:** none of its own. The owner's signed ruling
  `docs/rulings/OWNER-RULING-2026-10-03-mutation-timeout-1200.md` admits the raise; the weekly
  battery's re-sizing is its follow-up, #597. **Context(s):** CI (`.github/workflows/`) and the
  mutation scripts (`scripts/`); no crate changes.
- **Decided by:** ADR-328 (the per-mutant budget is 1200 seconds and the sizer charges the census).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-327.md`. **Mutation band:** `S32700-S32799`.

## 1. The problem, measured

Every number below is read from this repository's own CI, run 37131363071 (pull request #592, head
`9e746c94`), by `gh api` of its job logs and artifacts.

- **Every Rust mutation shard refused itself.** Each `mutation-rust` shard read `TIMEOUT Unmutated
  baseline in 71s build + 300s test` (job 111230487093, log line 401), `57s` (job 111230486891,
  :402) and `70s` (job 111230485905, :400). cargo-mutants then refused the shard: `cargo test failed
  in an unmutated tree, so no mutants were tested` (`mutants.out/debug.log`:12 of shard 2's
  artifact), and the pull request's mutation verdict read VOID.
- **One flag bounds the whole baseline.** The baseline ran `cargo nextest run` over
  `deck-streak-analytics`, `-api`, `-coordination` and `-progression` (debug.log:5), 400 tests
  (`Starting 400 tests across 100 binaries`). The run's `--timeout 300` bounds that whole test
  phase: debug.log:10 reads `process_status=Timeout elapsed=300.026895675s`.
- **Two tests outlive it.** nextest started the settle census's two tests between 29.1 and 50.8 s
  into the run (each shard's `Summary` of 296.2 to 298.8 s less the test's `SIGTERM` of 245.4 to
  269.6 s), so the timeout killed them at 245 to 270 s, unfinished.
- **What they cost in full.** The same run's `rust` job, which has no such bound, passed both
  (`check-stage-logs-rust` artifact, `test.log`):
  `the_census_refuses_every_caller_the_compiler_finds` in 652.931 s (:1371) and
  `only_progression_writes_xp_settlement_and_only_coordination_settles` in 736.931 s (:1373);
  `Summary [ 815.660s] 1319 tests run: 1319 passed` (:1375).
- **So the baseline's test phase needs about 788 s**: the slower test's 737 s after the latest start
  of 51 s, each rounded up. cargo-mutants runs each mutant against its own package's tests
  (`.cargo/mutants.toml` sets no `test_package` or `test_workspace`; its one key is
  `test_tool = "nextest"`, :17), so every mutant of `deck-streak-progression` runs both census
  tests too.
- **Control.** At pull request #568's head (`2f9847df`, run 37104314029), before the census
  landed, every one of the ten `mutation-rust` shards read `ok Unmutated baseline`, with test
  phases of 98 to 152 s.
- **The trap a cure must not open.** A `Timeout` outcome counts as killed in the weekly table
  (`scripts/mutation-verdict.py`:2959-2960) and does not fail the pull request's verdict (ADR-197,
  lines 208-210). A cure that let the baseline pass while each mutant's budget stayed under 788 s
  would score every progression mutant killed by timeout, a false kill. One `--timeout` bounds
  both the baseline and each mutant, so one raise cures both.
- **The sizer does not know the census.** `scripts/mutation-verdict.py` projects each mutant at its
  package's cost in `SECONDS_PER_MUTANT` and each shard's baseline at `BASELINE_SECONDS = 371`.
  Progression is not in that table and costs its highest, 126 s. #592's plan, from its own
  listing of 67 mutants (analytics 4, api 8, coordination 10, progression 45, in that order), took
  3 shards projected at 2877, 2741 and 2741 s, 8359 s in total. With the census paid, those three
  shards would need 15485, 15349 and 15349 s, past the job's `timeout-minutes: 120`
  (`.github/workflows/ci.yml`:363, 7200 s).

## 2. Requirements

R1. Every `cargo mutants` command in `.github/workflows/ci.yml` (lines 331, 337 and 428 at the
    base) and `.github/workflows/mutation-weekly.yml` (lines 91, 93, 168, 170, 322 and 437) carries
    `--timeout 1200 --build-timeout 600`, and `BOUNDS` in `scripts/tests/_mutants_finder.py` reads
    the same four words. `--build-timeout 600` is unchanged.
R2. 1200 is the census's measured need, 788 s, times a 1.5 margin (1182), rounded up to the next
    hundred. The margin covers the spread of 0.66 to 1.33 between projected and measured shard times
    recorded beside the sizer.
    Amended by SPEC-362 (R6): the need is re-derived by this rule. `CENSUS_SECONDS` is 1430, the
    census test's 1378.452 s in push run 37392351782 plus its start; times 1.5 that is 2145, and
    rounded up to the next hundred it is 2200, the figure of the owner's signed ruling that
    SPEC-362 cites. R1's commands and `BOUNDS` read `--timeout 2200`.
R3. `scripts/mutation-verdict.py` holds `CENSUS_SECONDS = {"deck-streak-progression": 788}`, a table
    apart from `SECONDS_PER_MUTANT`. Each listed mutant of a package it names is projected at its
    table cost plus that term; a mutant of any other package is projected exactly as before.
R4. A plan's baseline is `BASELINE_SECONDS` plus the census term of each distinct package its
    listing names, paid once per package however many of its mutants are listed. The plan's
    `baseline_seconds` is that value, and a listing that names no census package keeps 371.
R5. The pull request's plan (`shards`) and a package dispatch (`size`) both size by that baseline
    and those costs through the one `fewest_shards` (SPEC-129 R2). `projected` takes the baseline
    as an argument; `fewest_shards` defaults it to `BASELINE_SECONDS`.
R6. Rows S12904 to S12909, whose finds hold the weekly legs' `--timeout 300`, are re-anchored on the
    new text under their own ids, and S03912 and S03913, whose finds hold the two projection calls
    this change gives a baseline argument, are re-anchored under theirs. No row is retired.
R7. The census's two tests run for every mutant of `deck-streak-progression` exactly as at the base.
    No nextest filter or profile, `.cargo/mutants.toml` key, skip attribute or test-name filter is
    added, and no file under `crates/` changes.
R8. The weekly battery's whole-tree shard count stays `WHOLE_SHARDS = 32` (SPEC-129 R4, row
    S12903).
    Amended by SPEC-362 (R11): `WHOLE_SHARDS` is retired. The battery sizes the whole tree from its
    listing at its own ceiling, `LEG_CEILING["battery"]`, and row S12903, under its own id, pins
    that ceiling.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the gate's per-mutant budget covers the census's measured need with a 1.5 margin: the `--timeout` of the pinned bounds is at least 1182 (from 788), and the sizer's census term is that 788, the slower census test after its latest start | `test_dispatch_shards.TheGatesTimeoutCoversTheCensus.test_the_gates_timeout_covers_the_census_with_its_margin` |
| A2 | every `cargo mutants` command in every workflow carries the new bounds (the existing guard, against the changed pin) | `test_dispatch_shards.EveryMutationCommandKeepsTheGatesBounds.test_every_cargo_mutants_command_carries_the_gates_own_bounds` |
| A3 | each mutant of a census package is projected at its table cost plus its census term; a mutant of another package is projected exactly as at the base | `test_mutation_verdict.ACensusPackagePaysTheCensus.test_each_mutant_of_a_census_package_is_projected_with_the_census` |
| A4 | a plan whose listing names a census package pays the term once in its baseline, and `baseline_seconds` in the plan is that value; a plan naming none keeps 371 | `test_mutation_verdict.ACensusPackagePaysTheCensus.test_a_plan_naming_a_census_package_pays_the_census_once_in_its_baseline` |
| A5 | #592's listing, as a fixture of its 67 packages in listing order, shards into 23 legs, the slowest projected at 3113 s within the bound | `test_mutation_verdict.ACensusPackagePaysTheCensus.test_the_592_listing_fits_its_bound` |
| A6 | a package dispatch (`size`) of progression is sized with the census term and baseline too | `test_mutation_verdict.ACensusPackagePaysTheCensus.test_a_dispatch_of_a_census_package_is_sized_with_the_census` |
| A7 | the byte pins of each workflow command carry the new bound (the pull request's leg, the weekly legs, the rehearsal leg, the listings, and the weekly sweep's head commands) | `test_memory_scope.py`, and `test_dispatch_shards.py`'s pins by their existing names |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_gates_timeout_covers_the_census_with_its_margin
A2: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_cargo_mutants_command_carries_the_gates_own_bounds
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_each_mutant_of_a_census_package_is_projected_with_the_census
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_plan_naming_a_census_package_pays_the_census_once_in_its_baseline
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_592_listing_fits_its_bound
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_dispatch_of_a_census_package_is_sized_with_the_census
A7: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py
```

A1, A3, A4, A5 and A6 are new tests; A2 and A7 are existing guards whose pins this delivery changes,
so they turn red against the base's workflows and green when the workflows carry the new bound.
A3 to A6 assert exact `projected_seconds`, `baseline_seconds` and output values computed from
literals, never from the module's own constants. Two progression mutants and one coordination
mutant in one shard project 3051 s (1159 + 914 + 914 + 64), where the base projects 687 s
(371 + 126 + 126 + 64).

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/ci.yml` | CI | `--timeout 300` to `--timeout 1200` on the two listings and the run step; the plan's comment names the census term |
| `.github/workflows/mutation-weekly.yml` | CI | `--timeout 300` to `--timeout 1200` on its six commands; the timeouts comment names the baseline's tests |
| `scripts/mutation-verdict.py` | scripts | `CENSUS_SECONDS`, the census term in `mutant_costs`, `baseline_seconds`, and the baseline passed through `projected` and `fewest_shards` by `shards` and `size` |
| `scripts/tests/_mutants_finder.py` | scripts | `BOUNDS` reads `--timeout 1200 --build-timeout 600` |
| `scripts/tests/test_dispatch_shards.py` | scripts | A1's class; every fixture that stands for the gate's bounds re-spelled from `BOUNDS`; the head commands' pin at the new bound |
| `scripts/tests/test_memory_scope.py` | scripts | the leg, rehearsal and listing pins at the new bound |
| `scripts/tests/test_mutation_verdict.py` | scripts | A3 to A6's class |
| `scripts/tests/test_ci_workflows.py` | scripts | the workflow-read census lists A1's one new call of the verdict module's loader |
| `scripts/mutation-rows.d/S12900-S12999.json` | scripts | S12904 to S12909 re-anchored on the new bound |
| `scripts/mutation-rows.d/S03900-S03999.json` | scripts | S03912 and S03913 re-anchored on the projection calls that now pass the baseline |
| `scripts/mutation-rows.d/S32700-S32799.json` | scripts | added: this delivery's six rows (section 8) |
| `docs/schematics/mutation-testing.md` | docs | the bound at its three sites, the census term in the plan's step, and section 7 |
| `docs/specs/SPEC-327-the-mutation-budget-covers-the-settle-census.md` | docs | added |
| `docs/decisions/ADR-328-the-per-mutant-budget-is-1200-seconds-and-the-sizer-charges-the-census.md` | docs | added |
| `docs/red-first/SPEC-327.md` | docs | added |
| `changelog.d/mutation-baseline-327.md` | changelog | added |

This SPEC is the only record of the change. SPEC-039, ADR-057, SPEC-129, SPEC-290, ADR-290, SPEC-072
and ADR-197 keep their text, and each states the old bound as the bound of its own time:

- SPEC-039:517, "`--build-timeout 600` beside `--timeout 300`".
- ADR-057:112, "`--timeout 300` bounds each mutant's tests, above the engine's 140 s".
- SPEC-129:31, :56-57, :81 (A6), :106, :118 and :195, each naming `--timeout 300` with
  `--build-timeout 600`.
- SPEC-290:79 and ADR-290:116, "`--timeout 300 --build-timeout 600`".
- SPEC-072:709, "under a timeout of 300 s, which the killer alone exceeds".
- ADR-197:208-209, "the mutation battery's per-mutant timeout of 300 s".

From this delivery on, the bound is `--timeout 1200 --build-timeout 600`, and where those texts name
300 they describe the bound before it.

## 5. What this does NOT do

- It does not re-size the weekly battery's whole-tree sweep, which keeps `WHOLE_SHARDS = 32`, and it
  does not re-derive the census term from a weekly run; both wait for a weekly run under the new
  budget (#597).
- It keeps no census test out of cargo-mutants' runs: no nextest filter or profile, no
  `.cargo/mutants.toml` key, no skip attribute, no test-name filter and no move of the census to
  another package. Every mutant of progression runs both census tests, as at the base (#597, whose
  scope excludes such a change).
- It does not make the census cheaper. Its compiles stay cold, each in an empty target of its own
  (SPEC-072 section 14), and its cost in the `rust` job is #508's.
- It changes no Rust code and no `--build-timeout` (#597).

## 6. Risks

- **The census grows past the budget.** A census that later needs more than 1200 s brings the VOID
  back, loudly, at the baseline. A1 holds the census term and the budget's margin to one literal,
  so any re-derived term turns A1 red until the budget is checked against it again (#597).
- **A projection is not a measurement.** The census term is two tests' CI time, not a per-mutant
  measurement under cargo-mutants. The shard bound of 3600 s is half the job's 7200 s, and the
  projections the sizer was calibrated on came within 0.66 to 1.33 times their measurement.
- **More shards cost more runner time.** #592's listing goes from 3 shards to 23 (69363 s projected
  in total, against 8359 s). That is the census's cost, paid once per progression mutant and once
  per shard, and the price of a verdict that examines rather than refuses.
- **A slow mutant now holds a leg longer.** A mutant that hangs is stopped at 1200 s rather than
  300 s. The memory scope (SPEC-196) and the job's `timeout-minutes` still bound the leg.

## 7. Read after the merge

| id | criterion | read by |
|---|---|---|
| A8 | at the first pull-request head after landing whose plan lists a progression mutant, every `mutation-rust` shard's baseline reads `ok` and no shard reaches its job timeout | that run's shard logs (`ok Unmutated baseline`) and the shard jobs' conclusions |

This pull request's own `mutation-rust` leg is not started: its diff holds no Rust mutant. A8 is
therefore read on a later pull request.

## 8. The mutation rows

`scripts/mutation-rows.d/S32700-S32799.json`, `SCRIPT_MUTATIONS`. A changed literal constant owes a
row pinning its value (rows S32700 and S32705).

| row | target | mutant | killer |
|---|---|---|---|
| S32700 | `scripts/mutation-verdict.py` | the census term's value, 788 to 0 | A3 |
| S32701 | `scripts/mutation-verdict.py` | `mutant_costs` drops `+ CENSUS_SECONDS.get(package, 0)` | A3 |
| S32702 | `scripts/mutation-verdict.py` | the baseline drops the census term | A4 |
| S32703 | `scripts/mutation-verdict.py` | the baseline sums the listing's packages as a list, once per mutant | A4 |
| S32704 | `scripts/mutation-verdict.py` | `size` passes the plain `BASELINE_SECONDS` | A6 |
| S32705 | `.github/workflows/ci.yml` | the run step's `--timeout 1200` back to `--timeout 300` | A2 |

## 9. References

The owner's ruling `docs/rulings/OWNER-RULING-2026-10-03-mutation-timeout-1200.md`; ADR-199's
decision drivers (a raised or lowered timeout is a weakening that only the owner takes); ADR-197;
SPEC-039 R18; SPEC-129 R2 and R4; SPEC-072 section 14; ADR-328; #597; #508.
