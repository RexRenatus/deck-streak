---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The Other Hand attributes a skip day's rows by its card snapshot and window

## Context and Problem Statement

The Other Hand (#151) counts manual and reschedule review-log rows (types 4 and 5) and says whose
they are. Under ADR-089 the skip day writes its reschedule back to Anki: SPEC-083 R18 has the
engine's Set Due Date write one type-4 row with ease 0 for each card it moves, and those rows stay.
The owner's own Set Due Date writes the same row shape. How does the census tell the service's
rows from the owner's?

The predecessor answers through the skip ledger (`provenance.py` `build_skip_evidence`, predecessor
`27ee2bc`): a row is a skip day's when its card is in that skip's snapshot and its id falls between
that skip's creation and the next skip's, and an undone skip is flagged.

## Decision Drivers

- SPEC-083 R1 and R22 give ingest the `skip_days` record and its card snapshot (`skip_card_snapshot`); insights depends on the kernel and ingest (ADR-002).
- ADR-089 permits only the reschedule and its exact inverse into the owner's collection.
- The census exists to report the owner's behaviour on every day, skip days included.

## Considered Options (the alternatives it was chosen against)

- A row is the skip day's when its card is in that skip's snapshot and its id falls between that skip's creation and the next skip's; an undone skip's rows are named undone — chosen: it reuses records that already exist and it separates the two authors of one row shape.
- Every ease-0 type-4 row is DeckStreak's — rejected because the owner's own Set Due Date writes the same row shape, so it would attribute the owner's edits to the service.
- A marker written into the collection with each row — rejected because ADR-089 permits only the reschedule and its exact inverse, and a marker is a third kind of write to the owner's collection.
- No attribution, the census counting every type-4 row as the owner's — rejected because it misreports the owner's behaviour on every skip day, which is what the Other Hand exists to measure.

## Decision Outcome

Chosen option: "snapshot and window", because it needs no write beyond ADR-089's and no guess about
who authored a row.

- A manual or reschedule row belongs to a skip day when its card is in that skip's snapshot and its
  id lies between that skip's creation and the next skip's creation (the last skip's window is open
  at the end).
- The rows of an undone skip are counted as that skip's and named undone.
- A manual row whose card is in no skip's snapshot is the owner's edits or an earlier tool's; a row whose card is in a snapshot but in no one window of a skip holding it is counted apart as unresolved, never as the owner's or the service's (`compute_provenance`'s misattributed count).
- The report states the rows it attributes to DeckStreak's skip days and never claims a row outside
  a window.

### Consequences

- Good, because the census neither blames the owner for the service's rows nor the service for the
  owner's.
- Good, because it adds no write and no column.
- Bad, because a row the owner writes in a skip's window for a card in its snapshot is attributed
  to the skip day; the window is as narrow as the ledger allows.

### Confirmation

SPEC-097 R8 and A22: the census over a synthetic ledger of two skips, one undone, equals
`goldens/provenance_report_skips.json`.

## What would make this wrong

- If the engine ever tags the rows it writes, the tag replaces the window and this record is
  amended.

## More Information

SPEC-083 R1, R18 and R22; ADR-089; ADR-002; SPEC-097, which builds the census; the predecessor function
`build_skip_evidence`.
