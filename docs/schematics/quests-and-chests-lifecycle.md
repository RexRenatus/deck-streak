# Schematic: the daily quests, the weekly quest, chests, tokens and the ghost race

Kind: state machine. Read at DeckStreak `dev` c3d769b and at the predecessor's `27ee2bc`
(`gamification/quests.py`, `gamification/chests.py`, `gamification/ghost.py`,
`pipeline_layers/loot.py`, `pipeline_layers/ghost_race.py`, `database.py`'s draws). Added by
SPEC-080 and SPEC-081. Every step here runs inside the recompute's fold (ADR-071,
`docs/schematics/recompute-settles-each-study-day.md`): the quest, chest, sweep and race steps in its
phase 4, the token bonus in its phase 5. The first recompute's backfill of past days runs none of
the rules below that the predecessor applies only to the current study day: it mints no quest,
grants no chest and crowns no day.

## A daily quest slot

The first and second quests are minted at the first recompute that reads their day while it is
current; the challenge quest exists once an offer is picked. A declared skip day voids every slot of
its day (SPEC-080 R13), and undoing the skip returns them.

```mermaid
stateDiagram-v2
  [*] --> Minted: the first recompute that reads the day while it is current
  Minted --> Sealed: the second quest only
  Sealed --> Open: the day has a review, announced only while the day is current
  Minted --> Open: the first quest
  Offered --> Open: the owner picks, or the noon auto-pick once the day's noon has passed
  [*] --> Offered: the challenge offers, minted with the day
  Open --> Complete: is_done over the day's stats, the end-of-day snapshot at the settle
  Complete --> Paid: claimed once, XP or the challenge chest, and 10 coins
  Paid --> [*]
  Minted --> Voided: a declared skip day
  Sealed --> Voided: a declared skip day
  Open --> Voided: a declared skip day
  Voided --> Open: the skip is undone
```

A study day whose three slots are paid is a crown day, recorded once and celebrated once. The
reintroduction draw that can return a suppressed key is the kernel's `draw_bp` (ADR-080).

```mermaid
flowchart LR
  key["a key with no evidence in 30 days"] --> draw{"draw_bp of seed, quest_reintro, key and ISO date, below 500"}
  draw -- "yes" --> offered["offered again that day or week"]
  draw -- "no" --> suppressed["suppressed"]
  suppressed --> floor{"fewer selectable second-quest keys than the floor of 1"}
  floor -- "yes" --> yield["yield back the least recently native key"]
  floor -- "no" --> done["the day's pool"]
  yield --> done
```

## The weekly quest and the Perfect Week

```mermaid
stateDiagram-v2
  [*] --> Open: the week's first recompute, rotated by the week start's ordinal
  Open --> Complete: progress reaches the target
  Complete --> Rewarded: claimed once, the weekly Epic chest, pity untouched
  Rewarded --> Frozen: a freeze if this month's drops are under 1 and fewer than 3 are held
  Rewarded --> [*]: a capped freeze is dropped, with no token instead
  Frozen --> [*]
  Open --> Settled: the next Monday settles the Perfect Week once
  Settled --> [*]
```

A week is a Perfect Week when its crown days are at least its days that are not skip days and at
least 5; it adds one smoke bomb unless 2 are held, and it is recorded settled either way. A smoke
bomb has no use yet (#268), and nothing says it has one.

## A chest

A chest is rolled once, when it is earned: the rarity, the payout and the pity counters after it
are written in one write (ADR-081). Every later recompute reads the row.

```mermaid
stateDiagram-v2
  [*] --> Sealed: earned by a recompute before the vault hour and outside quiet hours
  [*] --> Vaulted: earned by a recompute at or after the vault hour, or inside quiet hours
  Sealed --> Opened: the owner taps Open, once
  Vaulted --> Opened: the owner taps Open, once
  Opened --> Resolved: Common, Rare or Legendary, its payout granted once on its own study day
  Opened --> Choice: Epic
  Choice --> Resolved: a token, or a freeze through the streaks when neither cap refuses it
  Sealed --> Resolved: the sweep on a later study day, payout granted, an untapped Epic pays 50
  Vaulted --> Resolved: the sweep, sparing a vaulted chest of the day before
  Opened --> Resolved: the sweep, an Epic whose choice never came pays 50
  Resolved --> [*]
```

The challenge chest is rolled with 10 Epic points and a 200 XP session base, moves the pity
counters, and is never vaulted; the weekly chest is an Epic without a roll and leaves them alone.

```mermaid
flowchart TD
  roll["a rolled chest"] --> rarity{"rarity"}
  rarity -- "Epic" --> epic["Epic counter to 0, Legendary counter plus 1"]
  rarity -- "Legendary" --> legendary["Legendary counter to 0, Epic counter plus 1"]
  rarity -- "Common or Rare" --> other["both counters plus 1"]
  epic --> write[("one write: the chest row and the counters")]
  legendary --> write
  other --> write
  fail["a failed draw"] --> nothing["nothing written, the next recompute rolls the session"]
```

## A double-XP token

```mermaid
stateDiagram-v2
  [*] --> Held: an Epic's choice, or a freeze refused by a cap
  Held --> Active: the owner activates the oldest held token, one window at a time
  Active --> Consumed: its two hours end, marked at the next recompute
  Consumed --> [*]
```

While a window is open, phase 5 of each settled or evaluated study day settles the token's bonus:
the day's review XP at the base rate inside the window, capped per day, outside the day's base.

## The ghost race

```mermaid
stateDiagram-v2
  [*] --> Minted: the current week's first recompute, from the 26 weeks before, capped
  [*] --> NoGhost: fewer than 4 active weeks
  Minted --> Settled: a later week's recompute, once, unless 2 or more skip days made it a rest week
  Minted --> Rest: a rest week, never recorded
  Settled --> Announced: outside a lapse, the week after, once
  Settled --> Silent: inside a lapse
  Announced --> [*]
  Silent --> [*]
  Rest --> [*]
  NoGhost --> [*]
```

A win is a `ghost_win` celebration, or a quieter `record` after more than three wins in a row; a
loss is told only when the owner studied that week, and names a photo finish only when the gap is
at most 5 cards or 5 percent. A change of the week's leader is told at most once a study day.
