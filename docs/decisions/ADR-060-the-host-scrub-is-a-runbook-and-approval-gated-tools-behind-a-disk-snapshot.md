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

### Consequences

- Good, because the owner approves bytes, not descriptions, and a changed host is caught before the
  first deletion.
- Good, because the snapshot restores any item, or the whole disk, without room on the host.
- Bad, because the snapshot is billed while it is kept; the runbook keeps it until the owner releases
  it after W2's first week.
- Bad, because the digest reads each candidate's content once; the tools run niced,
  off the predecessor's schedule as SPEC-027 R2 defines it, its sync minutes included, off every
  reserved slot, and off DeckStreak's own job slots.

### Confirmation

SPEC-060's acceptance tests (A1 to A8); the gate-2 evidence E1 to E4, recorded privately.

## What would make this wrong

- The owner wants items copied off the host individually (for example to keep one backup family
  beyond the snapshot's life). A bucket-backed copy step would then follow gate 7.
- The scrub becomes a recurring job rather than a W2 and cutover chore. A pack with rows would then
  earn its method.

## More Information

SPEC-060; ADR-010; ADR-011; ADR-069; the owner's scrub decision (#239).
