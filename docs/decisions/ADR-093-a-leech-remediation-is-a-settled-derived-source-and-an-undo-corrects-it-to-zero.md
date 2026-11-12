---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A leech remediation is a settled derived source, and an undo corrects it to zero

## Context and Problem Statement

The predecessor pays 40 XP when the owner remediates a leech, once per card ever, and an undo
deletes that XP row (`database.py:GamifyStore.delete_xp_source`, predecessor `27ee2bc`); a card
remediated again after an undo pays again. The day base leaves these rows out
(`database.py:GamifyStore.day_base_xp`). DeckStreak grants XP through the grant port (SPEC-040:
idempotent keys, an unsigned amount, never confiscable) and settles derived XP per source and day
(ADR-072: `settle(day, source, track, unsigned, cause)`, raise-only for a recompute, replaced by the
owner's correction). An undo must remove the pay. How is the remediation's XP recorded?

## Decision Drivers

- SPEC-040: a grant is never confiscated, so a grant cannot be undone.
- ADR-072: a settled source may be replaced by the owner's correction, which is exactly an undo.
- The rule's shape: once per card ever, removable, and payable again after removal.

## Considered Options (the alternatives it was chosen against)

- A derived source `leech:<card id>` settled through ADR-072, paid as the owner's correction and undone by the owner's correction to zero — chosen: the settlement already carries a removable, keyed amount, and its registry already excludes sources from the day base.
- A grant through the port — rejected because a grant is never confiscable, so an undo could not remove it.
- An XP table in curriculum — rejected because it is a second XP ledger that levels, totals and export would each have to learn.
- No XP for a remediation — rejected because it drops a behaviour the predecessor pays.

## Decision Outcome

Chosen option: "A derived source settled through ADR-072", because an undo is the owner's
correction, which the settlement already models.

- A remediation settles 40 on the current study day on the `language` track, as the owner's
  correction, when no `leech:<card id>` row with a positive amount exists on any day; otherwise it
  settles nothing.
- An undo settles every `leech:<card id>` row to 0 on its own day, as the owner's correction; the
  level is read again from the totals.
- Progression's derived registry gains `leech:<card id>`, and the day base's exclusions name it.

### Consequences

- Good, because the XP stays in one ledger with one audit trail.
- Good, because an undo is visible as a correction rather than a deletion.
- Bad, because the accepted leech schematic says an undo removes "the grant"; a new schematic
  records the settlement beside it rather than rewriting it.

### Confirmation

SPEC-093's criteria: a remediation settles 40 once, a regressed card never settles again, an undo
settles zero and a later remediation pays again, and the day base excludes the source.

## What would make this wrong

- The grant port gains a reversible grant; the remediation could then move to it.

## More Information

ADR-040; ADR-072; SPEC-040; SPEC-072; SPEC-093, which builds it; `docs/schematics/leeches-state-machine.md`.
