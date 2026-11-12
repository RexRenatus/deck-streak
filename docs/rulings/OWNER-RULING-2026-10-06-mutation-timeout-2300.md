Under the owner's delegation, in the owner's own words: "i delegate the mutation timeout issue to you to handwhile autnomously" and "create the ruling and send for me to sign" (owner, 2026-10-03), the owner admits ADR-199's held lever "a raised timeout" a third time: the per-mutant budget becomes `--timeout 2300` in place of the 2200 that `docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2200.md` admitted, at `.github/workflows/ci.yml:415`, `:421`, `:512`, `.github/workflows/mutation-weekly.yml:91`, `:93`, `:169`, `:171`, `:325`, `:440` and `scripts/tests/_mutants_finder.py:13` (BOUNDS), with `--build-timeout 600`, each Rust mutation leg's `timeout-minutes` 360 and its shard bound 10800 s unchanged, because a Timeout scores killed (`scripts/mutation-verdict.py`) and so a longer budget can only turn a timed-out mutant into caught or missed, never into a pass.

# OWNER RULING 2026-10-06: the per-mutant budget is 2300 seconds

Cited by SPEC-362 and ADR-373. It supersedes three figures of `docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2200.md`: the timeout, `CENSUS_SECONDS` and `BASELINE_SECONDS`. Everything else that ruling says stands. The build that carries the raise is `docs/specs/planned/SPEC-362`'s; nothing that depends on 2300 lands before the signed commit of this file is on `dev`.

## What was held

The 2200 ruling set one bound, `--timeout 2200 --build-timeout 600`, for the ten places that hold it equal. At `dev` 84094cd0 those places are on the same lines:

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

Two later `dev` push runs read the progression settle-census test (`xp_census`, `only_progression_writes_xp_settlement_and_only_coordination_settles`) slower than the 2200 ruling's sources, each in its `check-stage-logs-rust` artifact's `test.log`:

| run | `dev` | `xp_census` | the whole workspace's nextest run |
|---|---|---|---|
| 37438835490 | bad436c3 | 1469.349 s | 2089.093 s over 1588 tests |
| 37445796358 | 84094cd0 | 1441.022 s | 2057.103 s over 1588 tests |

The 2200 ruling's sources read 1378.452 s (run 37392351782) and 1144.5 s (run 37410215011).

- With its 51 s start, the slower census needs 1521 s (1469.349 s plus 51, rounded up). `1521 x 1.5 = 2281.5`, rounded up to the hundred by SPEC-327 R2's own rule: 2300.
- By the same rule, 2200 covers a census of at most 1415 s, and both later readings exceed it.
- SPEC-362 R7 names a leg VOID when 1.5 times its slowest baseline test exceeds the timeout. `1.5 x 1469.349 = 2204.0`, above 2200.

## What it closes

At 2200 the leg that runs the census reads VOID under R7 at the slower reading, so no release's progression legs can be judged. Below the census, a progression mutant that only the census can kill, or that no test kills, times out instead of reading as caught or missed.

At 2300 a census-only kill reads as caught and a survivor as missed. R7 stays the fail-closed check: a later census whose 1.5 times passes 2300 (a test over 1533 s) reads VOID by name again, never as a pass.

## What replaces it

- `--timeout 2300` (a 1.5 margin over the measured 1521 s), at all ten places at once. `--build-timeout 600` is unchanged.
- Each Rust mutation leg's `timeout-minutes` stays 360, and `SHARD_BOUND_SECONDS` stays 10800.
- `CENSUS_SECONDS` becomes 1521 and `BASELINE_SECONDS` 2141 (2089.093 s plus its 51 s start, rounded up). Both come from run 37438835490, the slower of the two readings, and each is held with that run id.

## Why it is admitted

The raise is a weakening by ADR-199's letter, because a timeout moved. In effect it cannot let a mutant pass that the 2200 s budget failed. A mutant that timed out at 2200 s was already scored as examined; at 2300 s it is either caught (the same verdict) or missed (a stricter one). The rejected alternatives:

- **Keeping 2200.** R7 voids the census leg at the measured reading, so a release cannot be judged at all.
- **2400.** It is headroom beyond any measured census. SPEC-327 R2 derives the budget from a measured census, and R7 already names a later overrun.
- **A one-time admission of the release past its mutation gate.** It lands a tree whose mutants were never examined (ADR-373).
- **Waiting for the census test's own speed-up (#692).** It stays the cure that can lower the budget again, but no release can be judged until it lands.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
