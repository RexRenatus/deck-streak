---
status: proposed
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# DeckStreak's database is backed up by DeckStreak's own units and configuration, on a Litestream binary the rail provides, and the collection copy never is

## Context and Problem Statement

ADR-010 decided "a Litestream 0.5 stanza for DeckStreak's database", "a daily backup and a weekly
restore drill that cover it", and an offsite bucket chosen with the owner. It did not decide who
owns each piece, where the Litestream binary comes from, or what the backups leave out. DeckStreak's
state directory is private to its service user (`UMask=0077`, SPEC-032 R2), and its release carries
only what the release workflow built from this repository (ADR-062). How is DeckStreak's database
replicated continuously, copied daily and restored weekly, and what is left out?

## Decision Drivers

- Only a process running as DeckStreak's service user can read its state directory.
- DeckStreak's backups depend on nothing DeckStreak does not own, so they hold from the first deploy
  through cutover and after (ADR-011, #164).
- The declared backups window is `P3D` (SPEC-021), and the copies kept on the host are budgeted
  (SPEC-064 R9).
- The release carries only DeckStreak's own build, whose provenance the deploy verifies (ADR-062).

## Considered Options (the alternatives it was chosen against)

- DeckStreak's own Litestream unit and configuration, its own daily backup and weekly drill, a Litestream binary of 0.5.2 or later that the private rail provides, and no copy of the collection: chosen, because each part runs as DeckStreak's user against its own state, nothing in it belongs to anyone else but the binary, whose version the rehearsal reads, and the release stays DeckStreak's own build.
- One Litestream configuration for several databases: rejected because DeckStreak's interval, retention and window (SPEC-021) must hold for its replica alone, whoever owns such a file could rewrite them, and its process would have to read DeckStreak's private state directory.
- Backing up through a script or unit that DeckStreak does not own: rejected because it would run as another user, which cannot read DeckStreak's state directory, and DeckStreak's backups would change or stop whenever their owner changed them.
- A Litestream binary inside DeckStreak's release: rejected because the release carries only what the release workflow builds from this repository, whose provenance the deploy verifies (ADR-062), and a third-party binary inside it would need provenance of its own.
- Backing up the collection copy as well: rejected because it is a copy of the owner's collection that the next sync downloads again (ADR-037), and a daily copy of it would be the largest file DeckStreak keeps.
- Daily copies only, with no continuous replication: rejected because ADR-010 and the house stack chose Litestream, and a whole day of lost streaks, grants and reading ticks is more than the owner should lose to one failed disk.

## Decision Outcome

Chosen option.
- **Replication.** `deck-streak-litestream.service` runs `litestream replicate -config` with
  DeckStreak's own configuration (SPEC-021's template), as the service user, on the binary the rail
  provides, against a bucket the owner approves at gate 7 (#166); the bucket in the replica's
  `gs://` URL is a `${VAR}` Litestream expands from the unit's environment, so no bucket is named in
  the repository and the URL still reads as off the host.
- **The binary.** The private rail provides Litestream at version 0.5.2 or later; the gate-2
  rehearsal reads and records the version, and the rail keeps such a binary in place for as long as
  DeckStreak's unit runs.
- **The daily copy.** `deck-streak-backup.service` takes an online copy with SQLite's backup API,
  checks its integrity, and keeps three.
- **The weekly drill.** `deck-streak-restore-drill.service` restores the replica with
  `litestream restore -o` to a private temporary path, checks it and the newest daily copy, and pages
  through `OnFailure=` on any failure.
- **The window.** Litestream's 24-hour interval plus 48-hour retention, and three daily copies, keep
  an erased row at most three days, inside the declared `P3D`; the bucket's soft delete is off so
  the window holds offsite too.
- **Left out.** The collection copy, the releases (downloadable again) and every credential.
- These units are ADR-010's backups; the rest of ADR-010 stands.

### Consequences

- Good, because the backups belong to DeckStreak from the first day and need no change at cutover.
- Good, because nothing that another service owns is changed.
- Bad, because DeckStreak's Litestream process stays resident; it has its own ceiling in the host
  budget.
- Bad, because the binary is the rail's to provide, not the release's; the rehearsal reads its
  version, and the weekly drill proves the replica with whichever binary runs.

### Confirmation

SPEC-064's A1 to A7; the box-run packs' backup and privacy rows (ADR-069);
the first restore drill on the host at gate 2, recorded privately.

## What would make this wrong

- A later Litestream release changes its configuration or its restore in a way DeckStreak's unit
  cannot follow. DeckStreak would then pin a version of its own, recorded in a new ADR.
- The owner wants a longer history than `P3D`. The declared window, the policy and the retention
  change together, by an amendment of SPEC-021.

## More Information

ADR-008; ADR-010; ADR-011; ADR-037; ADR-062; SPEC-021; SPEC-027; SPEC-064; the durable-services
pack's backup stage; the Litestream documentation on `-config`, environment expansion,
`restore -o`, snapshots and validation, read through Context7.
