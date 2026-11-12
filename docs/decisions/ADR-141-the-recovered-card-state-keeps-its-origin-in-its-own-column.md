---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The recovered card state keeps its origin in its own column

## Context and Problem Statement

The predecessor stamps each day's card state `live:<instant>` when it read it on the day,
`backup:<instant>` when its one-shot recovery (`backfill_card_state_from_backups.py:plan_backfill`)
restored it from its own retained backups, and `void:fossil` when that recovery proved the stored
counts a fabrication and zeroed them; a day with no stamp was never recorded, or was recorded before
the stamp existed. Its daily view (`pipeline_layers/base.py:PipelineBase._render_daily_from_rollup`)
renders no snapshot for an absent or void stamp and leaves the learning term out for a recovered
one. DeckStreak's `daily_rollup.card_state_src` is checked `GLOB 'live:[0-9]*'`, and SPEC-071's
recompute writes it. #61's third criterion asks that the recovered history arrive with its
provenance. Where does the provenance live?

## Decision Drivers

- No fabricated history (CHARTER 10): a reading nobody can vouch for arrives NULL.
- The recompute is the only writer of a day's card state (SPEC-071 R9); the provenance is the
  import's.
- The day view and the export must carry the provenance with its row (SPEC-021).

## Considered Options (the alternatives it was chosen against)

- A `card_state_origin` column (NULL, `backup`, `void`), the source kept `live:<epoch ms>`: chosen,
  because the source's shape stays one, and the origin is a separate fact the recompute never
  writes.
- Widening the source's check to `backup:` and `void:` stamps: rejected because every reader that
  parses `live:<ms>` would meet new shapes, and the recompute's own column would carry the import's
  provenance.
- A side table of provenance per day: rejected because it is a new table with its six files for one
  fact, and the day view and the export would join it on every read.
- Importing only the live readings: rejected because #61's third criterion asks for the recovered
  history, with its provenance.
- Treating an unstamped day with counts as recorded: rejected because an absent stamp cannot tell a
  real reading from a fabricated one; the day arrives with NULL counts, counted `unstamped`.
- Reading a stamp of an unknown prefix as recorded: rejected because DeckStreak's source check
  admits only `live:<ms>`, and a stamp the predecessor never minted is a reading nobody can vouch
  for. The predecessor's daily view renders a snapshot for every stamp but an absent one and
  `void:fossil`; the import refuses an unknown prefix instead (SPEC-140 R14), and the golden's
  unknown-prefix case carries `diverges` citing this record.

## Decision Outcome

Chosen option: the `card_state_origin` column, because it carries the provenance without changing
what the recompute writes or what the source means. SPEC-140 R7 and R8 hold it.

### Consequences

- Good, because the day view answers each day as the predecessor's view did (`Absent`,
  `Recorded`, `Recovered`, `Voided`), proved by a golden.
- Bad, because an unstamped day with counts loses them; the owner may choose otherwise (the plan's
  owner question 2), and the conservative answer holds until then.
- Bad, because on a stamp of an unknown prefix the import departs from the predecessor's view: it
  refuses the write where the view rendered a snapshot, and the golden records that case as a
  divergence.

### Confirmation

SPEC-140's migration check, the `import_card_state_reading` golden, and the recompute's test that
an imported day's card state is kept.

## More Information

#61, ADR-071, SPEC-071, SPEC-140.
