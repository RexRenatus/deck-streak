# SPEC-340: the sync server is hardened before its first deploy: its family runs as its own user, its offsite copy is sealed and dated, and its route is logged, bounded and kept apart from the web session

- **Issue:** #628 (the sync server's first deploy, which this SPEC gates); the review is #618, the
  host commands are #161, and the held exceptions are #649. **Context(s):** none (the deploy
  templates, the release workflow and the cutover runbook, not a bounded context).
- **Decided by:** ADR-351 (D1 to D9), and ADR-347's amendment "the sync key and its hash" (D13),
  under ADR-340 (the move), ADR-058 and ADR-336 (the fork), ADR-032 (the host budget), ADR-064 (the
  backup) and ADR-038 (the credentials).
- **Status:** this pull request delivers R1 to R12 (section 3) and the rows of section 8.
  **Mutation band:** S34000-S34099. **Model:** `formal/tla/SyncSnapshotWindow`, unchanged.

Public text: each finding is cited by its id and the neutral title below. Its detail stays in the
review's private record (#618).

## 1. The problem, measured

SEC-01's review of #647 (#618) recorded eleven findings on the sync server's surface. Each row below
was re-read at DeckStreak `dev` c56bbd11 with `git show c56bbd11:<path>`.

| id | neutral title | where the tree holds it at c56bbd11 |
|---|---|---|
| SEC01-F01 | the sync key's lifetime and the hash's class | ADR-347's D2 (line 46); `deploy/scripts/sync-server.sh` line 76 |
| SEC01-F02 | the sync family's service user | `deck-streak-sync-server.service` lines 22-23; `deck-streak-sync-snapshot.service` lines 26-27 |
| SEC01-F03 | the sync route's login bound | `deploy/caddy/deck-streak.caddy` lines 44-60 |
| SEC01-F04 | the sync route's client address in the record | `sync-server.sh` lines 68-72 and 75 |
| SEC01-F05 | the sync key kept out of every log | `deck-streak.caddy` lines 7-68: `grep -c log deploy/caddy/deck-streak.caddy` finds no `log` directive |
| SEC01-F06 | the sync server's dependency audit | `.github/workflows/release.yml` lines 72-81 |
| SEC01-F07 | the sync server's network peers | `deck-streak-sync-server.service` lines 58-59 |
| SEC01-F08 | the web session cookie on the sync route | `deck-streak.caddy` lines 55-59 |
| SEC01-F09 | the sync credential's hash parameters | `sync-server.sh` line 28; the runbook's `rekeyed` step, lines 127-133 |
| SEC01-F10 | the offsite snapshot's retention statement | `PRIVACY.md` lines 94-99 |
| SEC01-F11 | the offsite snapshot's encryption | `deploy/scripts/backup.py` lines 250-258 |

Two facts the cures depend on, each re-read:

- The window's three functions the model stamps (`backup.py::window`, `copy_stopped` and
  `refuse_a_holder`; `formal/tla/SyncSnapshotWindow/SyncSnapshotWindow.tla` lines 2-4) need no
  change from any cure here, and none is made.
- The restore drill's extraction is `archive.extractall(target, filter="data")`
  (`deploy/scripts/restore-drill.sh` line 133). That filter refuses absolute paths, parent
  references, links leaving the target, device files and set-id bits, and an interpreter without
  it fails the drill closed. No cure here touches that line.

## 2. Requirements

- R1. (F01) ADR-347 states that the sync server derives each client's key from the user name and the
  stored hash, so the hash is a secret of the password's class. The cutover runbook's `rekeyed`
  step names its triggers, a synced device is lost or the credential store may have been exposed,
  and what each needs: any new hash retires every key, and an exposure also needs a new password.
- R2. (F02) The sync server, its window, its archive and its sync drill run as the system user and
  group `deck-streak-sync`, and no other unit does. No unit of that family names the `deck-streak`
  state directory, and no other unit names the sync family's two state directories.
- R3. (F02) The snapshot's archive runs as its own oneshot unit, `deck-streak-sync-archive.service`.
  The backup's run pulls it in, after the window and before the database backup. The database
  backup no longer archives, reaches no network and reads no settings file.
- R4. (F02) The snapshot's restore drill runs as its own oneshot unit,
  `deck-streak-sync-restore-drill.service`, pulled in by the restore drill and ordered after it. It
  fails when the snapshot directory is missing, and the database drill never reads that directory.
- R5. (F03) The repository ships a ban filter and a jail over the edge's access log. They ban an
  address after five refused sync logins within ten minutes, for one hour. They are installed on
  the host only on the owner's go.
- R6. (F04, F05) The edge writes one access log, of the sync route alone. It holds the client's
  address, the method, the path and the status. It holds no request header and no `k` query
  parameter.
- R7. (F06) Before the release job builds the sync server, a job with read-only permissions audits
  the server package's dependency graph, under the fork's own lockfile at the pinned commit, for
  RustSec advisories. The release job waits on that job.
- R8. (F07) The sync server's unit admits loopback peers only (`IPAddressAllow=localhost`,
  `IPAddressDeny=any`).
- R9. (F08) The edge sends no `Cookie` header to the sync server and passes no `Set-Cookie` header
  back from it.
- R10. (F09) The launcher admits only the house's hash shape: 600000 to 999999 rounds, an optional
  `l=32`, a 16-byte salt and a 32-byte digest, each in canonical unpadded base64. The runbook's
  `rekeyed` and `started` steps name the standard-library command that makes such a hash.
- R11. (F10) `PRIVACY.md` states the offsite archives' period as an ISO 8601 duration, `P30D`. The
  bucket's lifecycle rule deletes each archive that period after it is written, and the runbook
  checks that rule before the window. `PRIVACY.md` also discloses the edge log of the sync route and
  the ban list.
- R12. (F11) The archive and its manifest are sealed to the owner's offline public key before any
  copy. A command the settings name does the sealing, run with arguments and no shell, from a
  public recipients file. A sealed file must begin with the format's header,
  `age-encryption.org/v1`. Only sealed files are copied, and an unset or failing seal fails the run
  with nothing copied.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | R1: the rekey step names both triggers and what each needs | `test_sync_server_runbook.py` TheCutoverRunbook |
| A2 | R2: the sync family runs as `deck-streak-sync`, every other service as `deck-streak`, and no state directory is named by units of both users | `test_deploy_templates.py` TheSyncServerRunsAsItsOwnUnit |
| A3 | R3: the archive unit's ordering, user, directory, settings, families, budget and alert; the server's `Also=` names its three units; the backup reaches no network and reads no settings file | `test_backup_units.py` SyncWindow |
| A4 | R3: `--sync-archive` checks, archives, copies and keeps three; the default run leaves a generation untouched | `test_backup_units.py` SyncWindow |
| A5 | R4: the sync drill unit; `--part sync` fails with no snapshot directory; `--part database` never reads it; any other part is refused | `test_backup_units.py` SyncWindow |
| A6 | R5: the filter matches a refused sync login and captures its address, and matches nothing else; the jail's numbers | `test_sync_ban.py` TheSyncBanJail (new) |
| A7 | R6: one site-level log, JSON, headers deleted, `k` deleted, every other route skipped | `test_deploy_templates.py` TheCaddyBlock |
| A8 | R7: the audit script fetches the pinned commit and runs the advisory check on the server's manifest with the house config under the fork's lockfile; a short commit is refused before any fetch; a failed check fails the audit | `test_audit_sync_server.py` TheAuditReadsThePin (new) |
| A9 | R7: the release job needs the read-only audit job; its own permissions are unchanged | `test_release_workflow.py` TheReleaseBuildsTheSyncServer |
| A10 | R8: the server unit's two IP keys, and the paging census bounds them | `test_deploy_templates.py` TheSyncServerRunsAsItsOwnUnit |
| A11 | R9: the sync route removes `Cookie` up and `Set-Cookie` down | `test_deploy_templates.py` TheCaddyBlock |
| A12 | R10: the launcher admits the house shape and refuses each departure | `test_sync_server_launcher.py` TheLauncherStartsTheServerWithHashedUsers |
| A13 | R10, R11: the runbook names the hash command and checks the bucket's rule with the policy's period | `test_sync_server_runbook.py` TheCutoverRunbook |
| A14 | R11: the policy states the period, and discloses the edge log and the ban list | `test_privacy_policy.py` ThePolicyDisclosesWhatAnEraseLeaves |
| A15 | R12: only sealed files are copied and the sealed files are removed; an unset seal, or one that leaves no header, copies nothing and fails | `test_backup_units.py` SyncWindow |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_sync_server_runbook.py -k test_the_rekey_step_names_its_triggers
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_family_runs_as_its_own_user
A3: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_archive_runs_as_its_own_unit_after_the_window
A4: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_sync_archive_checks_archives_copies_offsite_and_keeps_three
A5: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_sync_drill_runs_as_its_own_unit
A6: python3 -m unittest discover -s scripts/tests -p test_sync_ban.py
A7: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_route_alone_is_logged_without_its_key
A8: python3 -m unittest discover -s scripts/tests -p test_audit_sync_server.py
A9: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_release_waits_on_the_server_audit
A10: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_server_reaches_loopback_peers_only
A11: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_sync_route_carries_no_web_cookie
A12: python3 -m unittest discover -s scripts/tests -p test_sync_server_launcher.py -k test_the_launcher_admits_only_the_house_hash_shape
A13: python3 -m unittest discover -s scripts/tests -p test_sync_server_runbook.py -k test_the_runbook_names_the_hash_command_and_the_bucket_rule
A14: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_offsite_snapshot_has_a_stated_period
A15: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_offsite_copy_is_sealed_and_never_plain
```

A5 runs the restore drill's script and A9 reads the release workflow's module; both are decided by
CI by name, and `docs/red-first/SPEC-340.md` records why.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-340-the-sync-server-is-hardened-before-its-first-deploy.md` | docs | new: this SPEC |
| `docs/decisions/ADR-351-the-sync-family-runs-as-its-own-user-its-archive-is-sealed-and-dated-and-its-route-is-logged-and-bounded.md` | docs | new: D1 to D9 |
| `docs/decisions/ADR-347-the-sync-server-is-built-from-the-engines-pin-reads-hashed-users-from-credentials-and-serves-under-its-own-path.md` | docs | appended amendment: the sync key and its hash (D13), and the sentences ADR-351 replaces |
| `docs/decisions/ADR-032-deploy-templates-and-the-host-budget.md` | docs | appended amendment: the two new units' ceilings, in its budget table's form |
| `docs/specs/SPEC-337-the-sync-server-is-built-at-the-tag-runs-as-its-own-unit-and-has-a-drilled-offsite-snapshot.md` | docs | its section 10 gains a paragraph naming the requirements SPEC-340 replaces |
| `docs/schematics/sync-server-hardening.md` | docs | new: the unit graph by user, the offsite path, the edge and the ban |
| `docs/schematics/sync-server-packaging-and-cutover.md` | docs | one line pointing to the hardening schematic for the archive, the sync drill and the users |
| `docs/red-first/SPEC-340.md` | docs | new: the red-first record |
| `docs/runbooks/sync-server-cutover.md` | docs | "Before the window", `started`, `read_back`, `rekeyed`; the unban step |
| `PRIVACY.md` | docs | the snapshots bullet's period; the edge log and ban list bullet |
| `deploy/systemd/deck-streak-sync-server.service` | deploy | user and group; the two IP keys; `Also=` names three units |
| `deploy/systemd/deck-streak-sync-snapshot.service` | deploy | user and group; `StateDirectory=` |
| `deploy/systemd/deck-streak-sync-archive.service` | deploy | new |
| `deploy/systemd/deck-streak-sync-restore-drill.service` | deploy | new |
| `deploy/systemd/deck-streak-backup.service` | deploy | no settings file, `AF_UNIX` only, header comment |
| `deploy/scripts/sync-server.sh` | deploy | the entry's shape (line 28) |
| `deploy/scripts/backup.py` | deploy | `--sync-archive`; the default run no longer archives; the seal in `archive()` |
| `deploy/scripts/restore-drill.sh` | deploy | `--part database` or `--part sync` |
| `deploy/caddy/deck-streak.caddy` | deploy | the log, its skip, the two header removals |
| `deploy/fail2ban/filter.d/deck-streak-sync.conf` | deploy | new |
| `deploy/fail2ban/jail.d/deck-streak-sync.conf` | deploy | new |
| `deploy/rail-contract.json` | deploy | rows for the two new units; the backup's settings-file row removed |
| `deploy/host-budget.json` | deploy | entries for the two new units; the share unchanged |
| `deploy/README.md` | deploy | the rail table: the sync user, the ban jail, the seal command and its recipients file |
| `deploy/deck-streak.env.example` | deploy | `DECKSTREAK_SNAPSHOT_SEAL` |
| `.github/workflows/release.yml` | ci | the `audit-sync-server` job; the release job's `needs:` |
| `scripts/audit-sync-server.sh` | ci | new |
| `scripts/tests/_units.py` | tests | the paging census: two IP keys; `Also=` bound to three names |
| `scripts/tests/test_deploy_templates.py` | tests | A2, A7, A10, A11; `HARDENING` and `PER_SERVICE` rows; the unit tables name the two new units |
| `scripts/tests/test_backup_units.py` | tests | A3, A4, A5, A15; the window test's state directory; the drill test runs each part; the census counts two drills |
| `scripts/tests/test_sync_server_launcher.py` | tests | A12 |
| `scripts/tests/test_sync_server_runbook.py` | tests | A1, A13 |
| `scripts/tests/test_privacy_policy.py` | tests | A14 |
| `scripts/tests/test_release_workflow.py` | tests | A9 |
| `scripts/tests/test_sync_ban.py` | tests | new: A6 |
| `scripts/tests/test_audit_sync_server.py` | tests | new: A8 |
| `scripts/mutation-rows.d/S34000-S34099.json` | tests | new: the rows of section 8 |
| `changelog.d/sync-hardening-340.md` | docs | new |

## 5. What this does NOT do

- It changes no fork code, so the sync key still never expires on its own. A key with a lifetime,
  or one made per login, needs a fork patch under ADR-058 and ADR-336, which #649 holds.
- It runs nothing on a host: creating the user, installing the jail, the seal command, the
  recipients file and the bucket's rule are steps of #161, each on the owner's go.
- It audits the server's graph at the release only; an audit on the pull request that moves the
  pin is #649's.
- It bounds logins per address; a bound across addresses is #649's.
- It does not move the share: the memory and processor figures ruled for the sync server stand
  (#628).

## 6. Risks

- The edge's log spelling differs from the drafted form on the host's edge. Detected by the
  `started` step's validation of the rendered block before its reload (#161).
- The jail's pattern misses the edge's line order. Detected by the `started` step's check of the
  filter against one refused login of the staging user, before the jail is enabled (#161).
- The owner's own client is banned after mistyped logins. The runbook's unban step answers it.
- No snapshot exists when the server is first enabled, so the first sync drill fails and pages. The
  `started` step runs the backup once by hand on the owner's go.
- The seal command or the recipients file is missing on the host. The archive unit fails and pages,
  and copies nothing in plain text.

## 7. Delivered by the next pull requests

None.

## 8. The mutation rows

S34000-S34099, in `scripts/mutation-rows.d/S34000-S34099.json`. Each row's anchor and mutant are
copied from the cured file.

- The sync family's user and the split of the archive and the drill (A2 to A5): the server run as
  DeckStreak's user (S34001), the database run archiving again (S34002), a missing snapshot
  directory passing the sync drill (S34003), and the backup reaching the network (S34004).
- The server's peers (A10): a deny list that admits every peer (S34005).
- The hash's shape (A12): rounds below the floor (S34006) or above the ceiling (S34007), a salt
  (S34008) or a digest (S34009) of any length.
- S34010 has no row: the runbook's rekey step is prose and holds no function, decided by A1 and A13.
- The edge's log (A7): a log that writes the request's headers (S34011) or the `k` parameter
  (S34012).
- The web cookie (A11): a `Cookie` header passed up to the server (S34013).
- The ban (A6): a filter that counts any client error as a refused login (S34014), and one that
  counts a refusal on any sync path as a login (S34015).
- The audit (A8): a check that skips advisories (S34016), one that resolves the graph without the
  fork's lockfile (S34017), and a short commit admitted (S34018).
- The seal (A15): a sealed file accepted without its header (S34019), an unset seal command that
  copies anyway (S34020), and a sealed file left on the host (S34021).

A9's job is held by its test and by the workflow parsers, decided by CI by name; no row mutates the
release workflow in this band.

## 9. References

SPEC-337, ADR-347, ADR-351, ADR-340, ADR-058, ADR-336, ADR-064, ADR-038, ADR-032;
`docs/schematics/sync-server-hardening.md`; #618, #628, #647, #649, #161.
