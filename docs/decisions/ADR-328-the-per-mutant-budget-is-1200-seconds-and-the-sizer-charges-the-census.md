---
status: "accepted"
date: "2026-10-03"
decision-makers: "the owner, by the signed ruling docs/rulings/OWNER-RULING-2026-10-03-mutation-timeout-1200.md; the DeckStreak builder of SPEC-327"
---

# The per-mutant budget is 1200 seconds, and the sizer charges the census

## Context and Problem Statement

Every `cargo mutants` command runs under `--timeout 300 --build-timeout 600`, and the `--timeout`
bounds the unmutated baseline's whole test run as well as each mutant's. The settle census's two
tests in `deck-streak-progression` take 652.9 and 736.9 s in CI, started up to 51 s into the run,
so the baseline needs about 788 s. At #592's head every `mutation-rust` shard read `TIMEOUT
Unmutated baseline ... 300s test` and tested no mutant, and the verdict was VOID (SPEC-327 section
1). How should the gate examine progression's mutants again without weakening what it examines?

## Decision Drivers

- ADR-199's decision drivers hold the examined set as the owner's: "fewer tests or mutants, a raised
  or lowered timeout, a skipped baseline, or a narrowed diff is a weakening, and a lever that needs
  one is not taken". ADR-197 (lines 208-217) applies that rule to the settle census. Only the
  owner's signed ruling takes such a lever, and the owner signed one:
  `docs/rulings/OWNER-RULING-2026-10-03-mutation-timeout-1200.md`.
- A `Timeout` counts as killed (`scripts/mutation-verdict.py`:2959-2960). A cure that lets the
  baseline pass while a mutant's budget stays under the census's need would score every
  progression mutant killed by timeout: a false kill.
- The census's two tests run for every mutant of progression, as they do at the base (the owner's
  fence on any change that keeps them out).
- A shard must stay within its job's `timeout-minutes: 120` (`.github/workflows/ci.yml`:363).

## Considered Options (the alternatives it was chosen against)

| option | outcome |
|---|---|
| Raise the per-mutant budget to `--timeout 1200` on every command, and give the sizer the census's cost | chosen: one flag bounds the baseline and each mutant, so one raise cures both, and the sizer keeps each shard within its bound |
| A alone: raise the flag and leave the sizer untouched | lost: #592's three shards would project 15485, 15349 and 15349 s, and 20840 s if every progression mutant ran to its budget (15485 + 15 x (71 + 1200 - 914)), all past the job's 7200 s; a raise that pushes a shard past its bound is refused |
| B: a cheaper census, through a shared or reused build | lost: SPEC-072 section 14 decided that no cache serves the census and every census compile is cold, reversing round 6's cache; the test `a_target_copied_from_another_trees_census_does_not_move_the_verdict` (`crates/progression/tests/xp_census.rs`:5408) pins it, and round 7 measured a warm target moving a verdict. The run's own target would save nothing anyway, since the census compiles with its own flags and in place the run's sources are the mutated ones |
| C: keep either census test out of cargo-mutants' runs (a nextest filter or profile, a `.cargo/mutants.toml` key, a skip attribute, a move to another package) | lost: it runs fewer tests for a mutant of progression, which the owner fenced, and it would lose the only killer of rows S07274, S07275 and S07283 (the owner's ruling) |
| A progression-only leg with its own larger timeout | lost: the guard `EveryMutationCommandKeepsTheGatesBounds` holds one bounds string on every command (`scripts/tests/_mutants_finder.py`:13), and round-robin places progression mutants in every leg |
| A larger hosted runner | lost: not measured, billed, and not certain to bring the census under 300 s |
| Raise the baseline's budget alone and keep each mutant at 300 s | lost: cargo-mutants has one `--timeout` for both, and if it had two, every progression mutant would read Timeout, scored killed: the false kill above |

## Decision Outcome

Chosen option: "raise the per-mutant budget to `--timeout 1200` on every command, and give the
sizer the census's cost", because it is the one cure that examines progression's mutants with
their census and keeps every shard within its job.

- **The budget.** 1200 s is the census's 788 s need times a 1.5 margin (1182), rounded up to the
  next hundred. The margin covers the 0.66 to 1.33 spread between projected and measured shard times
  recorded beside the sizer (`scripts/mutation-verdict.py`:859-860). `--build-timeout 600` is
  unchanged. All nine commands and `BOUNDS` carry the one string.
- **Why it moves no verdict toward a pass.** A mutant that timed out at 300 s was already scored
  killed. At 1200 s it either still times out (the same outcome), finishes caught (the same), or
  finishes missed (stricter). No missed mutant becomes a pass, and the baseline goes from VOID to
  examined.
- **The census term.** `CENSUS_SECONDS = {"deck-streak-progression": 788}`, a table apart from
  `SECONDS_PER_MUTANT`. Each mutant of a named package costs its table cost plus its term, and a
  plan whose listing names the package pays the term once more in each shard's baseline. It is a
  separate table on purpose: `mutant_costs` charges a package the table does not name the table's
  highest, so putting progression into `SECONDS_PER_MUTANT` at 914 s would charge every unnamed
  package 914 s and over-shard every pull request.
- **The projection.** #592's own listing (67 mutants: analytics 4, api 8, coordination 10,
  progression 45) shards into 23 legs, the slowest at 3113 s of its 3600 s bound, 69363 s in total
  against the base's 8359 s.

### Consequences

- Good, because a pull request that changes progression gets a mutation verdict that examined its
  mutants, where today it gets VOID.
- Good, because no test leaves any mutant's run and no verdict moves toward a pass.
- Bad, because a pull request with progression mutants runs many more shards: the census's cost is
  paid once per progression mutant and once per shard.
- Bad, because a hung mutant now holds its leg for up to 1200 s before cargo-mutants stops it. The
  memory scope (SPEC-196) and the job's timeout still bound the leg.
- Bad, because the weekly battery's whole-tree sweep keeps 32 shards, sized before this budget;
  #597 re-sizes it from a measured weekly run and re-derives the census term from it.

### Confirmation

A1 holds the budget's margin over the census and the sizer's term to one measured literal. A2 and
A7 hold every workflow command and its byte pins to the new bound. A3 to A6 hold the sizer's
projections to exact values computed from literals. Rows S32700 to S32705 prove the term's value,
its two additions, the once-per-package baseline, the dispatch path and the run step's literal.
After landing, A8 reads the first pull request whose plan lists a progression mutant: every shard's
baseline `ok`.

## What would make this wrong

- A census that needs more than 1200 s. Its VOID returns, loudly, at the baseline, and A1 turns red
  as soon as the term is re-derived (#597).
- A mutant that cargo-mutants could only judge as missed by running past 1200 s. It still reads
  Timeout, scored killed, exactly as it did at 300 s; that is the gate's standing rule, unchanged
  here.

## More Information

SPEC-327; the owner's ruling `docs/rulings/OWNER-RULING-2026-10-03-mutation-timeout-1200.md`;
ADR-199; ADR-197; SPEC-072 section 14; SPEC-039 R18; SPEC-129; #597.
