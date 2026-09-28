# Schematic: the focus timer, its claim and its cycle

Kind: state machine. Read at DeckStreak `dev` c3d769b and at the predecessor's `27ee2bc`
(`focus.py`; `bot.py:CommandBot`'s focus methods, from `start_focus` to `_rearm_focus_timer`;
`pipeline_layers/focus.py:FocusLayer`; `database.py:GamifyStore.mark_focus_timer_fired`). Added by
SPEC-079 (ADR-079).

## The timer row

One row, `focus_timer`. Every start writes a new generation (`block`), so a timer armed for an older
generation can claim nothing. The claim, the block's `focus_log` row and the next state commit in
one write, so "claimed" is never a resting state: the row leaves it in the same transaction.

```mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: start, a new generation, minutes clamped to the entry range
  Running --> Paused: pause keeps the remaining seconds
  Paused --> Running: resume sets a new end from them
  Running --> Running: add five minutes moves the end by the minutes added
  Paused --> Paused: add five minutes grows the remainder by the minutes added
  Running --> Credited: the end passes and the claim for this generation wins
  Running --> Credited: stop, or a new start replaces the block
  Paused --> Credited: stop, or a new start replaces the block
  Credited --> Idle: a one-off block ends, or a stop ends the cycle
  Credited --> Running: a cycle advances to its next block, a new generation
  note right of Credited
    one write: the claim, the block row
    (minutes run, abandoned under the floor,
    certified only when timer-completed)
    and the next state
  end note
```

## The cycle

Lengths and rule from `goldens/focus_cycle_advance.json` and `goldens/focus.constants.json`.

```mermaid
flowchart LR
  W["work block of the cycle length"] -- "ends, the count of work blocks is not a multiple of four" --> S["short break"]
  W -- "ends, the count of work blocks is a multiple of four" --> L["long break"]
  S -- "ends" --> W
  L -- "ends" --> W
  W -- "stop" --> X["idle, the cycle ends"]
  S -- "stop" --> X
  L -- "stop" --> X
```

## Two roles, one block

Each long-running role arms a timer for the running block when it starts and whenever it writes the
row. Both may fire; the claim decides.

```mermaid
sequenceDiagram
  participant A as api role
  participant B as bot role
  participant T as focus_timer and focus_log
  participant R as the one router
  A->>T: start writes generation G with end E
  A->>A: arm a timer for G at E
  B->>T: the bot role restarts and reads the row
  B->>B: arm a timer for G at E
  A->>T: at E claim G if running, unclaimed and past E
  T-->>A: won, and the block row and the next state in the same write
  B->>T: at E claim G
  T-->>B: nothing claimed
  A->>R: raise the celebration event focus_block for G at T2
  R-->>R: deferred in quiet hours, deduplicated by the key that names G
```

## What a role does at start

The predecessor's restart decision (`goldens/focus_rearm.json`), under ADR-079's one-write claim:

| the row | the role |
|---|---|
| idle | arms nothing |
| paused | arms nothing; resume arms |
| running, end already passed | claims and credits the block once (the catch-up), then advances a cycle |
| running, end still ahead | arms a timer for the rest |
| claimed but not credited | cannot occur: the claim and the credit are one write |

## Focus XP, after each block and at each recompute

The recompute's focus XP step is registered in phase 4 of SPEC-071's fold, after phase 2 settles the
day's daily bonuses, and its focus badge step in phase 7.

```mermaid
flowchart TD
  block["a block credited, or the day settled by the recompute in phase 4"] --> day["focus: the day's completed blocks, capped"]
  block --> week["focusgoal: on the week's first study day, when the week reaches its goal"]
  block --> combo["focus_combo: when the day has a backlog-zero grant and its blocks reach the floor"]
  day --> settle["progression settle port (ADR-072)"]
  week --> settle
  combo --> settle
  bonuses["phase 2: the day's daily bonuses, backlog-zero among them"] --> combo
  settle --> badges["phase 7: the focus badges, awarded through the badge port"]
```
