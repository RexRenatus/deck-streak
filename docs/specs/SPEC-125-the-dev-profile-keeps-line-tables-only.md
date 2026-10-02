# SPEC-125: the dev profile keeps line tables only in workspace crates and none in dependencies

- **Wave:** W4. **Issue:** #353. **Context(s):** `repo` (the workspace `Cargo.toml` and one test).
- **Decided by:** ADR-125 (this SPEC's own: the two profile values, and what they were chosen
  against).
- **Status:** delivered. It holds `docs/red-first/SPEC-125.md`.

## 1. The problem, measured

Measured at `dev` 026d1f3, on a cold build of the daemon and api test targets
(`cargo test -p deck-streak-daemon -p deck-streak-api --no-run`).

- **The workspace has no `[profile]` section**, so every dev and test build carries full debuginfo
  for every crate, dependencies included (the Cargo book gives `full` as `dev`'s default).
- **Full debuginfo made the target about 3.6 times the size** of the same build with line tables
  only in workspace crates and no debuginfo in dependencies. That ratio compares a build with
  incremental compilation off against one with it on. The manifest alone, incremental unchanged,
  makes the target 64% smaller (about 2.8 times).
- **Nothing else changed.** Both crates' failure sets were empty under both profiles, and a panic
  backtrace under the lever still named a workspace frame's file and line
  (`tests/lifecycle.rs:187:5`).

The Cargo book (through context7) says: "`line-tables-only`: line tables only. Generates the minimal
amount of debug info for backtraces with filename/line number info"; "The `test` profile inherits
the settings from the `dev` profile"; and "To override the settings for all dependencies (but not
any workspace member), use the `"*"` package name". Its build-performance guide recommends exactly
`[profile.dev] debug = "line-tables-only"` with `[profile.dev.package."*"] debug = false`.

## 2. Requirements

R1. The workspace `Cargo.toml` sets `[profile.dev] debug = "line-tables-only"`, so workspace crates
    build with line tables only; the `test` profile inherits it.
R2. The same manifest sets `[profile.dev.package."*"] debug = false`, so every dependency builds
    with no debuginfo.
R3. A test pins both values by their whole value, and pins that no workflow or script sets a second
    profile by environment or `--config`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the test reads `debug = "line-tables-only"` under `profile.dev` and `debug = false` under `profile.dev.package."*"` from `Cargo.toml`; it is red before the change and green after | `test_build_profile.py` |
| A2 | the manifest declares the profile, no workflow or script overrides it, and the jobs that build Rust on this diff (`rust`, `engine (1)` and `engine (2)`) pass on the new profile; `mutation-rust` builds nothing on a diff that changes no Rust production file | `test_build_profile.py`, and the CI run of the head |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_build_profile.py -k test_workspace_crates_build_with_line_tables_only -k test_every_dependency_builds_without_debuginfo
A2: python3 -m unittest discover -s scripts/tests -p test_build_profile.py -k test_the_profile_lives_in_the_manifest_and_no_job_overrides_it
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `Cargo.toml` | repo | changed: R1, R2, the two profile sections |
| `scripts/tests/test_build_profile.py` | repo | added: A1, A2 |
| `docs/specs/SPEC-125-the-dev-profile-keeps-line-tables-only.md` | repo | added |
| `docs/decisions/ADR-125-the-profile-lives-in-the-manifest-not-in-environments-or-flags.md` | repo | added |
| `docs/red-first/SPEC-125.md` | repo | added |
| `changelog.d/build-dev-profile-125.md` | repo | added |

No schematic: the change adds no component and no state machine.

## 5. What this does NOT do

- It changes no `release` profile and no workflow: #353 names the `dev` profile only, and CI
  already sets `CARGO_INCREMENTAL=0` on its own.
- It adds no opt-in `debugging` profile: #353 asks for two values and a test, and a contributor
  who wants full debuginfo passes their own `--config`.

## 6. Risks

- **A debugger loses variable and type information for workspace crates.** Backtraces keep file and
  line; a contributor who needs full debuginfo builds once with their own profile.
- **A changed profile invalidates every cached build once.** Cargo's fingerprints include the
  profile, so the first CI run and each contributor's first build rebuild from scratch.

## 7. References

Issue #353; ADR-125; the Cargo book, "Profiles" and "Build Performance".
