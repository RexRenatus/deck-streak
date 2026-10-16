---
status: "proposed"
decision-makers: "the review that admits SPEC-072 section 18; the DeckStreak builder of SPEC-384"
---

# The real tree's census keeps a store of lock-pinned units, and compiles every member itself

## Context and Problem Statement

The real tree's settle census is the slowest test in the tree and runs for every progression mutant
(SPEC-327 R7; SPEC-362 section 1 reads it at 1378.452 s and 1441.022 s on two push runs). #692
asks for it to be faster without a skip, a filter or a weaker census, and with every census-only
kill still a kill.

SPEC-072 section 14 (round 8, ADR-197) decided that each census compiles in an empty target of its
own, so that the verdict reads nothing but the tree and what the lock file and the toolchain pin.
Its rule is wider than its means: "Every input the verdict can read is part of the tree, pinned by
the lock file or by the toolchain pin, made empty before each census compile, or refused by name."
This ADR keeps the rule and narrows the means for the real tree's census alone: units pinned by the
lock may be kept between censuses; every unit of the tree is compiled by the census that reads it.

## Decision Drivers

- The verdict depends on the tree, the lock file and the toolchain pin, and on nothing an earlier
  census left behind (SPEC-072 section 14; round 7 measured a warm target moving a verdict).
- The census examines the same population and asserts the same facts for every mutant (#692,
  SPEC-327 R7).
- A raised or lowered timeout or price is not a speed-up (ADR-199; #597).
- A count is evidence only when the examination produced it: the new counts come from cargo's own
  messages, never from a loop's counter.

## Considered Options (the alternatives each decision was chosen against)

### D1. Where the time goes: chosen against

- The members' own compile as the main cost: `clippy` over every member and target, with the
  dependencies already built, took 45 s in job 113820846590, a small share of a census of more
  than twenty minutes.
- The sixteen passes as sixteen cold builds: the passes share one target, so passes under the same
  (debug-assertions, panic) pair and features reuse each other's units; the cold work is the four
  pairs and the feature changes, not sixteen.
- The planted censuses beside it, the killer's workers included, as the main cost: their share of a
  mutant's run is not measured from the tree or from a named job, so it is left as a question and
  not decided here.

### D2. The speed-up: chosen against

- The four pairs run at once, each in an empty target of its own: it removes no work, only overlaps
  it, on a machine the killer's workers already share, and its gain is unmeasured.
- A store under `target/`: CI saves and restores `target/` whole, which brings back the census cache
  that round 8 removed (ADR-197) and grows every job's restore.
- `debug-assertions` set on the members only: a pinned package's macro defined under that package's
  own `debug_assertions` can expand to a call of `settle` in a member, so the census would stop
  seeing a use it sees today.
- Fewer selections, features or pairs: that shrinks the population the census examines.
- `#[ignore]`, a nextest filter or profile, or a `.cargo/mutants.toml` key: each takes the census out
  of a mutant's run, which #692 and SPEC-327 R7 forbid.
- A raised `--timeout`: it is a signed act under ADR-199, and it speeds nothing.
- ADR-197's option (b), a cache stripped of build-script output: rejected there because no strip
  can be shown complete; this store strips nothing from pinned units and keeps every member unit
  out of it instead.

### D3. The proof of no weakening: chosen against

- A wall-clock assertion: it reads a loaded machine as a failure; the time is read in CI by job id
  after the merge (SPEC-384 section 8).
- The census-only rows trusted to the scheduled battery: it runs after the merge, so a row the store
  let survive would land first.
- A pass count from the loop's own counter: it counts what the loop meant to do, not what cargo
  did; the counts come from cargo's messages.

### D4. The tests: chosen against

- The new rows killed by the real-tree test: each row would pay a real-tree census; planted trees
  decide each one in a fraction of that.
- `## FORMAL` NOT APPLICABLE as a strictly sequential carry-over: two censuses of one checkout can
  overlap on one store (two test runs, or a row's run beside a suite), and the removal followed by
  the passes is a check followed by an act that the other census can change in between.

### D5. Shape: chosen against

- An insert-only amendment of SPEC-327: SPEC-327 decided the budget, not how the census compiles,
  and its R7 is kept whole.
- An insert-only amendment of SPEC-362: SPEC-362 sizes the legs, and its section 5 records this work
  as its own delivery.
- An amendment of SPEC-072 alone: the store, its key and its lock are new requirements with their
  own criteria and rows, so they need a SPEC of their own; SPEC-072 keeps the rule and gains the
  exception by insert.

### D6. The store's scope: chosen against

- One store for every checkout on a machine: a single lock would serialise sibling checkouts'
  censuses.
- A store enabled by a variable the mutation workflows set: it edits workflow commands whose bytes
  are pinned by tests that run only in CI, for a confinement the job's temporary directory already
  gives.
- A key without the tree root: two checkouts with one lock file would share a store and its lock.

## Decision Outcome

### D1. Where the time goes

The real tree's census compiles the 615 packages that `Cargo.lock` pins (608 registry, 7 git) cold,
once under each of the four (debug-assertions, panic) pairs `pass()` sets for every package by
name, and again where a selection changes their features. Those units are the same for every
mutant. The members are 31 packages and a small share. The share of the planted censuses running
beside it is not decided here.

### D2. The speed-up

The real tree's census compiles in a store (SPEC-384 R1 to R4): a directory under the test
process's temporary directory, outside `target/`, named by a key over the lock file, the toolchain
pin, the census's flags and environment and the tree root. Before its first pass the census takes
the store's lock and removes every unit of every path package; each member unit a pass reads must
be one this census compiled, or it is refused by name. The second and later censuses of a run
reuse the pinned units and compile every member. Planted censuses keep their empty targets.

### D3. The proof of no weakening

The builder records the real-tree test's existing examined counts at the cut and at the head, equal
by name; the test prints and asserts the new counts of SPEC-384 R7; rows S07274, S07275 and S07283
(the census-only kills) and S07295 read KILLED at the head beside a passing control. No price or
timeout moves.

The owner's signed ruling `docs/rulings/OWNER-RULING-2026-10-16-census-store.md` admits the store as a bounded weakening of
SPEC-072 section 14's cold rule, on one measured check before anything relies on the store: a
warm-store census and a cold census of one tree report the same killed and survived set, each run
named (SPEC-384 P3).

### D4. The tests

A1 to A6, red first, each an assertion failure at the red commit: A1 to A5 on planted trees or pure
inputs, A6 the real-tree test's new assertions. Rows S38400 to S38406 are each killed by a planted
test. `## FORMAL` is REQUIRED: the store is shared state two censuses can overlap on, modelled by
`formal/tla/CensusStore` with two censuses, two trees, one store, the lock and the removal, and the
invariant that an accepted verdict reads only member units its own census compiled. Its witnesses
switch off the lock and the removal, and each is caught.

### D5. Shape

One delivery under SPEC-384 and ADR-398, with insert-only text in SPEC-072 (section 18), ADR-197,
ADR-328 and two schematics.

### D6. The store's scope

The key names the tree root, and the store lives in the test process's temporary directory, so each
CI job starts with an empty store and a developer's checkouts do not share one.

### Consequences

- Good: a progression mutant after the first in a run compiles the members of the four pairs and
  reuses the pinned units, and every census-only kill stays a kill.
- Good: the census gains a refusal it did not have: a member unit it did not compile.
- Bad: pinned packages' build-time output is kept within one store rather than re-run per census;
  the class stays disclosed (#445).
- Bad: a developer's machine keeps a store per lock file until it is removed.
- Neutral: the `rust` job's census stays cold (#508), and prices wait for a measurement (#597).

### Confirmation

SPEC-384's A1 to A6, its rows S38400 to S38406, the builder's records P1 and P2, the formal check
F1, and the read after the merge, L1.

## What would make this wrong

- A pinned package whose compiled output differs between two censuses with one key: the key would
  be missing an input, and A4 gains a case for it.
- A member unit read with `"fresh": true` that this census did not compile, accepted: R2's refusal
  would be broken, and A2 or the model's witness would read it.
- L1 reading a second mutant's census at about the first one's time: the pinned units were not the
  cost, and D1 is wrong.

## More Information

SPEC-384; SPEC-072 sections 12 and 14; ADR-197 round 8 and its options; ADR-328 option B;
SPEC-327 R7; SPEC-362 section 5; ADR-199; #692, #597, #508, #445.
