---
status: "accepted"
date: "2026-10-03"
decision-makers: "the maintainer, the builder of #473"
---

# The gate tests the profile the product ships and refuses build-selected code

## Context and Problem Statement

Tests run with debug assertions on and the shipped daemon is built with them off. Code selected by
`cfg!`, `#[cfg(debug_assertions)]` or `option_env!` is compiled in one build and not the other. How
does the gate prove the tests' rules in the shipped build, and keep such code from arriving unseen?

## Decision Drivers

- The tests must prove their rules in the build the product ships (#473).
- A new job should cost what CI can bear; the engine set is already its own job.
- The shipped profile has one definition, `.github/workflows/release.yml:70`, which passes
  `--release` and no other profile flag; the workspace declares no `[profile.release]`.

## Considered Options (the alternatives it was chosen against)

- A gate stage that runs `cargo nextest run --release` over the workspace minus the engine set -
  chosen: it is the shipped build's own flags, nothing is declared that could drift from them.
- A named test profile inheriting from release (`[profile.release-test]`) - lost: a second profile
  to keep equal to release by hand, and a build that shares no artifacts with `--release`, so it
  costs the same compile as `--release` with a way to diverge.
- `RUSTFLAGS="-C debug-assertions=off"` on the dev profile - lost: it differs from the shipped
  profile in optimisation level and overflow checks, and a changed `RUSTFLAGS` rebuilds every
  dependency, so it costs a full build and proves a build nobody ships.
- Running the engine set in release as well - lost: two binaries on Anki's engine, the heaviest
  compile, and `engine_budget` already runs in release in `engine-measure.yml`.
- A source scan for `cfg!` and `option_env!` in a Python test - lost: clippy resolves the macro's
  path, so a renamed import or a macro-expanded use is seen, which a text scan misses.

## Decision Outcome

Chosen option: the `test-release` stage in `scripts/check.sh`, run by a `release` job in `ci.yml`
beside `rust`, and `disallowed-macros` for `std::cfg` and `std::option_env` in `clippy.toml`, each
with its reason. The job restores the Rust cache and saves none: `rust` alone saves it, because two
jobs saving one key race, and a release build lands under `target/release`, which the cache does
not hold, so the job pays its own compile. The compile is the cost: CI's `rust` job measures the dev
build, and the release job is expected to take about as long as the whole `rust` job; its
`timeout-minutes` is 60 and a slice matrix is the answer if CI's own timing shows it near that.

### Consequences

- Good, because a rule that holds only with debug assertions on now fails the new job.
- Good, because a build-selected `cfg!` or `option_env!` fails clippy until it is allowed by name.
- Bad, because one more job compiles the workspace; it runs in parallel with the others, so the
  pull request's wall time grows only if it is the slowest.
- Measured, not chosen: a pinned dependency enables the tracing crate's `release_max_level_debug`
  feature, which Cargo unifies into the workspace, so `trace!` is compiled out of the shipped
  daemon. The logging test that probes every level therefore reads the build's static maximum
  level and expects exactly the levels at or below it, with the TRACE line absent when it is
  compiled out; the dev build still expects all five.

### Confirmation

`scripts/tests/test_ship_profile.py` (SPEC-330 A1, A2) and the planted uses in
`docs/red-first/SPEC-330.md`.

## More Information

#473; SPEC-330; ADR-017 and ADR-055 (the gate and its jobs).
