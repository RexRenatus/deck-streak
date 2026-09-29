# Schematic: the insights instruments run on one frame

Kind: data flow and state machine. Read at DeckStreak `dev` dd98601 (`crates/insights/src/` holds
only `lib.rs`; `crates/coordination/src/sync_cycle.rs`) and at the predecessor's `27ee2bc`
(`pipeline_layers/read_api.py:ReadApiLayer`, `bot.py:CommandBot`, each instrument's
`build_*_report`). Decided by ADR-094 (the frame), ADR-095 (the reads keep the scope) and ADR-096
(the owner's note conventions). Added by SPEC-094; SPEC-095 to SPEC-098 register their instruments
on it. It extends `docs/schematics/data-flow.md` (the insights box) and
`docs/schematics/sync-cycle-and-change-gate.md` (what runs after a recompute), and changes neither.

## After a sync

```mermaid
flowchart TD
  sync["a successful sync"] --> recompute["the recompute commits (SPEC-071)"]
  recompute --> cando["the Can-Do unlock pass (SPEC-099)"]
  cando --> step["the instruments step (SPEC-094)"]
  step --> cleared["mark the dealt hand's cards answered after the deal as cleared (SPEC-098)"]
  cleared --> due{"a weekly instrument whose report is absent or 7 study days old?"}
  due -- "yes, the next in registry order" --> run["run it through the offload, one at a time"]
  run --> store["replace its row in instrument_reports"]
  store --> due
  due -- "none left" --> done["the step ends"]
```

The registry order puts the Docket after the Echo Test, the Price of a Day Off and Bench II, so it
folds their reports of the same run (SPEC-095). The cleared marks read only the sync's own study
events. A failure of one instrument is stored as its report's failed read and never stops the next.

## One instrument run

```mermaid
flowchart LR
  ask["a weekly turn, a route or a command"] --> busy{"is any instrument running?"}
  busy -- "yes" --> refuse["answer that a run is in progress, start nothing"]
  busy -- "no" --> reads["coordination gathers the reads: ingest, analytics rollups, curriculum"]
  reads --> build["insights builds the report, a pure function of its reads"]
  build --> write["one write: the report, its study day and schema version"]
  write --> serve["the route, the bot and the Mini App read the stored report"]
```

## One instrument's report

```mermaid
stateDiagram-v2
  [*] --> Pending: no report stored yet
  Pending --> Stored: its first run
  Stored --> Stored: a later run replaces it
  Stored --> Failed: a run whose read failed replaces it with the failure
  Failed --> Stored: a later run succeeds
  Stored --> Pending: the owner's erase
  Failed --> Pending: the owner's erase
```

A registry row marked inert never runs and never shows. The instruments' reads are ingest's
(ADR-095): every read keeps SPEC-023's scope, a read of answers keeps its window's floor, and a set
of ids ever reviewed or a lag reads the whole scoped log. The weekly report (#130) reads the stored
reports through the one router; W4 sends none.
