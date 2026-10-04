# SPEC-337: the sync server is built at the tag from the engine's pin, runs as its own unit behind the web origin, and its data has a drilled offsite snapshot

- **Issue:** #617 (the app campaign's packaging item; the host commands are #161). **Context(s):**
  none (the release workflow and the deploy templates, not a bounded context).
- **Decided by:** ADR-347 (the build, the credential path, the unit's name, the route, the snapshot's
  form and its stopped-server window, the runbook's place, the cutover's data path), under ADR-340 (the move), ADR-058 and ADR-336 (the fork), ADR-032 (the
  host budget), ADR-064 (the backup), ADR-010 and ADR-038 (the credentials).
- **Status:** this pull request delivers R1 to R7 (section 3 and section 8), the measurement R3
  needs (section 1) and the measurements that decided the snapshot's window (sections 6 and 10).
  **Mutation band:** S33700-S33799 (section 8). **Model:** `formal/tla/SyncSnapshotWindow`.

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

Note on the share row: the share was then moved by the owner's decision to 1152 MiB, recorded as
ADR-064's and ADR-032's amendments of this delivery; ADR-347's amendment (D7) splits the share's
two processors among the five daemons (the API and the sync server 75% each, the bot 20%, the
replicator and the MCP server 15% each). The row above stays as it was measured.

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
R5. The daily backup's run also snapshots `SYNC_BASE` in a stopped-server window (ADR-347 D12): a
    oneshot unit of its own, ordered before the backup, stops the server, copies each user's two
    databases by SQLite's online backup, refused at once when a process holds them, and the media
    files from that one stopped generation, publishes the copy only when it is whole, and the
    server is started again whether the copy succeeds or fails. After the restart the backup checks
    each database with `PRAGMA integrity_check`, writes one archive with its sha256 digests, copies
    it to an offsite bucket whose name is configuration and which admits no public access and no
    listing, and keeps three; the weekly restore drill restores the newest archive into a scratch
    directory, checks it and opens the collection. Enabling the server enables the window, and
    disabling the server removes it. The privacy policy names the snapshots among the copies an
    erase cannot reach, with where they are kept.
R6. `docs/runbooks/sync-server-cutover.md` holds ADR-340's cutover in order, the staging rehearsal
    first, each host step marked as the owner's go (#161), the rollback, and the owner's own acts;
    it names no host, address, provider, date or secret.
R7. Every new Python or shell function a test owns, and the release step's guard, has a row in
    S33700-S33799, proved on a committed tree.

## 3. Acceptance criteria

Every criterion, R1 to R6; R7 is section 8's rows.

| id | criterion | decided by |
|---|---|---|
| A1 | the release builds the server after the tag guard and the protobuf compiler, from the fork and commit read at run time (no 40-hex literal in the step), and installs it into the tarball before the manifest is written | `python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_sync_server_is_built_after_the_guard_and_shipped_in_the_tarball` |
| A2 | run under bash with cargo stubbed, the step passes the patch entry's fork and commit to `cargo install --locked`, and refuses a manifest with no patch entry, a branch for a commit and a short commit before cargo runs; at the tree's own manifest it builds the fork, not upstream | `python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_build_step_reads_the_fork_and_the_commit_from_the_engines_patch_entry` |
| A3 | the unit runs the release's launcher, carries the hardening set and the one waiver with its why, equals its own budget entry, which holds the measured peak under `MemoryHigh=`, and the share holds it: the five daemons and the largest job fill 1152M, and their quotas divide the two processors as ADR-347 splits them | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_server_runs_hardened_within_its_entry_and_the_share_holds_it` |
| A4 | the unit's two credentials are in the socket form under two distinct ids the launcher declares, no unit passes a user through its environment and no settings line names one, and no file under `deploy/` or `docs/` holds a password hash | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_servers_two_users_come_from_the_socket_and_never_an_environment` |
| A5 | the launcher refuses a missing, empty or unhashed entry, a name that leaves the data directory, two users of one name and an address that is not loopback with a port, each by name and before the server runs, and never prints an entry; otherwise it clears what the server would read and execs it with the two users, `PASSWORDS_HASHED`, its state directory and its address | `python3 -m unittest discover -s scripts/tests -p test_sync_server_launcher.py -k test_the_launcher_refuses_a_bad_entry_and_execs_the_server_with_hashed_users` |
| A6 | the block renders with the route, the redirect, the stripped prefix, the hidden health route, the body bound and the buffer, and names the path: render-caddy fills a fourth key, the sync server's upstream, checked as the API's is and refused under its own name | `python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_caddy_block_routes_the_sync_server_under_its_own_path`, `python3 -m unittest discover -s scripts/tests -p test_caddy_render.py -k test_the_render_refuses_placeholders_and_keeps_the_headers` |
| A7 | the snapshot's window is a oneshot unit of its own that stops the server before its copy, starts it again on success and on failure, carries no condition, is bounded, needs no new privilege and is pulled in by the backup alone; a planted copy failure fails the unit and publishes nothing; a held database is refused at once and the stopped pair is copied from one generation; the daily run checks each database, archives the generation with its sha256 digests, copies it offsite and keeps three, and a generation that fails its check or has no bucket is kept and fails the run | `python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_window_stops_the_server_and_starts_it_again_whatever_the_copy_does`, `python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_a_planted_copy_failure_fails_the_unit_and_publishes_nothing`, `python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_copy_reads_only_a_stopped_generation`, `python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_daily_run_checks_archives_copies_offsite_and_keeps_three` |
| A8 | the drill restores the newest archive into a scratch directory, checks its digests and both databases, opens each user's collection and removes the scratch copy; it fails on a snapshot directory with no archive and on a changed byte, naming the member | `python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_drill_restores_the_newest_snapshot_and_opens_its_collections` |
| A9 | the runbook holds every state of the schematic's sequence in order, each host step marked as the owner's go and every other step as the owner's own act, the schematic's table naming each state; the hold on the full upload has its section before the first step; the runbook holds no address, date or URL | `python3 -m unittest discover -s scripts/tests -p test_sync_server_runbook.py -k test_the_runbook_holds_every_state_of_the_cutover_in_order_each_host_step_the_owners_go` |
| A10 | the privacy policy names the sync server's snapshots among the copies an erase cannot reach, with the three archives the host keeps and the offsite bucket | `python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_policy_discloses_every_copy_an_erase_cannot_reach` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_sync_server_is_built_after_the_guard_and_shipped_in_the_tarball
A2: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_build_step_reads_the_fork_and_the_commit_from_the_engines_patch_entry
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_server_runs_hardened_within_its_entry_and_the_share_holds_it
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_servers_two_users_come_from_the_socket_and_never_an_environment
A5: python3 -m unittest discover -s scripts/tests -p test_sync_server_launcher.py -k test_the_launcher_refuses_a_bad_entry_and_execs_the_server_with_hashed_users
A6: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_caddy_block_routes_the_sync_server_under_its_own_path
A6: python3 -m unittest discover -s scripts/tests -p test_caddy_render.py -k test_the_render_refuses_placeholders_and_keeps_the_headers
A7: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_window_stops_the_server_and_starts_it_again_whatever_the_copy_does
A7: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_a_planted_copy_failure_fails_the_unit_and_publishes_nothing
A7: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_copy_reads_only_a_stopped_generation
A7: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_daily_run_checks_archives_copies_offsite_and_keeps_three
A8: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_drill_restores_the_newest_snapshot_and_opens_its_collections
A9: python3 -m unittest discover -s scripts/tests -p test_sync_server_runbook.py -k test_the_runbook_holds_every_state_of_the_cutover_in_order_each_host_step_the_owners_go
A10: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_policy_discloses_every_copy_an_erase_cannot_reach
```

The deploy scripts' own Caddy install (`scripts/tests/test_deploy_scripts.py`) also renders the
block with the fourth key; that module is decided by CI by name and is never run on the box.

The window's interleavings are the model's, `formal/tla/SyncSnapshotWindow` (ADR-347 D12): the copy
reads only while the server is stopped, the archived pair comes from one generation, a client's
sync in the window is refused whole, and the server is up again after every window, the copy's
failure included. Each property has a witness the check must catch.

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
| `deploy/systemd/deck-streak-sync-server.service`, `deploy/scripts/sync-server.sh` | deploy | added | this |
| `deploy/systemd/deck-streak-sync-snapshot.service` | deploy | added (the snapshot's window) | this |
| `deploy/systemd/deck-streak-bot.service`, `deploy/systemd/deck-streak-litestream.service`, `deploy/systemd/deck-streak-mcp.service` (`CPUQuota=` 20%, 15% and 15%) | deploy | changed | this |
| `deploy/host-budget.json`, `deploy/rail-contract.json` | deploy | changed | this |
| `docs/decisions/ADR-032-deploy-templates-and-the-host-budget.md`, `docs/decisions/ADR-064-deckstreak-backs-up-with-its-own-units-and-never-the-collection.md` (each an amendment) | docs | changed | this |
| `deploy/deck-streak.env.example`, `deploy/README.md` | deploy | changed | this |
| `scripts/tests/test_deploy_templates.py`, `scripts/tests/_units.py` | tests | changed | this |
| `scripts/tests/test_sync_server_launcher.py` | tests | added | this |
| `deploy/caddy/deck-streak.caddy`, `deploy/scripts/render-caddy.py` | deploy | changed | this |
| `scripts/tests/test_caddy_render.py`, `scripts/tests/test_deploy_scripts.py` (the Caddy configuration's fourth key) | tests | changed | this |
| `deploy/scripts/backup.py`, `deploy/scripts/restore-drill.sh`, `deploy/systemd/deck-streak-backup.service` (the drill's unit is unchanged: the drill reads the snapshot directory beside the backups by default) | deploy | changed | this |
| `PRIVACY.md`, `scripts/tests/test_privacy_policy.py` | privacy | changed (the snapshot as a store; `privacy.json` is unchanged, because its categories are the database's tables and the sync server's store is none) | this |
| `docs/runbooks/sync-server-cutover.md` | docs | added | this |
| `scripts/tests/test_sync_server_runbook.py` | tests | added | this |
| `scripts/tests/test_backup_units.py` | tests | changed | this |
| `formal/tla/SyncSnapshotWindow/SyncSnapshotWindow.tla`, `formal/tla/SyncSnapshotWindow/MCSyncSnapshotWindow.cfg`, `formal/tla/SyncSnapshotWindow/MCLiveness.cfg` | formal | added | this |
| `formal/tla/SyncSnapshotWindow/witness/a-copy-not-ordered-after-the-stop.cfg`, `formal/tla/SyncSnapshotWindow/witness/a-generation-published-before-it-is-whole.cfg`, `formal/tla/SyncSnapshotWindow/witness/a-session-committed-step-by-step.cfg`, `formal/tla/SyncSnapshotWindow/witness/a-write-that-lands-during-the-copy.cfg`, `formal/tla/SyncSnapshotWindow/witness/a-failed-copy-that-starts-nothing.cfg`, `formal/tla/SyncSnapshotWindow/witness/a-window-gated-by-a-condition.cfg` | formal | added | this |
| `config/formal.json`, `scripts/tests/test_formal_config.py` (the model's budget) | formal | changed | this |

## 5. What this does NOT do

- It runs nothing on any host: the install, the unit's first start, the route's install, the
  bucket's creation and its access policy, and every cutover step are commands the seat records in
  #161 before any run.
- It moves the share once, to the owner's decided 1152 MiB, and leaves no room in it: the next
  long-running unit needs the share moved first, by ADR-064's record (#617).
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
- A full upload the server completes can still fail at the client. Measured with ADR-022's shape
  and the server held to part of one processor, the client gave up at its 30-second stall limit
  while the server finished the upload and answered it with status 200 a few seconds later, so the
  server held the new collection that the client reported as failed, a divergence. The stall is the
  server reading and checking the received collection before it answers, and it grows as the share
  shrinks. ADR-347 D7 records the readings, the quota they chose (`CPUQuota=75%`) and why each
  reading is a lower bound. That figure's reading approximated a quota, so the runbook's hold takes
  the first reading under the real quota on the host and stops the cutover on a client failure or
  a stall over 20 seconds, and its upload step reads the server's collection back before any
  retry, so a retry never follows an upload the server completed.
- The server holds each user's `media.db` in SQLite's exclusive locking mode for its whole life
  (the fork's media database opens with `locking_mode=exclusive`), so no second process can read it
  while the server runs. Measured: a read and an online backup of `media.db` are refused with
  `database is locked` after the server's start, after a full upload and after a full download, and
  both succeed once the server has stopped. The collection opens the same way; it read and backed up
  after a full upload and after a full download, and its lock during a normal sync is not measured.
  So ADR-347 D5's snapshot, SQLite's online backup of both databases while the server runs, cannot
  take `media.db`, and ADR-347 D12 takes it in a stopped-server window inside the daily backup's
  run instead: for the window's seconds no client can sync, a sync in flight when the server stops
  is refused whole, and the client syncs again at its next sync. The model holds the refusal whole
  only while the engine applies a sync in one transaction, which the pinned fork does; a fork
  commit that changes it is read against the model when the patch entry moves.
- A window that does not end would leave the server stopped. The unit's start is bounded, and its
  failure, a timeout included, starts the server again and pages, as a failed copy does (A7).
- The copy refuses a database a process still holds, so a server that has not stopped fails the
  window instead of yielding a torn copy; the window then pages and the previous archives stay.

## 7. Delivered by the next pull requests

Nothing. A7 and A8 waited here on the snapshot's form; ADR-347 D12 decided it, and both are in
section 3, delivered by this pull request.

## 8. The mutation rows

S33700-S33799. This part's rows hold the release step's guard and its tarball line against A1 and
A2: a step that admits a branch or a short commit, a build without `--locked`, a second copy of the
pin, and a tarball without the server (S33701-S33705). The launcher's rows hold A5 (S33706-S33722):
an entry that is not a name and a pbkdf2-sha256 hash, an empty or missing entry, an address that is
not loopback or an octet or port out of range, a setting that would reach the server, two users of
one name, the hashed-users switch, the data directory, a missing credentials or state directory, and
a refusal that would not fail the unit. The route's
rows hold A6 (S33723-S33735): render-caddy's fourth key, its check and a refusal that names the
wrong key, and the block's redirect, prefix, hidden health route, body bound, buffer and upstream.
The runbook (A9) is prose and holds no function, so it adds no row. The snapshot's rows hold A7
(S33736-S33743): a stopped database refused as if held, a busy wait on a held one, a generation
written under its final name, a broken database archived, a manifest that does not digest its
members, an offsite copy without the manifest or without its bucket, and a fourth archive kept. The
drill's rows hold A8 (S33744-S33747): an older archive restored, a digest or a database left
unchecked, and an empty snapshot directory not refused by name. No row drops the window's refusal
of a busy step: the library retries such a step with no bound and the killer holds the lock in its
own thread, so that mutant would hang its killer rather than fail it. S33737 and the model
`formal/tla/SyncSnapshotWindow` carry that property.

## 9. References

ADR-340, ADR-347, ADR-058, ADR-336, ADR-344, ADR-032, ADR-064, ADR-062, ADR-010, ADR-038; SPEC-334
(row 1.3, R8, R9), SPEC-064, SPEC-062; `docs/schematics/sync-server-packaging-and-cutover.md`;
`RELEASING.md`; `deploy/README.md`.

## 10. Amendments: what this part measured after section 1

SPEC-333 R3 is amended by this delivery: `deploy/README.md` "## The host budget" names the
`CPUQuota=` of five daemons, not four, at ADR-347 D7's split: the API's 75%, the sync server's 75%,
the bot's 20%, the replicator's 15% and the MCP server's 15%, which divide the share's two
processors exactly.

Section 6's risks on the divergence and on `media.db`'s lock are measurements taken after section
1, with the same server and the same scratch client. The lock's measurement decided the snapshot's
form: ADR-347 D12's stopped-server window, which replaces D5's online backup while the server runs.
A copy bounded by the online backup with no busy wait was measured against a process holding a
database the server's way: it is refused at once, and once the holder has ended the same call
copies the database, reading through a write-ahead log a server ended without a clean close
leaves. R5 is restated for the window, A7 and A8 joined section 3, and the snapshot's files joined
section 4.
