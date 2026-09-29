# Schematic: the smoke bomb's spend and the chest lock's effect

Kind: data flow. Read at DeckStreak `dev` af0693f and at the predecessor's `27ee2bc`
(`pipeline_layers/loot.py:LootLayer._grant_session_chests` and `_maybe_earn_smoke_bomb`). Added by
the W5 architect turn. SPEC-108 builds it.

It extends two schematics without changing them: `docs/schematics/quests-and-chests-lifecycle.md`
(a chest's roll inside the fold's phase 4, and the Perfect Week that earns a smoke bomb) and
`docs/schematics/fine-verdict-and-refund.md` (a fine's reversal, whose reason `smoke_bomb` this
spend uses).

## The chest lock in a study day's grant

A lock set by a rung-1 fine (SPEC-104) is asked once, and bites the first chest the day grants.

```mermaid
flowchart TD
  G[the chest step grants for a study day] --> SK{a declared skip day}
  SK -- yes --> N1[no chest, the lock not asked]
  SK -- no --> EL{an eligible session}
  EL -- no --> N1
  EL -- yes --> ASK[ask the lock port once]
  ASK --> S{next session in order of its start}
  S -- none left --> DONE[done]
  S -- a session --> CAP{the day at its cap}
  CAP -- yes --> DONE
  CAP -- no --> SKIP{a chest of its key or one within a session gap}
  SKIP -- yes --> S
  SKIP -- no --> DRAW[take both draws and roll the rarity]
  DRAW --> L{the lock stands and is not consumed}
  L -- yes --> C[store Common, pay a Common payout, pity as after a Common, consume the lock]
  L -- no --> R[store the rolled rarity and its payout and pity]
  C --> W[one write: chest, pity and the lock]
  R --> W
  W --> S
```

- A failed draw writes nothing and consumes nothing; a failed write leaves the lock standing.
- A day that stops at its cap, or skips every session, keeps its lock unconsumed.

## A spend

One transaction, checked in order; a refusal changes nothing.

```mermaid
flowchart TD
  P[the owner confirms a spend for a study day] --> T[begin one immediate transaction]
  T --> A{a spend row for the day}
  A -- yes --> AS[already spent, answer the row]
  A -- no --> WIN{the day in the revision window of the current day and 7 closed days}
  WIN -- no --> R1[refused, outside the window]
  WIN -- yes --> B{a smoke bomb held}
  B -- no --> R2[refused, no smoke bomb]
  B -- yes --> F{a standing fine above 0 coins on the day}
  F -- no --> R3[refused, no pending fine]
  F -- yes --> TAKE[take one smoke bomb]
  TAKE --> REV[reverse each pending fine with the reason smoke_bomb]
  REV --> ROW[write the day's row with the count and the coins]
  ROW --> OK[spent, answer the fines, the coins and the bombs left]
```

- A fine levied on the day after its spend stands; a second spend answers `already spent`.
- A reversed fine is never levied again, so settling or revising the day again books nothing.
