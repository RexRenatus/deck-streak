UNSIGNED DRAFT. This line takes effect only in a commit the owner signs. Under the owner's delegation of the mutation timeout issue (owner, 2026-10-03), the owner is asked to admit ADR-199's held lever "a raised timeout" a second time: the per-mutant budget `--timeout 1200` becomes `--timeout 2200` at `.github/workflows/ci.yml:415`, `:421`, `:512`, `.github/workflows/mutation-weekly.yml:91`, `:93`, `:169`, `:171`, `:325`, `:440` and `scripts/tests/_mutants_finder.py:13` (BOUNDS), and each Rust mutation leg's `timeout-minutes` becomes 360 with its shard bound 10800 s (half of it), with `--build-timeout 600` unchanged, because a Timeout scores killed (`scripts/mutation-verdict.py`) and so a longer budget can only turn a timed-out mutant into caught or missed, never into a pass.

# OWNER RULING 2026-10-06 (unsigned draft): the per-mutant budget is 2200 seconds

Cited by SPEC-362 and ADR-373. The build that carries the raise is `docs/specs/planned/SPEC-362`'s; nothing that depends on 2200 lands before the signed commit of this file is on `dev`.

## What was held

ADR-199's Decision Drivers hold the examined set as the owner's: "fewer tests or mutants, a raised or lowered timeout, a skipped baseline, or a narrowed diff is a weakening, and a lever that needs one is not taken". The 2026-10-03 ruling admitted `--timeout 1200` on a measured census need of about 788 s with a 1.5 margin.

At `dev` 25db4f60, one bound, `--timeout 1200 --build-timeout 600`, is held equal at ten places:

| place | what it bounds |
|---|---|
| `.github/workflows/ci.yml:415` | the diff listing |
| `.github/workflows/ci.yml:421` | the whole-tree listing |
| `.github/workflows/ci.yml:512` | the pull request's mutation run, its baseline included |
| `.github/workflows/mutation-weekly.yml:91`, `:93` | the weekly sizing listings |
| `.github/workflows/mutation-weekly.yml:169`, `:171` | the weekly package and shard runs |
| `.github/workflows/mutation-weekly.yml:325` | the weekly whole listing |
| `.github/workflows/mutation-weekly.yml:440` | the weekly single-file run |
| `scripts/tests/_mutants_finder.py:13` | `BOUNDS`, which the byte pins hold the sites to |

## What was measured

The design round (read at `dev` 25db4f60) measured it:

- The progression settle-census test `xp_census` took 1378.452 s in `dev`'s push run 37392351782 and 1144.5 s in run 37410215011, against a priced need of 788 s. With its 51 s start the need is 1430 s.
- The whole workspace's nextest run, which a round-robin leg's baseline repeats in full, took 1717.322 s over 1585 tests in run 37410215011 (the sizer's `BASELINE_SECONDS` is 371, measured over 7 packages only).
- `1430 x 1.5 = 2145`, rounded up to the hundred by SPEC-327 R2's own rule: 2200.

## What it closes

At 1200 s the budget is below the census it must cover. A progression mutant that only the census can kill, or that no test kills, runs past 1200 s when the census takes 1378 s, and cargo-mutants reports `Timeout`. The verdict counts a Timeout as examined and never as failing, so a surviving mutant can pass the gate. That holds on every pull request into `dev` that touches progression, not only on a release. At 2200 s a census-only kill reads as caught and a survivor reads as missed.

The leg bound is raised with it: a hosted job runs up to 360 minutes, and the per-leg bound is half of that (10800 s), so that a release's whole tree is judged in one run of round-robin legs (SPEC-362 R2). The new verdict check (SPEC-362 R7) names a leg VOID when 1.5 times its slowest baseline test exceeds the timeout, so the budget cannot silently fall below a test again.

## What replaces it

- `--timeout 2200` (a 1.5 margin over the measured 1430 s), at all ten places at once. `--build-timeout 600` is unchanged.
- Each Rust mutation leg's `timeout-minutes` is 360 and `SHARD_BOUND_SECONDS` is 10800.
- `CENSUS_SECONDS` becomes 1430 and `BASELINE_SECONDS` 1768, each held with the run id it came from.

## Why it is admitted

The raise is a weakening by ADR-199's letter, because a timeout moved. In effect it cannot let a mutant pass that the 1200 s budget failed: a mutant that timed out at 1200 s was already scored as examined. At 2200 s it is either caught (the same verdict) or missed (a stricter one). The rejected alternatives:

- **Leaving 1200.** It leaves the gate hole above open on every progression pull request.
- **Keeping `xp_census` out of progression's mutant runs.** It runs fewer tests, which the owner fenced, and it would lose the only killer of the census-only mutants (SPEC-327 R7).
- **`--baseline=skip` on the legs.** A pre-existing failure would read as a kill.
- **A one-time admission of the release past its mutation gate.** It lands a tree whose mutants were never examined (ADR-373).

The census test's own speed-up is a separate delivery (#692).

## Signature

Unsigned. The owner signs the commit that adds this file; until then no part of this ruling is in force.
