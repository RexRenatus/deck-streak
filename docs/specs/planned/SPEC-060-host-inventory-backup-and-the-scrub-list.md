# SPEC-060: the host is inventoried and snapshotted before anything changes, and nothing on it is deleted without the owner's approval of that exact item

- **Wave:** W2. **Issue:** #239 (epic #3). **Context(s):** `deploy` (`deploy/host-scrub/`), `repo`
  (the runbook and the tests).
- **Decided by:** ADR-010 (every first-time change on the host is an owner gate), ADR-011 (side by
  side until the owner's go at cutover), and this SPEC's ADR-060 (a runbook and three tools, the
  approval bound to each item's digest, and a disk snapshot as the backup).
- **Waits for:** owner gate 2 (#161) for the inventory's first run, the snapshot and every deletion.
  Nothing here needs a device key (ADR-054).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-060.md` (ADR-016).

## 1. The problem, measured

- **The owner's rule for the scrub (#239).** Before anything changes on the host, it is inventoried
  and everything on it is backed up; then every obsolete item is listed for the owner, and an item
  goes only once the owner has approved it. The host's other services keep running throughout, and
  their health is checked before and after.
- **Nothing in the repository can do it.** There is no inventory, no list format and no deletion
  tool, and no ADR says where the scrub lives (ADR-060 decides it).
- **A summed size can overstate the space a file holds.** A backup family may keep identical copies
  as hard links to one file, so sizes added up path by path count those bytes more than once, and
  deleting one link frees nothing while another survives. A copy that a service's own rotation makes
  again is not worth deleting either: the next rotation brings it back.
- **The first deploy needs room of its own.** DeckStreak puts on the disk its collection copy, and
  as much again beside it during a full download (SPEC-022's risk names this inventory as the owner
  of that headroom); three releases side by side (SPEC-062 R6); the database and its daily copies
  (SPEC-064); the journal, capped by SPEC-021's drop-in; and, with the AI route only, the agent's
  command-line tool, about 234 MB for one native build (measured on the maintainer's machine;
  SPEC-063). The inventory measures the host against that need, and the figures are recorded
  privately at gate 2 (#161).
- **One small host.** DeckStreak runs on one small host shared with other services, within a stated
  budget (CHARTER 3, ADR-032). So the scrub leaves every other service's files and health as they
  were, which this SPEC states as DeckStreak's own requirements (R7, R9), fed by lists the private
  rail provides.

## 2. Requirements

R1. `deploy/host-scrub/inventory.py` is standard-library Python. It reads the host into one JSON
    file: each mount's used and free space; for every root the rules name, each entry's size,
    blocks on disk, link count, modification time, owner and mode, with a hard-linked file counted
    once in every total; the virtual environments and git worktrees under those roots; the installed
    packages (`dpkg-query -W`); the unit files and the loaded units with their states
    (`systemctl list-unit-files`, `systemctl list-units --all`); and the ten largest memory users.
    It records every command it ran, with its exit status.
R2. The inventory runs only the read commands of its own allow list. It writes nothing but its
    output file, never restarts, stops or reloads a unit, and never calls a package manager with a
    changing verb. Each command runs under `nice` and `ionice -c3`.
R3. The roots, the retention of each backup family, the protected paths and the package list are
    private configuration. `deploy/host-scrub/rules.example.json` holds neutral example values
    only. A rule selects a candidate by class (a backup older than its family's retention, counted
    in copies or in days; a virtual environment or worktree that no loaded unit's command names; a
    package the owner lists; a loose file that matches a pattern) and states its reason.
R4. `deploy/host-scrub/plan.py INVENTORY RULES` writes a deletion list: one item per path, with an
    id, its class, the rule and reason that selected it, its bytes, and a digest. A file's digest is
    SHA-256 over its path, size, modification time, mode and content; a directory's is SHA-256 over
    the sorted list of those fields for every entry under it. The list carries its own digest and
    the total bytes it would reclaim, where a file whose other links survive reclaims nothing. The
    plan deletes nothing and writes only its output.
R5. The backup before any deletion is one snapshot of the host's boot disk, taken from the
    maintainer's machine after the inventory and before the list is approved. The approval record
    names the snapshot and the instant it was taken. No per-item copy is made on the host's own disk.
R6. `deploy/host-scrub/apply.py LIST APPROVAL` runs dry by default and deletes only with
    `--apply`. It deletes an item only when all of these hold, and otherwise refuses the whole run
    before its first deletion, naming the item and the reason:
    - the approval record carries the list's digest, names the item's id, the approver and the date;
    - the approval names a snapshot taken after the inventory;
    - the item's digest, computed again now, equals the listed one.
R7. `apply.py` never deletes a path outside the approved list, never follows a symbolic link out of
    an item, and refuses an item under any protected path whatever the approval says. The protected
    paths come from the private rail's protected-path list, which holds every path of the host's
    other services and their data; the example rules protect `/etc`, `/usr`, `/boot`, the credential
    socket's directory and every DeckStreak release directory, and the tests prove the refusal on a
    synthetic list.
R8. The inventory, each list, each approval and each apply log are private: the tools write them to
    a directory the private rail names, outside this repository, and the public tools and the
    example rules hold no private value.
R9. Every check of the private rail's health-check list, which covers each service on the host, is
    read before the inventory and after each apply. A check that is red after an apply stops the
    scrub, and the record says which check turned.
R10. The only classes W2 deletes are the owner's: backups older than their retention, obsolete
    worktrees and virtual environments, one-off files left loose, and packages the owner lists as
    unused. An item that a service's own rotation would recreate is never listed, and neither is
    anything under a protected path.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the inventory runs only the commands of its read-only allow list, and a planted changing command is refused before it runs (examined count reported) | `test_host_scrub.py` |
| A2 | the inventory of a synthetic host root reports the free space, each root's sizes with a hard-linked file counted once, the stale backup copies, the virtual environments and the worktrees (examined count reported) | `test_host_scrub.py` |
| A3 | the plan lists only the items its rules select, each with its reason and digest, and leaves the synthetic tree byte for byte as it was | `test_host_scrub.py` |
| A4 | apply with no approval, or an approval that does not carry the list's digest, deletes nothing and names the reason | `test_host_scrub.py` |
| A5 | apply deletes exactly the approved items when every digest matches, and deletes nothing when one approved item changed after the list was made | `test_host_scrub.py` |
| A6 | apply refuses an approval that names no snapshot, or a snapshot taken before the inventory | `test_host_scrub.py` |
| A7 | apply refuses an approved item under a protected path of a synthetic list, and never follows a symbolic link out of an item | `test_host_scrub.py` |
| A8 | no host-scrub file names a private value, and a planted one is refused by the public scrub | `test_host_scrub.py`; `scripts/public-scrub.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_the_inventory_runs_only_its_read_only_allow_list
A2: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_the_inventory_reports_space_backups_venvs_and_worktrees
A3: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_the_plan_lists_rule_selected_items_and_deletes_nothing
A4: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_without_a_matching_approval_deletes_nothing
A5: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_deletes_exactly_the_approved_items_or_nothing
A6: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_refuses_an_approval_without_a_later_snapshot
A7: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_refuses_protected_paths_and_symbolic_links_out
A8: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_no_host_scrub_file_names_a_private_value
```

A1 runs the inventory with stub commands first on its `PATH`, each recording its argument vector
into a `TemporaryDirectory`. A2 to A7 build a synthetic host tree in a `TemporaryDirectory` at run
time, and A7 a synthetic protected-path list; no fixture holds a real path, size or name. A8 writes
its planted value at run time, as SPEC-032's A6 does, so no private literal is ever committed.

## 4. The owner's gate and the evidence it records

Every step on the host waits for gate 2 (#161). The exact commands, the host's names, the rail's
lists and the snapshot's name are in the maintainer's private gate packet; this SPEC names each step
only.

| step | what the owner approves | evidence recorded (privately) | rollback |
|---|---|---|---|
| E1 | the inventory's first run, read-only | the health-check list read before; the inventory file; the commands it ran | none needed: nothing changed |
| E2 | the boot-disk snapshot | the snapshot's name, size and ready state | delete the snapshot once the owner releases it |
| E3 | the deletion list, item by item | the list's digest, and the owner's approval of each item id | none needed: nothing changed |
| E4 | the apply run | the apply log, the health-check list read after, and the free space before and after | restore the item from the snapshot (attach it read-only, copy the item back) |

## 5. File manifest

| file | context | change |
|---|---|---|
| `deploy/host-scrub/inventory.py` | deploy | added: the read-only inventory |
| `deploy/host-scrub/plan.py` | deploy | added: the deletion list, with digests |
| `deploy/host-scrub/apply.py` | deploy | added: the approval-gated delete |
| `deploy/host-scrub/rules.example.json` | deploy | added: neutral example rules and protected paths |
| `docs/runbooks/host-scrub.md` | docs | added: the runbook, inventory to apply, and the rollback |
| `scripts/tests/test_host_scrub.py` | repo | added: A1 to A8 |
| `docs/specs/SPEC-060-host-inventory-backup-and-the-scrub-list.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-060-the-host-scrub-is-a-runbook-and-approval-gated-tools-behind-a-disk-snapshot.md` | docs | changed: status accepted |
| `docs/red-first/SPEC-060.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 6. What this does NOT do

- It deletes nothing by itself: each deletion is an item the owner approved at gate 2 (#161).
- It performs no step of the cutover (#164).
- It resizes nothing; a resize is the owner's decision at gate 2 (#161).
- It copies nothing to a bucket (#166).
- It installs no DeckStreak unit (#42).

## 7. Risks

- **An item changes between the list and the apply.** The digest is computed again at apply time,
  and one changed item refuses the whole run (A5).
- **A deletion breaks another service.** The protected-path list refuses that service's paths
  whatever the approval says (A7), the health-check list is read after each apply (R9), and the
  snapshot restores any item.
- **The inventory adds work to a small host.** Its commands run niced and idle-class, and the
  runbook runs it
  off the predecessor's schedule as SPEC-027 R2 defines it, its sync minutes included, off every
  slot of the reserved-slot list the private rail provides (SPEC-053 R2), and off DeckStreak's own
  job slots.
- **The snapshot costs money while it is kept.** Its size is about the disk's used blocks; the
  runbook keeps it until the owner releases it after W2's first week, and the gate packet names the
  cost.
- **The list reads private data to hash it.** Content is read only to compute a digest, which
  reveals nothing, and the list itself is private (R8).
