# SPEC-584: backups keep at most seven days, every copy is disclosed with that window, and an erase will destroy the learner's key first

- **Issue:** `#44`. The declared backups window is three days (`P3D`) and the offsite archives'
  period thirty (`P30D`); both move to seven days (`P7D`), with crypto-erase by one data key per
  learner, daily disk snapshots and a second-region copy of the shared records.
- **Context(s):** `docs` (the records, `PRIVACY.md`, `privacy.json`, the cutover runbook and two
  schematics) and `scripts/tests` (one new text-test module and one moved constant).
- **Decided by:** ADR-603. **Amends:** SPEC-021 R5 to R7, SPEC-340 R11, ADR-064 and ADR-351 D9,
  each by a section appended to its file; SPEC-064 R3, R5, R8 and R9 by section 2b below, its own
  file unchanged while its surface is parked (`#158`).
- **Schematic:** `docs/schematics/backup-window-and-crypto-erase.md` (added).
- **Status:** one delivery of documents and text tests; R8 to R13 bind the later deliveries of
  section 9. **Mutation band:** `S58400-S58499` (rows `S58401` to `S58406`, section 8).
  **Model:** none (section 7).

## 1. The problem, measured

Every figure below was read at this delivery's base on `dev` by `git show <dev>:<path>` with `grep -n`.

### 1a. The window and what states it

| fact | where |
|---|---|
| the declared backups window is `P3D` | `privacy.json:450` |
| the replica keeps a snapshot every 24 hours for 48 hours; its comment says 72 hours, inside `P3D` | `deploy/litestream.yml:5-11` |
| the daily copy keeps the newest three on the host, and a failed run leaves the old copies untouched | `deploy/scripts/backup.py:5-6`, `:43` |
| the policy states both inside `P3D`, as 72 hours and 3 days | `PRIVACY.md:99-105` |
| the record keeps an erased row at most three days, with the bucket's soft delete off | `ADR-064:52-54` |
| a longer window changes the declared window, the policy and the retention together, by an amendment of SPEC-021 | `ADR-064:103-104`; `SPEC-021:58-70` (R5, R6) |
| the replica row of the deploy schematic says inside `P3D` | `docs/schematics/first-deploy-and-gates.md:24` |
| the offsite archives are deleted `P30D` after writing by a bucket rule | `ADR-351:125-132` (D9); `SPEC-340:75-78` (R11) |
| the policy, the runbook, a schematic and one test constant state `P30D` | `PRIVACY.md:120-122`; `docs/runbooks/sync-server-cutover.md:52-55`; `docs/schematics/sync-server-hardening.md:61`; `scripts/tests/test_sync_server_runbook.py:27` |

A census of the SPECs and ADRs outside the parked names that hold `P3D` or `P30D` finds four:
SPEC-021, SPEC-340, ADR-064 and ADR-351. SPEC-064 states the window too (its R3, read from the
records that cite it); its file name is in the parked surface, so it is counted and not opened.

### 1b. What pins it

- `scripts/tests/test_backup_units.py:607-633` requires the replica's interval plus retention and
  the daily copies' age to fit the declared window, and the policy to say "3 days" near "daily
  copies" and "72 hours" near "replica". It executes deploy scripts, so it runs only in CI.
- `scripts/tests/test_privacy_policy.py:86-96` and `:117-137` require a replica bullet holding the
  declared window in words; `:139-159` requires one sync-server snapshot bullet holding the offsite
  period, read from `scripts/tests/test_sync_server_runbook.py:27`, which `:137-147` also holds
  against the runbook's bucket rule.
- Rows `S06401` to `S06403` (`scripts/mutation-rows.d/S06400-S06499.json`) mutate the daily copy's
  integrity check, its prune and `KEEP = 3`.

### 1c. The erase, the copies and the keys

- The erase deletes every port's rows in one transaction with `secure_delete` on, rebuilds the
  file and empties its write-ahead log (`crates/privacy/src/erase.rs:5-16`, `:46`).
- No copy is encrypted per learner, no wrapped-key table exists (no migration names one), and the
  host's identity may delete the replica's history. No disk snapshot is scheduled and no copy is
  kept in a second region. No record states a recovery point or a recovery time; R13 now sets
  both as targets: a recovery point of 1 minute for the learner stores and the shared records,
  and a recovery time of 15 minutes for one learner and 2 hours for the whole service, never
  stated as met before a restore has been timed on the server.

## 2. Requirements

- R1. `privacy.json` declares the backups window `P7D`.
- R2. `PRIVACY.md`'s replica and daily-copies bullets each state the window as `P7D` and as
  "7 days", and keep stating today's ages: 72 hours for the replica and 3 days for the copies.
- R3. `PRIVACY.md` discloses the disk snapshots, when the service keeps them, in one bullet: one a
  day, each deleted 6 days after it is taken, so erased data leaves them within 7 days (`P7D`).
- R4. `PRIVACY.md` discloses the second-region copy of the shared records and the directory of
  accounts, when the service keeps one, in one bullet with the same window.
- R5. The offsite archives' period is `P7D` in `PRIVACY.md`, the cutover runbook (the bucket rule
  deletes each archive 6 days after it is written), `docs/schematics/sync-server-hardening.md` and
  the one constant `OFFSITE_PERIOD`.
- R6. SPEC-021, SPEC-340, ADR-064 and ADR-351 each end with an amendment section headed
  `Amendment (SPEC-584)` or `N. Amendments (SPEC-584)` that states `P7D`, and no other SPEC or ADR
  outside the parked names states `P3D` or `P30D`, except SPEC-584 and ADR-603, which decide the
  change.
- R7. `PRIVACY.md` claims crypto-erase exactly when a migration creates `user_data_keys`, and then
  in this sentence, placed after the erase bullets of `## Your copy of your data, and erasing it`:
  "An erase destroys the key your data is encrypted with before it removes anything else, so every
  backup copy of your data is unreadable from that moment, and each copy is gone within 7 days
  (`P7D`)."
- R8. (Later.) Each learner's store is encrypted under one data key of 256 random bits, kept only
  wrapped by one key-service key in `user_data_keys` (the learner's id, the wrapped key, the wrap
  key's version, when it was made), as one object per learner in two key stores, one beside the
  server and one in a second region, never backed up, with no history and no key on any disk. An
  erase deletes the learner's wrapped key from both key stores and confirms both before anything
  else, then erases the rows. A restore reads the wrapped key from the key store beside the server,
  or from the second-region one when that is lost, and unwraps it through the server's identity.
  The weekly restore rehearsal restores a synthetic rehearsal learner through the wrap key and
  proves a synthetic erased learner's store restores as ciphertext that nothing opens.
- R9. (Later.) Two spikes on synthetic data decide nothing and measure: (a) for the at-rest layer,
  page encryption of each store under its raw data key against row-level authenticated encryption
  of each personal column, the open cost per store, the overhead on the answer write and the
  due-card read, resident memory per open store, the replica's bytes and requests per store per
  day, whether each builds for the server, the iPhone and iPad app and the web engine, what each
  leaves readable, and one store's restore time through the wrap key; (b) for the replicator,
  that it replicates with its retention enforcement off under an identity that cannot delete,
  whether it writes a snapshot of an unchanged database each interval, and how long the bucket
  rule takes to delete an object past its age.
- R10. (Later.) Media stored on the service is encrypted under its learner's data key before any
  copy is made; an erased learner's media backups go within 6 days: the erase writes a time mark
  on each of that learner's media backup objects, through a grant that may update an object's
  metadata and never delete it, and a deletion rule on age from that mark removes each marked
  object within 6 days. No identity holds an object-delete right on the media backup.
- R11. (Later.) Every backup layer prunes a copy by a rule the host's identity cannot override,
  at 6 days old, or at 4 for the second-region copy: the replica bucket's rule, with the
  replicator's retention enforcement off and its snapshot retention at 120 hours; the daily copy
  keeping the newest six and removing any copy older than 6 days even when that day's run fails;
  one disk snapshot a day of the data disk, each deleted 6 days after it is taken, while the
  service is at its first size and removed before it grows past that size; and a second replica
  of the shared database and the user directory in a second-region bucket under a create-only
  identity, whose deletion rule removes each object at 4 days old. Soft delete stays off and no
  retention lock is set on any of them.
- R12. (Later.) Every personal field of the shared database and the user directory is either
  encrypted under its learner's data key or a random id that names no person once that learner's
  key and sign-in links are gone.
- R13. The recovery point target is 1 minute for the learner stores and the shared records, and
  the recovery time target is 15 minutes for one learner and 2 hours for the whole service. Both
  are targets, never stated as met before a restore has been timed on the server.

### 2b. SPEC-064's amended requirements

SPEC-064's file is unchanged while its surface is parked; these replace its requirements as
follows, and its file gains them when the park lifts.

- R3: the declared window is `P7D`, and every copy is pruned at 6 days old, or at 4 for the
  second-region copy (R11).
- R5: when the per-learner keys exist, the weekly rehearsal also restores through the wrap key and
  proves an erased learner unreadable (R8).
- R8: the bucket keeps soft delete off and no retention lock; its rule deletes each object 6 days
  after it is written; the replicator deletes nothing and the host's identity creates and reads
  only.
- R9: six daily copies on the host, and up to about seven snapshots per database in the bucket.

## 3. Acceptance criteria

The base is the red commit: the new module and the moved constant over the unchanged documents.
No stubs are needed: the tests read text. No test pins an engine-computed literal.

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | `privacy.json` declares `P7D`, and the replica and daily-copies bullets each state `P7D` and "7 days"; a planted `P3D` inventory and planted bullets are refused by name | `assertEqual` fails: the inventory reads `P3D` | `scripts/tests/test_backup_window.py::TheBackupWindow::test_the_declared_backup_window_is_seven_days` |
| A2 | the sync-server snapshot bullet states the offsite period `P7D` and "7 days" | `assertIn` fails: the bullet states `P30D` while the constant reads `P7D` | `scripts/tests/test_privacy_policy.py::ThePolicyDisclosesWhatAnEraseLeaves::test_the_offsite_snapshot_has_a_stated_period` |
| A3 | the runbook's `Before the window` section checks the lifecycle rule with `P7D` | `assertIn` fails: the section states `P30D` | `scripts/tests/test_sync_server_runbook.py::TheCutoverRunbook::test_the_runbook_names_the_hash_command_and_the_bucket_rule` |
| A4 | one bullet each discloses the disk snapshots and the second-region copy with `P7D` and "7 days"; a policy without either is refused by that copy's name | `assertEqual` fails: neither bullet exists | `scripts/tests/test_backup_window.py::TheBackupWindow::test_the_policy_discloses_the_disk_snapshots_and_the_second_region_copy` |
| A5 | every SPEC and ADR outside the parked names that states `P3D` or `P30D` is one of the four, each ends with its SPEC-584 amendment stating `P7D`, and planted records are refused by name | `assertEqual` fails: the four carry no amendment | `scripts/tests/test_backup_window.py::TheBackupWindow::test_every_record_that_states_an_older_period_carries_its_amendment` |
| A6 | the page claims crypto-erase exactly when a migration creates `user_data_keys`: neither at `dev`, and a planted claim and a planted table are each refused | not red: the base holds neither the claim nor the table, so this is a guard whose two plants prove it can fail | `scripts/tests/test_backup_window.py::TheBackupWindow::test_the_policy_claims_crypto_erase_exactly_when_the_keys_exist` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_backup_window.py -k test_the_declared_backup_window_is_seven_days
A2: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_offsite_snapshot_has_a_stated_period
A3: python3 -m unittest discover -s scripts/tests -p test_sync_server_runbook.py -k test_the_runbook_names_the_hash_command_and_the_bucket_rule
A4: python3 -m unittest discover -s scripts/tests -p test_backup_window.py -k test_the_policy_discloses_the_disk_snapshots_and_the_second_region_copy
A5: python3 -m unittest discover -s scripts/tests -p test_backup_window.py -k test_every_record_that_states_an_older_period_carries_its_amendment
A6: python3 -m unittest discover -s scripts/tests -p test_backup_window.py -k test_the_policy_claims_crypto_erase_exactly_when_the_keys_exist
```

The red-first record is `docs/red-first/SPEC-584.md`: `red at <sha>: <failure>` and
`green at <sha>` for A1 to A5, and `not red: <why>` for A6.

Shipped tests this delivery leaves as they are: every test of `test_privacy_policy.py` and
`test_backup_units.py` keeps its name and its text, and `test_sync_server_runbook.py` changes only
its comment and its constant (lines 25 to 27).

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-584-backups-keep-at-most-seven-days-every-copy-is-disclosed-and-an-erase-will-destroy-the-learners-key-first.md` | docs | added |
| `docs/decisions/ADR-603-backups-keep-at-most-seven-days-each-copy-is-pruned-at-six-and-an-erase-destroys-the-learners-key-first.md` | docs | added |
| `docs/schematics/backup-window-and-crypto-erase.md` | docs | added |
| `docs/red-first/SPEC-584.md` | docs | added |
| `changelog.d/backup-window-584.md` | docs | added |
| `scripts/mutation-rows.d/S58400-S58499.json` | rows | added: `S58401` to `S58406` |
| `scripts/tests/test_backup_window.py` | tests | added: A1, A4, A5 and A6 |
| `scripts/tests/test_sync_server_runbook.py` | tests | lines 25 to 27: the comment and `OFFSITE_PERIOD`, `P30D` to `P7D` |
| `PRIVACY.md` | docs | lines 99 to 105 and 121: the window, today's ages, two new bullets, the archives' period |
| `privacy.json` | docs | line 450: the backups window `P7D` |
| `docs/runbooks/sync-server-cutover.md` | docs | lines 53 to 55: the bucket rule at 6 days, inside `P7D` |
| `docs/schematics/sync-server-hardening.md` | docs | line 61: the bucket rule's label |
| `docs/schematics/first-deploy-and-gates.md` | docs | line 24: the replica row, inside `P7D` |
| `docs/specs/SPEC-021-data-rights-export-and-erase.md` | docs | appended: `## 8. Amendments (SPEC-584)` |
| `docs/specs/SPEC-340-the-sync-server-is-hardened-before-its-first-deploy.md` | docs | appended: `## 10. Amendments (SPEC-584)` |
| `docs/decisions/ADR-064-deckstreak-backs-up-with-its-own-units-and-never-the-collection.md` | docs | appended: `## Amendment (SPEC-584)` |
| `docs/decisions/ADR-351-the-sync-family-runs-as-its-own-user-its-archive-is-sealed-and-dated-and-its-route-is-logged-and-bounded.md` | docs | appended: `## Amendment (SPEC-584)` |

## 5. What this does NOT cover

- It changes no deploy file: the replicator's settings and its comment (`deploy/litestream.yml:5-11`), the daily copy's keep count and its prune by age (`deploy/scripts/backup.py:43`, `:109`), and their pins (`scripts/tests/test_backup_units.py:617-618`, `:632-633`; row `S06403`) move in a later delivery (`#44`).
- It builds no key: the wrapped-key table, the at-rest layer, the erase's key-first step (`crates/privacy/src/erase.rs:46`) and R7's sentence land in a later delivery after R9's spike (`#14`).
- It enables no key service and creates no key, key store, bucket, lifecycle rule, disk snapshot schedule or second-region copy: each is the owner's step on the host (`#161`, `#166`).
- It changes no restore rehearsal unit: those files are parked, and R8's rehearsal steps land after the park lifts (`#158`).
- It edits no line of SPEC-064's file while its surface is parked (`#158`).
- It designs no media backup beyond R10, since no media is stored on the service yet (`#44`).
- It states R13's recovery targets as targets only: neither is stated as met before a restore has been timed on the server (`#44`).
- It decides no key hierarchy for end-to-end sync, which is another delivery's (`#14`).
- It changes no engine, scheduler or web screen, and no README or about page (`#44`).

## 6. Risks

- **A page that promises ahead of the code.** The window holds today, since every copy is younger
  than seven days, and the extra copies are disclosed as "when the service keeps them"; the
  crypto-erase sentence waits for the keys, and A6 refuses it early.
- **A failed daily copy keeps old copies.** Until the later delivery prunes by age, a run of
  failed copies keeps the three it has past three days; the failure pages through the unit's
  alert, and the window still holds for seven days of failures.
- **A shared line moves.** `PRIVACY.md` is touched by an open pull request; the build applies each
  edit only where its whole old line still occurs once, and stops otherwise.
- **The census meets a new record.** A later record that states `P3D` or `P30D` reds A5 by its
  name, which is the census working.
- **The two key stores.** Losing both makes every backup unreadable; R8 keeps them in two regions,
  and the risk is the owner's to accept before the keys are built (ADR-603 Consequences).

## 7. Formal model

None: no code changes, and no interleaving is built here. The key-first erase of R8 is an
interleaving (an erase, a restore and a disk snapshot over two key stores); its delivery models it
before it is built.

## 8. Mutation rows

Band `S58400-S58499`, table `SCRIPT_MUTATIONS`, each row `[id, path, find, replace, description,
killer]`:

| row | path | mutant | killer |
|---|---|---|---|
| `S58401-THE-DECLARED-WINDOW-IS-THREE-DAYS-AGAIN` | `privacy.json` | the backups window back to `P3D` | A1 |
| `S58402-THE-ARCHIVES-STAY-THIRTY-DAYS-AGAIN` | `PRIVACY.md` | the archives' period back to `P30D` | A2 |
| `S58403-THE-RUNBOOK-CHECKS-THIRTY-DAYS-AGAIN` | `docs/runbooks/sync-server-cutover.md` | the checked period back to `P30D` | A3 |
| `S58404-THE-DISK-SNAPSHOTS-GO-UNDISCLOSED` | `PRIVACY.md` | the disk snapshots' bullet renamed | A4 |
| `S58405-AN-AMENDMENT-IS-NOT-AN-AMENDMENT` | ADR-351 | its amendment heading renamed | A5 |
| `S58406-THE-PAGE-CLAIMS-CRYPTO-ERASE-BEFORE-THE-KEYS` | `PRIVACY.md` | a crypto-erase claim with no key table | A6 |

## 9. Delivered by later pull requests

| # | requirement | delivered by |
|---|---|---|
| R8 | the wrapped-key table, the key stores, the key-first erase, the restore through the wrap key and the rehearsal's two synthetic learners | the key-layer delivery, after R9 (a) |
| R9 | the at-rest spike and the replicator spike | two spike deliveries on synthetic data |
| R10 | the media backup under each learner's key | the media delivery |
| R11 | each layer's prune at its age (6 days, the second-region copy 4), the disk snapshots and the second-region copy | the deploy delivery, after the owner's steps |
| R12 | the shared records' personal fields | the account delivery |

## 10. What only CI proves

`scripts/tests/test_backup_units.py` executes deploy scripts, so it runs only in CI, where CI
reads `test_the_copies_fit_the_declared_backups_window` green over the new `privacy.json` and
`PRIVACY.md`. The threat model's line census runs in CI.
