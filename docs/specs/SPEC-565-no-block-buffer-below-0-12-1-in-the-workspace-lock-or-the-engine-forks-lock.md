# SPEC-565: no block-buffer below 0.12.1 in the workspace's lock or in the engine fork's lock the release builds from

- **Issue:** #1025, which the maintainer's daily dependency scan filed for GHSA-qwgh-2vcv-g2f7
  (block-buffer, affected `< 0.12.1`, first patched 0.12.1). **Context(s):** the workspace's
  dependency graph (`Cargo.toml`, `Cargo.lock`, `deny.toml`), the sync server's release audit
  (`scripts/audit-sync-server.sh`) and the guard tests; no crate's own source changes.
- **Decided by:** ADR-583 (the seven sqlx crates come from a fork until a sqlx release moves them
  to sha2 0.11; why a patched copy of block-buffer 0.10.4 is refused; why linkchecker leaves the
  engine fork's workspace), and a note appended to ADR-058 for the engine pin's move.
- **Schematic:** none. No component, data flow or state machine of the service changes: the engine,
  the kernel and the sync server call the same hash functions, and only the crate versions that
  provide them move.
- **Status:** delivered by the pull request that adds this file, in two commits, red first. Commit
  1 adds the guard, its tests and fixtures, the audit line and the migration hold guard, with the
  pins and the lock untouched; commit 2 moves the engine pin, adds the sqlx patch entries and the
  allowed git source, and lets cargo write the lock. **Mutation band:** none claimed (section 8).
  **Model:** none (section 8).

## 1. The problem, measured

- **The advisory.** GHSA-qwgh-2vcv-g2f7 (medium, CVSS 4.0 6.3, CWE-119) names block-buffer
  `< 0.12.1`, first patched 0.12.1. The defect, a panic in the caller's closure that leaves the
  eager buffer's inline position corrupt, lives in 0.11.0 to 0.12.0. 0.10.4 keeps its position in a
  separate field, written only after every closure returns, so it matches the range only through
  its open lower bound; no test can be red at 0.10.4 for the advisory's defect.
- **The workspace's lock at 4e600dbb.** block-buffer 0.10.4 comes through digest 0.10.7, which
  hmac 0.12.1, pbkdf2 0.12.2, sha1 0.10.7 and sha2 0.10.9 use. The engine brings pbkdf2, sha1 and
  axum-extra's `headers` (sha1 0.10); sqlx-core 0.9.0 (feature `migrate`) and sqlx-macros-core 0.9.0
  bring sha2 ^0.10.0. The guard reads `block-buffer below 0.12.1: 1 of 2` and
  `digest below 0.11.0: 1 of 2`.
- **The engine fork's lock at a078820.** The release builds the sync server from it
  (`cargo install --locked` at the pinned commit). It pins block-buffer 0.12.0, the version with the
  defect, and keeps 0.10.4; the guard reads `2 of 2` and `1 of 2`. Before this delivery nothing was
  red there: the release audit's cargo deny passes, because no RustSec record names the advisory.
- **No sqlx release moves.** The crates.io index lists 0.9.0 as the newest sqlx-core and
  sqlx-macros-core, each requiring sha2 ^0.10.0, while sqlx-postgres and sqlx-mysql 0.9.0 already
  take sha2 0.11.0 (read on 2026-10-29).

## 2. Requirements

- **R1.** `scripts/lock-digest-line.py LOCK` (stdlib `tomllib`) reads every `[[package]]` named
  block-buffer or digest. RED, exit 1: a block-buffer below 0.12.1 or a digest below 0.11.0 is
  present; it prints `block-buffer below 0.12.1: K of N` and `digest below 0.11.0: K of M`, each
  version with its direct dependents. GREEN, exit 0: every block-buffer is 0.12.1 or later and
  every digest 0.11.0 or later. VOID, exit 2: the lock holds no block-buffer, is missing, does not
  parse, or holds a version that is not semver. Versions compare by semver, so a pre-release sorts
  below its release.
- **R2.** The workspace's `Cargo.lock` reads GREEN.
- **R3.** The engine fork's `Cargo.lock` at the pinned commit reads GREEN. The test finds it in
  cargo's checkout of the commit the root manifest's `[patch]` entry names, and never skips.
- **R4.** `scripts/audit-sync-server.sh` runs the guard over the fetched commit's lockfile before
  cargo deny. A RED or a VOID ends the audit with its exit code, and cargo deny never runs.
- **R5.** The engine pin moves to the fork commit that removes digest 0.10 from the fork's lock
  (ADR-058's note). The seven sqlx crates the lock holds (sqlx, sqlx-core, sqlx-macros,
  sqlx-macros-core, sqlx-mysql, sqlx-postgres and sqlx-sqlite) come from one RexRenatus/sqlx
  commit, which moves sqlx-core and sqlx-macros-core to sha2 0.11 (ADR-583), so the lock keeps one
  of each, every one at 0.9.0.
- **R6.** Every `MIGRATOR` migration's checksum stays the one sqlx stored before the move.
- **R7.** No advisory is ignored or excepted. `deny.toml`'s advisory ignore list does not change;
  `[sources] allow-git` gains the sqlx fork and nothing else, and the exact list in
  `scripts/tests/test_engine_pin.py` gains it too.

## 3. Acceptance criteria

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | the workspace's lock holds no block-buffer below 0.12.1 and no digest below 0.11.0 (R1, R2, R5) | red at 4e600dbb: `block-buffer below 0.12.1: 1 of 2`, `digest below 0.11.0: 1 of 2` | `test_the_lock_holds_no_block_buffer_below_0_12_1_and_no_digest_below_0_11_0` |
| A2 | the guard's verdicts over its fixtures: a078820's digest line RED with `2 of 2` and `1 of 2`; a green lock GREEN; a planted block-buffer 0.10.4, digest 0.10.7 or block-buffer 0.12.1-rc.0 RED; no block-buffer, a missing lock or one that does not parse VOID (R1) | not red: controls any guard keeps | `TheGuardJudgesItsFixtures` |
| A3 | the engine fork's lock at the pinned commit reads GREEN (R3, R5) | red at 4e600dbb: a078820's lock, `2 of 2` and `1 of 2` | `test_the_pinned_commits_own_lock_reads_green` |
| A4 | the workspace's lock with one block-buffer 0.10.4 stanza added reads RED, one more below 0.12.1 than the lock itself (R1) | not red: the non-vacuity control | `test_a_planted_block_buffer_0_10_4_turns_the_check_red` |
| A5 | the release audit ends at a RED fork lock with exit 1 and at a missing one with exit 2, and runs no cargo (R4) | red with 4e600dbb's `audit-sync-server.sh`: exit 0 for both | `test_a_red_fork_lock_ends_the_audit_before_the_check`, `test_a_missing_fork_lock_ends_the_audit_before_the_check` |
| A6 | every `MIGRATOR` checksum equals its golden, taken at 4e600dbb (R6) | not red: a hold guard; a planted wrong golden reads red | `every_migration_checksum_is_the_one_stored_before_the_sha2_move` |
| A6b | `allow-git` names exactly the engine fork, rust-url, fsrs-rs, the web-push-native fork and the sqlx fork (R7) | red with `deny.toml` edited and the test unchanged: `allow-git is [...], not exactly the fork, rust-url, fsrs-rs and web-push-native` | `test_the_engine_is_patched_by_rev_to_a_commit_of_the_fork` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_lock_digest_line.py -k test_the_lock_holds_no_block_buffer_below_0_12_1_and_no_digest_below_0_11_0
A2: python3 -m unittest discover -s scripts/tests -p test_fork_lock_guard.py -k TheGuardJudgesItsFixtures
A3: python3 -m unittest discover -s scripts/tests -p test_fork_lock_guard.py -k test_the_pinned_commits_own_lock_reads_green
A4: python3 -m unittest discover -s scripts/tests -p test_lock_digest_line.py -k test_a_planted_block_buffer_0_10_4_turns_the_check_red
A5: python3 -m unittest discover -s scripts/tests -p test_audit_sync_server.py -k fork_lock_ends_the_audit_before_the_check
A6: cargo test --locked -p deck-streak-kernel --test migration_checksums
A6b: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_the_engine_is_patched_by_rev_to_a_commit_of_the_fork
```

A1 to A5 and A6b run in the python stage of `scripts/check.sh`, A6 in the Rust test stage.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/lock-digest-line.py` | guard | added: R1 |
| `scripts/tests/test_lock_digest_line.py` | tests | added: A1, A4 |
| `scripts/tests/test_fork_lock_guard.py` | tests | added: A2, A3 |
| `scripts/tests/fixtures/fork-lock/a078820-digest-line.lock` | fixtures | added: the four digest-line stanzas of the engine fork's lock at a078820 |
| `scripts/tests/fixtures/fork-lock/green.lock`, `planted-block-buffer.lock`, `planted-digest.lock`, `planted-prerelease.lock`, `void.lock` | fixtures | added |
| `scripts/audit-sync-server.sh` | release audit | changed: the guard line before cargo deny (R4) |
| `scripts/tests/test_audit_sync_server.py` | tests | changed: setUp plants a green fork lock; A5's two tests added |
| `crates/kernel/tests/migration_checksums.rs` | tests | added: A6 |
| `crates/kernel/tests/fixtures/migration-checksums.txt` | fixtures | added: 33 goldens taken at 4e600dbb |
| `Cargo.toml` | workspace | changed (commit 2): the engine pin's rev and its comment; seven sqlx `[patch.crates-io]` entries |
| `Cargo.lock` | workspace | changed (commit 2): written by cargo |
| `deny.toml` | workspace | changed (commit 2): `allow-git` gains the sqlx fork, and the `[sources]` comment counts it |
| `scripts/tests/test_engine_pin.py` | tests | changed (commit 2): the sqlx fork joins the exact `allow-git` list (A6b) |
| `docs/decisions/ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md` | docs | changed (commit 2): a note appended for the pin's move |
| `docs/specs/SPEC-565-no-block-buffer-below-0-12-1-in-the-workspace-lock-or-the-engine-forks-lock.md` | docs | added: this file |
| `docs/decisions/ADR-583-the-sqlx-crates-come-from-a-fork-until-a-sqlx-release-moves-them-to-sha2-0-11.md` | docs | added |
| `changelog.d/deps-block-buffer-1025.md` | docs | added (commit 2): the changelog fragment |

## 5. What this does NOT do

- It does not patch block-buffer 0.10.4: there is no defect in it to fix, and a git-sourced copy
  would clear cargo deny by its source alone (ADR-583).
- It changes no engine or kernel source. The engine fork's own commits move sha1 to 0.11, pbkdf2 to
  0.13 and block-buffer to 0.12.1, and remove axum-extra and the fork's link checker; they carry
  their own hold guards (checksums, host keys, a PHC string made before the move, the sync header).
- It sends nothing upstream. The sqlx change and the advisory's affected range are each offered
  upstream once this delivery is green.

## 6. Risks

- **The sqlx fork goes or is rewritten.** The pin is a full commit id, and the fork's branch keeps
  it fetchable; a clean build fetches it (ADR-583).
- **The fork's lock moves under a later pin.** The guard reads the pinned commit's own lock, so a
  later pin that brings back block-buffer below 0.12.1 or digest 0.10 turns A3 red, and the release
  audit stops before cargo deny.
- **A RustSec record arrives.** cargo deny then reads the advisory as well; both checks agree on
  the fixed versions.

## 7. What only CI proves

A1 to A6 and A6b on GitHub's runner, the advisory audit (`audit-rust`) over the moved lock, and the release
audit at the pinned commit when a release runs.

## 8. Formal model and mutation rows

None. The change adds no actor, shared state or write path. The guard's verdicts are pinned by the
fixture legs (A2), its RED by A1's control (A4) and the audit's two exits by A5; no mutation rows
are claimed in this delivery.
