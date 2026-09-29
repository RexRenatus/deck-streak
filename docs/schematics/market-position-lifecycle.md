# Schematic: a market position's life, the markets step and the Oracle's reads

Kind: state machine and sequence. Read at DeckStreak `dev` 26263de and at the predecessor's
`27ee2bc` (`pipeline_layers/markets.py:MarketsLayer`'s `market_board`, `place_position`,
`cancel_position`, `_settle_markets`, `_market_truth`, `_day_survived`, `_void_position`,
`_apply_panic` and `_brier_rank`, and `charts.py:calibration_chart`). Added by the W5 architect
turn, under ADR-107 (a lost position is judged again for 7 closed study days), ADR-104 (a verdict
is provisional until its evidence settles) and ADR-085 (charts are drawn on the client). SPEC-107
builds it.

It extends four accepted schematics without changing them:
`docs/schematics/sync-cycle-and-change-gate.md` (the cycle whose markets step this is),
`docs/schematics/streaks-and-governor-state-machine.md` (the governor that hides the board, and the
freeze markers a `streak_sun` position reads), `docs/schematics/celebration-ladder-on-the-router.md`
(a long shot and a rank-up celebrate) and `docs/schematics/skip-day-record-and-effects.md` (a skip
day voids a position). The panic that calls the void port is
`docs/schematics/wager-and-contract-lifecycle.md`.

## A position's life

```mermaid
stateDiagram-v2
  [*] --> Open: a trade, the board shown, priced again at the trade, stake escrowed once
  Open --> Deleted: cancelled before the outcome day, stake refunded
  Open --> Voided: the engine off, a standby, a lapse, a skip day or too few answers
  Open --> Voided: the panic applies through the void port
  Open --> Voided: the truth cannot be resolved
  Open --> Won: the first cycle after the outcome day, the truth holds, payout credited
  Open --> Lost: the first cycle after the outcome day, the truth fails
  Lost --> Won: judged again within 7 closed study days, the truth now holds, payout credited
  Lost --> [*]: 7 closed study days pass
  Won --> [*]
  Voided --> [*]
  Deleted --> [*]
```

- Every arrow that moves coins moves them in the same transaction as the status, and each status
  changes once: a settlement and a void only from open, a revision only from lost to won. So a
  stake is escrowed, refunded or paid out at most once.
- A cancel deletes the row, as the predecessor does, and the table's ids are never reused, so a
  refund's reference never names a later position.
- The mercy is judged at the settlement only: a lost position is judged again on its truth alone.

## The markets step of a sync cycle

The step runs after discipline's steps, so a panic applied this cycle has already voided the open
positions through the void port.

```mermaid
sequenceDiagram
  participant C as coordination sync cycle
  participant K as markets verdicts
  participant D as discipline and streaks reads
  participant A as analytics and progression reads
  participant E as economy wallet
  participant R as router and celebration ladder
  C->>D: the engine switch, the governor, the skip set, the occurrences, the freeze markers
  C->>A: the outcome days' rollups, the study days with a review, the law track's XP days
  C->>K: the Brier and the rank before the step
  C->>K: settle each open position past its outcome day
  K-->>E: a win credited or a void refunded, once with its status
  C->>K: judge each lost position of the 7 closed study days again
  K-->>E: a revised win credited, once with its status
  C->>K: the Brier and the rank after the step
  K-->>R: a long shot and a rank-up raised at the tick (pending first), each once, no coin moved
```

- While the engine is off, the step voids every open position and refunds each stake, and settles
  nothing.
- The step records each raise pending; an owner's sync raises it through its router at once,
  a scheduled sync leaves it for `discipline_tick`. The digest's Oracle block reports what settled.

## A trade

```mermaid
sequenceDiagram
  participant U as bot or Mini App
  participant C as coordination trade
  participant K as markets verdict
  participant E as economy wallet
  U->>C: key, confidence, stake, outcome day
  C->>C: begin an immediate transaction
  C->>K: the inputs as they stand, the wallet and the open positions
  K-->>C: a refusal with its reason, or the price and the payout
  C->>E: purchase the stake, the position's id as the reference
  C->>C: commit the row and the stake together, or roll both back
  C-->>U: the position or the reason
```

- The price is made again inside the transaction and never read from the caller, so a stale board
  can only be refused.
- Two trades from two connections serialise on the write lock, so the cap of 3 open positions holds
  across processes.

## The Oracle's reads

```mermaid
flowchart LR
  P[market positions, settled won or lost] --> B[Brier over the 30 most recent]
  B --> RK[rank after 10 settled]
  P --> S[summary, digest block, weekly line]
  L[wallet movements of the three market sources] --> S
  P --> CS[calibration series, by settled day and id]
  CS --> CH[chart route oracle_calibration]
  CH --> MA[Mini App markets screen draws it]
```

- The calibration is a JSON series; the chart is drawn on the client and the bot's button opens the
  screen.
- No rank, rise or chart moves a coin.
