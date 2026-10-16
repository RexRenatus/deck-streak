In the owner's answer to the question on #692 (option (A)), the owner admits one bounded weakening of SPEC-072 section 14's cold rule ("no cache serves the census, and every census compile is cold"): the real tree's settle census, `only_progression_writes_xp_settlement_and_only_coordination_settles` in `crates/progression/tests/xp_census.rs`, may keep the units of the packages `Cargo.lock` pins from a registry or git between censuses, in a store under the test process's temporary directory, outside `target/`, keyed by the lock file, the toolchain pin, the census's flags and inherited environment, the cargo program and the tree root; every member unit is removed before each census and compiled by the census that reads it, and none is ever kept; every planted tree's census stays cold in an empty target of its own; and nothing relies on the store until a warm-store census and a cold census of one tree are shown, each run named, to report the same killed and survived set.

# OWNER RULING 2026-10-16: the real tree's settle census keeps lock-pinned units in a store of its own

Cited by SPEC-384 and ADR-398, and by the insert-only amendments they make to SPEC-072, ADR-197 and ADR-328. The build that carries the store is SPEC-384's (#692); it lands on `dev` only after the signed commit of this file is there.

## What was held

| place | what it holds |
|---|---|
| SPEC-072 section 14 | every input the census verdict can read is part of the tree, pinned by the lock file or by the toolchain pin, made empty before each census compile, or refused by name; "An empty target for each census" |
| ADR-197, round 8 | option (a): nothing is carried between censuses; option (b), a stripped cache, rejected because no strip can be shown complete |
| ADR-328, option B | "a cheaper census, through a shared or reused build", lost on SPEC-072 section 14 |

## What was measured

| run | reading |
|---|---|
| 37410215011 | the census test took 1144.5 s |
| 37392351782 | the census test took 1378.452 s |
| 37445796358 | the census test took 1441.022 s |
| 38864023497 | 104 of the 192 `mutation-rust` legs read a red baseline (`Timeout`), each with this census test cut by the per-mutant budget; in leg 4 (artifact 12034287584) the test ran 1371.187 s inside a 2296.957 s baseline |

Each census runs 16 `cargo check --workspace` passes in an empty target and compiles the 615 packages the lock pins (608 from a registry, 7 from git) cold each time, for every progression mutant. The 31 workspace members are a small share of that work.

## What it closes

A release's progression legs cannot be judged while the census cannot finish inside the per-mutant budget, and raising the budget again only moves the wall. The store removes the repeated cold compile of pinned units without shrinking the census: the same passes, the same selections, features and pairs, the same members compiled, and the same assertions.

## What replaces it

- The real tree's census compiles in its store, keyed as line 1 says, under an exclusive lock held from the removal of member units to its last pass.
- Before its first pass the census removes every unit of every path package from the store, and a member unit that is reported as already built but that this census did not compile is refused by name.
- Every planted tree's census, the killer `the_census_refuses_every_caller_the_compiler_finds` included, still compiles in an empty target of its own (#917).
- Each CI job starts with an empty store, so no cache carries the store between jobs.
- No test gains a skip, a filter, a sampled population or a weaker assertion, and no price, budget or timeout moves.

## The check before anything relies on the store

SPEC-384's builder runs the census on one tree twice per state: with the store emptied first (cold) and with the store warm from a census of the same tree. The unmutated tree passes in both states, and each census-only kill (rows S07274, S07275 and S07283) reads KILLED in both. Each run is named by its command, its head and its log in `docs/red-first/SPEC-384.md`, and the pull request's own CI run is named by id beside them. A different killed or survived set in either state is a refusal of this ruling, not a finding to waive.

## Why it is admitted

It is a weakening by section 14's letter, because a unit compiled by an earlier census is read by a later one. Its residual is the disclosed class of build-time code in a pinned package that reads host state (#445): within one store that code's output is kept rather than re-run. A store born empty in each CI job bounds it. Rejected:

- **Keeping every census compile cold.** The census then times out under the budget, and no release's progression legs can be judged.
- **A stripped cache (ADR-197 (b)).** No strip can be shown complete; the store strips nothing from pinned units and keeps every member unit out instead.
- **Fewer passes, selections, features or pairs.** A shrunk census measures less.
- **A raised timeout.** It is a signed act of its own and speeds nothing.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
