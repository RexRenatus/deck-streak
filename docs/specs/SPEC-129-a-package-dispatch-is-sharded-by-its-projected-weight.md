# SPEC-129: a package dispatch is sharded by its projected weight, so a small package does not build thirty-two baselines

- **Wave:** W4. **Issue:** #368. **Context(s):** `repo` (`.github/workflows/mutation-weekly.yml`,
  `scripts/mutation-verdict.py`, their tests and `docs/`).
- **Decided by:** ADR-129 (this SPEC's own: size the dispatch from its listing, and what that was
  chosen against), SPEC-057 R14, R18 and R22 (the package dispatch, the projection and the listing;
  it takes a dated amendment) and ADR-057 D7.
- **Status:** delivered. It holds `docs/red-first/SPEC-129.md`.

## 1. The problem, measured

- **A dispatch that names a package still fans out to 32 legs.** `mutation-weekly.yml`'s `rust` job
  has a fixed matrix of shards 0 to 31 and passes `--shard <i>/32` to `cargo mutants`, with or
  without `--package`. Every leg checks out, installs the toolchain and the tools, restores the
  cache and builds and tests the unmutated baseline before its first mutant (SPEC-057 R18's
  `BASELINE_SECONDS`).
- **A small package leaves most legs with nothing to do but the baseline.** Shard `k` of a
  round-robin sweep holds a mutant only when the scope lists more than `k`, so a package of a few
  dozen mutants gives the rest of the 32 legs a baseline and no mutant. The battery already knows
  this: it owes a report only from the shards the scope's listing gave a mutant.
- **The per-pull-request plan sizes its legs and the dispatch does not.** `mutation-verdict.py
  shards` projects each shard's seconds from the listing (`projected`, `SECONDS_PER_MUTANT`,
  `BASELINE_SECONDS`) and writes the fewest round-robin shards whose slowest is within
  `SHARD_BOUND_SECONDS` as the matrix (SPEC-057 R18). The weekly dispatch has the listing and does
  not use it.

## 2. Requirements

R1. **A dispatch that names a package is sized before its legs run.** A `size` job, before `rust`,
lists that package's mutants with the command the `listing` job runs plus `--package` (the same
`--no-shuffle --list --json --in-place --timeout 300 --build-timeout 600`, the same pinned
cargo-mutants and test tool), and runs `mutation-verdict.py size` on the listing.

R2. **The count is the fewest round-robin shards whose slowest is projected within the bound, by
the plan's own function.** `size` and `shards` share one function that returns that count, over
the same `projected`, `SECONDS_PER_MUTANT` (a package the table does not name costs the table's
highest), `BASELINE_SECONDS` and `SHARD_BOUND_SECONDS`. `size` writes `shards=<n>` and
`matrix=<[0..n-1]>` to the step's outputs.

R3. **n is at least 1 and at most `MAX_SHARDS`, and a projection past the limit is refused, never
capped.** A package that lists no mutant is sized to 1. A listing that needs more than `MAX_SHARDS`
shards within the bound makes `size` exit 1 with `REFUSED`, the mutant count and the projected
serial seconds, and write no output, as `shards` does. A file that is not a cargo-mutants listing
is `VOID` (exit 3) and writes no output.

R4. **A scheduled run and a dispatch with no package keep 32.** `size` with no `--package` reads no
listing and writes `shards=32` and the matrix `[0..31]`. A dispatch naming `miniapp` runs no rust
leg and sizes as no package.

R5. **The legs, their `--shard` argument and the battery read the one count.** The `rust` job
`needs: size`, its matrix is `fromJSON(needs.size.outputs.matrix)`, its step `env:` sets `SHARD`
from `matrix.shard` and `SHARDS` from `needs.size.outputs.shards`, and its command passes
`--shard "$SHARD/$SHARDS"`. The survivors job, which needs `size`, sets `SHARDS` the same way and
runs `mutation-verdict.py battery --reports "$reports" --shards "$SHARDS" …`. No `${{ }}` expression is interpolated into a `run:` block, and
no `--shard` or `--shards` argument of a rust or survivors command is a literal number. The
`--timeout 300`, `--build-timeout 600`, `--in-place` and test tool of every leg are unchanged, and
every `cargo mutants` command line of every workflow file carries `--timeout 300 --build-timeout
600` literally: the bounds are read as whole values (`6000` is not `600`), a command continued
with `\` is read as one line, and a bare `cargo mutants` counts.

R6. **The battery refuses a report set whose shard count differs from n.** A missing report is
already `MISSING`; a report `mutants-shard-<k>` with `k` at least n is now `FOREIGN` and fails the
battery, so a set produced at another count is never read as n.

R7. **The examined total equals the package's listing at the same commit.** The sized shards take
every listed mutant exactly once, as any round-robin count does (mutant `i` in shard `i mod n`), so
the union over n shards is the union over 32. The survivors job's `table --package <name> --listed
"$reports/listing/whole.json"` (`table_rust`) already refuses the run when it does not hold: a
listed mutant no report tested is `VOID` (`never tested: <name>`), and one tested more or fewer
times than it is listed is a failure. The delivery adds no second check of it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a fixture listing of a small package sizes to 1, larger ones to the fewest shards within the bound, the count equals `shards`' on the same listing, a projection past the limit is refused with its projection and writes nothing, and an empty or unreadable listing is sized to 1 or VOID | `test_dispatch_shards.py` `TheDispatchIsSizedFromItsListing` |
| A2 | with no package `size` reads no listing and writes 32 and the matrix 0 to 31 | `test_dispatch_shards.py` `TheWholeTreeKeepsThirtyTwo` |
| A3 | the rust matrix, its `--shard` argument and the battery's `--shards` read the size job's one count, no fixed count remains, and a plant that puts 32 back into any of the three goes red | `test_dispatch_shards.py` `TheWorkflowReadsTheOneCount` |
| A4 | a report set beyond n fails the battery as `FOREIGN` and a set of exactly n passes | `test_dispatch_shards.py` `TheBatteryRefusesAForeignShardCount` |
| A5 | the sized shards' mutants are the same set, and the same count, as the 32 shards' | `test_dispatch_shards.py` `TheExaminedTotalIsTheListing` |
| A6 | every `cargo mutants` command line in every workflow file, at least one in each of `ci.yml` and `mutation-weekly.yml`, carries `--timeout 300 --build-timeout 600` literally, read as whole values, with a `\`-continued command read as one line and a bare `cargo mutants` counted | `test_dispatch_shards.py` `EveryMutationCommandKeepsTheGatesBounds` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_small_package_takes_one_shard_and_a_large_one_the_fewest_within_the_bound
A1: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_projection_past_the_limit_is_refused_with_its_projection_never_capped
A2: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_no_package_is_thirty_two_shards_whatever_the_listing
A3: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_matrix_the_argument_and_the_battery_read_the_sized_count
A3: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_plant_that_puts_the_fixed_count_back_goes_red
A4: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_report_beyond_the_count_fails_the_battery
A5: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_union_of_the_sized_shards_equals_the_union_of_the_thirty_two
A6: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_cargo_mutants_command_carries_the_gates_own_bounds
```

The live proof is one `workflow_dispatch` of this workflow at the delivery's branch naming the
smallest package the listing shows: the legs it fans out to, the battery's examined line, and the
survivors job, quoted in the pull request.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/mutation-weekly.yml` | repo | changed: the `size` job, the rust matrix and `--shard`, the survivors job's `--shards` (R1, R4, R5) |
| `scripts/mutation-verdict.py` | repo | changed: the shared `fewest_shards`, the `size` verb, and the battery's `FOREIGN` check (R2, R3, R6) |
| `scripts/tests/test_dispatch_shards.py` | repo | added: A1 to A6 |
| `scripts/tests/test_mutation_workflows.py` | repo | changed: the three assertions that read the fixed matrix and the fixed divisor now read the sized count |
| `scripts/mutation-rows.d/S12900-S12999.json` | repo | added: six rows: S12901 to S12903 for the verdict's clauses the per-PR Python gate cannot reach, S12904 and S12905 for the rust leg's `--timeout 300` and `--build-timeout 600`, and S12906 for the same build timeout raised by an appended digit (`6000`), which only a whole-value read of the bound kills, by `EveryMutationCommandKeepsTheGatesBounds` |
| `docs/specs/SPEC-129-a-package-dispatch-is-sharded-by-its-projected-weight.md` | repo | added |
| `docs/decisions/ADR-129-a-package-dispatch-is-sized-from-its-own-listing.md` | repo | added |
| `docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md` | repo | changed: a dated amendment at its end (insert-only) |
| `docs/red-first/SPEC-129.md` | repo | added |
| `changelog.d/ci-dispatch-shards-129.md` | repo | added |

No schematic: the change adds no component. The data flow is SPEC-057's (a listing, a plan, shards,
a battery), with a listing and a sizing step in front of the legs.

## 5. What this does NOT do

- It never lowers a gate: the mutants, `--timeout 300`, `--build-timeout 600` and the test tool are
  the 32-leg dispatch's, and only the number of legs changes (#368).
- It does not size a scheduled run or a dispatch with no package, which keep 32 legs; sizing the
  whole sweep is a separate finding (#368).
- It does not cache the baseline build across legs: that is a repository cache and storage
  decision the orchestrator proposes separately (#368).
- It does not change `SECONDS_PER_MUTANT`, `BASELINE_SECONDS` or `SHARD_BOUND_SECONDS`, so the
  projection is the per-pull-request plan's and no new calibration is claimed (#368).
- It changes the per-pull-request plan's behaviour in no way: `shards` keeps its outputs and its
  refusal (#368).

## 6. Risks

- **The package listing and the legs disagree.** Both use the same command and `--no-shuffle`
  order; R7's table check refuses any mutant a report set did not test, and A5 pins the
  arithmetic.
- **A projection is not a measurement.** A leg's time is the plan's projected time, whose table
  the shards of the weekly runs measured at 0.66 to 1.33 times its projection (SPEC-057 R18); the
  job's timeout is twice the bound.
- **The survivors job downloads one more artifact.** The `sizing` artifact joins the run's others
  and the survivors job reads its reports by name and by recursive search, so the extra directory
  changes no path it reads (the battery ignores every directory not named `mutants-shard-<k>`).

## 7. References

Issue #368; SPEC-057 R14, R18, R22; SPEC-039 R12; ADR-057 D7; ADR-129; cargo-mutants 27.1.0
(`--shard`, `--sharding round-robin`, `--list --json`).
