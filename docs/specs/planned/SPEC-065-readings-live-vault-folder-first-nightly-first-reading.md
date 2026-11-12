# SPEC-065: the readings folder exists with group write, DeckStreak is its only writer, and the first nightly run and the first reading reach the owner

- **Wave:** W2. **Issue:** #45 (epic #3). **Context(s):** `deploy` (`deploy/optional/vault-archive/`,
  `deploy/scripts/`); the private rail (the folder, the fences and the settings on the host).
- **Decided by:** ADR-011 (side by side, one writer per vault contract), ADR-019 (the readings
  surface: the Mini App first, the vault as the archive), ADR-053 (the vault archive switch stays off
  until DeckStreak is the single writer), ADR-054 (no-AI mode is the default), ADR-061 (host values
  as drop-ins), and this SPEC's ADR-065 (every other writer is fenced from the readings folder, and
  each fence proved, before DeckStreak writes it).
- **Waits for:** owner gate 2 (#161) for the folder, the fences and the vault writer's drop-in. The
  first reading also waits for gate 3 (#162) and SPEC-063, or for the owner's choice of an API key
  (ADR-054). Until then, the first nightly run records that readings are not enabled.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-065.md` (ADR-016).

## 1. The problem, measured

- **The folder is made on purpose, once.** DeckStreak's vault adapter never creates a folder at the
  vault's top level, and it refuses to start while the readings folder is missing or not writable
  (SPEC-042 R1). So the rail creates it and its archive folder, with the owner, group and mode the
  vault's private file contract names.
- **One writer at every moment.** The readings folder has one writer (ADR-011, ADR-053). Once the
  folder exists with group write, any unit on the host, other than DeckStreak's, whose settings let
  it write the vault could write the folder as well, whatever its own code intends. ADR-065 fences
  every such unit, and proves each fence, before DeckStreak's archive switch goes on.
- **DeckStreak's archive switch is off by default.** SPEC-053 R8 switches it on only once the
  owner's go makes DeckStreak the readings folder's writer; ADR-065 decides how (R2). Until the
  switch goes on, the Mini App serves every reading and nothing reaches the vault.
- **The service's umask would break the vault's file contract.** SPEC-032 R2 gives every unit
  `UMask=0077`, so a note written under it could be read by the service user alone, while the
  contract requires every file in the folder to be readable and writable by the vault's group.
- **With no AI route, a night generates nothing** (ADR-054): every topic ends `ai_route_absent`, the
  Mini App and the bot say readings are not enabled, and nothing alerts (SPEC-050 R10).

## 2. Requirements

R1. The rail creates the readings folder and its archive folder with the owner, group and mode the
    vault's private file contract names, and nothing else at the vault's top level, through
    `deploy/scripts/make-readings-folder.py`. The mode gives the group write and passes the group on
    to everything created inside (a setgid, group-writable directory), and the tool refuses a mode
    that does not. The step is idempotent on folders that already match, and refuses folders that
    exist with another owner, group or mode.
R2. At once after R1, in the same step, off every slot of the reserved-slot list the private rail
    provides (SPEC-053 R2), off DeckStreak's own job slots,
    and off every scheduled slot of each unit on the other-writer list, which that list carries,
    each unit on the rail's other-writer list is fenced from the readings folder: the rail installs
    a drop-in that makes that folder read-only inside the unit's mount namespace, and restarts the
    unit once (ADR-065). The folder must exist first, because a path the drop-in lists must exist
    when the unit starts. `deploy/scripts/prove-read-only.sh` then proves, for each unit on the
    list, that a write from inside its namespace, as its user, is refused. R1 and R2 happen before
    DeckStreak's archive switch goes on, and the list and the proofs are recorded privately.
R3. The unit that writes the vault runs with the vault group as a supplementary group, `UMask=0002`,
    and write access to the lock directory SPEC-048 R2 names and the readings folder only, through
    the drop-in `deploy/optional/vault-archive/deck-streak-readings-generate.conf`, which narrows
    SPEC-053 R9's vault root to the readings folder. A file it creates there is readable and
    writable by the vault's group, as the contract requires. The drop-in waives durable-services'
    advisory `sandbox.umask` with that reason, as the pack allows (`X-DurableServices-Waive=`); the
    vault root's own mode keeps other users out.
R4. The vault settings (SPEC-042 R1) are added to the environment file, and the archive switch is
    turned on, only after R1 to R3's evidence is recorded: SPEC-053 R8's condition, as ADR-065 meets
    it.
R5. The first nightly run on the host, before gate 3, records `ai_route_absent` for every topic,
    raises no alert, writes nothing, and the Mini App, the bot and the status panel say readings are
    not enabled (ADR-054, SPEC-050 R10).
R6. With the route enabled (SPEC-063): a night with new cards after a successful sync produces one
    reading per topic in the Mini App and, with the switch on, in the vault; a night without new
    cards shows "No new cards today" and raises no failure alert (SPEC-050 R2, R4); every note passes
    the vault rails (SPEC-042 R3), and nothing is ever a placeholder (SPEC-046 R8).
R7. The first reading keeps the owner's reading rules (SPEC-001 section 9): honest states, fail loud,
    no placeholder, and a failed topic shows its closed reason.
R8. The rollback turns the switch off and removes the vault settings and the vault writer's drop-in;
    no note already written is deleted, and the owner's ticks stay. The fences stay until the owner
    decides otherwise (#164); removing one is its drop-in and one restart of its unit, after the
    switch is off.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the vault writer's drop-in, merged with the readings unit's template by the units reader (`scripts/tests/_units.py`), sets `UMask=0002`, one supplementary group, and write access to the lock directory and the readings folder only, and waives the umask advisory with a why of more than five words | `test_vault_archive.py` |
| A2 | the folder tool creates exactly the readings folder and its archive folder, with the setgid group-writable mode it is given, and nothing else under the root (examined count reported) | `test_vault_archive.py` |
| A3 | the folder tool leaves matching folders as they are, and refuses a folder that exists with another mode or group, and a mode without group write or setgid | `test_vault_archive.py` |
| A4 | the read-only prover passes when the write inside the unit's namespace is refused, and fails, removing its probe file, when the write succeeds | `test_vault_archive.py` |
| A5 | the vault writer's drop-in adds no credential pair | `test_vault_archive.py` |
| A6 | no vault-archive file names a private value, and a planted one is refused by the public scrub | `test_vault_archive.py`; `scripts/public-scrub.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_vault_archive.py -k test_the_vault_writer_drop_in_narrows_its_writes_and_waives_the_umask
A2: python3 -m unittest discover -s scripts/tests -p test_vault_archive.py -k test_the_folder_tool_creates_exactly_the_two_folders
A3: python3 -m unittest discover -s scripts/tests -p test_vault_archive.py -k test_the_folder_tool_is_idempotent_and_refuses_a_foreign_mode
A4: python3 -m unittest discover -s scripts/tests -p test_vault_archive.py -k test_the_read_only_prover_passes_only_on_a_refused_write
A5: python3 -m unittest discover -s scripts/tests -p test_vault_archive.py -k test_the_vault_writer_drop_in_adds_no_credential_pair
A6: python3 -m unittest discover -s scripts/tests -p test_vault_archive.py -k test_no_vault_archive_file_names_a_private_value
```

A2 and A3 build a synthetic vault root in a `TemporaryDirectory` owned by the test's own user and
group; A4 runs the prover against a synthetic unit, with stub `systemctl` and `nsenter` that report
the verdict the test chooses. None of them reaches a host or a vault. The box-run packs judge the
merged unit as well (ADR-069); that verdict is the delivery's evidence, not a criterion here.

## 4. The owner's gates and the evidence they record

The exact commands, the vault's paths, the group's and users' names and the other-writer list are in
the maintainer's private gate packet; this SPEC names each step only.

| step | gate | what is approved | evidence recorded (privately) | rollback |
|---|---|---|---|---|
| E1 | 2 (#161) | the two folders, off every reserved slot and off every scheduled slot of each unit on the other-writer list, which that list carries | owner, group and mode of each | remove the two empty folders |
| E2 | 2 (#161) | at once after E1, off every reserved slot and off every scheduled slot of each unit on the other-writer list, which that list carries: the fence on each unit of the list, and one restart of each | each prover's refused write; the health-check list read before and after | remove that drop-in, restart that unit |
| E3 | 2 (#161) | the vault settings, and the drop-in for the vault writer | `effective-check.py`; a file created under the unit's umask, then read back by another member of the vault's group and removed | R8 |
| E4 | 2 (#161) | the first nightly run, before gate 3 | every topic `ai_route_absent`; no alert in the journal; the Mini App's "not enabled" state | none: nothing was written |
| E5 | 2 (#161) | the archive switch on, after E1 to E3 | the setting's change and the next run's `vault_archive_off` gone | R8 |
| E6 | 3 (#162) | the first night with new cards, with the route enabled | one reading per topic in the Mini App and the vault; the owner confirms the note reached the vault on a device (#45's second criterion) | R8 |
| E7 | 2 (#161) | a night without new cards | "No new cards today"; no alert (#45's third criterion) | none needed |

## 5. File manifest

| file | context | change |
|---|---|---|
| `deploy/optional/vault-archive/deck-streak-readings-generate.conf` | deploy | added: the vault writer's drop-in, neutral values |
| `deploy/scripts/make-readings-folder.py` | deploy | added: the folder tool |
| `deploy/scripts/prove-read-only.sh` | deploy | added: the read-only prover |
| `deploy/README.md` | deploy | changed: the first live night's checklist |
| `scripts/tests/test_vault_archive.py` | repo | added: A1 to A6 |
| `docs/specs/SPEC-065-readings-live-vault-folder-first-nightly-first-reading.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-065-every-other-writer-is-fenced-from-the-readings-folder-before-deckstreak-writes-it.md` | docs | changed: status accepted |
| `docs/red-first/SPEC-065.md` | docs | added |
| `changelog.d/` fragment | repo | added |

The fences' drop-ins and the other-writer list name units of the host, and the vault's paths and
names are the owner's: all of them stay in the private rail.

## 6. What this does NOT do

- It enables no AI route; the first reading waits for gate 3 or the owner's choice of an API key
  (#162).
- It changes nothing of another service's use of the vault beyond R2's fences (#161).
- It performs no step of the cutover (#164).
- It changes no readings rule and no readings code (#32).
- It writes no stats file, drill or inbox capture (#153, #136, #154).

## 7. Risks

- **A fence's restart interrupts its unit.** It is one restart per unit, off every reserved slot
  and off every scheduled slot of each unit on the other-writer list, which that list carries, and
  the health-check list is read before and after (E2).
- **The vault's group cannot read DeckStreak's notes.** R3's umask and group make them readable, and
  E3 proves it before the switch goes on.
- **The vault's folder modes change.** DeckStreak's start check then fails and the readings health
  reports a broken rail (SPEC-042 R1, SPEC-050); the host findings are tracked privately (gate 8,
  #167).
- **A writer is missing from the list.** It would be a second writer. The list is built from every
  loaded unit's settings in SPEC-060's inventory, and the owner approves it at gate 2 before the
  switch goes on.
- **The first reading needs a study day with new cards.** The owner can trigger a sync and tap to
  regenerate (SPEC-048) once the route is enabled; a day without new cards is honest, not a failure.
