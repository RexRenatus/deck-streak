---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The import rehearses on a copy, applies once over a restored backup, and rolls back by rename

## Context and Problem Statement

ADR-008 asks for a dry run, a verified backup first, and "a rollback that discards the target while
v9 keeps serving"; ADR-011 runs the import last, after the checklist's moves, from a verified
backup. By the time of the one apply the predecessor has stopped (SPEC-144), and DeckStreak's
database already holds its own side-by-side records. How does the import rehearse, where does its
backup come from, and how does it roll back?

## Decision Drivers

- The import reads a copy, never the live source, verified by digest (#61).
- The rehearsal writes nothing live, so it can run side by side, as often as the owner likes.
- The backup must be one the owner has proved restorable, not one merely written.
- A rollback must not tear a database in WAL mode, and must lose nothing.

## Considered Options (the alternatives it was chosen against)

- A rehearsal on a copy, one apply over a restored backup, a rollback by rename: chosen, because
  the rehearsal writes only a copy it discards, as often as the owner likes before the go, and the
  apply's backup is the restore path the owner relies on (ADR-010), proved before it is needed.
  The dry run reads a `VACUUM INTO` copy of DeckStreak's database, and the apply's backup is
  restored from the Litestream replica and verified.
- A backup the tool takes itself: rejected because a copy nobody restored is unproved, and a copy
  of a live WAL database under its writers is the torn-copy class the data-migration rules refuse.
- The import before the moves, before the predecessor stops: rejected because the copy is stale by
  the time DeckStreak takes over, and a second import would be needed after all.
- The dry run as a rolled-back transaction on the live database: rejected because it holds the live
  write lock for the whole run, stalling DeckStreak's writers side by side.
- Restoring by copying the backup's bytes over the live file: rejected because a copy over an open
  WAL database can tear; a rename in one directory is atomic, and the failed file is kept.

## Decision Outcome

Chosen option: rehearse on a copy, apply once over a restored backup, roll back by rename. ADR-008's
"discards the target while v9 keeps serving" is the rehearsal: the dry run discards its copy while
the predecessor keeps serving; the apply comes after the go and the retirement (#164). SPEC-142 R5
to R13 hold it.

### Consequences

- Good, because the owner can rehearse against every fresh copy before the go, and the report of
  the final rehearsal predicts the apply's.
- Good, because the rollback copies no file and keeps the failed one for the owner.
- Bad, because the apply needs DeckStreak's writers stopped for its span, a step of the runbook.

### Confirmation

SPEC-142's dry-run, idempotency, backup-refusal and rollback criteria, and the runbook test that
names no `cp`, `rsync` or `scp` of a database file.

## More Information

#61, #164, ADR-008, ADR-010, ADR-011, SPEC-064, SPEC-142, SPEC-144.
