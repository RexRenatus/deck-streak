# Schematic: the skip day, recorded by DeckStreak and read by every context it touches

Kind: data flow and sequence. Read at DeckStreak `dev` c3d769b and at the predecessor's `27ee2bc`
(`pipeline_layers/skip.py:SkipDaysLayer`,
`pipeline_layers/economy.py:EconomyLayer._charge_skip_tariff` and `_refund_skip_tariff`,
`skip.py`, and every reader of `database.py:GamifyStore.skip_days_set`). Added by SPEC-083, under
ADR-083's option (b).

It extends `docs/schematics/data-flow.md`, whose dashed arrow from coordination to the sync server
marks the skip day as W3's, held to ADR-037's no-upload rule. Under option (b) that arrow carries
nothing: no step below reaches the sync server or writes the collection copy.

## Taking a skip

```mermaid
sequenceDiagram
  participant O as owner
  participant S as bot or Mini App
  participant C as coordination skip use cases
  participant A as analytics rollup
  participant I as ingest skip record
  participant E as economy wallet
  O->>S: skip, or cheat
  S->>C: preview
  C->>A: the study day's due review count, absent when the day has no rollup yet
  C->>I: an active skip today, and the skips applied this calendar month
  C->>E: the tariff's price from economy.json, and the balance
  C-->>S: due count, tariff, funded or not, the search and the day spec for Anki
  O->>S: confirm
  S->>C: take
  C->>I: record the skip, once per study day
  C->>E: purchase the tariff, clipped to the wallet, outside the daily loss cap
  C->>I: record a shortfall when the wallet could not cover it
  C-->>S: recorded, with the search, the day spec and the steps to reschedule in Anki
  Note over C,I: no request reaches the sync server, and the collection copy is never written
```

## What reads the skip, at the next recompute

```mermaid
flowchart LR
  rec[("skip_days, owned by ingest")] --> port{{"the skip set: study days with a skip not undone"}}
  port --> ls["language streak: bridged, no freeze consumed"]
  port --> law["law streak: bridged"]
  port --> run["consistency run: left unchanged"]
  port --> asc["Ascendant: never armed on the day"]
  port --> gov["governor: the silent run neither counts nor ends at the day"]
  port --> qs["quests: the day's quests voided, never failed, SPEC-080"]
  port --> ch["chests: none rolled on the day, SPEC-081"]
  port --> gh["ghost race: a week of two or more skip days is exempt, SPEC-080"]
  port -.-> w5["discipline and evening nudges: read the same set in their wave"]
```

Each reader holds its own rule and proves it in its own SPEC; the skip set is the one port they all
read, through coordination, so no context queries `skip_days` a second way.

## Undoing a skip

```mermaid
flowchart TD
  undo["undo, confirmed"] --> latest{"a skip not undone?"}
  latest -- "no" --> none["nothing_to_undo"]
  latest -- "yes" --> mark["mark the most recent one undone, at the undo's instant"]
  mark --> refund["credit what it paid, on the undo's study day"]
  refund --> copy["say that a reschedule made in Anki is undone in Anki"]
  mark --> next["next recompute: the set no longer holds the day"]
  next --> miss["the day is a missed day wherever a rule counts one"]
  next --> kept["transitions already settled stay as they were"]
```

The skip may be taken again on the same study day after an undo: the migration's key allows one
skip not undone per study day, not one row.
