---
status: "accepted"
---

# ADR-655: the census set runs once per run in its own ci job

Decides SPEC-632.

## Context and Problem Statement

The test binary `deck-streak-progression::xp_census` judges the workspace's source, not the
shipped build: each of its tests spawns `cargo check --workspace` in a target directory of its own,
with a cleared environment and no `--release`. Its verdict is therefore the same in the test and
the release profile, yet it ran in both test stages, in the `rust` job and in the `release` job,
and its slowest test set the floor of both (1,357 s and 1,045 s in run 41962635280). Where should
it run, and what does one more job do to the pull request run's Rust leg ceiling?

## Decision Drivers

- A test runs where its verdict can differ: the census's cannot differ between profiles.
- Every test of the workspace runs in exactly one stage of a profile, by one filterset each, as
  the engine set already does (SPEC-038 R13).
- The release job's region of `ci.yml` is another change's work, so the new job stays clear of it.
- A pull request run never generates more than the 256 jobs GitHub allows it.

## Considered Options (the alternatives each was chosen against)

- Chosen: one census set, `CENSUS_TESTS` in `scripts/check.sh` beside `ENGINE_TESTS`, taken out
  of `test` and `test-release` and run once by a new stage `test-census` in a new job `census`,
  placed after the `rust` job and before the engine job's comment, because the census then costs
  one test-profile run per pull request and the release region stays untouched.
- Keep the census in both stages: rejected because the second run measures nothing the first did
  not, and it holds the release job's floor at the census's slowest test.
- Run the census in the `rust` job only, out of `test-release` alone: rejected because the `rust`
  job then still waits on the census's slowest test after fmt, clippy and the rest of its tests,
  so the run's floor does not move.
- Run the census in the existing `engine` job beside the engine set: rejected because the engine
  job is a two-slice matrix that deals the engine set's tests out in turn, and a third binary in
  it would change what each slice runs and its cache-sized bound.
- Keep the ceiling at 185 and count the census job against the headroom: rejected because SPEC-417
  R11 derives the ceiling from the jobs a run generates, and a ceiling one past that derivation
  lets a run need 257 jobs.

## Decision Outcome

Chosen option: the census set runs once per run, in the test profile, in the job `census`, which
restores the Rust cache and saves none, has no matrix and no needs, and is a need of `ci` right
after `engine`. `stage_test` and `stage_test_release` run `not ($ENGINE_TESTS | $CENSUS_TESTS)`;
`stage_test_census` runs `$CENSUS_TESTS` with its `--test` targets derived from the definition, in
`stage_test_engine`'s form with no slice. `LEG_CEILING["ci"]` falls from 185 to 184 by SPEC-417
R11's unchanged rule, 256 less 72 other `ci.yml` jobs, with every pin of it in the same pull
request; the maintainers' rule that each later `ci.yml` job lowers it by its own job count needs
no signed record for the step.

### Consequences

- Good: the release job's floor no longer includes the census, and the census runs once.
- Good: `test_census_set_spawns_its_own_cargo` keeps the set to binaries that spawn their own
  cargo, so a test of shipped behaviour cannot be moved out of the release stage through it.
- Bad: one more hosted job per run, and one more `ci` need a later layout change must keep.
- Bad: the ceiling's pins move again, with this change, in five test modules and one row.

### Confirmation

A1 to A10 of SPEC-632, and, after the land, one push run on dev whose `census` job runs every
test of the set and whose `rust` and `release` nextest summaries list no `xp_census` test.

## What would make this wrong

- A census test whose verdict depends on the outer profile: it would then belong in both stages.
- A run whose census job takes longer than the `rust` job and the release job together: the job's
  split would then be measured again.

## More Information

SPEC-330 and SPEC-038 each gain an insert-only amendment section that states the census set;
SPEC-618 section 9 is the precedent the ceiling's pins follow.
