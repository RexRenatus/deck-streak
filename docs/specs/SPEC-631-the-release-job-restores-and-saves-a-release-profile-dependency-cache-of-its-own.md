# SPEC-631: the release job restores and saves a release-profile dependency cache of its own

- **Issue:** none. Amends SPEC-038 R1 and SPEC-330 R3 by insert-only sections appended to each.
  **Context(s):** `.github/workflows/ci.yml` (the `release` job only) and the tests that read it; no
  script, crate, dependency or lockfile changes.
- **Decided by:** ADR-654 (the release job keeps a release-profile cache key of its own and no
  longer restores the debug key).
- **Schematic:** `docs/schematics/release-profile-cache.md`, added.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-631.md`. **Mutation band:** none: no row is written and no band is claimed.
  **Models:** none.

## 1. The problem, measured

Measured at the base of this pull request (`dev` when it was cut).

- The `release` job runs the workspace's tests in the release profile (SPEC-330), and its only
  cache restore is the debug key of the `rust` job: `git show <base>:.github/workflows/ci.yml`
  holds, in the `release` job (lines 162-209), one `actions/cache/restore` step (lines 184-195)
  under `rust-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-${{ hashFiles('Cargo.lock') }}`.
- That key holds `target/debug`, which the release profile never reads. The job's own comment says
  so (lines 156-161): the release build lands under `target/release`, "which the cache does not
  hold, so this job compiles its own". So every run compiles every dependency cold in the release
  profile, and the restore only spends time downloading what the job never reads.
- The `rust` job alone saves the debug key, after `cargo clean --workspace` (lines 81-92), and
  the scheduled workflow `rust-cache.yml` saves the same key from a second writer (SPEC-191).
  Two writers of one key race, so the `release` job could not simply save the debug key too.
- The census tests pin that shape: `test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile`
  (`scripts/tests/test_ci_workflows.py`, lines 2928-2976) asserts exactly one save step, and
  `test_its_key_and_paths_are_the_ones_ci_and_the_weekly_battery_restore`
  (`scripts/tests/test_rust_cache_workflow.py`, lines 145-155) asserts that every restore of the
  cache's four paths in `ci.yml` and `mutation-weekly.yml` uses the lookup key of `rust-cache.yml`.
  A release-profile key fails both, by design, until each is restated per key (R5, R6).

## 2. Requirements

R1. The `release` job's restore step has the id `release-cache`, restores the four paths of the
    Rust cache (`~/.cargo/registry/index/`, `~/.cargo/registry/cache/`, `~/.cargo/git/db/`,
    `target/`) under the key
    `rust-release-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-${{ hashFiles('Cargo.lock') }}`
    and falls back to the restore-keys prefix
    `rust-release-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-`. The job restores no
    other Rust cache key: the debug key, whose `target/debug` the release profile never reads, is
    dropped from it.
R2. After the `test-release` stage the job runs `cargo clean --workspace --release`, so the
    workspace's own release artifacts are removed and the dependencies' stay in `target/`, and then
    saves the four paths with `actions/cache/save` under
    `${{ steps.release-cache.outputs.cache-primary-key }}`. The clean step is the step directly
    before the save and both carry the `rust` job's condition with `steps.release-cache` in place of
    `steps.rust-cache`: a push to `dev` or `main` whose restore missed its exact key. A pull request
    restores and never saves (SPEC-038 R2).
R3. The job prints `df -h /` before the restore and after the `test-release` stage, because the
    `rust` job already ran out of runner disk (SPEC-602).
R4. One writer per key. The release key has one writer, the `release` job. The debug key keeps its
    two writers (the `rust` job and the scheduled `rust-cache.yml`). Neither restore prefix matches
    the other key: `rust-Linux-` is not a prefix of `rust-release-Linux-`, nor the reverse. Two runs
    that miss at once both try one key; a cache entry is immutable, so the second save is refused
    with a warning and nothing is overwritten (ADR-654).
R5. `test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile` asserts one save per
    profile key, each by the one `ci.yml` job that restores it, with `cargo clean --workspace` before
    the debug key's save and `cargo clean --workspace --release` before the release key's. Every
    assertion it made before stays or is restated per key.
R6. `test_its_key_and_paths_are_the_ones_ci_and_the_weekly_battery_restore` asserts that every
    restore of the cache's paths except the `release` job's uses the lookup key of `rust-cache.yml`,
    and that the `release` job's restore uses the `rust-release-` key, which no other workflow
    restores or saves.
R7. The `release` job's name, runner and `timeout-minutes: 100` (lines 162-164) stay byte-unchanged,
    the job is the same single job (no job, matrix or leg is added), and the aggregate `ci` job
    still needs it.
R8. The bounded gain: at most the cold dependency build. The workspace's own crates still compile in
    the release profile on a warm run, and the release-profile figure of a warm run is not
    measured before the land (section 7).

## 3. Acceptance criteria of SPEC-631

| id | criterion | decided by |
|---|---|---|
| A1 | the `release` job restores a `rust-release-` key under the id `release-cache` and no other Rust cache key, and after its stage cleans with `cargo clean --workspace --release` and saves the `release-cache` primary key under the push-to-dev-or-main-on-a-miss condition, with `df -h /` before the restore and after the stage | `python3 -m unittest discover -s scripts/tests -p test_release_cache.py -k test_the_release_job_restores_and_saves_its_own_profile_key` |
| A2 | every restore of the Rust cache's paths but the `release` job's uses the lookup key of `rust-cache.yml`, and the `release` job's restore uses the `rust-release-` key no other workflow holds | `python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k test_its_key_and_paths_are_the_ones_ci_and_the_weekly_battery_restore` |
| A3 | the Rust cache is saved once per profile key, each by the one job that restores it, after a clean of the matching command | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_release_cache.py -k test_the_release_job_restores_and_saves_its_own_profile_key
A2: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k test_its_key_and_paths_are_the_ones_ci_and_the_weekly_battery_restore
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile
```

A1 is red at the base by its assertion: the `release` job's only restore is the debug key. A2 and
A3 are each red against the new `ci.yml` before their amendment, which records that the amendment
was needed: A2 because the `release` job's key is not the lookup key, A3 because the workflow now
holds two saves.

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/ci.yml` | release job | changed: the restore step, two `df -h /` steps, the clean and the save, and the comment above the job |
| `scripts/tests/test_release_cache.py` | tests | added: A1 |
| `scripts/tests/test_ci_workflows.py` | tests | changed: the body of the A3 test only |
| `scripts/tests/test_rust_cache_workflow.py` | tests | changed: the body of the A2 test only |
| `docs/specs/SPEC-038-ci-runs-the-gate-in-parallel-jobs-and-only-dev-and-main-save-a-cache.md` | records | changed: one amendment section appended |
| `docs/specs/SPEC-330-the-tests-prove-their-rules-in-the-build-the-product-ships.md` | records | changed: one amendment section appended |
| `docs/specs/SPEC-631-the-release-job-restores-and-saves-a-release-profile-dependency-cache-of-its-own.md` | records | added: this SPEC |
| `docs/decisions/ADR-654-the-release-job-keeps-its-own-release-profile-cache-key-and-stops-restoring-the-debug-key.md` | records | added: its decision |
| `docs/schematics/release-profile-cache.md` | records | added: the keys, their writers and their restorers |
| `docs/red-first/SPEC-631.md` | records | added: the red-first record |
| `changelog.d/release-profile-cache-631.md` | records | added: the changelog fragment |

## 5. What this does NOT cover

- Any other job's cache: the `rust`, `engine`, `hygiene` and `web-engine` jobs restore the debug key
  and save none but `rust`, unchanged.
- `rust-cache.yml` and `mutation-weekly.yml`: neither warms nor restores the release key.
- The cache's retention, quota and cost: the repository settings are the maintainers'.
- Any change to the `test-release` stage, the release profile, `timeout-minutes` or the job count.
- Deleting a cache entry: a rollback is a revert of the pull request, and removing the entry is the
  maintainers' act.

## 6. Risks

- **A release key that is never saved.** The save runs only on a push to `dev` or `main` that
  missed its key, so the first pull request run after the land still misses; A1 holds the
  condition, and section 7 names the run that shows the entry restored.
- **Disk.** The release target directory with its dependencies is large and the runner has run out
  of disk before; R3 prints the free space before the restore and after the stage so a shortfall is
  seen in the log.
- **A second writer of the release key.** None exists; a future workflow that saves it reads A2 red,
  which pins that no other workflow restores or saves it.
- **Two simultaneous misses.** Both try one immutable key and the second save is refused with a
  warning (ADR-654); the entry the first wrote is intact.

## 7. What only CI proves

Measured after the land, on dev, by the maintainers, and named as such here:

- The first pull request run after the saving push to `dev` restores from a `rust-release-` key in
  the `release` job's log and reads a `Finished` line for the `release` profile under 600 seconds.
- The `df -h /` line after the stage reads at least 2 GiB free.
- The pull request's own runs miss the release key (no entry exists before the first saving push),
  and the `release` job's clean and save steps read skipped there.

## 8. Formal model

None: the change moves one job's cache steps and restates two census tests, with no protocol,
concurrency invariant or state machine.

## 9. Mutation rows

None. No Rust or Python function is added whose mutant a row could reach: the new test reads a
workflow file, and the two amended tests are census bodies.
