# SPEC-060: the host is inventoried and snapshotted before anything changes, and nothing on it is deleted without the owner's approval of that exact item

- **Wave:** W2. **Issue:** #239 (epic #3). **Context(s):** `deploy` (`deploy/host-scrub/`), `repo`
  (the runbook and the tests).
- **Decided by:** ADR-010 (every first-time change on the host is an owner gate), ADR-011 (side by
  side until the owner's go at cutover), and this SPEC's ADR-060 (a runbook and three tools, the
  approval bound to each item's digest, and a disk snapshot as the backup).
- **Waits for:** owner gate 2 (#161) for the inventory's first run, the snapshot and every deletion.
  Nothing here needs a device key (ADR-054).
- **Status:** judged: delivered with its tests, `docs/red-first/SPEC-060.md`, the runbook and the
  tools' schematic. The delivery made R1, R4, R6, R7, R9 and the manifest exact where the code
  decided them, and its review's fix rounds added A9 to A12 and the hand-proved rows (§8).

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
    (`systemctl list-unit-files`, `systemctl list-units --all`), and each loaded service's,
    socket's and timer's settings that say what it runs, from where, as whom, what it may write and
    when it fires (`systemctl show`, never its environment); and the ten largest memory users.
    It records every command it ran, with its exit status.
R2. The inventory runs only the read commands of its own allow list. It writes nothing but its
    output file, never restarts, stops or reloads a unit, and never calls a package manager with a
    changing verb. Each command runs under `nice` and `ionice -c3`.
R3. The roots, the retention of each backup family, the protected paths and the package list are
    private configuration. `deploy/host-scrub/rules.example.json` holds neutral example values
    only. A rule selects a candidate by class (a backup older than its family's retention, counted
    in copies or in days; a virtual environment or worktree that no loaded unit's command names; a
    package the owner lists; a loose file that matches a pattern) and states its reason.
R4. `deploy/host-scrub/plan.py INVENTORY RULES --out FILE` writes a deletion list: one item per
    path (or per listed package), with an id, its class, the rule and reason that selected it, its
    bytes, and a digest. A file's digest is SHA-256 over its path, size, modification time, mode and
    content; a directory's is SHA-256 over the sorted list of those fields for every entry under it.
    The list carries its own digest and the total bytes it would reclaim, where a file whose other
    links survive reclaims nothing. The plan deletes nothing and writes only its output.
R5. The backup before any deletion is one snapshot of the host's boot disk, taken from the
    maintainer's machine after the inventory and before the list is approved. The approval record
    names the snapshot and the instant it was taken. No per-item copy is made on the host's own disk.
R6. `deploy/host-scrub/apply.py LIST APPROVAL --rules RULES --log FILE` runs dry by default and
    deletes only with `--apply`. It deletes an item only when all of these hold, and otherwise
    refuses the whole run before its first deletion, naming the item and the reason:
    - the approval record carries the list's digest, names the item's id, the approver and the date;
    - the approval names a snapshot taken after the inventory;
    - the item's digest, computed again now, equals the listed one.
R7. `apply.py` never deletes a path outside the approved list (a package's own removal scripts, which
    the package manager runs when it removes an approved package, are outside that promise), never
    follows a symbolic link out of an item, refuses an item that is a mount point, holds one, or lies
    inside a mount whose root is not `/` or inside a file system mounted whole at two points, and
    refuses an item under any
    protected path, or holding one, whatever the approval says. The protected paths come from the private rail's protected-path list, which holds every
    path of the host's other services and their data; the example rules protect `/etc`, `/usr`,
    `/boot`, the credential socket's directory and every DeckStreak release directory, and the tests
    prove the refusal on a synthetic list.
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
| A3 | the plan lists only the items its rules select, each with its reason and digest, never a path it does not read canonically, and leaves the synthetic tree byte for byte as it was | `test_host_scrub.py` |
| A4 | apply with no approval, an approval that does not carry the list's digest, or a list that holds a key twice, deletes nothing and names the reason | `test_host_scrub.py` |
| A5 | apply deletes exactly the approved items when every digest matches, and deletes nothing when one approved item changed between the list and the apply's checks: a file changed at the same size and modification time, or an entry added inside an approved directory; an item that changes after the apply's checks, at its own entry or anywhere below it, is not deleted, and the run stops there, exit 3, with earlier deletions kept | `test_host_scrub.py` |
| A6 | apply refuses an approval that names no snapshot, a snapshot taken at or before the inventory's instant, compared as instants whatever offset each is written in, or one dated later than the apply's own clock | `test_host_scrub.py` |
| A7 | apply refuses an approved item under or holding a protected path however the item's path is written or reached, and refuses an item inside a bind mount of any directory, since a directory bind-mounted elsewhere is reached at a path the protected list does not name; it never follows a symbolic link out of an item, including an item that is itself a link: the link goes, the target stays | `test_host_scrub.py` |
| A8 | no host-scrub file names a private value, and a planted one is refused by the public scrub | `test_host_scrub.py`; `scripts/public-scrub.py` |
| A9 | apply runs as health checks only the read commands of the inventory's allow list, from the rules the inventory read: a changing command given as a health check, or other rules, are refused before any command runs, with 0 package-tool calls | `test_host_scrub.py` |
| A10 | each tool parses and binds a file from one read, and opens it once: the rules' digest the inventory records, the plan checks and the apply checks is taken over the rules each acts on; the list names its inventory by the bytes the plan parsed; and the apply deletes only items of the list whose digest the approval carries, so a file that serves other bytes to a second read is refused or acted on exactly as its digest says, and nothing the list's rules protect or the approval does not name is deleted | `test_host_scrub.py` |
| A11 | the inventory refuses to be written, and the apply refuses to delete, while the host clock does not read synchronised or when the inventory did not record that it did; and an item any of whose entries lies on another device than its own is neither digested, listed nor removed | `test_host_scrub.py` |
| A12 | an item that is a mount point, or holds one (any mount point strictly under it), or lies inside a mount whose root is not `/` or inside a file system mounted whole at two points, is not listed by the plan and is refused by the apply, whether it is a directory or a file, and the mount point is read from the kernel's mount table with every octal escape decoded; a mount table that cannot be read refuses every path item, in the plan and in the apply; an item with no mount point at or under it is listed and passes | `test_host_scrub.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_the_inventory_runs_only_its_read_only_allow_list
A2: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_the_inventory_reports_space_backups_venvs_and_worktrees
A3: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_the_plan_lists_rule_selected_items_and_deletes_nothing
A4: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_without_a_matching_approval_deletes_nothing
A5: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_deletes_exactly_the_approved_items_or_nothing
A6: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_refuses_an_approval_without_a_later_snapshot
A7: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_refuses_protected_paths_and_symbolic_links_out
A8: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_no_host_scrub_file_names_a_private_value
A9: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_apply_runs_only_read_health_checks_from_the_listed_rules
A10: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_each_tool_binds_the_bytes_it_parsed -k test_each_tool_opens_each_file_it_binds_once
A11: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_the_tools_refuse_a_clock_that_is_not_synchronised -k test_an_item_holding_another_device_is_never_digested_or_removed
A12: python3 -m unittest discover -s scripts/tests -p test_host_scrub.py -k test_an_item_that_is_or_holds_a_mount_point_is_refused
```

A1 runs the inventory with stub commands first on its `PATH`, each recording its argument vector
into a `TemporaryDirectory`. A2 to A7 and A9 to A12 build a synthetic host tree in a
`TemporaryDirectory` at run time, and A7 a synthetic protected-path list; no fixture holds a real
path, size or name. A5 and A7 change the synthetic tree while the apply reads a health check's
address, which the test serves on the loopback, and A4, A6, A7, A9 and A10 write lists the plan
did not make, each with its own digest taken again, so the apply's own checks are what refuse them.
A8 writes its planted value at run time, as SPEC-032's A6 does, so no private literal is ever
committed. A10 runs each tool through a reader that serves a file's bytes differently on a second
open, in either order, so a tool that parsed one read and bound another would show it. A11 fakes the
host's time-sync reading through the stub on the tools' `PATH`, changing it between the inventory
and the apply, and fakes a file system mounted inside an item at the walk's seam, since the box that
runs the tests refuses unprivileged mounts. A12 fakes the kernel's mount table through the reader
the tools read it by (`read_mountinfo`): an item that is a mount point, an item holding one, an
escaped paths (two escapes in one path), a file mounted over a file, an item inside a bind mount, an item inside a
file system mounted whole at two points, a mount table that cannot be read, a mount point that only shares an item's name as a prefix, and a
control with none.

## 4. The owner's gate and the evidence it records

Every step on the host waits for gate 2 (#161). The exact commands, the host's names, the rail's
lists and the snapshot's name are in the maintainer's private gate packet; this SPEC names each step
only.

| step | what the owner approves | evidence recorded (privately) | rollback |
|---|---|---|---|
| E1 | the inventory's first run, read-only | the health-check list read before; the inventory file; the commands it ran | none needed: nothing changed |
| E2 | the deletion list, item by item | the list's digest and the skipped candidates with their reasons | none needed: nothing changed |
| E3 | the boot-disk snapshot, then the owner's approval | the snapshot's name, size and ready state, and the owner's approval of each item id | delete the snapshot once the owner releases it |
| E4 | the apply run | the apply log, the health-check list read after, and the free space before and after | restore the item from the snapshot (attach it read-only, copy the item back) |

## 5. File manifest

| file | context | change |
|---|---|---|
| `deploy/host-scrub/inventory.py` | deploy | added: the read-only inventory |
| `deploy/host-scrub/plan.py` | deploy | added: the deletion list, with digests |
| `deploy/host-scrub/apply.py` | deploy | added: the approval-gated delete |
| `deploy/host-scrub/rules.example.json` | deploy | added: neutral example rules and protected paths |
| `docs/runbooks/host-scrub.md` | docs | added: the runbook, inventory to apply, and the rollback |
| `docs/schematics/host-scrub.md` | docs | added at delivery: the tools' data flow and the apply's refusals (§8) |
| `scripts/tests/test_host_scrub.py` | repo | added: A1 to A12 |
| `scripts/mutation-rows.d/S06000-S06099.json` | repo | added in the fix rounds: the hand-proved rows of the checks that stand before a deletion (§8) |
| `docs/specs/SPEC-060-host-inventory-backup-and-the-scrub-list.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-060-the-host-scrub-is-a-runbook-and-approval-gated-tools-behind-a-disk-snapshot.md` | docs | changed: accepted, with the decisions made at delivery |
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
  and one changed item refuses the run before its first deletion (A5).
- **An item, or a directory above it, changes between the apply's checks and its deletion.** The
  health checks are read between the two, so the window holds every health read. Each deletion
  reads its item again immediately before it deletes, through directories opened one at a time
  without following a link, and deletes it only while its device, inode and modification time are
  the ones its checks read and its digest, measured again over the item and everything below it,
  is the approved one; otherwise the apply stops there, and the log names what went (A5, A7). What
  remains is the interval between that last measurement and the removal itself, which no check of
  a path can close without holding the tree still.
- **The clock the approval's instants are ordered on.** The snapshot's instant is compared with the
  inventory's and with the apply's own clock, so a host clock that is not synchronised can order
  them wrongly. The inventory records whether the time-sync reading says synchronised and refuses to
  be written when it does not, and the apply reads it again and refuses before any deletion when it
  does not read synchronised now or when the inventory did not record it (A11). A host whose time
  sync is down therefore cannot be scrubbed until it reads synchronised. The reading bounds the
  clock's error by the kernel's own 16 s and not to zero, and no check here narrows it further.
- **A file system mounted inside an item, or over it.** A mount inside a directory item would be
  walked, digested and removed with the item. `measure` refuses an item any of whose entries lies
  on another device than its top entry, which separates a file system of its own. A bind mount
  shares its device with the tree around it, and a file mounted over a file leaves a directory-only
  device check nothing to compare, so the plan and the apply also read the kernel's mount table
  (`/proc/self/mountinfo`, the mount point field, its octal escapes decoded) and refuse an item that
  is a mount point or holds one (or lies inside a bind mount or a file system mounted whole at two points, below), whatever its device: the plan skips it with its reason and the
  apply refuses it before any deletion (A11, A12). With it, a mount at or under an item is refused
  whatever its device. An item inside a mount whose root is not `/` is refused too, since a bind
  mount of a directory shows that directory of the disk at that place and no list of protected
  paths is asked to know it (A7, A12); so is an item inside a file system that the table lists
  mounted whole (its root `/`) at two points, which is how a bind of a file system's root
  directory reads. That refusal has a cost: a host whose root file system is itself mounted from a
  sub-tree (its root field is not `/`) refuses every path item (an approved package is still
  removed) until the maintainer runs the scrub elsewhere, and a host that mounts one file system
  whole at two points refuses every path item under either point; both fail closed. A mount made after the apply's check is not seen, and belongs
  to the interval above.
- **A deletion breaks another service.** The protected-path list refuses that service's paths
  whatever the approval says (A7), the health-check list is read after each apply (R9), and the
  snapshot restores any item on the boot disk.
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

## 8. Amended in delivery

The code decided these statements of the planned SPEC. Each is corrected above, or stated here; the
reasons are these, and ADR-060 records each decision with what it was chosen against.

- **The manifest: the tools' schematic.** `docs/schematics/host-scrub.md` draws the files between
  the three tools and the order of the apply's refusals, since the order of work asks for a
  schematic before the code of a new component and data flow;
  `docs/schematics/first-deploy-and-gates.md` draws the scrub at the level of the owner's gates
  only.
- **R1: each loaded unit's settings.** R3's environment class selects what no loaded unit's command
  names, so the inventory reads each loaded service's, socket's and timer's commands and working
  directory, with the settings ADR-065's other-writer list is built from: its user and group, the
  paths it may write, and its timers' calendars. It never reads a unit's environment, which can
  carry a secret. The inventory is invoked as `inventory.py RULES --out FILE`; it also records each
  backup family's copies and stale copies and each loose rule's matches (A2), and it counts a
  virtual environment or worktree as one entry with its own totals rather than file by file.
- **R3: one rules file.** The private rules file holds the roots (a root may be `sizes_only`), the
  rules by class (`backup`, `environment`, `loose`, `package`), the protected paths fed by the
  private rail's list, and the health checks fed by its list, in the shape `rules.example.json`
  gives. The example protects R7's paths and DeckStreak's own state directory.
- **R4: a package is one item, and the plan runs where the files are.** A package is not a path, so
  a listed package's item names it (`name:architecture`), and its digest is over its name,
  architecture, version and state. The digest reads each candidate's content, so the plan runs on
  the host. It refuses rules other than the ones the inventory read, and it names each candidate it
  left out, with why.
- **R6: the refusals the code added.** Beyond R6's three, the apply refuses a list whose own digest
  no longer matches its content (an edit would keep the digest field), an approval that names an id
  the list does not hold, a snapshot instant without an offset (it cannot be ordered), an item
  reached through a symbolic link (R7), and a package that `dpkg --dry-run --remove` would not
  remove alone. It takes the protected paths and the health checks from the rules, and its log from
  `--log`; it exits 1 on a refusal, 3 when a deletion failed part way and 4 when a health check is
  red after the apply.
- **R7: an item holding a protected path.** Deleting a directory deletes what it holds, so an item
  that holds a protected path is refused as an item under one is. A protected entry is a path, never
  a pattern: compared as a path, a pattern would protect nothing, so the tools refuse rules that
  hold one.
- **R8: no output inside this repository.** Each tool refuses an output path inside the repository
  it was run from, before it reads anything.
- **R9: the health checks' form.** A check is a read command of the inventory's allow list, or one
  GET of an http(s) address, green on a 2xx answer. The inventory reads them before its first read;
  the apply reads them before its first deletion and after its last, and its log's `turned` names
  each check that was green before and is red after.
- **R10: a service's own rotation.** A backup rule states the copies its service's rotation keeps as
  `rotation_keeps`, and none of that many newest copies is ever listed, whatever the retention says.
- **§3: two tests beyond the criteria.** `test_no_tool_writes_its_output_inside_the_repository` pins
  R8's refusal, and `test_a_health_check_red_after_an_apply_stops_the_scrub` pins R9's stop.

The delivery's review (#289) asked for a fix round, which amended these statements too:

- **R6, R7: a path read canonically, and nothing deleted that was not checked.** The tools refuse
  a path that is not absolute and canonical: in the rules by the path's key, never its value; in
  the plan, which leaves such a candidate out and says why; and in the apply, by the item's id.
  Each deletion reads its item again, as §7 says. A5, A6 and A7 name the cases the review's plants
  separated, and the red-first record's fix round lists them.
- **R6, R9: the apply reads the rules the inventory read.** It refuses rules whose digest is not the
  one the list names, as the plan does, where that digest is taken over the very bytes it parsed
  the rules from, in one read (A10), and it runs as health checks only the read commands of the
  inventory's allow list; the changing commands it admits run for a listed package's item alone
  (A9, which is new).
- **R5, R6: a snapshot is not dated after the apply's own clock**, and a list, an approval or rules
  that hold a JSON key twice are refused (A4, A6).
- **R2, R9: an `is-active` read names its units after `--`**, and a unit's name begins with a letter
  or a digit.
- **R6: the Python the apply needs.** A directory item is removed through its parent's descriptor,
  which Python's `shutil.rmtree` takes from 3.11, so the apply refuses a directory item under an
  older Python; the runbook names 3.11 as the tools' floor.
- **§5: the rows.** No mutation tool generates mutants of the Python under `deploy/`, so
  `scripts/mutation-rows.d/S06000-S06099.json` holds hand-proved rows (SPEC-039) for the checks
  that stand before a deletion, each proved with `scripts/mutation_rows.py prove`.

Its second fix round amended these statements too:

- **R4, R6: one read binds what is parsed.** A tool that both parses a file and binds it by its
  digest reads the file once, and takes the digest over the bytes it parsed: the inventory records
  the digest of the rules it read its health checks and roots from, the plan checks that digest
  against the rules it lists from and names its inventory by the bytes it parsed, and the apply
  checks the list's rules digest against the rules it acts on. Read twice, a file that changed
  between the reads is parsed as one content and bound as another, so the apply could act on rules
  the list does not name while its check passed. A10 is new, and a test beyond the criteria,
  `test_each_tool_opens_each_file_it_binds_once`, pins one open of each such file per run.

Its third fix round amended these statements too:

- **R6, A5: a deletion reads the item's digest again.** Each deletion measures the item again over
  its whole tree, right after its entry check, and stops when the digest is not the approved one, so
  a change below a directory item's top entry, or a rewrite in place with its time put back, is
  caught where the entry check alone would not. A5 grew four cases; the interval between that
  measurement and the removal is named in section 7.
- **R6, A10: the list and the approval are bound too.** The apply deletes only items of the list it
  bound whose digest the approval carries, the plan lists from the inventory it named, and the
  apply's protected paths come from the rules it bound; A10's text and fence say so, and the
  open-count test adds the apply's list and approval.
- **R6, R7: A11.** The clock reading and the crossed device above are new; the clock reading is one
  more read command of the allow list, `timedatectl show -p NTPSynchronized --value`.

Its fourth fix round amended these statements too:

- **R7, A12: a mount point is refused whatever its device.** The plan and the apply read the
  kernel's mount table through one reader and refuse an item that is a mount point or holds one, so
  a bind mount, which shares its device, and a file mounted over a file are refused where the device
  check alone would list and delete them. A12 is new.

Its fifth fix round amended these statements too:

- **A5: the run stops at a change after the checks.** A5 said the apply deletes nothing when an item
  changed after the list; an item that changes after the apply's checks is not deleted, and the run
  stops there with earlier deletions kept, so the criterion now says where the promise ends.
- **R7, A7, A12: an item inside a bind mount is refused.** A mount whose root is not `/` shows another
  directory of the disk at its mount point, so an item under such a mount is refused by the plan and
  the apply, whatever the protected list says. A host whose root is itself mounted from a sub-tree
  refuses every path item (an approved package is still removed). A12 also holds an unreadable mount table, in the plan and in the apply, and
  an escape in each of two places of one path.
- **R7: a package's removal scripts.** The promise never to delete a path outside the approved list
  does not cover what a package's own removal scripts do when the package manager runs them.
- **The snapshot restores an item on the boot disk.** A path on another disk is not in the snapshot.

Its sixth fix round amended these statements too:

- **R7, A7, A12: a file system mounted whole at two points.** A bind of a file system's root
  directory, of `/` or of a protected directory that is itself a mount point, reads `/` in the
  mount table's root field, so the root-field clause of the fifth round did not see it, and the
  device check compares an item with a device the whole item lies on. The plan and the apply now
  also refuse an item under any mount point of a file system the table lists mounted whole at two
  points. A12 fakes each mount on its own device, since one device for every row models a bind of
  `/` as the control. A host that mounts one file system whole at two points refuses every path
  item under either point.
- **R7, A12: what "a bind mount" names.** A mount whose root is not `/` is a mount of a
  sub-tree, which a bind of a directory is; the criteria now say what the code checks.
