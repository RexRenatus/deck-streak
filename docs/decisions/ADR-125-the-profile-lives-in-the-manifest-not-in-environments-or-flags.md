---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The dev profile lives in the manifest and keeps line tables only

## Context and Problem Statement

The workspace has no `[profile]` section, so every dev and test build carries full debuginfo for
every crate, dependencies included. On a cold build of two crates' test targets, that made the target
about 3.6 times the size of the same build with line tables only in workspace crates and none in
dependencies (that ratio compares a build with incremental compilation off against one with it on;
the manifest alone, incremental unchanged, makes the target 64% smaller, about 2.8 times), with identical test results and backtraces that still name each workspace frame's file
and line (SPEC-125 section 1). Where is the setting made, and to what?

## Decision Drivers

- Every contributor, CI job and tool that spawns cargo must get the same build.
- A backtrace frame in a workspace crate must keep its file and line.
- The setting should be one reviewed, tested place.

## Considered Options (the alternatives it was chosen against)

- Chosen: `[profile.dev] debug = "line-tables-only"` and `[profile.dev.package."*"] debug = false`
  in the workspace `Cargo.toml`, the shape the Cargo book's build-performance guide recommends; the
  `test` profile inherits `dev`.
- Builder-only environment settings: rejected, because CI and contributors never inherit them, so
  the build they run would differ from the one a builder measured.
- Per-command `--config` flags: rejected, because every tool that spawns cargo (the mutation runner
  among them) would need them, and two profiles on one target double it.
- `debug = false` everywhere: rejected, because backtrace frames in workspace crates would lose
  their file and line.
- Full debuginfo, as today: rejected, because of the ratio above (about 3.6 times the target with incremental off against on, and about 2.8 times from the manifest alone, for
  information no test or backtrace here reads).

## Decision Outcome

The two sections go into the workspace `Cargo.toml`. `scripts/tests/test_build_profile.py` pins both
values by their whole value and pins that no workflow or script sets a second profile, so neither
value is dropped, and no override appears, silently.

### Consequences

- Good, because the target is smaller for every build that uses the dev or test profile.
- Good, because the manifest is the one place the profile is read, so CI and builders agree.
- Bad, because a debugger loses variable and type information in workspace crates.

### Confirmation

SPEC-125's A1 and A2, and CI's Rust jobs on the new profile.

## What would make this wrong

- A test or tool that reads variable-level debuginfo from a dev build: then that consumer would take
  its own profile (`inherits = "dev"`, `debug = true`), and this ADR would stand for the default.

## More Information

SPEC-125, issue #353, the Cargo book ("Profiles", "Build Performance").
