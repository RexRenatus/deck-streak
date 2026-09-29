---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The stats job fires after the day's one sync, reads its outcome, and never syncs

## Context and Problem Statement

#153 ports the predecessor's nightly stats file (`vault_bridge.py:build_vault_stats`) and places
its job at minute 5 after the rollover. ADR-037 gives DeckStreak one scheduled sync per study day,
at minute 7 after the rollover (SPEC-027 R5), and a job that needs the day's data reads the study
day's sync outcome and never runs a sync. When does the stats job fire, and what does it do when
the day's sync did not succeed?

## Decision Drivers

- The file reports the current study day's data, after the day's sync.
- One scheduled sync per study day (ADR-037).
- Each job on its own minute, off the predecessor's schedule (SPEC-027 R2) and every other slot.
- One writer of the file (ADR-011).

## Considered Options (the alternatives it was chosen against)

- A job at the rollover hour plus 12 minutes, after the day's sync: chosen, because it follows the
  sync, never runs one, and minute 12 is on no predecessor minute and no other slot. It takes the
  collection lock shared, reads the study day's sync outcome, and writes only when it succeeded.
- The issue's minute 5: rejected because it precedes the day's only scheduled sync, so every file
  would report the day before the sync, a day stale.
- A sync, then the write, inside the job: rejected because it adds a second scheduled sync a day,
  which ADR-037 forbids.
- A step of the sync job: rejected because it couples the sync's outcome to a vault write, and it
  would also run on the owner's sync triggers, rewriting the file mid-day.
- A write after every successful sync: rejected because the owner's triggers would rewrite the
  file at any hour, and no once-a-day claim, catch-up or missed record would hold.

## Decision Outcome

Chosen option: minute 12, after the sync, reading its outcome. A day whose sync did not succeed
writes nothing and keeps the file as it was; the sync's own failure has paged. The job is gated by
its checklist item (SPEC-143 R10) until the owner's go makes DeckStreak the file's writer. SPEC-146
R13 to R15 hold it.

### Consequences

- Good, because the file reports the day the sync just read, and the job stays a reader of sync.
- Bad, because a night whose sync did not succeed leaves the previous night's file in place; its
  `generated_utc` says how old it is.

### Confirmation

SPEC-146's job-table criterion (the slot after the sync, off every predecessor minute and every
other slot), its no-sync criterion, and its gate criterion.

## More Information

#153, #164, ADR-011, ADR-027, ADR-037, SPEC-027, SPEC-053, SPEC-143, SPEC-146.
