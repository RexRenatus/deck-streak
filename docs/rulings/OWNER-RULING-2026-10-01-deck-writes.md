The owner amends CHARTER item 4 by ADR-301, in the owner's own words: "Sign, with this never-list (Recommended)" (owner, 2026-10-01): DeckStreak writes to the collection only through declared write classes, each with its own ADR in ADR-089's form.

# OWNER RULING 2026-10-01: declared write classes

## What was held

At `b1321773`, three documents limited every write to the collection to the skip day:

- `CHARTER.md:33-34`, item 4: "The skip day is the ONLY write back to Anki."
- `docs/decisions/ADR-089-the-skip-day-writes-its-reschedule-back-to-anki-and-every-other-path-never-uploads.md`,
  the owner's guardrails: (i) at lines 55-57, "The only writes ever made are the skip day's
  reschedule of that day's due review cards, and its exact inverse."; (ii) at lines 58-59,
  "Incremental sync only."; (iii) at lines 60-61, "The write runs only on the owner's explicit skip
  declaration, inside ADR-037's owner-trigger rule. No new scheduled syncs."; (iv) at lines 62-63,
  "A preview of the cards to be rescheduled is shown before the write, and their prior due dates
  are recorded so the skip can be undone."
- `docs/decisions/ADR-037-sync-once-a-study-day-plus-owner-triggers-and-never-upload.md`: (a) at
  line 14, "no upload path, proven against a fake sync server that records every request"; (b) at
  lines 15-17, "at most one sync per day, plus the owner's explicit triggers".

Each keeps that text, and each carries a dated note naming ADR-301.

## What replaces it

DeckStreak writes to the collection only through declared write classes, each with its own ADR in
ADR-089's form. The skip day is the first such class. No write class ever, at any rung and under
any approval: edits review history; forgets or resets a card; deletes a preset; forces a full sync;
mass-reschedules; sets a per-card due-date policy; changes a reviewed note's type; or deletes a
reviewed card or note. ADR-301 states what each entry protects, the backup each batch takes first,
the promotion ladder, the single writer and the objective.

## Why

It is the owner's decision at #514: one writer, DeckStreak; efficiency as the only objective;
derived card data kept under data-rights; card text to a model only through the one model route;
and writes "Autonomous within guard rails". Advisory-only was rejected because a proposal the owner
does not act on moves no card, so no trial can measure it. Approval-only was rejected because an
approval that never comes leaves the batch unrun. Both stay as rungs of the ladder below the
autonomous rung.
