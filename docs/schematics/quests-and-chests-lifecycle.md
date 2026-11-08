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

## A chest's grants, from the fold and from the owner's routes (E2b)

Kind: data flow. Read at DeckStreak `dev` `ef4d9077`. Added by SPEC-411 (ADR-425), the first part
of SPEC-081's E2b. Each grant below is made in the write that resolves its chest, its choice or its
window, or not at all, and `xp_ledger`'s once scope turns a replay into a write of nothing.

The fold. The chest step and the sweep step run in phase 4's `DaySteps`, the token-bonus step in
phase 5's `DerivedBonuses` (`crates/coordination/src/recompute/chests.rs`, registered by
`crates/daemon/src/wiring.rs`). The chest step rolls only on a `Settle` or a `Current` evaluation,
and on `Current` only for sessions ended at least `SESSION_GAP_MS` before now, because an open
session's partial base XP would cap its payout. The sweep runs on `Current` only and pays each stale
chest on the chest's own study day.

```mermaid
flowchart TD
  fold["the recompute's fold: one write per study day"] --> phaseFour{"phase 4, DaySteps: which evaluation?"}
  phaseFour -- "a backfill or a revisit" --> rollNone["no roll and no grant"]
  phaseFour -- "Settle or Current" --> chestStep["chest step: sessions ended at least SESSION_GAP_MS before now"]
  chestStep --> grantChests["quests: grant_session_chests_on"]
  grantChests --> chestRows[("the chest rows and the pity counters")]
  grantChests -- "a draw error" --> keepDay["one warning naming the day and the session; the day's other writes stay"]
  phaseFour -- "Current only" --> sweepStep["sweep step"]
  sweepStep --> sweepChests["quests: sweep_stale_chests_on"]
  sweepChests --> grantSwept["progression: grant_on, source chest:id, once, on the chest's own day and track"]
  grantSwept --> ledgerRows[("xp_ledger")]
  fold --> phaseFive["phase 5, DerivedBonuses: every evaluation"]
  phaseFive --> bonusStep["token-bonus step"]
  bonusStep --> bonusRule["quests: settle_token_bonuses_on"]
  bonusRule --> settleBonus["progression: settle, source 2x:id, replaced while open, raised only once closed"]
  settleBonus --> ledgerRows
```

The owner's routes. Each route takes the owner's session first (401), and each POST the state-change
guard (403); the API calls coordination's use case, which holds the one write
(`crates/coordination/src/chests.rs`). The API depends on coordination only, so its answers are
coordination's types. The bot's callbacks (E2b-3) and the Mini App (E2b-2) are dashed: they arrive
later and call the same use cases, the bot directly and the Mini App through the routes.

```mermaid
flowchart LR
  theOwner["the owner"] --> apiRoute["API route: owner session, then the state-change guard on a POST"]
  apiRoute --> useCase["coordination use case: one write"]
  useCase --> openPort["quests: open_chest_on"]
  useCase --> pickPort["quests: settle_epic_choice_on"]
  useCase --> activatePort["quests: activate_token_on"]
  useCase --> inventoryRead["quests: the chest store and the holdings read"]
  openPort --> grantOpened["progression: grant_on, source chest:id, once"]
  grantOpened --> ledgerOwner[("xp_ledger")]
  pickPort -- "freeze, below the cap" --> freezeOn["coordination: grant_freeze_on"]
  freezeOn --> freezeRows[("the freeze rows")]
  pickPort -- "token, or a capped freeze" --> tokenRows[("xp_tokens")]
  activatePort --> tokenRows
  inventoryRead -- "no rarity and no payout for a sealed or vaulted chest" --> apiRoute
  botLater["bot callbacks, E2b-3"] -.-> useCase
  appLater["Mini App, E2b-2"] -.-> apiRoute
```

The odds. `economy.json`'s `chests` is the one source: the quests constants the roll draws from are
pinned to it by `crates/quests/tests/chests_roll.rs:88` (SPEC-411 A20), and the odds page E2b-2
delivers reads the same file, pinned by SPEC-081 A16. No route restates the odds.

```mermaid
flowchart LR
  economyFile[("economy.json: chests")] --> chestConstants["quests: the chest constants, pinned by A20"]
  chestConstants --> theRoll["the roll"]
  economyFile -.-> pageOdds["the odds page, E2b-2, pinned by SPEC-081 A16"]
```
