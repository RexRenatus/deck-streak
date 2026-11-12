Under the owner's delegation, in the owner's own words: "i delegate the mutation timeout issue to you to handwhile autnomously" and "create the ruling and send for me to sign" (owner, 2026-10-03), the owner admits ADR-199's held lever "a raised timeout": the per-mutant budget `--timeout 300` becomes `--timeout 1200` at `.github/workflows/ci.yml:331`, `:337`, `:428`, `.github/workflows/mutation-weekly.yml:91`, `:93`, `:168`, `:170`, `:322`, `:437` and `scripts/tests/_mutants_finder.py:13` (BOUNDS), with `--build-timeout 600` unchanged, because a Timeout scores killed (`scripts/mutation-verdict.py:2959-2960`) and so a longer budget can only turn a timed-out mutant into caught or missed, never into a pass.

# OWNER RULING 2026-10-03: the per-mutant budget is 1200 seconds

## What was held

ADR-199's Decision Drivers (lines 20-21) hold the examined set as the owner's: "fewer tests or mutants, a raised or
lowered timeout, a skipped baseline, or a narrowed diff is a weakening, and a lever that needs one is not taken".
ADR-197 (lines 210-217) applies that rule to the settle census and names the decision without taking it.

At dev `c6d29f73`, one bound, `--timeout 300 --build-timeout 600`, is held equal at ten places:

| place | what it bounds |
|---|---|
| `.github/workflows/ci.yml:331` | the diff listing |
| `.github/workflows/ci.yml:337` | the whole-tree listing |
| `.github/workflows/ci.yml:428` | the pull request's mutation run, its baseline included |
| `.github/workflows/mutation-weekly.yml:91`, `:93` | the weekly sizing listings |
| `.github/workflows/mutation-weekly.yml:168`, `:170` | the weekly package and shard runs |
| `.github/workflows/mutation-weekly.yml:322` | the weekly whole listing |
| `.github/workflows/mutation-weekly.yml:437` | the weekly single-file run |
| `scripts/tests/_mutants_finder.py:13` | `BOUNDS`, which the byte pins and rows S12904-S12909 hold the nine sites to |

## What was measured

The design round (`design-MB1`, read at dev `c6d29f73`) measured it at #592, run 37131363071:

- The baseline's `--timeout 300` bounds the whole `cargo nextest run` of 4 packages. Two settle-census tests in
  `deck-streak-progression` (`xp_census.rs:1161` and `:3817`) take 736.9 s and 652.9 s in the rust job (test.log
  :1371, :1373). The baseline's test phase therefore needs about 788 s, and every shard holding a progression mutant
  read `process_status=Timeout` at 300 s. Its mutation-verdict was VOID.
- A Timeout scores killed (`scripts/mutation-verdict.py:2959-2960`). Rescuing only the baseline at 300 s would score
  every progression mutant whose census outlives the budget as killed, whether its tests caught it or not.

## What replaces it

- `--timeout 1200` (a 1.5 margin over the measured 788 s), at all ten places at once. `--build-timeout 600` is
  unchanged. The shard sizer gains a separate census term of 788 s, charged to each progression mutant and once to a
  progression plan's baseline, taken from run 37131363071 until the next weekly refresh re-derives it.
- The weekly battery's shard count (`WHOLE_SHARDS`, S12903) is NOT re-sized by this ruling. It stays a measured
  follow-up.

## Why it is admitted

The raise is a weakening by ADR-199's letter, because a timeout moved. In effect it cannot let a mutant pass that the
300 s budget failed: a mutant that timed out at 300 s was already scored killed. At 1200 s it is either caught (the
same verdict) or missed (a stricter one). The rejected alternatives:

- **The baseline alone.** One `--timeout` bounds the baseline and every mutant, so a baseline-only cure is the false
  KILL above.
- **A shared or warm census build.** It reverses SPEC-072 section 14 (:765-770), an owner-decided round.
- **Keeping `xp_census` out of progression's mutant runs.** It runs fewer tests, which the owner fenced. It would also
  lose the only killer of rows S07274, S07275 and S07283.

The build delivery carries its own SPEC and ADR, red-first tests, the census term, and the re-anchor of S12904-S12909.
