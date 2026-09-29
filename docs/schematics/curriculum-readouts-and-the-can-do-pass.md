# Schematic: curriculum's readouts at the current study day, and the Can-Do pass after it

Kind: data flow. Read at DeckStreak `dev` dd98601 (`crates/curriculum/src/` holds only `lib.rs`)
and at the predecessor's `27ee2bc` (`velocity.py`, `cross_language.py`, `coaching.py`, `horizon.py`,
`strands.py`, `lsat.py`, `leeches.py` and `cando.py:unlock_pass`). Decided by ADR-091 (curriculum's
readouts are computed in the current study day's step and stored as the latest) and ADR-099 (the
Can-Do pass runs after each sync's recompute and records each unlock once). Added by SPEC-091;
SPEC-090, SPEC-092 and SPEC-093 register their steps on it, and SPEC-099 adds the pass. It extends
`docs/schematics/recompute-settles-each-study-day.md` (phase 4 of the fold) and changes none of it.

## Phase 4 of the current study day

```mermaid
flowchart TD
  fold["the recompute evaluates the current study day (SPEC-071)"] --> p3["phase 3, streaks and the governor"]
  p3 --> p4["phase 4, the day steps"]
  p4 --> pace["the pace step: forecasts, the goal and the balance into pace_readouts (SPEC-090)"]
  p4 --> memory["the memory step: health and the horizon into memory_readouts (SPEC-091)"]
  p4 --> strands["the strands step: strands, weak spots and the board into strand_readouts (SPEC-092)"]
  p4 --> leech["the leech step: the snapshot into leech_snapshot (SPEC-093)"]
  pace --> p5["phase 5, derived bonuses"]
  memory --> p5
  strands --> p5
  leech --> p5
```

Each step runs in the current study day's recompute only, never when an owed day settles, and
replaces its rows in one write. Coordination passes each step what it needs from other contexts
(analytics' rollups, ingest's cards, each home deck's course); curriculum reads no other context's
table. Before the first recompute every readout reads as pending, and a route or command serves the
stored readout without a read.

## After the recompute commits

```mermaid
flowchart LR
  commit["the recompute commits"] --> read["ingest reads the Can-Do field's notes and their cards, in scope"]
  read --> rungs["curriculum builds the rungs, maturity and cards remaining"]
  rungs --> write["one write"]
  write --> unlocks["can_do_unlocks: each unlocked rung once, with its first instant"]
  write --> ladder["can_do_ladder: the latest pass, replaced"]
  ladder --> instruments["then the instruments step (SPEC-094)"]
```

The pass runs through the offload, off the fold's write path. A failed read replaces the ladder with
the failure and records no unlock; the unlocks already recorded stay. A rung that falls back stays
in `can_do_unlocks` and is listed as locked too.
