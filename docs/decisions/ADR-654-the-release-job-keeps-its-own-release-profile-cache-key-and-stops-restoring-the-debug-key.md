---
status: "accepted"
---

# ADR-654: the release job keeps its own release-profile cache key and stops restoring the debug key

Decides SPEC-631.

## Context and Problem Statement

CI's `release` job runs the workspace's tests in the release profile (SPEC-330). Its only cache
restore is the debug key of the `rust` job, `rust-<os>-<toolchain hash>-<lockfile hash>`, which
holds `target/debug`; the release profile never reads it, and the job's own comment says the
release build "lands under `target/release`, which the cache does not hold". So the job downloads
a cache it does not use and compiles every dependency cold on every run. How does the job get a
dependency cache it can use, without two jobs racing on one key?

What `actions/cache` does, read from its documentation before this was written:

- The `restore` sub-action takes `key` and ordered `restore-keys`, and exposes `cache-hit` (true
  only for an exact `key` match; a prefix match from `restore-keys` leaves it false),
  `cache-primary-key` (the key it tried) and `cache-matched-key`.
- The `save` sub-action saves the given paths under the given `key`, and the documented pattern
  passes it the restore step's `cache-primary-key` behind a `cache-hit != 'true'` condition.
- A cache entry is immutable and cannot be updated. A save under a key that already exists is not
  applied: the action catches the error and logs it as a warning, so the step does not fail and
  the first entry stays as it was.

## Decision Drivers

- The release job needs the dependencies' release-profile artifacts, not the debug ones.
- One key must not have two jobs that both save it (SPEC-038 R1 and R2 keep a single writer
  per key).
- The census tests must still say what they said, key by key, and weaken nothing.
- Only a push to `dev` or `main` saves a cache, and a pull request never does (SPEC-038 R2).

## Considered Options (the alternatives each was chosen against)

- Chosen: a key of its own for the `release` job, `rust-release-<os>-<toolchain hash>-<lockfile hash>`
  with the prefix `rust-release-<os>-<toolchain hash>-` as its fallback, restored on every run and
  saved only by a push to `dev` or `main` that missed it, because the job then restores what it
  reads and the key has exactly one writer.
- Keep restoring the debug key and add a release save to it: rejected because two jobs would save
  one key (the `rust` job and the `release` job, besides the scheduled warmer), and the debug key
  holds `target/debug` the release profile never reads.
- Fold both profiles into the one existing key: rejected because the `rust` job's key would then
  change with every release build, and its clean step removes only the debug workspace artifacts.
- Switch to a cache action that saves by itself in a post step: rejected because SPEC-038 R2 admits
  only the explicit `restore` and `save` pair, which a pull request can restore from and never saves.
- Leave the job cold and accept the cost: rejected because the cold dependency build is the larger
  part of the job and a cache for it costs one key.

## Decision Outcome

Chosen: the `release` job restores and saves a release-profile key of its own.

- **D1.** The restore step of the `release` job has the id `release-cache`, the four paths of the
  Rust cache, the key `rust-release-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-${{ hashFiles('Cargo.lock') }}`
  and the restore-keys prefix `rust-release-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-`.
  The debug key is no longer restored by the job.
- **D2.** After the `test-release` stage a step runs `cargo clean --workspace --release`, then a
  step saves the four paths under `${{ steps.release-cache.outputs.cache-primary-key }}`. Both carry
  the `rust` job's condition with `steps.release-cache` in place of `steps.rust-cache`.
- **D3.** `df -h /` prints before the restore and after the stage.
- **D4.** The key never races. The release key has one writer. The debug key keeps its two (the
  `rust` job and `rust-cache.yml`). `rust-Linux-` is not a prefix of `rust-release-Linux-`, and
  `rust-release-Linux-` is not a prefix of any debug key, so neither fallback restores the other
  profile. Two runs that miss at once both try one immutable key and the second save is refused with
  a warning, which leaves the first entry intact.
- **D5.** The two census tests are restated per key (SPEC-631 R5 and R6): one save per profile key by
  the one job that restores it, with the clean command of its profile; every restore but the
  `release` job's uses the lookup key, and the release restore uses the key no other workflow holds.

### Consequences

- Good: a warm run compiles only the workspace in the release profile; the gain is at most the cold
  dependency build, and the release-profile figure of a warm run is measured after the land.
- Good: the release key has one writer, so the entry is never contended by a second job.
- Bad: the repository holds one more cache entry per `Cargo.lock` change, and the entry is large.
- Neutral: a pull request run misses the key until a push to `dev` or `main` has saved it.

### Confirmation

SPEC-631's A1, A2 and A3 in the `hygiene` job's Python stage, and, after the land, the first pull
request run after the saving push restoring from a `rust-release-` key.

## What would make this wrong

- If the release profile's dependency artifacts do not survive `cargo clean --workspace --release`
  in `target/`, the saved entry would hold nothing a later run reuses, and the warm run would read
  no faster than the cold one.
- If the runner's disk cannot hold the release target directory beside the restored entry, the job
  fails on space; the `df -h /` lines before the restore and after the stage show it.
- If a second workflow starts to save the release key, the one-writer rule fails and A2 reads red.

## More Information

SPEC-631; SPEC-038 R1 and R2 (the single saving job, and only a push to `dev` or `main` saves);
SPEC-330 R3 (the release job); SPEC-191 (the scheduled warmer of the debug key); SPEC-602 (the disk
step).
