---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The day alone is a report over one full day after the import, recorded by its digest

## Context and Problem Statement

#63 asks that a day of DeckStreak alone pass every SLO before the cutover is complete. SPEC-031
declares the SLOs over a rolling window and pages on burn rates (ADR-031). After the retirement
and the import, what judges "a day alone", and what enters the ledger?

## Decision Drivers

- The day must follow the retirement and the import, so it judges the system that will run.
- A day that saw no traffic proves nothing.
- The verdict must be an exact comparison, never a float's near-miss.
- The report holds counts of personal use: it stays private.

## Considered Options (the alternatives it was chosen against)

- A day report over the first full day after the import: chosen, because it judges one day of the
  system that stays, and a quiet day cannot pass. `alone` refuses it unless it begins at or after
  the retirement; it holds each SLO's good over total against its objective as an exact fraction,
  `void` with no response, and the ledger holds the report's sha256.
- The SLOs' rolling 28-day window: rejected because it holds the release for four weeks, and mixes
  days before the retirement into the verdict.
- The silence of the burn alerts: rejected because silence is not evidence: a day with no traffic,
  or a journal nobody could read, pages no one.
- A day before the import: rejected because the import changes the data every surface reads, and
  the day must include that change.
- The report itself in the ledger: rejected because its counts are personal; the digest proves
  which report was judged without carrying it.

## Decision Outcome

Chosen option: the day report after the retirement, recorded by its digest. A failed or void day
is the owner's decision (#164), with the import's rollback and an item's revert as the ways back.
SPEC-144 R3 to R5 hold it.

### Consequences

- Good, because the day report reads the burn alerts' own events, so the two cannot disagree on
  the data.
- Bad, because the release waits at least one full day after the import.

### Confirmation

SPEC-144's day-report criteria (pass, fail, void, no page) and its `alone` refusals.

## More Information

#63, #64, #164, ADR-031, SPEC-031, SPEC-144.
