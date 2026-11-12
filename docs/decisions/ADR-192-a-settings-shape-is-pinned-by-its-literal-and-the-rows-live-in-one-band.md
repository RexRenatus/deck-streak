---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# A setting's shape is pinned by its literal, its row lives in one band, and a guard walks the tree

## Context and Problem Statement

Every `impl Setting for` carries a `const SHAPE`, and the refusal an operator reads is `the setting
<NAME> is malformed: it must be <SHAPE>`. The audit of #336 planted 22 shapes twice each; 14
survived, because their tests refused the value with `is_err()` or `matches!`, or compared the error
against the constant (`Hour::SHAPE`) and so read the code under test back to itself. Two questions
were left open: where do the mutation rows for the 15 unpinned shapes live (SPEC-057 R20's allotment,
S05700-S05799, is full, and analytics, bot and readings have none), and how does the next
`impl Setting for` not slip past the same way?

## Decision Drivers

- The row proves the test kills the mutant the tool never generates (a rewritten string constant).
- A band belongs to the delivery whose SPEC allots it; one fix should not amend another SPEC.
- A guard must be red on the base and name the survivors, or it proves nothing.
- The guard must not depend on a git pathspec: `'crates/*/src'` matches nothing.

## Considered Options (the alternatives it was chosen against)

- **All rows in this delivery's own band, S19200-S19299, in one new band file with the one `MUTATIONS` table** — chosen because the band is free, the rows are this delivery's work, and no other SPEC changes.
- **Each row in its crate's own SPEC band, as the audit wrote them** — rejected because those bands belong to other deliveries, and SPEC-057 R20's allotment is full (the audit measured it).
- **Amend SPEC-057 R20 to allot more** — rejected because it changes a second delivery's SPEC for one fix, and the rows would still not be SPEC-057's work.
- **No rows, tests only** — rejected because a test that names the literal is the pin, but only the row proves it kills the mutant; #336's criteria ask for one or the other, and the audit proved both.
- **A guard that walks `crates/*/src` with `pathlib` and reads each `SHAPE` literal** — chosen because it enumerates the population the compiler sees, prints an examined count, and turns red on the next unpinned implementation.
- **A guard that uses `git grep` with a `'crates/*/src'` pathspec** — rejected because that pathspec matches nothing and reads as a false zero.
- **A lint or a trait change that makes `SHAPE` a computed value** — rejected because it edits production code (R6) for a test-side property, and the literal is what the operator reads.

## Decision Outcome

Chosen option: the 15 rows in `scripts/mutation-rows.d/S19200-S19299.json`, tests that spell each
literal, and `scripts/tests/test_setting_shapes.py` as the guard, because each of the three keeps the
mutant, the pin and the population in the delivery that found them. `Freshness` and `VaultRoot` are
killed by named tests that already spell their literals and need no row.

### Consequences

- Good, because a rewrite of any shape's words fails a named test, and a new implementation with an
  unspelled shape fails the guard.
- Good, because no other SPEC's band or text changes.
- Good, because a shape counts as pinned only by a test of the crate (its `tests/`, or the
  `#[cfg(test)]` module of the impl's own file, comments of both forms not counting, and nothing
  after that module) or by a row on the impl's own file, and a literal that two impls of one crate share needs a row on each file.
- Bad, because the guard reads text, not the compiler: a spelling in a test that never asserts on
  it still counts, and the row is what proves the test kills the mutant.

### Confirmation

`python3 -m unittest discover -s scripts/tests -p test_setting_shapes.py` prints `examined 24
Setting impl(s)` and passes; `python3 scripts/mutation_rows.py prove` reads KILLED for each of the
15 rows; `census` and `retired --base <merge base>` exit 0.

## More Information

Issue #336; SPEC-192; ADR-057; SPEC-057 R20.

## Amendment (2026-09-30): the band continues at S19300 to S19309

SPEC-192's band, S19200 to S19299, is full, and five more rows pin arms of the guard's reader
(SPEC-192 section 11). They take the ids S19300 to S19309, the continuation of this delivery's band,
in `scripts/mutation-rows.d/S19300-S19399.json`, a file whose name spans S19300 to S19399 and that
holds only ids from S19300 to S19309. The rows still live in the delivering SPEC's band, as the
decision above requires; only the band's end moves. The file's name is a hundred wide because
`scripts/mutation_rows.py` refuses a fragment that is not named for one SPEC's band,
`S<NNN>00-S<NNN>99.json` (ADR-057), so a ten-wide `S19300-S19309.json` is refused.

- Chosen: the continuation S19300 to S19309 in `scripts/mutation-rows.d/S19300-S19399.json`, because the rows are this delivery's work and the loader reads only a hundred-wide file name.
- Rejected, a ten-wide file `scripts/mutation-rows.d/S19300-S19309.json`, because the loader refuses a fragment not named `S<NNN>00-S<NNN>99.json`.
- Rejected, the five rows in another SPEC's band, because a band belongs to the delivery whose SPEC allots it (the second decision driver above).
- Rejected, retiring rows of S19200 to S19299 to free their ids, because `retired` refuses a row that leaves while its target file stays, and each of those rows still kills a mutant.
