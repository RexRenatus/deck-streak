# Schematic: streaks, freezes and the governor across every settled study day

Kind: state machine and data flow. Read at DeckStreak `dev` c3d769b and at the predecessor's
`27ee2bc` (`gamification/streak.py`, `skip.py:real_misses`, `analytics.py:bridged_streak`,
`gamification/strength.py:advance`, `gamification/governor.py:assess`,
`pipeline.py:GamifyPipeline._advance_streak` and `_freeze_events_for`,
`pipeline_layers/governor.py:GovernorLayer._update_governor`,
`pipeline_layers/showcase.py:ShowcaseLayer._relight`, and the freeze grants of
`pipeline_layers/economy.py:EconomyLayer.buy_item`, `pipeline_layers/loot.py:LootLayer._update_weekly_quest`,
`LootLayer.pick_epic_prize` and `pipeline_layers/showcase.py:ShowcaseLayer._evaluate_season_nodes`).
Added by SPEC-076.

It extends `docs/schematics/streaks-and-governor-state-machine.md`, which stands as written: that
file draws the streak's and the governor's states; this one draws how W3 drives them once for every
settled study day (ADR-071), the law streak's daily evaluation (ADR-076), the one freeze port every
freeze source calls, and the lapse anchor stored past the silence walk.

## The streak step of one study day

The step runs in phase 3 of SPEC-071's fold, for every study day the fold settles, in order, and
then for the current study day. Each call is idempotent for its day: a second recompute of one day
changes nothing. The days the first recompute backfills with no today-only rule advance neither
track; strength folds them, and the first day the fold settles bootstraps an empty language row
from its raw streak, as the predecessor's first run did. The relight sits in phase 3 too, after the
governor, so its XP is in the day's base before the derived bonuses and the mint read it.

```mermaid
flowchart TD
  start["phase 3 of the fold reaches day d, ADR-071"] --> study{"d has a study review?"}
  study -- "yes" --> upd["language: update_on_study with d, the raw streak and the skip days"]
  study -- "no" --> dec["language: decay_on_lapse with d and the skip days"]
  upd --> events["the transition's freeze events, written with the state in one write"]
  dec --> changed{"a persisted field changed?"}
  changed -- "yes" --> events
  changed -- "no" --> law
  events --> law["law: current is bridged_streak of the law days at d, longest is the maximum"]
  law --> strength["strength of d from strength of the day before, and whether d was studied"]
  strength --> walk["silent run: back from d over days with no study review, skip days transparent, at most 120 days"]
  walk --> verdict["assess strength and the silent run: armed, standby or lapse"]
  verdict --> exhausted{"a lapse, and the walk ran out of days?"}
  exhausted -- "yes, with a stored anchor at or before the horizon" --> keep["keep the stored anchor"]
  exhausted -- "no" --> fresh["the anchor is the run's first silent day that is not a skip, or none outside a lapse"]
  keep --> relight
  fresh --> relight{"the state settled for the day before held a lapse, and d is a study day?"}
  relight -- "yes, and d has at least 3 reviews" --> pay["grant the relight XP once and raise its celebration once"]
  relight -- "no" --> store["store the governor state as of d, when d settles"]
  pay --> store
```

The governor state is stored only when a day settles, so the current study day's verdict is read
from the state settled for the day before and the current day's reviews so far. A return day whose
third review lands after a mid-day recompute therefore still relights at its settle, which reads
the day's whole count. The language row, by contrast, is written at every recompute: a study day
already recorded is the classifier's same-day case and changes nothing.

## The law streak (ADR-076)

```mermaid
stateDiagram-v2
  [*] --> Zero
  Zero --> Running: a study day with a law review starts a run of 1
  Running --> Running: another study day with a law review adds 1
  Running --> Running: a declared skip day neither counts nor breaks
  Running --> Running: the day asked about has no law review yet, so the run through the day before stands
  Running --> Zero: the study day before the day asked about had no law review and was not a declared skip
  note right of Running
    evaluated on every settled study day and on the current one, with law study or without
  end note
```

The predecessor's caller wrote this row only on days with a law review, so the move back to Zero
never ran and a finished streak kept its value. DeckStreak asks the same function on every day. The
law row has no freezes: nothing consumes one from it or grants one to it.

## The freeze port

Only the streaks context writes `streak_state` and `freeze_events`. Every freeze that does not come
from the streak's own transitions is requested through one port, in coordination.

```mermaid
flowchart TD
  req["grant_freeze with a study day and a reason"] --> held{"the language row already holds 3 freezes?"}
  held -- "yes" --> refuseHold["refused: the hold cap"]
  held -- "no" --> isDrop{"the reason is chest, weekly_quest or season?"}
  isDrop -- "no, it is the shop" --> grant["one more freeze and its event, in one write"]
  isDrop -- "yes" --> month{"freezes those reasons granted in the day's calendar month already reach 1?"}
  month -- "yes" --> refuseMonth["refused: the monthly drop cap"]
  month -- "no" --> grant
```

| reason | requested by | what a refusal means to the caller |
|---|---|---|
| `shop` | the shop's freeze purchase (SPEC-082) | the purchase writes no coin movement and says why |
| `chest` | an Epic chest's choice (SPEC-081), for the chest's own study day | the choice becomes a Double-XP token, labelled as such |
| `weekly_quest` | the weekly quest's reward (SPEC-080) | the reward pays its Epic chest without the freeze |
| `season` | season node 5 (SPEC-074) | the node pays its coins without the freeze |

`streak_earn` (one every 7 streak days), `consumed` and `streak_break` are the streak's own
transitions, written with the state they change, never requested through the port.

## The governor and its stored anchor

```mermaid
stateDiagram-v2
  [*] --> Armed
  Armed --> Standby: strength below 60 percent
  Standby --> Armed: strength back at 60 percent or more
  Armed --> Lapse: a silent run of 3 study days that are not skips, anchored at its first
  Standby --> Lapse: a silent run of 3 study days that are not skips, anchored at its first
  Lapse --> Lapse: the walk runs out of days, so the stored anchor is kept
  Lapse --> Armed: a study day ends the run, with the relight at 3 or more reviews
  Lapse --> Standby: a study day ends the run while strength is still below 60 percent
```

The anchor is SPEC-049's lapse id. For every silent run shorter than the walk it is computed exactly
as SPEC-049's slice computes it (the golden `lapse_episode`); the stored value matters only past the
walk, where the predecessor kept it rather than let the walk's horizon slide and open a new episode
every day (the golden `lapse_anchor_beyond_the_walk`). The standby notice's rule (entering standby,
outside a lapse and outside quiet hours, and not within 7 days of the last notice) is computed here;
no notice is sent until the first discipline device exists (#109).

## What each step reads and writes

| step | reads | writes |
|---|---|---|
| language streak | the day's study reviews and the raw streak (through coordination), the declared skip days, the stored row | `streak_state` (language), `freeze_events` |
| law streak | the window's law study days, the declared skip days | `streak_state` (law) |
| strength | whether each day since the first study day was studied | `habit_strength` |
| governor | the strength, the silent run, the stored state | `governor_state` |
| relight | the day's review count, the governor state settled for the day before | progression's grant port, and the router |
| freeze port | the language row, the month's drop events | `streak_state` (language), `freeze_events` |
