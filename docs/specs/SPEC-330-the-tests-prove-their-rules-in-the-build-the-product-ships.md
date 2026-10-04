# SPEC-330: the tests prove their rules in the build the product ships

- **Issue:** #473. **Context(s):** none (the gate and the lint configuration, not a bounded context).
- **Decided by:** ADR-330 (the release profile as is, one more gate stage, two disallowed macros).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-330.md`. **Mutation band:** none (section 6).

## 1. The problem, measured

Measured at dev `f9cd40be`.

- The daemon ships from `cargo build --release --locked -p deck-streak-daemon --bin deckstreakd`
  (`.github/workflows/release.yml:70`; the package is `deck-streak-daemon`). The workspace
  `Cargo.toml` declares `[profile.dev]` and `[profile.dev.package."*"]` only (lines 20-24) and no
  `[profile.release]`, so the shipped build has Cargo's release defaults: `debug-assertions = false`
  and `overflow-checks = false`.
- Every test stage runs the `test` profile, which inherits `dev`: `debug-assertions = true` and
  `overflow-checks = true` (`scripts/check.sh` `stage_test`, `stage_test_engine`, `stage_doctest`).
  Code the build selects, with `cfg!(debug_assertions)`, `#[cfg(debug_assertions)]` or an
  environment read at compile time with `option_env!`, is compiled in one of the two builds and not
  the other, so a rule proved by the tests may not hold in the shipped binary.
- `grep -rn 'cfg!\|option_env!' --include='*.rs' crates` finds no use at f9cd40be. The only
  build-selected shapes in the tree are spelled inside string literals of
  `crates/progression/tests/xp_census.rs`, which is the census that refuses them
  and `crates/progression/build.rs:11` declares the `settle_census` cfg.
- One existing release-profile test run exists: `.github/workflows/engine-measure.yml:55-67`, which
  covers the one `engine_budget` binary only.

## 2. Requirements

R1. The gate has a stage `test-release` that runs the workspace's tests, every crate, in the
    release profile, whose `debug-assertions` and `overflow-checks` are the shipped build's.
R2. The stage runs every test the `test` stage runs: the workspace minus the engine set that
    `test-engine` runs (`ENGINE_TESTS`, `scripts/check.sh`), under the same `--locked` and
    `--no-fail-fast`.
R3. CI runs the stage in one job `release`, beside `rust`, with the same runner, toolchain pin,
    protoc pin and Rust cache restore, and the aggregate `ci` job needs it.
R4. `clippy.toml` lists `std::cfg` and `std::option_env` under `disallowed-macros`, each with its
    reason, so a use in workspace code fails `cargo clippy -- -D warnings`.
R5. A use that must stay is allowed by name at its site with
    `#[expect(clippy::disallowed_macros, reason = "...")]`. There is none at f9cd40be.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | code that changes a tested outcome only when debug assertions are off fails the new stage: the stage exists, runs the release profile over the workspace, and CI runs it in a job `ci` needs | `python3 -m unittest discover -s scripts/tests -p test_ship_profile.py -k test_the_release_stage` |
| A2 | a use of `cfg!` or `option_env!` in workspace code fails clippy: both macros are disallowed, each with a reason | `python3 -m unittest discover -s scripts/tests -p test_ship_profile.py -k test_clippy_refuses` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ship_profile.py -k test_the_release_stage
A2: python3 -m unittest discover -s scripts/tests -p test_ship_profile.py -k test_clippy_refuses
```

The behaviour each test stands for is also measured with a planted use, quoted in
`docs/red-first/SPEC-330.md`: a `#[cfg(not(debug_assertions))]` branch that changes a tested outcome
fails `bash scripts/check.sh test-release` and passes without it, and a planted `cfg!` fails
`cargo clippy`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/check.sh` | gate | the `test-release` stage and its roster entry |
| `.github/workflows/ci.yml` | gate | the `release` job and the `ci` job's need of it |
| `clippy.toml` | lint | `disallowed-macros` for `std::cfg` and `std::option_env` |
| `scripts/tests/test_ship_profile.py` | gate | A1 and A2 |
| `scripts/tests/test_ci_workflows.py` | gate | the job in the owner layout, the compile set and the `ci` needs |
| `scripts/tests/test_check_gate.py` | gate | the stage's tools |
| `docs/TESTING.md` | docs | the job in the layout table |
| `crates/kernel/tests/logging.rs` | test | `each_json_line_opens_with_its_journal_priority` reads the build's static max level (ruling 150) |
| `docs/specs/SPEC-330-...md`, `docs/decisions/ADR-330-...md`, `docs/red-first/SPEC-330.md`, `changelog.d/ci-ship-profile-473.md` | docs | added |

## 5. What this does NOT do

- It does not run the engine set (`sync`, `engine_budget`) a second time in release: `engine-measure.yml`
  already runs `engine_budget` in release and the set is two binaries built on Anki's engine (#473).
- It does not set `[profile.release]` or add a named test profile (ADR-330 chose against both) (#473).
- It does not run the doctests in release: a doctest is compiled by rustdoc under its own flags (#473).

## 6. Risks

- The release build of the workspace is slower than the dev one; the job's `timeout-minutes` bounds
  it and CI's own duration decides whether it needs a slice matrix (ADR-330).
- Mutation rows: none. The change is a shell stage, YAML and TOML; no Rust or Python function is
  added whose mutant a row could reach, and the Python test reads files rather than computing.
- FORMAL: not applicable, no state machine and no arithmetic invariant.
