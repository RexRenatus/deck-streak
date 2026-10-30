---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Curriculum's readouts are computed at the current study day's step and stored as the latest readout

## Context and Problem Statement

W4's curriculum features answer from the collection's present state: the forecast, the adaptive
goal and the balance (#86, #87, #89), memory health, readiness, depth and the horizon (#90, #91),
the strands and the test-prep board (#88, #135), and the leech snapshot (#133). Several read the
clock: retrievability decays and the collection's day number moves, so a readout is true only at
the instant it was computed. The predecessor computes most of them when asked, over a full
collection read. SPEC-071's fold recomputes the window after each sync, and its phase 4 runs a step
for the current study day. When are these readouts computed, and where are they kept?

## Decision Drivers

- A full collection read is slow, so analytics roll up within the read window and asking never
  reads the collection.
- A readout that reads the clock cannot be recomputed honestly for a past day.
- SPEC-021: every table owes six files, and a category of personal data is declared per purpose.

## Considered Options (the alternatives it was chosen against)

- A step in phase 4 of the fold, for the current study day only, writing one latest-readout table per SPEC — chosen: one computation per sync over data the fold already holds, asking reads a row, and each SPEC's table carries its own category.
- Computing each readout when it is asked — rejected because every request would pay a full collection read.
- Storing a readout for every study day — rejected because a readout that reads the clock is false for any day but the one it was computed on, and the window would hold hundreds of copies.
- One key-value table shared by every readout — rejected because it gives up `STRICT` typing and folds several privacy purposes into one category.

## Decision Outcome

Chosen option: "A step in phase 4 of the fold, for the current study day only", because it
computes each readout once per sync from data already read, and serves it without a read.

- Each SPEC registers its step in phase 4 of SPEC-071's fold; the step runs in the current study
  day's recompute only, and replaces its rows in one write.
- Coordination passes each step what it needs from other contexts (analytics' rollups, ingest's
  cards); curriculum reads no other context's table.
- Before the first recompute, every readout reads as pending.

### Consequences

- Good, because a request never reads the collection and every readout is at most one sync old.
- Good, because each table's rows and category stay small and typed.
- Bad, because a readout between syncs is stale by up to one sync interval; each route shows when
  it was computed.

### Confirmation

SPEC-090's A9 and A10, SPEC-091's A12 and SPEC-092's and SPEC-093's step criteria: two recomputes
store one row per kind and scope, and before the first every readout is pending.

## What would make this wrong

- A readout must be exact at the moment it is asked; it then moves to an on-demand run (ADR-094's
  frame) and pays the read.
- The fold's current-day step grows past the sync's budget; a readout then moves after the
  recompute, as ADR-099's pass does.

## More Information

ADR-012; SPEC-021; SPEC-071, which builds the fold; SPEC-090 to SPEC-093.
