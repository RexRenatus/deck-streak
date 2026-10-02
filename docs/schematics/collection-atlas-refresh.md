# Schematic: the collection atlas, refreshed after each sync and read in pages

Kind: sequence and decision flow. Read at DeckStreak `dev` 5216bcf (ADR-009, ADR-085,
`docs/schematics/sync-cycle-and-change-gate.md`, `docs/schematics/charts-from-json-series.md`), and
at the predecessor's `27ee2bc` for the behaviour it ports (`charts.py:read_collection_atlas`,
`_deck_family`). Added by SPEC-120 (ADR-120). It adds one step after the sync cycle's flush, and
one read path from insights to the agent beside the charts' path to the Mini App; it rewrites
neither.

## The refresh

```mermaid
sequenceDiagram
  participant S as coordination sync cycle
  participant I as ingest atlas reader
  participant N as insights atlas table
  S->>S: a sync ran and succeeded, the flush done
  S->>N: refresh, whose error never changes the cycle's report
  loop pages of at most 500 card ids, in id order, until a short page
    N->>I: keys after the last id: card id and its stamp
    I-->>N: the page, or a read error
    N->>N: merge with the table's rows in the same order
    N->>I: rows for the new and re-stamped ids only
    I-->>N: id, note, ordinal, deck and memory state
    N->>N: one BEGIN IMMEDIATE batch: insert, replace, delete
  end
  N->>N: deck names digested, a changed digest resets each family by deck id
  N->>N: state row: measured, the digest, the refresh instant
```

A stamp that went backwards, as a card changed offline on another device does, differs from the
stored stamp and is read again; an unchanged stamp is never read. A read error keeps the rows and
marks the series unmeasured until the next refresh.

## The read

```mermaid
flowchart LR
  agent["the agent, over the MCP server's core scope"] --> sum["the summary resource: count, pages, the refresh instant, measured"]
  agent --> page["one page resource: at most 2000 rows in note, ordinal and card order"]
  sum --> co["coordination atlas page"]
  page --> co
  co --> tbl["the insights table, never the collection copy"]
  tbl --> asof["as_of from the kernel's study-day rule, computed at the read"]
```

An unmeasured series answers a count of 0 and no rows, never an empty collection. No route draws
it and nothing publishes it: the agent reads it as data.
