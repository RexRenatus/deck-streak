---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The engine legs name the test targets their set derives

## Context and Problem Statement

The `engine` job runs `cargo nextest run --workspace --locked --no-fail-fast -E '<the engine set>'`
in two slices (SPEC-038 R13, R16). Nextest builds every test target of the selected packages before
it filters, so each slice builds the whole workspace's test targets and then runs the two binaries
the set names (#407). How does the engine stage stop building targets its filterset never runs,
without narrowing the package scope and without running fewer tests?

## Decision Drivers

- Every examined count is unchanged: each slice's test count, the `test` stage's count, the mutation
  census and the tests that run in exactly one stage (the lane's hard limit).
- No dependency recompiles: the package selection fixes the feature resolution the cache was built under.
- The set is defined once (`ENGINE_TESTS`), so a second copy of its binaries must not exist.

## Considered Options (the alternatives it was chosen against)

- Chosen, because the set already names its binaries and the targets follow from them: derive one
  `--test <target>` per `binary_id(=<package>::<target>)` in `ENGINE_TESTS` inside `stage_test_engine`,
  and keep `--workspace`, `--locked`, `--no-fail-fast`, the filterset and the partition.
- `-p deck-streak-ingest`: rejected, because SPEC-038 measured it and Cargo unifies features across
  the selected packages, so a build of the ingest package alone resolved other features and
  recompiled 22 dependencies.
- A hand-written `--test sync --test engine_budget` beside the filterset: rejected, because it is a
  second definition of the set that can drift from `ENGINE_TESTS`, and a target missing from it would
  silently drop tests from both stages.
- `--workspace` kept as it is, with no target flag: rejected, because each slice then builds every
  test target of the workspace to run two of them, the cost #407 removes, and it runs no test the
  derived flags leave out.
- One slice instead of two: rejected, because it is R16's reason reversed; the wall of one leg would
  return to the whole set, and it does not change what a leg builds.

## Decision Outcome

`stage_test_engine` reads `ENGINE_TESTS` and emits one `--test <target>` for each whole binary it
names. `test` names no target. A21 in `test_check_gate.py` holds the derivation, and A16's comparison
of the two commands ignores the derived pairs and nothing else. Two hand-proved rows in band
S03800-S03899 kill a change of the flag and of the scope.

### Consequences

- Good, because a slice builds two test targets in place of every one of the workspace, with the
  same tests run.
- Good, because a target cannot drift from the set.
- Bad, because the derivation reads the binary ids by pattern, so it is complete only for a set that
  is a `|` union of whole `binary_id(=<package>::<target>)` terms. A17 refuses every other set, and
  that refusal is what keeps it complete: a set naming whole binaries beside another term (a
  `test(...)` or `package(...)` predicate, a glob or regex binary id, `&` or `not`) would derive
  only those binaries, and a test the other term names would run in neither stage. A set naming no
  whole binary derives no target and builds every target, as before, which A21's planted set shows.

### Confirmation

SPEC-038 A21, rows S03801 and S03802, and equal counts at one commit: each slice's `Starting N tests
across 2 binaries` line, the `test` stage's count and the census, before and after, with the engine
log's `Compiling` lines naming workspace crates only.

## What would make this wrong

- A dependency recompiling under the derived flags: SPEC-038's reason for `--workspace` would then
  stand against this change, and it is withdrawn.

## More Information

SPEC-038 (amendment of 2026-09-29), ADR-055, issue #407.
