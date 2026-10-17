---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The collection atlas is a table refreshed from the cards whose stamp changed, and read in pages

## Context and Problem Statement

Issue #282 revives the predecessor's collection atlas as a data series the agent reads: every
card, in note creation order, with its deck family and its memory state, as of the study day's start
(`charts.py:read_collection_atlas`, at `27ee2bc`). The predecessor read and parsed every card of the
collection on each request and drew the result as an image. SPEC-085 R11 left it out (#270), and the
host's memory budget allows no per-request read of every card. Where is the series held, how is it
kept current, and how does the agent read it?

## Decision Drivers

- The memory budget: a request reads a bounded page, and no step holds every card at once.
- Parity: the rows, their order, their family and `measured` equal the predecessor's goldens.
- No change is missed: a card changed on another device and synced in late must still reach the
  series.
- An empty collection and a failed read never look the same.
- The atlas is read by the agent through the MCP server, never drawn and never published.

## Considered Options (the alternatives it was chosen against)

- A ledger table walked against the copy after each sync, read in pages: chosen, because each step
  holds one page, only the cards whose stamp changed are parsed, and no change is missed. The
  refresh compares each card's stamp (`cards.mod`) with the stamp its row holds, in either
  direction, after every sync that ran and succeeded; the agent reads 2000 rows to a page.
- The predecessor's read per request: rejected because each request would read and parse every card
  of the collection, which the memory budget does not allow on the host.
- A mark on the largest stamp read, re-reading only the cards at or after it: rejected because a
  card changed offline on another device syncs in with that device's earlier stamp, below the mark,
  and would never be read.
- A whole re-read and rewrite of the table after each sync: rejected because it parses every card's
  memory state and rewrites every row on each sync, although most syncs change a few cards.
- The series held in memory by the process that serves it: rejected because the sync cycle and the
  MCP server run in different roles, and a restart would answer nothing until the next sync.
- A chart route and a Mini App screen for it: rejected because #282 names the agent as the reader,
  and SPEC-085 R6's route set is closed and draws on the client.
- One resource holding the whole series: rejected because a large collection's series in one
  response would be held whole by the server and by the client.
- The family by course (SPEC-071) instead of the deck's top-level name: rejected because the
  predecessor's family is the deck root (`charts.py:_deck_family`), and a card in no course would
  have none.

## Decision Outcome

Chosen option: "a ledger table walked against the copy after each sync, read in pages", because it
is
the only option that keeps each step within one page while missing no change.

- **The tables.** `collection_atlas` (one row a card) and `collection_atlas_state` (one row:
  `measured`, the deck-name digest, the last refresh), owned by insights, in SPEC-120's migration.
- **The walk.** Keys (a card id and its stamp) are read 500 at a time in card id order and merged
  with the table's rows in the same order: a new or re-stamped card is read and written, a gone card
  is deleted, an unchanged one is not read. A changed deck-name digest rewrites the family by deck
  id.
- **Failure.** A failed read keeps the rows and marks the series unmeasured, which answers no rows;
  the next refresh walks again. The sync cycle's outcome never depends on the refresh.
- **The read.** `charts://collection-atlas` answers the summary and
  `charts://collection-atlas/{page}` one page of 2000, under SPEC-119's guard and its `core` scope;
  `as_of` is computed at the read from the kernel's study-day rule.

### Consequences

- Good, because a request reads one page from the ledger and never touches the copy.
- Good, because a sync that changes ten cards parses ten cards.
- Bad, because every refresh still reads every card's id and stamp. They are two integer columns,
  read 500 at a time, which is the least that can find a change whose stamp went backwards.
- Bad, because the series is as current as the last successful sync, not the request. The summary
  carries the last refresh's instant, and the agent reads it with the rows.

### Confirmation

SPEC-120's A7 (a stamp that fell is re-read and an unchanged one is not), A9, A12, A13 and A14, its
rows S12001 to S12003 and S12014, and the parity goldens of its §7.

## What would make this wrong

- A collection whose id and stamp walk alone exceeds the budget: the walk would move to the change
  probe of SPEC-023 R9, skipping a refresh when the card fingerprint has not changed.
- A reader other than the agent that must see the series drawn: SPEC-085's closed set would grow by
  its own SPEC, and this table would feed it unchanged.

## More Information

SPEC-120, SPEC-085 (R6, R11 and A14), SPEC-023 (R2, R4, R7, R9), SPEC-077 R1, SPEC-119, ADR-085,
ADR-119, ADR-121, and the schematics `docs/schematics/sync-cycle-and-change-gate.md` and
`docs/schematics/charts-from-json-series.md`.
