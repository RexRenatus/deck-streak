# Schematic: habit XP settles from its log

Kind: data flow and sequence. Read at DeckStreak `dev` 02758d4 (`crates/progression/src/settle.rs`,
`crates/coordination/src/recompute/mod.rs` and `recompute/xp.rs`, `crates/coordination/src/level_up.rs`,
`crates/daemon/src/wiring.rs`, `crates/daemon/src/role_bot.rs`) and at the predecessor's `27ee2bc` for
what it ports (`pipeline_layers/habits.py:HabitsLayer.record_reading`, `undo_last_reading`,
`_recompute_reading_xp`). Decided by ADR-078, beside ADR-071 and ADR-072. Added by SPEC-078 part
078a.

## 1. Who writes the log and its XP

```mermaid
flowchart LR
  bot[bot: /read, /undo, the Undo button hb:u:id] -->|Courses, the router| uc[coordination::habits: log_minutes, undo_newest, undo_entry]
  uc -->|pure rules: token, bounds, per-day XP, week start, bonus| rules[habits::minutes]
  uc -->|one Db write, BEGIN IMMEDIATE| w1{{one write}}
  w1 -->|insert or delete| log[(minutes_log)]
  w1 -->|settle read:code on the entry's day, readgoal:code on its week's first study day, the owner's correction| settled[(the settled XP)]
  uc -->|after commit: the level before and after| announce[level_up::announce_level_up]
  announce -->|once-ever key level:N| router[the router]
  fold[recompute fold: scheduled and owner cycles] -->|Phase::DaySteps, inside the day's write| step[recompute::habits::HabitsStep]
  step -->|read the day's and the week's minutes| log
  step -->|settle read:code, and readgoal:code on a week's first study day, cause Recompute| settled
  cycle[sync cycle] -->|after its fold| announce
```

`settle` admits a habit source only through `is_derived`: one of the nine derived names, or the
prefix `read:` or `readgoal:` followed by a valid course code; anything else is refused before a
write. Two actors write `minutes_log` and the habit settles: the owner's use cases and the fold's
habit step. Both run inside one write, so they serialise. Two actors may raise a level-up, the habit
use case and the sync cycle, and the router's once-ever key lets only the first through.

## 2. The study week

```mermaid
flowchart LR
  instant[an entry's instant] -->|local offset, 04:00 rollover| day[the study day d]
  day -->|"d - (d + 3) mod 7"| first[the week's first study day, its Monday]
  first -->|"sum of the week's minutes per course; 150 at 210 or more, else 0"| bonus[readgoal:code on the first day]
  day -->|"min(240, 2 x the day's minutes)"| xp[read:code on d]
```

Epoch day 0 was a Thursday, so `(d + 3) mod 7` is 0 on a Monday. An entry at local Monday 03:59 is
on the Sunday study day and belongs to the week before.

## 3. An entry, an undo across the rollover, and a fold run

```mermaid
sequenceDiagram
  participant O as owner (bot)
  participant U as coordination::habits
  participant D as Db (one write each)
  participant F as recompute fold
  participant R as router
  O->>U: /read qaa 30 on day 7 (open)
  U->>D: BEGIN IMMEDIATE#59; insert entry#59; settle read:qaa on 7, readgoal:qaa on 4 (owner's correction)#59; COMMIT
  U->>R: announce_level_up(before, after) after commit
  Note over D: rollover: day 7 closes, today is 8
  F->>D: day 8's write: HabitsStep settles day 8's courses (cause Recompute)
  O->>U: /undo on day 8
  U->>D: BEGIN IMMEDIATE#59; delete the newest entry (day 7)#59; settle read:qaa on 7 closed, readgoal:qaa on 4 closed (owner's correction lowers them)#59; COMMIT
  O->>U: tap a stale Undo button hb:u:id
  U->>D: BEGIN IMMEDIATE#59; the newest entry is not id: remove nothing#59; COMMIT
  U-->>O: undo-stale
```

The undo lowers day 7's closed amounts in the write that removed the entry. Were the settle a write
of its own and it failed, the fold could never lower day 7 again: a recompute keeps the larger amount
on a closed day (ADR-072). The model `formal/tla/HabitXpFollowsItsLog/` checks this interleaving, and
its witness `an-undo-whose-settle-is-a-write-of-its-own` is that failure.
