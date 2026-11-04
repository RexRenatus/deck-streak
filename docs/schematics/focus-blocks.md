# Schematic: a focus block in the study clients, and its one timer

Kind: state machine and data flow. Read at DeckStreak `dev` from `crates/focus/src/lib.rs`,
`web/app/src/routes/study/+page.svelte`, `ios/App/Sources/ReviewView.swift`, ADR-079 and
`docs/schematics/focus-timer-state-machine.md`. Added by SPEC-607 (ADR-628, ADR-629).

## The block's states

A block runs in the web app or the iPhone and iPad app, on the service's one timer row. Its length
counts minutes or cards. The end is soft: the card in hand is finished before the block offers to
continue, and a card is never taken off the screen mid-answer. A block writes nothing to a card
beyond the learner's own answers.

```mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: start, a new generation, the length clamped to the entry range
  Running --> Paused: pause keeps the remaining seconds
  Paused --> Running: resume sets a new end from them
  Running --> Running: add five minutes moves the end by the minutes added
  Running --> Finishing: the end instant passes, or the block's last card is shown
  Finishing --> Offering: the card in hand is answered
  Offering --> Next: continue
  Offering --> OnBreak: take the suggested pause, or the cycle's break
  Offering --> Ended: stop
  OnBreak --> Next: the break ends, or the learner skips it
  OnBreak --> Ended: stop
  Running --> Ended: stop, or the daily limit is reached after the card in hand
  Paused --> Ended: stop
  Next --> Running: a new generation from the card states the last block left
  Ended --> Idle: the claim, the block row and the idle state in one write
  Idle: idle
  Running: running
  Paused: paused
  Finishing: finishing the card in hand
  Offering: offering to continue
  OnBreak: on a break
  Ended: ended
  Next: the next block, chained
```

| state | what the client shows | what the service holds |
|---|---|---|
| idle | the picker: presets or a typed length, in minutes or in cards | the row idle |
| running | the block's own time left and its progress, never a date, a day count or days left | the row running, its generation, its end instant |
| paused | the time left, held | the remaining seconds |
| finishing the card in hand | the card, untouched | the block claimed at its end, its row in the focus log |
| offering to continue | continue, a pause, or stop; no copy urges | the row idle, or a cycle's next block |
| on a break | the break's own time left | a break block; the scheduler's clock runs on for cards waiting on a learning step |
| ended | the block's result, from the service's answer | the claim, the block's row and the next state, committed together |
| the next block, chained | a new block from the card states the last one left | a new generation; due cards the last block did not reach stay due |

A block counted in cards reaches its soft end when its last card is shown, and the client reports
the count reached; the service claims it by the generation, as a stop claims. Its fallback end
instant is the entry cap after its start.

## The one timer, from the service's clock to both clients

```mermaid
flowchart LR
  clock["the service's clock"] --> row["focus_timer: one row, its generation and end instant"]
  apirole["the api role arms the timer, claims once by the generation"] --> row
  row --> claim["the claim, the block's focus_log row and the next state, one write"]
  claim --> log["focus_log: minutes, abandoned or certified"]
  log --> rows["the focus rows: block XP, the day's cap, the weekly goal, the combo, the focus streak, the badges"]
  rows --> settle["progression's settle port: a minute pays XP once"]
  row --> focusapi["GET /api/focus: the end instant and the service's own instant"]
  focusapi --> web["the web app counts down from the service's end"]
  focusapi --> ios["the iPhone and iPad app counts down from the service's end"]
  web --> answers["the learner's own answers, the only card writes"]
  ios --> answers
```

The deprecated bot role arms no focus timer, and the Mini App has no block screen.

## Where the rest guard's store sits

```mermaid
flowchart TB
  subgraph device["one device"]
    session["the open block's own timings: minutes today, minutes since the last break, each answer's time"] --> guard["the rest guard, off until the learner turns it on"]
    guard --> store["the rest guard's device store, apart from the collection that syncs"]
    guard --> hint["an in-session suggestion to rest, never a stop"]
    switch["turning the guard off"] --> erased["the store erased at once"]
  end
  store -.->|never sent| service["the service"]
```

The rest guard reads nothing from a health store or from any other app's data, and no request
either client sends carries a reading (ADR-629).
