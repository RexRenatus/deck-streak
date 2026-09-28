---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The host scrub is a runbook and three approval-gated tools, and its backup is one disk snapshot

## Context and Problem Statement

W2 opens, by the owner's decision (#239), with a scrub of the host: an inventory, a backup of
everything, and then the removal of only what is obsolete, each deletion approved by the owner item
by item before it happens. The architect's dispatch left open where the scrub lives: a checklist in
the release-ops pack, or a pack of its own. How is the scrub run, how is the owner's approval tied
to what is deleted, and what is the backup?

## Decision Drivers

- No deletion without the owner's approval of that exact item (#239, gate 2).
- An approval given to one state of the host must not delete a different state.
- The backup must not need room on the host itself.
- The host's other services keep running, with their health read before and after each step
  (#239).
- Nothing private enters the repository (CHARTER 11): the inventory and the lists are private.

## Considered Options (the alternatives it was chosen against)

- A runbook and three tools, inventory, plan and apply, with each item's digest in the approval and one boot-disk snapshot as the backup: chosen, because the approval binds the exact bytes approved, a changed item refuses the whole run, and the snapshot covers the whole disk without using any of it.
- A checklist inside the release-ops pack: rejected because the packs are the maintainer's and judged in the box run (ADR-069), so DeckStreak cannot add a row to one, and one host's one-off chore does not belong in a portable pack.
- A pack of DeckStreak's own: rejected because the pack-wave method's research, coverage matrix and planted-defect fixtures buy nothing for three tools whose own tests are the checks, used in W2 and again at cutover.
- Deleting by hand from a written checklist: rejected because nothing then ties the owner's approval to what is deleted, and a file that changed after the list was read would be deleted anyway.
- Copying each item to the host's own disk first: rejected because each copy would sit on the same disk as its item, needing the room its deletion frees, and one failed disk would take both.
- Copying each item to a bucket first: rejected because it is a new bucket, an owner gate of its own (#166), copies only what the list names, and costs egress, where one snapshot covers the whole disk.

## Decision Outcome

Chosen option.
- **The tools.** `deploy/host-scrub/inventory.py` reads the host with an allow list of read
  commands only; `plan.py` turns the inventory and the private rules into a deletion list with a
  digest per item; `apply.py` deletes, all or nothing, only items whose approval carries the list's
  digest and the item's id, whose digest is unchanged, and which lie under no protected path.
  `docs/runbooks/host-scrub.md` is the order of work.
- **The backup.** One snapshot of the host's boot disk, taken from the maintainer's machine after the
  inventory and before the approval; the approval names it, and `apply.py` refuses an approval that
  names none, or one taken before the inventory.
- **Private by construction.** The rules, the inventory, the lists, the approvals and the logs live
  in the private rail's directory, and so do its protected-path and health-check lists; the
  repository holds the tools and neutral example rules.
- **Scope.** W2 deletes only the classes the owner named: backups older than their retention,
  obsolete worktrees and virtual environments, one-off files left loose, and packages the owner
  lists as unused.

### Decided at delivery (SPEC-060 §8)

The delivery decided what this record left open, each against its alternatives:

- **One private rules file.** It holds the roots, the rules by class, the protected paths and the
  health checks, which the private rail's lists feed, in the shape `rules.example.json` gives, and
  the apply reads the protected paths and health checks from it at run time. A protected entry is
  a path, which protects everything under it, and a pattern is refused. Chosen against a file per
  list, which each tool would be handed separately; against the list carrying its own protected
  paths, which an edited list could drop; and against matching protected patterns, where a pattern
  a tool read as a path would protect nothing and say so nowhere.
- **The plan runs where the files are.** The digest reads each candidate's content, so the plan runs
  on the host, niced, over the candidates only. Chosen against digesting on the maintainer's
  machine, which holds no content, and against content hashes in the inventory, which would read
  every file under every root to digest the few the rules select.
- **A package is removed by `dpkg --remove`, after `dpkg --dry-run --remove` passes.** dpkg refuses
  a removal another installed package depends on, so an approved package goes alone or not at all,
  and it keeps the package's configuration files. Chosen against `apt-get remove`, which would also
  remove every package that depends on it, and against a purge, which deletes configuration files
  under `/etc`, a protected path.
- **A health check is a read command of the allow list, or one GET of an http(s) address.** Chosen
  against a free shell command, which could change the host, and against `curl`, whose options can
  write files and send bodies.
- **The apply refuses more than the three checks above.** It also refuses an item that holds a
  protected path, an item reached through a symbolic link, a list whose own digest no longer
  matches its content, an approval naming an id the list does not hold, and a snapshot instant
  without an offset. Chosen against trusting a list's digest field, which an edit would keep, and
  against comparing a snapshot's instant that carries no offset, which cannot be ordered.
- **A virtual environment or worktree is one inventory entry, with its own totals.** Chosen against
  recording its files one by one, which would multiply the inventory by every environment's files
  without adding a candidate.
- **A service's own rotation is stated by its rule.** A backup rule's `rotation_keeps` names the
  copies the service keeps and makes again, and none of them is listed. Chosen against leaving that
  family out of the rules altogether, which would also hide the copies beyond the rotation.

### Decided in the fix round (SPEC-060 §8)

The delivery's review (#289) asked for a fix round, which decided these, each against its
alternatives:

- **The apply reads the rules the inventory read.** It refuses rules whose digest is not the one
  the list names, as the plan does, and it checks and runs its health checks through the
  inventory's read allow list alone; the changing commands it admits run only for a listed
  package's item. Chosen against rules read unbound at run time, because the owner approves a list
  made under one set of rules, and rules edited after that approval would change what the apply
  protects and what it runs while the list still matched; and against one allow list shared by the
  health checks and a package's removal, which would admit a changing command where a read belongs.
- **A path is refused unless it is absolute and canonical.** The rules name the refused path by its
  key, the apply by the item's id, and the plan leaves such a candidate out. Chosen against
  normalising a path before comparing it, since a spelling a tool normalises and the file the system
  reaches can differ, so a normalised comparison could protect one file and delete another.
- **Each deletion reads its item again, through directory descriptors.** Immediately before it
  deletes, the apply opens the directories above the item one at a time without following a link,
  and deletes through the last of them only while the item's device, inode and modification time are
  the ones its checks read; otherwise it stops. Chosen against deleting by path after checks made
  earlier, since the health read between them leaves a window in which a path can come to reach
  something else, and against checking again by path alone, which narrows that window without
  closing it.
- **A snapshot dated later than the apply's own clock is refused**, since it cannot have been taken
  yet. Chosen against trusting the approval's instant once it was ordered after the inventory's.
- **A JSON file that holds a key twice is refused.** Chosen against Python's reading, which keeps
  the last of the two, because another reader may keep the first, so the list the owner reads and
  the list the apply reads could differ while their digests agree.
- **An `is-active` read names its units after `--`, and a unit's name begins with a letter or a
  digit.** Chosen against admitting every name the unit pattern allowed, which left the read to
  depend on how the command parses its words.

### Consequences

- Good, because the owner approves bytes, not descriptions, and a changed host is caught before the
  first deletion.
- Good, because the snapshot restores any item, or the whole disk, without room on the host.
- Bad, because the snapshot is billed while it is kept; the runbook keeps it until the owner releases
  it after W2's first week.
- Bad, because a directory item needs Python 3.11 or later, whose `shutil.rmtree` takes a directory
  descriptor; the runbook names that floor.
- Bad, because the digest reads each candidate's content once; the tools run niced,
  off the predecessor's schedule as SPEC-027 R2 defines it, its sync minutes included, off every
  reserved slot, and off DeckStreak's own job slots.

### Confirmation

SPEC-060's acceptance tests (A1 to A9) and its hand-proved mutation rows
(`scripts/mutation-rows.d/S06000-S06099.json`); the gate-2 evidence E1 to E4, recorded privately.

## What would make this wrong

- The owner wants items copied off the host individually (for example to keep one backup family
  beyond the snapshot's life). A bucket-backed copy step would then follow gate 7.
- The scrub becomes a recurring job rather than a W2 and cutover chore. A pack with rows would then
  earn its method.

## More Information

SPEC-060; ADR-010; ADR-011; ADR-069; the owner's scrub decision (#239).
