# SPEC-064: DeckStreak's database is replicated continuously, copied daily and restored weekly by its own units, inside a declared window and budget

- **Wave:** W2. **Issue:** #44 (epic #3). **Context(s):** `deploy` (the Litestream, backup and
  restore-drill units and scripts), `repo` (`privacy.json`, `PRIVACY.md`); the box-run packs'
  private wiring (ADR-069).
- **Decided by:** ADR-010 (Litestream, a daily backup and a weekly restore drill, the offsite
  bucket chosen with the owner), ADR-008 (one SQLite database in WAL mode), ADR-037 (the collection
  copy is re-downloaded by the day's sync and never uploaded), ADR-032 (the host budget), ADR-061
  (host values as drop-ins), ADR-062 (the release carries only what CI built), and this SPEC's
  ADR-064 (DeckStreak's own units and configuration run the backups, on a Litestream binary the
  rail provides, and the collection copy is never backed up).
- **Waits for:** owner gate 7 (#166) for the bucket, and gate 2 (#161) for the units, the grant on
  the bucket and the first drill. SPEC-062's first deploy precedes it, and nothing here needs a
  device key (ADR-054).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-064.md` (ADR-016).

## 1. The problem, measured

- **DeckStreak's database has no backup yet.** ADR-010 decided continuous replication with
  Litestream, a daily backup and a weekly restore drill that cover DeckStreak's database, and an
  offsite bucket chosen with the owner; none of them exists for it.
- **Only DeckStreak's own user can read its state.** The state directory is private to the service
  user (`UMask=0077`, SPEC-032 R2), so a backup that runs as any other user cannot read the
  database, and a backup DeckStreak does not own would change whenever its owner changed it.
- **The template is delivered, not the units.** SPEC-021 delivered `deploy/litestream.yml`:
  Litestream 0.5, one replica, a global snapshot interval of 24 hours and retention of 48 hours, and
  a validation interval of 24 hours, inside a declared backups window of `P3D`. Its replica URL is
  `${DECKSTREAK_REPLICA_URL}`, a placeholder with no scheme, left to the private rail (SPEC-021
  section 7). SPEC-032 R8 deferred durable-services' three backup rows to this issue, a deferral the
  box-run packs' private wiring now holds (ADR-069).
- **The drill is owed.** SPEC-027's risks leave the proof that a `TRUNCATE` checkpoint beside
  Litestream's read lock keeps the replica sound to this issue's restore drill.
- **The copies take room that DeckStreak budgets.** At most three daily copies of the database stay
  on the host (R9), and the first week's figures are measured against the host's free space,
  privately.

## 2. Requirements

R1. `deploy/systemd/deck-streak-litestream.service` runs `litestream replicate -config` with
    DeckStreak's own configuration file, as the service user, hardened as SPEC-032 R2's units are,
    with `Restart=on-failure`, a start limit that can trip, `OnFailure=` the alert template and its
    ceilings in the host budget. It runs the Litestream binary the private rail provides, at
    version 0.5.2 or later, which the gate-2 rehearsal reads and records.
R2. The configuration is SPEC-021's `deploy/litestream.yml`, installed as committed, with one
    change: the replica's URL becomes a `gs://` URL whose bucket is a `${VAR}` that Litestream
    expands from the unit's environment file, which the rail renders. The committed file then names
    no bucket, and the URL reads as off the host by its scheme. It keeps SPEC-021's interval,
    retention and validation, and sets no replica-level retention.
R3. The declared window holds: an erased row survives at most the snapshot interval plus the
    retention in the replica (72 hours) and three days in the daily copies, both inside `P3D`.
    `privacy.json`'s `backups` block and `PRIVACY.md` state the replica and the daily copies with
    that window.
R4. `deploy/systemd/deck-streak-backup.service` and its timer run `deploy/scripts/backup.py`
    (standard-library Python) once a day. It copies the live database with SQLite's online backup API
    into a temporary file beside the backups directory, runs `PRAGMA integrity_check` on the copy,
    renames it into place, keeps the newest three, and exits non-zero on any failed step, leaving the
    previous copies untouched. It copies neither the collection copy nor any credential.
R5. `deploy/systemd/deck-streak-restore-drill.service` and its timer run
    `deploy/scripts/restore-drill.sh` once a week. It restores the replica with
    `litestream restore -config <DeckStreak's configuration> -o <a private temporary path> <the
    database>`, opens the newest daily copy, runs `PRAGMA integrity_check` on both, compares each
    copy's migration version with the live database's, removes both copies, and exits non-zero on any
    failed step, paging through `OnFailure=`.
R6. The daily backup runs at 03:24 and the drill on Sundays at 06:24, in the owner's zone as the
    rail renders it (SPEC-032 R4). A test proves both slots off the predecessor's schedule as
    SPEC-027 R2 defines it, its sync minutes included, off every slot of the reserved-slot list the
    private deploy rail provides (SPEC-053 R2), and off every slot of DeckStreak's job table; CI
    proves the logic with a synthetic list.
    The two timers are not in that table: it is the schedule of the jobs `deckstreakd job <id>`
    runs, each claimed in the cron-fire ledger (ADR-027), while these units run a script and
    Litestream, as SPEC-031's evaluator and memory watch timers do.
R7. The units are timer-activated oneshots, except the Litestream daemon, each with `OnFailure=`, a
    ceiling in `deploy/host-budget.json`, `UMask=0077` and a private temporary directory.
R8. The bucket is the owner's gate 7 (#166). Recommended: a new bucket for DeckStreak alone, with
    public access prevention enforced, uniform bucket-level access, no public principal, soft delete
    off (so an erased row is gone when the window ends), no retention policy (Litestream prunes its
    own files), and object admin on that bucket only for the identity the unit runs with. The
    alternative is a prefix of DeckStreak's own in an existing bucket of the owner's; the
    anti-enumeration rule holds either way, and the choice is recorded.
R9. Budget, stated in numbers: the host keeps at most three daily copies of the database; the
    bucket holds at most one snapshot interval plus the retention of Litestream's files. The first
    week's figures (the database's size, the backups directory, Litestream's local files and the
    bucket's size) are recorded and compared with the host's free space and the bucket's.
R10. The box-run packs' private wiring ends the three backup deferrals SPEC-032 placed on
    durable-services, and the durable lint's wait for a Litestream unit. The delivery hands that
    change back to the maintainer, and the box run on its head is its evidence (ADR-069).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the backup units keep three daily copies, one replica off the host by its URL's scheme, one restore drill, and one replica per database in Litestream 0.5's form (examined count reported) | `test_backup_units.py` |
| A2 | the backup copies a synthetic database in WAL mode while it is written, checks the copy's integrity, keeps the newest three and removes older ones (examined count reported) | `test_backup_units.py` |
| A3 | a copy that fails its integrity check fails the backup and leaves the previous copies untouched | `test_backup_units.py` |
| A4 | the drill restores through `litestream restore -o` to a private path, checks both copies' integrity and migration version, removes them, and fails when either check fails | `test_backup_units.py` |
| A5 | the snapshot interval plus the retention, and the daily copies' age, fit the declared backups window, and no replica sets its own retention | `test_backup_units.py` |
| A6 | the backup and drill slots are off the predecessor's schedule, off every slot of a synthetic reserved-slot list and off the job table (examined count reported) | `test_backup_units.py` |
| A7 | no backup file names a bucket, a project or a host, the replica's bucket is an environment reference inside a `gs://` URL, and a planted value is refused by the public scrub | `test_backup_units.py`; `scripts/public-scrub.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_units_keep_three_copies_one_replica_and_a_drill
A2: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_backup_copies_checks_and_keeps_three
A3: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_a_failed_integrity_check_fails_the_backup_and_keeps_the_old_copies
A4: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_drill_restores_checks_and_removes_both_copies
A5: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_copies_fit_the_declared_backups_window
A6: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_the_backup_and_drill_slots_are_off_every_other_slot
A7: python3 -m unittest discover -s scripts/tests -p test_backup_units.py -k test_no_backup_file_names_a_private_value
```

A2 and A3 run the script over a synthetic database in a `TemporaryDirectory`, A3 with its integrity
step made to report a failure; A4 runs the drill with a stub `litestream` that records its argument
vector and writes a synthetic copy. None of them reaches a bucket or a host. The box-run packs judge
the units and the policy as well (ADR-069); that verdict is the delivery's evidence, not a criterion
here.

## 4. The owner's gates and the evidence they record

The exact commands, the bucket's name and the host's paths are in the maintainer's private gate
packet; this SPEC names each step only.

| step | gate | what is approved | evidence recorded (privately) | rollback |
|---|---|---|---|---|
| E1 | 7 (#166) | the bucket, per R8 | its settings read back: public access prevention, uniform access, soft delete, no public principal; an anonymous listing refused | delete the empty bucket |
| E2 | 2 (#161) | the grant on the bucket | the binding read back, on that bucket only | remove the binding |
| E3 | 2 (#161) | the Litestream unit | `litestream version`; the first snapshot and the replica's file list | stop and disable the unit |
| E4 | 2 (#161) | the backup and drill units | the first daily copy; the first drill run by hand, restoring DeckStreak's database with its integrity check green (#44's second criterion) | stop and disable the timers |
| E5 | 2 (#161) | the first week | R9's figures against the host's free space and the bucket's size | a shorter retention, by a SPEC-021 amendment |

## 5. File manifest

| file | context | change |
|---|---|---|
| `deploy/systemd/deck-streak-litestream.service` | deploy | added |
| `deploy/systemd/deck-streak-backup.service`, `deploy/systemd/deck-streak-backup.timer` | deploy | added |
| `deploy/systemd/deck-streak-restore-drill.service`, `deploy/systemd/deck-streak-restore-drill.timer` | deploy | added |
| `deploy/scripts/backup.py` | deploy | added |
| `deploy/scripts/restore-drill.sh` | deploy | added |
| `deploy/litestream.yml` | deploy | changed: the replica's URL as a `gs://` URL whose bucket is an environment reference |
| `deploy/deck-streak.env.example` | deploy | changed: the replica bucket's setting, with a neutral value |
| `deploy/host-budget.json` | deploy | changed: the three units' ceilings |
| `privacy.json`, `PRIVACY.md` | repo | changed: the replica and the daily copies, with their window |
| the box-run packs' private wiring (ADR-069) | the maintainer's | changed: durable-services' three backup deferrals and the Litestream unit's wait ended |
| `scripts/tests/test_backup_units.py` | repo | added: A1 to A7 |
| `docs/specs/SPEC-064-backups-litestream-daily-backup-and-restore-drill.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-064-deckstreak-backs-up-with-its-own-units-and-never-the-collection.md` | docs | changed: status accepted |
| `docs/red-first/SPEC-064.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 6. What this does NOT do

- It backs up no collection copy: the day's sync downloads it again (ADR-037, #15).
- It creates no bucket; the owner does, at gate 7 (#166).
- It changes no backup, replica or drill that is not DeckStreak's own (#161).
- It keeps no copy past the declared window, and adds no long-term archive (#14).
- It restores nothing into the live database; a restore is a runbook step taken with the owner's go
  (#161).

## 7. Risks

- **The Litestream binary changes under DeckStreak.** The rehearsal records its version, the rail
  provides one of 0.5.2 or later for as long as DeckStreak's unit runs, and the weekly drill proves
  the replica with whichever binary runs.
- **A `TRUNCATE` checkpoint meets Litestream's read lock.** The checkpoint then completes partly and
  returns busy, which is not an error (SPEC-027's risk); the weekly drill proves the replica.
- **The bucket keeps deleted objects.** A new bucket's default soft delete would keep erased rows
  past the window; R8 turns it off and E1 reads it back.
- **A backup fills the disk.** Three copies of a small database, and the first week's figures
  measure it (R9); a failed write leaves the previous copies intact (A3).
- **The drill pages on a slow restore.** It runs on Sunday morning, off the predecessor's schedule,
  every reserved slot and DeckStreak's own job slots (R6), with its own ceiling; a failure pages
  once through `OnFailure=`.
- **The replica's identity is the host's.** Its grant is on one bucket only (R8), and the host
  findings are tracked privately (gate 8, #167).
