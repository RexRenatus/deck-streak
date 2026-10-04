# SPEC-337: the sync server is built at the tag from the engine's pin, runs as its own unit behind the web origin, and its data has a drilled offsite snapshot

- **Issue:** #617 (the app campaign's packaging item; the host commands are #161). **Context(s):**
  none (the release workflow and the deploy templates, not a bounded context).
- **Decided by:** ADR-347 (the build, the credential path, the unit's name, the route, the snapshot's
  form, the runbook's place), under ADR-340 (the move), ADR-058 and ADR-336 (the fork), ADR-032 (the
  host budget), ADR-064 (the backup), ADR-010 and ADR-038 (the credentials).
- **Status:** delivered in parts by the pull requests that add and extend this file. This part
  delivers R1 (section 3) and the measurement R3 needs (section 1); section 7 names the parts after
  it and what each waits on. **Mutation band:** S33700-S33799 (section 8).

## 1. The problem, measured

Each figure below was read at DeckStreak `dev` 03a4f16f, or measured on the maintainer's machine
with the server binary this SPEC's build step produces.

| what | measured | how |
|---|---|---|
| the release builds the server | no: one build line, `deckstreakd` alone, and one binary installed into the tarball | `grep -n 'cargo build\|cargo install\|install -m' .github/workflows/release.yml` (lines 70 and 87) |
| a unit runs the server | no | `ls deploy/systemd \| grep -c '^deck-streak-sync-server\.'` reads 0 |
| the edge routes to it | no | `grep -ci 'anki-sync' deploy/caddy/deck-streak.caddy` reads 0 |
| the backup copies its data | no: the database alone, "never the collection" (ADR-064) | `deploy/scripts/backup.py`'s docstring; `restore-drill.sh` restores the replica and the newest daily copy only |
| the share's room for a new long-running unit | none: the long-running ceilings (API 128M, bot 96M, MCP 32M, replicator 64M: 320M) plus the largest oneshot (384M) equal the 704M share, and their processor quotas (75 + 50 + 25 + 50) equal the share's 2 processors | `deploy/host-budget.json`, the units' `CPUQuota=`, `test_the_daemons_and_the_largest_job_fit_the_stack_share` (which asserts the sum EQUAL to the share) |
| the server's resident memory, idle | 9.2 MiB (`VmRSS` 9384 kB), with one worker thread per processor | the server on loopback over an empty scratch `SYNC_BASE` with two synthetic users, `/proc/<pid>/status` |
| the server's resident memory, one full sync of ADR-022's synthetic collection (250,000 cards, 200,000 reviews) | peak 319 MiB (`VmHWM` 326600 kB) during the full upload, back to 24 MiB after it; the full download set no new peak (27 MiB after it) | the same server, a scratch client built on the same engine commit driving `full_upload` then `full_download`, `VmHWM` read after each and `VmRSS` sampled every tenth of a second |
| the server's configuration names | `SYNC_USER<n>` (`name:password`, or `name:<PHC hash>` when `PASSWORDS_HASHED` is set), `SYNC_BASE`, `SYNC_HOST`, `SYNC_PORT`, `SYNC_IP_HEADER`, `MAX_SYNC_PAYLOAD_MEGS`; routes `/sync/`, `/msync/`, `/health`; a graceful stop on an interrupt only; no readiness notification | the fork's own source at the pinned commit (`rslib/sync/main.rs`, `rslib/src/sync/http_server/mod.rs`), and the server's documentation in the same tree |
| the server's data layout | `<SYNC_BASE>/<user>/collection.anki2`, `media.db` and `media/`; a full upload writes a temporary file and renames it over the collection | the same source (`http_server/user.rs`, `media_manager/mod.rs`, `collection/upload.rs`) |

So the server's own entry, sized from the peak, cannot fit the share as it stands: a resize is the
owner's decision (ADR-340, "What would make this wrong"; ADR-032), and this SPEC changes no share.

## 2. Requirements

R1. The release's one job builds the sync server (`anki-sync-server`) with `cargo install --locked`
    from the fork and the commit that `Cargo.toml`'s `[patch."https://github.com/ankitects/anki.git"]`
    entry names, after the tag guard and the protobuf compiler, and the tarball carries it as
    `bin/anki-sync-server`, so `MANIFEST.sha256`, `SHA256SUMS` and the one attestation cover it.
    The step refuses, before cargo runs, a manifest with no such entry and a `rev` that is not a
    full 40-hex commit. The workflow holds no second copy of the fork or the commit, and its
    trigger, permissions and token holders are unchanged.
R2. `deploy/systemd/deck-streak-sync-server.service` runs the server as DeckStreak's user on
    loopback, `Type=exec`, stopped by an interrupt, with the hardening every DeckStreak unit carries;
    its two users (the owner's and ADR-344's staging user) come from two `LoadCredential=` lines in
    the socket form, each holding `name:<PHC pbkdf2-sha256 hash>`; the launcher
    `deploy/scripts/sync-server.sh` refuses an entry of any other shape, sets `PASSWORDS_HASHED`
    and execs the server. No secret, real or placeholder hash, appears in the tree.
R3. The unit's `MemoryHigh=`/`MemoryMax=` equal its own `deploy/host-budget.json` entry, sized from
    section 1's peak, and the entry is added only when the share holds it
    (`test_the_daemons_and_the_largest_job_fit_the_stack_share`).
R4. The Caddy block serves the server on the web app's origin under `/anki-sync/`: the bare path
    redirects to the slash form, the prefix is stripped, the server's own health route answers 404
    at the edge, the request body is bounded at the server's own payload limit, and the proxy reads
    with a larger buffer; the security headers and the hidden health routes stay as they are.
R5. The daily backup also snapshots `SYNC_BASE` (each user's two databases by SQLite's online
    backup and `PRAGMA integrity_check`, the media files, one archive with its sha256 digests),
    copies it to an offsite bucket whose name is configuration and which admits no public access
    and no listing, and the weekly restore drill restores the newest archive into a scratch
    directory, checks it and opens the collection.
R6. `docs/runbooks/sync-server-cutover.md` holds ADR-340's cutover in order, the staging rehearsal
    first, each host step marked as the owner's go (#161), the rollback, and the owner's own acts;
    it names no host, address, provider, date or secret.
R7. Every new Python or shell function a test owns, and the release step's guard, has a row in
    S33700-S33799, proved on a committed tree.

## 3. Acceptance criteria

This part's criteria (R1). Section 7 holds the rest.

| id | criterion | decided by |
|---|---|---|
| A1 | the release builds the server after the tag guard and the protobuf compiler, from the fork and commit read at run time (no 40-hex literal in the step), and installs it into the tarball before the manifest is written | `python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_sync_server_is_built_after_the_guard_and_shipped_in_the_tarball` |
| A2 | run under bash with cargo stubbed, the step passes the patch entry's fork and commit to `cargo install --locked`, and refuses a manifest with no patch entry, a branch for a commit and a short commit before cargo runs; at the tree's own manifest it builds the fork, not upstream | `python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_build_step_reads_the_fork_and_the_commit_from_the_engines_patch_entry` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_sync_server_is_built_after_the_guard_and_shipped_in_the_tarball
A2: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_build_step_reads_the_fork_and_the_commit_from_the_engines_patch_entry
```

## 4. File manifest

| file | context | change | part |
|---|---|---|---|
| `docs/specs/SPEC-337-the-sync-server-is-built-at-the-tag-runs-as-its-own-unit-and-has-a-drilled-offsite-snapshot.md` | docs | added | this |
| `docs/decisions/ADR-347-the-sync-server-is-built-from-the-engines-pin-reads-hashed-users-from-credentials-and-serves-under-its-own-path.md` | docs | added | this |
| `docs/schematics/sync-server-packaging-and-cutover.md` | docs | added | this |
| `docs/red-first/SPEC-337.md` | docs | added | this |
| `.github/workflows/release.yml` | CI | changed | this |
| `scripts/tests/test_release_workflow.py` | tests | changed | this |
| `scripts/tests/test_ci_workflows.py` | tests | changed (one census entry for A2's process) | this |
| `scripts/mutation-rows.d/S33700-S33799.json` | tests | added | this |
| `changelog.d/sync-server-packaging-337.md` | docs | added | this |
| `deploy/systemd/deck-streak-sync-server.service`, `deploy/scripts/sync-server.sh` | deploy | added | the unit's |
| `deploy/host-budget.json`, `docs/decisions/ADR-032-deploy-templates-and-the-host-budget.md` (a note) | deploy | changed | the unit's |
| `deploy/deck-streak.env.example`, `deploy/README.md` | deploy | changed | the unit's |
| `deploy/caddy/deck-streak.caddy`, `deploy/scripts/render-caddy.py` | deploy | changed | the route's |
| `deploy/scripts/backup.py`, `deploy/scripts/restore-drill.sh`, `deploy/systemd/deck-streak-backup.service`, `deploy/systemd/deck-streak-restore-drill.service` | deploy | changed | the snapshot's |
| `PRIVACY.md`, `privacy.json` | privacy | changed (the snapshot as a store) | the snapshot's |
| `docs/runbooks/sync-server-cutover.md` | docs | added | the runbook's |
| `scripts/tests/test_deploy_templates.py`, `scripts/tests/test_caddy_render.py`, `scripts/tests/test_backup_units.py`, a launcher test module | tests | changed or added | each part's |

## 5. What this does NOT do

- It runs nothing on any host: the install, the unit's first start, the route's install, the
  bucket's creation and its access policy, and every cutover step are commands the seat records in
  #161 before any run.
- It raises no share. The unit's entry waits on the owner's decision on a resize, which section 1's
  measurement informs (#617).
- The security review of the server, the browser's sync credential and the route before the server
  faces the internet is ADR-340's, with any host finding in #167.
- The browser client's sync and the one-way upload's snapshot check (SPEC-334 R8, R9) are the app
  campaign's (#617).
- Repointing the owner's devices and retiring the old server are the owner's acts, asked at the
  time (#161).
- Anki's hosted sync service stays outside DeckStreak (#172).

## 6. Risks

- The release job gains a second build of the engine. Its timeout bounds it, and the first tagged
  run's timing reads it; the build is the fork's own lockfile under `--locked`, so a fork commit
  whose lockfile is stale fails the release at the tag instead of shipping an unpinned build.
- The server builds with DeckStreak's pinned toolchain (`rust-toolchain.toml` at the root), not the
  fork's own pin. Measured: it builds at the pinned commit. A future fork commit that needs a newer
  toolchain fails at the tag; a pull request that moves the patch entry is where to catch it first
  (section 7, the unit's part).
- A malformed hash fails only at the first login, inside the server, so the launcher checks the
  shape at start (R2): the unit then fails at once and pages through `OnFailure=`.
- The peak scales with the collection uploaded whole. Section 1 measures ADR-022's shape, which is
  the budget's reference; the memory watch reads the real figure after the cutover (SPEC-031).

## 7. Delivered by the next pull requests

| id | criterion | delivered by |
|---|---|---|
| A3 | the unit renders, carries the hardening set, equals its budget entry, and the share holds it | the unit's part, after the owner's decision on the share (#617) |
| A4 | the unit's two credentials are in the socket form and no unit passes a user through its environment | the unit's part |
| A5 | the launcher refuses a missing, empty or unhashed entry and execs the server with `PASSWORDS_HASHED` set | the unit's part |
| A6 | the block renders with the route, the redirect, the stripped prefix, the hidden health route, the body bound and the buffer, and names the path | the route's part |
| A7 | the snapshot holds each user's checked databases and media with their digests, and a failed check leaves the previous snapshots | the snapshot's part |
| A8 | the drill restores the newest archive into a scratch directory, checks it and opens the collection, and fails on a corrupt archive | the snapshot's part |
| A9 | the runbook holds every state of the schematic's sequence in order, each host step marked as the owner's go | the runbook's part |

## 8. The mutation rows

S33700-S33799. This part's rows hold the release step's guard and its tarball line against A1 and
A2: a step that admits a branch or a short commit, a build without `--locked`, a second copy of the
pin, and a tarball without the server. The later parts add rows for the launcher, render-caddy's
new placeholder, the snapshot and the drill.

## 9. References

ADR-340, ADR-347, ADR-058, ADR-336, ADR-344, ADR-032, ADR-064, ADR-062, ADR-010, ADR-038; SPEC-334
(row 1.3, R8, R9), SPEC-064, SPEC-062; `docs/schematics/sync-server-packaging-and-cutover.md`;
`RELEASING.md`; `deploy/README.md`.
