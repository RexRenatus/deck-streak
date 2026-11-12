---
status: accepted
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-603: backups keep at most seven days, each copy is pruned within six by a rule the host cannot override, and an erase destroys the learner's key first

Decides SPEC-584 (issue `#44`). The declared backup window moves from three days (`P3D`) to seven
(`P7D`), and the offsite archives' period from thirty days (`P30D`) to the same seven. This record
decides how every backup layer keeps that window, how one data key per learner makes an erased
learner's data unreadable in every copy at once, which extra copies are kept and for how long, and
which records change. It changes no code, unit, job or setting: each deploy and code change it
needs is a later delivery that SPEC-584 section 5 names.

**Amends:** ADR-064 (its window), ADR-351 D9 (the offsite archives' period), SPEC-021 R5 and R6,
and SPEC-340 R11, each by a section appended to its file. SPEC-064's window, restore rehearsal and
bucket requirements (its R3, R5, R8 and R9) are amended by this record and SPEC-584; SPEC-064's own
file is unchanged in this delivery, because its surface is parked (`#158`), and gains its pointer
when the park lifts.

## Context and Problem Statement

Measured at this delivery's base on `dev`:

- `privacy.json:450` declares the backups window `P3D` against `deploy/litestream.yml`, which keeps
  a snapshot every 24 hours for 48 hours (`litestream.yml:9-11`); `deploy/scripts/backup.py:43`
  keeps three daily copies on the host; `PRIVACY.md:99-105` tells the learner both, inside `P3D`.
- `ADR-064:52-54` keeps an erased row at most three days and turns the bucket's soft delete off so
  the window holds offsite too; `ADR-064:103-104` says a longer window changes the declared window,
  the policy and the retention together, by an amendment of SPEC-021 (`SPEC-021:58-70`, R5 and R6).
- The sync server's archives are sealed to a key that is never on the host (`ADR-351:43-55`, D2)
  and deleted offsite `P30D` after writing by a bucket rule (`ADR-351:125-132`, D9), a period
  `PRIVACY.md:116-122`, `SPEC-340:75-78` (R11), `docs/runbooks/sync-server-cutover.md:52-55` and
  `scripts/tests/test_sync_server_runbook.py:27` state.
- The erase deletes every port's rows in one transaction with `secure_delete` on, rebuilds the
  file and empties its write-ahead log (`crates/privacy/src/erase.rs:5-16`, `:46`). No backup
  copy is encrypted per learner, so an erased learner's rows stay readable in each copy until that
  copy ages out.
- The host's identity may delete the replica's whole history, and no record states a recovery
  point or a recovery time. D4 sets both as targets: a recovery point of 1 minute for the learner
  stores and the shared records, and a recovery time of 15 minutes for one learner and 2 hours for
  the whole service, never stated as met before a restore has been timed on the server.

## Decision Drivers

- An erased learner's data is unreadable in every copy at once, and its bytes are gone from every
  copy within seven days.
- No copy outlives the window for any reason: no soft-delete hold, no retention lock, no vault
  that cannot delete.
- A host that is compromised cannot delete the backups' history.
- Every clause of `PRIVACY.md` is true of `dev` when it lands (ADR-379's driver); a copy that may
  exist before the next delivery is disclosed before it exists.
- Enabling the key service, creating its keys, the disk snapshot schedule and the second-region
  copy are the owner's steps on the host (`#161`, `#166`): this record designs them and performs
  none.

## Considered Options (the alternatives each was chosen against)

### D1. The surfaces, measured

- Amend exactly the records that state an old period: `SPEC-021`, `SPEC-340`, `ADR-064` and `ADR-351`, the only SPECs and ADRs outside the parked names whose text holds `P3D` or `P30D` at `dev`, each by an appended section — chosen, because a census over `docs/specs` and `docs/decisions` finds these four and no other, and the test of SPEC-584 A5 holds the set.
- Rewrite every file the backup words reach: rejected because the broad census lists 741 paths, almost all of them using "snapshot", "archive" or "restore" for other things, and a rewrite of each would change records that state no window.
- Edit SPEC-064's own file in this delivery: rejected because its name is in the parked surface (`#158`), so no build may open it; this record and SPEC-584 carry its amendment until the park lifts.

### D2. The window and crypto-erase, layer by layer

- Every layer prunes each copy by six days old at the latest, the second-region copy at four, by a rule the host's identity cannot override, and the seventh day is margin for the pruner's own delay — chosen, because a bucket rule acts some hours after its condition is met, and a window filled to the hour would be broken by that delay.
- The logical layer (the replica): the replicator deletes nothing (its retention enforcement is switched off), the replica bucket's lifecycle rule deletes each object six days after it is written, the host's identity may create and read objects but never delete them, the snapshot interval stays 24 hours and the snapshot retention is set to 120 hours so that interval plus retention reads six days — chosen, because the replicator then needs no delete permission, so a compromised host cannot delete the history, and a restore reaches any point in the last five days and most of the sixth.
- The daily copies on the host: the newest six are kept, and any copy older than six days is removed even when the day's copy failed — chosen, because the copy job today keeps the old copies untouched when a run fails, so a run of failures would keep erased data past any window.
- Soft delete stays off on every backup bucket, and no retention lock or locked vault is used — chosen, because a seven-day soft-delete hold on top of a six-day rule keeps deleted bytes up to thirteen days, and a lock can neither be shortened nor coexist with destroying a key.
- The media layer, when media is stored on the service: each blob is encrypted under its learner's data key before any copy is made, so an erase makes every copy unreadable at once, and an erased learner's media backups go within six days: the erase writes a time mark on each of that learner's backup objects, through a grant that may update an object's metadata and never delete it, and a deletion rule on age from that mark removes each marked object within six days, so no identity holds an object-delete right on the media backup — chosen, because a blob outlives a six-day age rule while its learner keeps it, so an age rule alone would delete live media, while the mark reaches only an erased learner's objects; the cost this form accepts is that a compromised host could age every object early (SPEC-584 R10).
- The sync bridge: the host keeps the newest three generations and archives as now, the offsite archives' lifecycle rule deletes each archive six days after it is written, inside `P7D`, and the seal to the owner's offline key stays — chosen, because the bridge's store is the learner's own Anki data, which an erase leaves (`PRIVACY.md:116-118`), so the seven-day period bounds how long a sealed copy exists, and the bridge retires when the service's own sync replaces it.
- Keep the three-day window: rejected because it gives up the longer restore history the larger window pays for, and leaves the replica's history deletable by the host.
- Thirty days with crypto-erase: rejected because its copies hold about four times the bytes of seven days, and an erase would rest on the key alone for a month.
- Litestream's own retention with a delete grant: rejected because a compromised host could then delete every snapshot.
- Keep the offsite archives at thirty days: rejected because a thirty-day sealed copy outlives the seven days every other copy keeps, and it is the copy an attacker who stores ciphertext for later would target.
- A sweep identity holding the only delete right on the media backup, run by a scheduled job with its own credential: rejected because a stolen sweep credential could delete every media copy at once, while every other backup bucket leaves deletion to its rule alone.

### D3. One data key per learner, and the at-rest layer

- One data key per learner (256 random bits from the operating system), stored only wrapped by one key-service key, one row per learner in the wrapped-key table `user_data_keys` (the learner's id, the wrapped key, the wrap key's version and when it was made), with the unwrapped key held in the server's memory only while that learner's store is open — chosen, because one key-service key costs the same for every learner count, no key ever sits on a disk or in a settings file, and destroying one row's copies destroys that learner's key.
- The wrapped-key table has no history: its rows live as one object per learner in two key stores, one beside the server and one in a second region, with versioning, soft delete and every backup off, and the server keeps no key on any disk — chosen, because a wrapped key inside any copy that keeps history (the replica, a daily copy, a disk snapshot) would stay unwrappable by the same key-service key for that copy's life, and the erase would not be at once.
- The erase destroys the key first: it deletes the learner's wrapped key from both key stores, and reports nothing done until both deletes are confirmed; it then erases the rows as `crates/privacy/src/erase.rs:46` does today, and the copies age out within the window — chosen, because a crash between the two steps then leaves ciphertext with no key, never readable rows with a live key.
- A restore reaches the wrap key through the server's own identity: the store is restored to a private path (the replica's point in time, a daily copy or a disk snapshot), its wrapped key is read from the key store beside the server or, when that is lost, from the second-region key store, unwrapped by the key service, and the store is opened and checked; an erased learner's store restores as ciphertext that nothing opens — chosen, because the key stores are the only place a key comes from, so a restore can never revive an erased learner.
- The at-rest layer is chosen by a measured spike between page encryption of each learner's whole store under its raw data key and row-level authenticated encryption of each personal column by the application, which this record does not decide: the spike measures, over synthetic stores and never a learner's, the open cost per store, the overhead on the answer write and the due-card read, resident memory per open store, the replica's bytes and requests per store per day, whether each builds for the server, the iPhone and iPad app and the web engine, what each leaves readable (indexes, sizes, timestamps), and one store's restore time through the wrap key — chosen, because each figure decides the choice and none is measured.
- The disk and the buckets carry DeckStreak's own key on the key service, beside the per-learner keys — chosen, because a disk or bucket copy taken outside the application is then unreadable without that key too.
- A key-service key per learner: rejected because its price grows with every learner, while one wrap key's does not.
- The wrapped key inside each learner's store, or in a table of the backed-up database: rejected because every backup of that store would hold the key beside the data for the copy's whole life.
- A history of the key table kept for a short window: rejected because any history holds an erased learner's key, unwrappable, for that window.
- A daily rotation of the wrap key, old versions destroyed after the window: rejected because destroying a version ages out every learner's keys together and makes no single erase immediate.

### D4. The extra copies, and the recovery targets

- Daily disk snapshots of the data disk, one a day, each deleted six days after it is taken, kept while the service is at its first size and removed before it grows past that size, the disk holding no key — chosen, because a whole-disk copy restores a lost disk faster than a store-by-store restore, and an erased learner's store in a snapshot is ciphertext whose key is in no snapshot.
- A second-region copy of the shared database and the user directory: a second replica of each, in a second-region bucket under a create-only identity, whose deletion rule removes each object at four days old, with soft delete off, read back by the restore rehearsal — chosen, because losing either leaves every learner unroutable, both are small, and the rule bounds an erased learner's directory row to the window.
- Every personal field of the shared database and the user directory is either encrypted under its learner's data key or a random id that names no person once that learner's key and sign-in links are gone — chosen, because the directory must route a request before any key is unwrapped, and crypto-erase reaches only what a key protects; a field that can be neither is a question for the account design (SPEC-584 R12).
- The recovery targets are a recovery point of 1 minute for the learner stores and the shared records, and a recovery time of 15 minutes for one learner and 2 hours for the whole service; both are targets, never stated as met before a restore has been timed on the server — chosen, because a stated target gives the first timed restore a figure to be checked against, and no restore has been timed there yet.
- A whole-disk backup vault that cannot delete: rejected because it would keep every copy past the window.
- A two-region bucket for every learner's data: rejected because it doubles the bytes of the largest layer for a loss the single-region bucket's own redundancy already covers at this size.
- Disk snapshots at every size: rejected because past that size their bytes cost more than the logical layer they duplicate.

### D5. The records

- Append an amendment section to ADR-064, ADR-351, SPEC-021 and SPEC-340, in the house's insert-only form, each naming SPEC-584 and `P7D`; change `PRIVACY.md`'s window, archive and copies bullets, `privacy.json`'s backups window, the cutover runbook's bucket rule, the two schematics that state an old period and the test constant that holds the offsite period — chosen, because each states an old period today and an insert-only section keeps every earlier line as it was.
- The privacy page states the seven-day window and the extra copies in this delivery, and states crypto-erase in the delivery that builds the keys, in the words SPEC-584 R7 fixes, a test refusing either without the other — chosen, because the page states only what is true of `dev` (ADR-379), and today no key exists, so a crypto-erase sentence now would promise a protection no code gives.
- The extra copies are disclosed before they exist, each as "when the service keeps them" — chosen, because the owner creates them on the host, outside the repository, and a page that waited for the next delivery would be false from the moment one exists.
- State crypto-erase on the page now: rejected because it would be false of `dev` until the key layer lands.
- Leave the page at three days until the deploy changes land: rejected because the window is the promise the deploy changes are built to keep, and the copies today already fit inside seven days.

### D6. Shape, tests and rows

- One docs delivery: the SPEC, this record, a schematic, the amendments, the text changes, one new test module `scripts/tests/test_backup_window.py`, one moved constant in `scripts/tests/test_sync_server_runbook.py`, a red-first record, a changelog fragment and rows `S58401` to `S58406`, with one push — chosen, because every criterion is a text test that runs over a fixture tree outside CI and reads red before its text changes.
- Two pushes, the red read in CI: rejected because no criterion's red needs a module that runs only in CI; the deploy tests that read the window keep reading green, since the copies stay inside the new window.
- Rows over the changed text lines, each killed by its criterion's test — chosen, because each row's mutant restores an old period or removes a disclosure, which is the regression each test exists to refuse.
- No rows: rejected because a pin whose text no mutant moves proves nothing about the regression it names.

### D7. Not in this delivery

- The deploy and code changes are later deliveries, each named by SPEC-584 section 5 with the lines it changes, and the owner's steps on the host stay the owner's — chosen, because this delivery is the record they are built against, and a docs delivery that changed a unit would carry a deploy-class risk it cannot test outside CI.
- Carry the keep counts and the replicator's settings here: rejected because those files are read only by tests that run in CI, and the window change does not need them to land first.

### D8. Public text

- Every public file names the change only: no provider, host, bucket, key name, price or date, and the key service, the buckets and the regions in plain words — chosen, because public text carries none of them.
- Name the provider's products: rejected because public text names no provider.

## Decision Outcome

D1 to D8 as chosen above. The declared backups window is `P7D`; every backup copy is pruned by a
rule the host cannot override, the second-region copy at four days old and every other copy at
six; soft delete stays off; an erase destroys the learner's wrapped key in both key stores before
anything else; a restore reaches the wrap key only through the server's identity; daily disk
snapshots and a second-region copy of the shared database and the user directory are the owner's
steps, designed here; the recovery targets are a recovery point of 1 minute for the learner
stores and the shared records and a recovery time of 15 minutes for one learner and 2 hours for
the whole service, never stated as met before a restore has been timed on the server.
`PRIVACY.md` states the window and the copies now and crypto-erase when the keys exist.

## Consequences

- Good: an erase will reach every copy at once once the keys exist, with no copy older than seven
  days at any time.
- Good: a compromised host cannot delete the replica's, the archives' or the snapshots' history.
- Good: the page states only what holds, and a test refuses a crypto-erase sentence before the
  keys and a key table without the sentence.
- Bad: the two key stores are the one place every key lives; losing both makes every backup
  unreadable, and the host's identity must hold the one delete right the erase needs on them.
- Bad: the host's identity may write the media backup's time marks, so a compromised host could
  age every media object early, though it holds no right to delete one.
- Bad: the replica's bytes grow to about seven snapshots per database from three.
- Bad: until the deploy changes land, the copies keep three days inside a seven-day window, so the
  restore history the window allows is not yet kept.
- Neutral: a database idle for more than six days could lose its last snapshot to the bucket rule
  if the replicator takes no snapshot of an unchanged database; the spike of SPEC-584 R9 measures
  it, and the deploy change keeps one snapshot a day if it does not.

### Confirmation

SPEC-584's A1 to A6, their planted controls and rows `S58401` to `S58406`; in CI, the deploy tests
that hold the copies inside the declared window.

## What would make this wrong

- The bucket rule's delay is measured above a day: the six-day prune moves earlier, by an
  amendment of this record.
- The spike finds the replicator cannot run with its retention switched off: the layer keeps its
  own retention under a separate pruning identity, recorded in a new ADR.
- The owner keeps an escrow of the keys for disaster recovery: the page's crypto-erase sentence
  must then say what the escrow holds, and SPEC-584 R7 changes first.

## More Information

ADR-064; ADR-351; ADR-379; SPEC-021; SPEC-064; SPEC-340; SPEC-584;
`docs/schematics/backup-window-and-crypto-erase.md`; the Litestream documentation on snapshot
retention and on switching retention enforcement off, read through Context7; `#44`, `#14`,
`#158`, `#161`, `#166`.
