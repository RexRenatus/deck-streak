# Schematic: streaks, freezes, the governor and the relight

Kind: state machine. Read at the predecessor's `27ee2bc` (`gamification/streaks.py` gap outcomes and
`decay_on_lapse`, `governor.py`, the relight ritual) and at DeckStreak `main` 769ee62 (`streaks`
context). Constants: start freezes 1, cap 3, one freeze per 7 streak days, comeback minimum 7,
strength half-life 13 days, standby below 60 percent, lapse after 3 non-skip zero-review days,
relight on 3 reviews for 100 XP once.

```mermaid
stateDiagram-v2
  [*] --> Active: first study day (bootstrap)
  Active --> Active: study day, 0 real misses (continue); +1 freeze every 7th day, cap 3
  Active --> Active: 1 real miss and a freeze banked (consume 1)
  Active --> Active: declared skip day (bridged, free)
  Active --> Broken: 2 or more real misses, or 1 miss with no freeze
  Broken --> Active: next study day starts at 1 (a 7+ day break arms the comeback)
  note right of Broken: a non-study day zeroes the streak only once it is provably unsavable
```

```mermaid
stateDiagram-v2
  [*] --> Armed
  Armed --> Standby: habit strength below 60 percent
  Standby --> Armed: strength back at 60 percent or more
  Armed --> Lapse: 3 consecutive non-skip zero-review study days
  Standby --> Lapse: 3 consecutive non-skip zero-review study days
  Lapse --> Relit: a study day with 3 or more reviews (+100 XP once, lapse id closed)
  Relit --> Armed
  Relit --> Standby
```

In Standby, penalties refuse, wagers void with a refund and markets hide. In Lapse, the whole
discipline engine is silent, digests and morning messages switch to comeback mode, and the lapse
id keys the comeback cap (three messages) and the one comeback reading. The law streak counts days
with a law review, is bridged by skip days, has no freezes, and (unlike the predecessor, whose law
row never decayed) decays on the same unsavable rule: the predecessor's defect is fixed, not ported.
