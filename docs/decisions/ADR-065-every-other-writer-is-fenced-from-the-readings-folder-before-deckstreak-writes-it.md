---
status: proposed
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Every other writer is fenced from the readings folder by a read-only mount, and each fence proved, before DeckStreak writes it

## Context and Problem Statement

Each vault contract has one writer at any moment (ADR-011). ADR-053 keeps DeckStreak's vault archive
switch off until the owner's go makes DeckStreak the readings folder's one writer, and SPEC-053 R8
switches it on only then; neither says how that is made true. The folder is created for DeckStreak
with the group write the vault's private file contract requires (SPEC-065 R1), so from that moment
any unit on the host whose own settings let it write the vault could write the folder too, whatever
its code intends. DeckStreak changes no other service's code, and the contract fixes the folder's
owner, group and mode. How is every other writer kept out of the readings folder, reliably, before
DeckStreak writes it?

## Decision Drivers

- One writer, enforced rather than hoped for: nothing another unit decides may reach the folder.
- DeckStreak changes no other service's code or contract, and its changes on the host are the
  fewest that hold, each with a one-step rollback (gate 2).
- Every member of the vault's group keeps reading the notes, as the contract requires.
- A fence counts only once a refused write proves it, never because a setting says so.

## Considered Options (the alternatives it was chosen against)

- A drop-in on each unit of the rail's other-writer list that makes the readings folder read-only in its mount namespace, applied with one restart and proved by a refused write: chosen, because the kernel then refuses every write such a unit attempts there, whatever its code decides; the contract stays as it is; and undoing a fence takes one file and one restart.
- A new group for the readings folder, without the other writers in it: rejected because it breaks the owner, group and mode the vault's contract fixes, and every reader in the vault's group would have to join the new group.
- Relying on each other writer's own switch or configuration to keep it out: rejected because a switch can be turned back on and a configuration can change on its owner's schedule, and nothing would then stop a second writer.
- Changing another service's code: rejected because it is not DeckStreak's to change.
- Keeping the archive switch off until cutover: rejected because the owner's reading decisions put the vault copy in the first live night (#45), and cutover is W8.
- A different folder for DeckStreak's notes: rejected because the vault's top-level folder map is closed, and the contract names this folder for the readings.

## Decision Outcome

Chosen option.
- **The list.** The private rail keeps the other-writer list: every unit on the host, other than
  DeckStreak's, whose settings let it write the vault, with each unit's scheduled slots. It is
  built from the units SPEC-060's inventory reads, and the owner approves it at gate 2 (#161);
  like every host value, it is private.
- **The fence.** In one step, off every reserved slot, off DeckStreak's own job slots,
  and off every scheduled slot of each unit on the other-writer list, which that list carries,
  the rail creates the readings folder and its archive (SPEC-065 R1), then at once installs, for
  each unit on the list, a drop-in that lists the folder in `ReadOnlyPaths=`, and restarts that
  unit. systemd applies a unit's mounts parent first, so the deeper read-only path is applied on top
  of any writable vault path the unit has; and a listed path must exist when the unit starts, which
  is why the folder comes first. Then `deploy/scripts/prove-read-only.sh` shows, for each unit, a
  write from inside its namespace, as its user, refused (SPEC-065 R2).
- **The order.** The switch goes on only after every fence's proof, the folder's modes and a probe
  file read by the vault's group are recorded (SPEC-065 R4).
- **ADR-053 met.** This is how the owner's go makes DeckStreak the readings folder's one writer
  (ADR-053, SPEC-053 R8); ADR-053 carries a note that cites it.
- **Rollback.** The fences stay until the owner decides otherwise (#164). Removing one is one file
  and one restart, after the archive switch goes off.

### Consequences

- Good, because the one-writer rule holds in the kernel from the first write, and the vault's
  contract is untouched.
- Good, because each change to another unit is one small file, reversible.
- Bad, because each fenced unit restarts once, briefly, off every reserved slot
  and off every scheduled slot of each unit on the other-writer list, which that list carries.
- Bad, because a writer missing from the list is not fenced; the list is built from every loaded
  unit's settings and approved by the owner, and the proofs cover only the units it names.

### Confirmation

SPEC-065's A4 and the gate-2 evidence E1 to E3; the health-check list read before and after each
restart, recorded privately.

## What would make this wrong

- A unit on the list reaches the vault by a path the fence's mount does not cover; the prover's
  refused write, required before the switch goes on, would show it.
- No other unit can write the vault by the first live night; the list is then empty, and so is the
  work of the fence.

## More Information

ADR-011; ADR-019; ADR-053; SPEC-042; SPEC-053; SPEC-060; SPEC-065; systemd.exec(5) on
`ReadOnlyPaths=` and `ReadWritePaths=`, and systemd's `namespace.c` on the order of a unit's mounts,
read through Context7; the vault's file contract (private).
