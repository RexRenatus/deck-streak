# Schematic: the recompute settles each study day once, in order

Kind: state machine and data flow. Read at DeckStreak `dev` c3d769b (ADR-037,
`crates/coordination/src/sync_cycle.rs`, whose recompute has no domain consumer before W3) and at
the predecessor's `27ee2bc` (`pipeline.py:GamifyPipeline._recompute_day`, `_days_to_recompute`,
`_advance_streak`, `_baseline`; `pipeline_layers/governor.py:GovernorLayer._on_pace_run`). Decided
by ADR-071 (the fold) and ADR-072 (settled XP). Added by SPEC-071. It extends, and does not
change, `docs/schematics/data-flow.md` (the recompute box after the read) and
`docs/schematics/sync-cycle-and-change-gate.md` (the recompute the gate lets through).

## One study day

```mermaid
stateDiagram-v2
  [*] --> Current: the rollover opens the study day
  Current --> Current: each recompute evaluates it as far as it has gone, with the live card state
  Current --> Owed: the next rollover closes it
  Owed --> Owed: no successful sync has started since it closed
  Owed --> Settled: the first recompute after such a sync settles it, oldest owed day first
  Settled --> Settled: a later recompute re-rolls it only when its reviews changed, and only raises its pay
```

A day settled right after it closed keeps the card state it had at its close, with its provenance.
A day settled after a gap has no card state, and keeps none. The very first recompute, which finds
no settled day, rolls up and scores every past day of the window in the predecessor's historical
form, with no today-only rule, and the fold settles from the most recently closed day on.

## One recompute

```mermaid
flowchart TD
  sync["a successful sync, scheduled or the owner's"] --> read["read the review window once (SPEC-023)"]
  read --> finger["fingerprint each study day: its reviews and the courses digest"]
  finger --> first{"is any day settled yet?"}
  first -- "no, the first recompute" --> backfill["roll up and score the window's past days in the historical form"]
  first -- "yes" --> owed{"a closed day after the cursor, whose close came before the sync started?"}
  backfill --> owed
  owed -- "yes, the oldest" --> settle["settle it: every phase, with that day as the day evaluated"]
  settle --> mark["record the instant it was settled on its rollup: the cursor moves"]
  mark --> owed
  owed -- "none left" --> current["evaluate the current study day as far as it has gone"]
  current --> rescore["re-score the window's study days in the historical form, card state untouched"]
  rescore --> flush["the router flushes deferred celebrations (SPEC-041)"]
```

## The phases of one day

```mermaid
flowchart LR
  p1["1 rollup and score: analytics"] --> p2["2 base XP: progression"]
  p2 --> p3["3 streaks and the governor"]
  p3 --> p4["4 habits, focus, quests and chests"]
  p4 --> p5["5 derived bonuses: consistency and Ascendant"]
  p5 --> p6["6 the coin mint: economy"]
  p6 --> p7["7 awards: badges, records and season nodes"]
```

| phase | what it ports from the predecessor | registered by |
|---|---|---|
| 1 rollup and score | `analytics.py:compute_daily_metrics`, `compute_card_snapshot`, `cross_language.py:language_daily_metrics`, `scoring.py:compute_score` | SPEC-071 |
| 2 base XP | the review XP and the daily bonuses of `pipeline.py:GamifyPipeline._recompute_day`, and the day's Ascendant armed from the day before (`GovernorLayer._maybe_grant_ascendant`) | SPEC-072 |
| 3 streaks and the governor | `_advance_streak`, `DigestsLayer._update_law_streak` (evaluated every day, ADR-076), `GovernorLayer._update_governor`, `ShowcaseLayer._relight` | SPEC-076 |
| 4 the other contexts' day steps | Road to C2's progress and band-ups, the habit and focus XP, the daily and weekly quests, the ghost race, session chests and tokens | SPEC-077, SPEC-078, SPEC-079, SPEC-080, SPEC-081 |
| 5 derived bonuses | `GovernorLayer._apply_day_bonuses` (consistency and Ascendant), after every base source of the day | SPEC-072 |
| 6 the coin mint | the mint of `_recompute_day`, over the day's base XP | SPEC-082 |
| 7 awards | the badges of `_evaluate_and_award` (study, habit, focus and band badges through one award port), the records of `rewards.py:detect_records`, the season node track and the chapter ceremony (`ShowcaseLayer._evaluate_season_nodes`, `ShowcaseLayer._month_ceremony`) | SPEC-073, SPEC-074, SPEC-077, SPEC-078, SPEC-079 |

Every step is keyed by the day it evaluates, so running it again for a settled day changes nothing
unless that day's own reviews changed; and every XP amount a step writes for a closed day is only
ever raised (ADR-072).

## Two recomputes that overlap (added 2026-10-02, #311)

Kind: sequence and state machine. Insert-only: the sections above are kept as they are. Read at
DeckStreak `dev` 9b0bf65f9, where `Fold::run` reads the settle cursor once, in its first write
(`crates/coordination/src/recompute/mod.rs:479`), and opens each owed day's write with no re-read
(`mod.rs:503`). Decided by ADR-313; SPEC-071 sections 11 and 12; modelled by
`formal/tla/FoldSettlesOnce`.

The scheduled cycle and the owner's recompute can both reach the fold before either has cleared
the rescore mark. Every write is one `BEGIN IMMEDIATE` transaction, so two writes never interleave;
what overlapped was the decision, taken from a cursor read in an earlier transaction. Each owed
day's write now re-reads the cursor before the day's steps run, and settles only the day after it.

```mermaid
sequenceDiagram
  participant sched as scheduled fold
  participant ledger as ledger file
  participant owner as owner's fold
  sched->>ledger: first write: read the cursor, last settled day C
  owner->>ledger: first write: read the cursor, still C
  Note over sched,owner: both owe C+1
  sched->>ledger: BEGIN IMMEDIATE, re-read the cursor: C, so C+1 is owed
  sched->>ledger: the steps of C+1 and its settled_at, COMMIT: the cursor is C+1
  owner->>ledger: BEGIN IMMEDIATE waits for the lock, then re-reads the cursor: C+1
  Note over owner: C+1 is not owed now: the write is dropped, nothing commits
  owner->>ledger: BEGIN IMMEDIATE, re-read the cursor: C+1, so C+2 is owed
  owner->>ledger: the steps of C+2 and its settled_at, COMMIT: the cursor is C+2
```

The owed day, as each owed day's write decides it:

```mermaid
flowchart TD
  owing{"the run's day has closed, and the run's sync started after its close?"}
  owing -- "no" --> current["evaluate the current study day"]
  owing -- "yes" --> lock["BEGIN IMMEDIATE: the write lock"]
  lock --> reread["re-read the cursor"]
  reread --> owed{"the owed day: the day after the cursor, or the run's own day with no cursor"}
  owed -- "the run's day" --> settle["the day's steps and its settled_at, then COMMIT"]
  settle --> step["the run's day is the day after it"]
  owed -- "another day" --> drop["drop the write: nothing commits"]
  drop --> jump["the run's day is the owed day"]
  step --> owing
  jump --> owing
```

Before the first settled day there is no cursor, so each fold owes its own most recently closed day.
A fold whose day another fold already passed skips forward; a fold that read no cursor while
another settled an earlier day goes back to the day after it, so no day after the first settled
one is skipped. A day before the first settled day has a row only as a study day of the window or
as some recompute's current day (SPEC-071 section 11).
