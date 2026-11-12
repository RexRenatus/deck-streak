---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Every other experimental model only reorders cards already due

## Context and Problem Statement

The owner asked for the study client to carry experimental scheduling models beyond FSRS-7, with
RWKV-Instant first (the owner's answer ANK-01), and chose that such a model may influence what
is studied (OQ4). A model that writes due dates or intervals meets ADR-301 (a) entries 5 and 6:
it reschedules cards no one named and sets a per-card due-date policy the scheduler did not.
RWKV-Instant predicts recall at query time and gives no interval at all, and it has no licence
that would let this repository or the client carry its code or its weights (OQ2). FSRS-7 on its
one preset is the single exception the owner chose (ADR-338). How far may every other model reach?

## Decision Drivers

- No model other than FSRS-7 on its preset writes to the collection, so entries 5 and 6 stay
  untouched.
- The model's effect can be measured on observed outcomes, as ADR-301 (e) scores any change.
- The repository never carries a model whose licence does not admit it.
- Switching a model off returns the stock order exactly.

## Considered Options (the alternatives it was chosen against)

- Reorder only: a model may change the order of the cards the stock scheduler already made due today, and nothing else — chosen because the owner chose it (OQ4), it writes nothing, it never changes which cards are due or when, and switching it off restores the stock order.
- Let a model schedule, writing due dates or intervals — rejected because it meets never-list entries 5 and 6, and the owner kept that power for FSRS-7 on its one preset (ADR-338).
- Display only, showing a model's predictions and changing nothing — rejected because the owner chose that a model may influence what is studied.
- Ship a model's code or weights in the client — rejected because RWKV-Instant has no licence that admits it, and written permission comes before any use.

## Decision Outcome

Proposed option: reorder only.

- **The input.** The client takes a generic input of external due-card scores: for each card
  due today, an identifier and a score. A model that cannot be carried in the repository runs
  outside it and reaches the client only as these scores. The repository carries the input's
  shape and no model's code or weights.
- **The rule.** When the owner turns the input on, the client orders today's due queue by the
  scores. It never adds a card that is not due, never removes one that is, and never writes a due
  date, an interval or any card field. A card with no score keeps its stock position after the
  scored ones, and missing or stale scores leave the stock order.
- **The test.** For any queue and any scores, the reordered queue holds the same set of cards,
  each with the same due date and interval, as the stock queue; a planted model that writes a
  due date is refused.
- **CHARTER 4.** With ADR-338, this ADR carries the scheduler sentence of the app-surfaces
  ruling's CHARTER 4 amendment and of the PRD's third non-goal
  (`docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`): an experimental model only reorders
  cards the stock scheduler already made due, except FSRS-7 on the one preset the owner switches.

### Consequences

- Good, because an experimental model can be tried, and its effect read from the review log,
  with no write to the collection and no risk to the schedule.
- Good, because the repository stays free of models whose licences do not admit it.
- Bad, because a model that predicts recall well but cannot set intervals can only order the day,
  never spread the load.

### Confirmation

- The reorder test: same set, same due dates and intervals, with its examined count.
- The input's shape test: no score input reaches any write path of the engine core.

## What would make this wrong

- The owner chooses to let a second model schedule a preset, which is a new ruling and a
  supersession of this ADR.
- A model's authors grant a licence that admits shipping it, which changes where it runs, never
  the reorder-only rule.

## More Information

- SPEC-334 (R11; the RWKV-Instant exclusion in section 5).
- ADR-301 (a) entries 5 and 6, ADR-301 (e), ADR-337, ADR-338.
