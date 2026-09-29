# Schematic: a wager's life, a contract's days, a weakening's horizon and the panic

Kind: state machine and sequence. Read at DeckStreak `dev` 26263de and at the predecessor's
`27ee2bc` (`pipeline_layers/discipline.py:DisciplineLayer`'s `arm_wager`, `_settle_wagers`,
`build_contract`, `_contract_verdict`, `_settle_contracts`, `queue_contract_weakening`,
`_apply_contract_changes`, `pardon_latest_breach`, `panic`, `_apply_panic` and
`_sunday_stake_review`, and `database.py:GamifyStore.stake_override_for_day`). Added by the W5
architect turn, under ADR-104 (a verdict is provisional until its evidence settles), ADR-105 (the
discipline tick) and ADR-106 (the money rung waits for the owner). SPEC-106 builds it.

It extends four accepted schematics without changing them:
`docs/schematics/sync-cycle-and-change-gate.md` (the cycle whose stakes step this is),
`docs/schematics/streaks-and-governor-state-machine.md` (the governor that licenses a stake, and the
freeze markers a wager reads), `docs/schematics/notification-router.md` (quiet hours withhold a
nudge) and `docs/schematics/celebration-ladder-on-the-router.md` (a won wager and a finished
contract celebrate). A breach's fine and its refund are
`docs/schematics/fine-verdict-and-refund.md`; the tick that speaks is
`docs/schematics/discipline-tick-and-standby-notice.md`.

## The stakes step of a sync cycle

The order is the predecessor's: the wager settles before the panic applies, and the panic before
the weakenings land and the contract days are judged.

```mermaid
sequenceDiagram
  participant C as coordination sync cycle
  participant W as discipline wager
  participant P as discipline panic
  participant K as discipline contracts
  participant M as markets void port
  participant E as economy wallet and fine port
  C->>W: settle the active wager, governor and markers and skips
  W-->>E: refund or payout, keyed by the wager, once
  C->>W: judge a lost wager again within 7 closed study days
  C->>P: the panic day reached
  P->>W: void the active wager, reason panic
  P->>K: revoke the active contracts
  P->>M: void the open positions
  P-->>E: each stake refunded once
  C->>K: land the due weakenings
  C->>K: judge each unjudged day, 7 closed study days back
  K-->>E: a breach fines the stake in force, once
  C->>K: judge each breach again within 7 closed study days
  K-->>E: a failed breach reversed once, reason revision
```

- Every step and its coins are one transaction, so a crash between two steps leaves neither half.
- While the engine is off, the contract days are not judged and the rail fines nothing.

## A wager's life

```mermaid
stateDiagram-v2
  [*] --> Active: armed while the engine is on and the governor armed, stake debited once
  Active --> Voided: the governor not armed at a settle, stake refunded
  Active --> Voided: the panic applies, stake refunded
  Active --> Lost: a streak break after its start up to its effective end
  Active --> WonEarly: more than 3 flat spots, pays the stake and a half
  Active --> Won: the effective end passes, pays twice the stake
  Lost --> Voided: a gap day gains a late study review within 7 closed study days, stake refunded
  Lost --> [*]: 7 closed study days pass
  Voided --> [*]
  WonEarly --> [*]
  Won --> [*]
```

- A flat spot is a consumed freeze or a skip day inside the term; each adds a day to the end, 3 at
  most.
- A void records its reason: `standby`, `panic` or `revision`. Every refund uses the one key
  `wager_refund` and the wager's id, so a stake is refunded once however many voids are tried.

## A contract and one of its days

```mermaid
stateDiagram-v2
  [*] --> Authored: an offered contract, the engine on, the governor armed, 2 at most
  Authored --> Judging: each closed study day of its term
  Judging --> Done: the first cycle after its end
  Judging --> Revoked: the panic applies
  Done --> [*]
  Revoked --> [*]
```

```mermaid
stateDiagram-v2
  [*] --> Unjudged
  Unjudged --> Flat: a skip day, a lapse, standby, or a freeze consumed the next day
  Unjudged --> Clean: the metric meets the threshold
  Unjudged --> Breach: otherwise, fined at the stake in force
  Breach --> Clean: judged again within 7 closed study days, fine reversed
  Breach --> Flat: judged again within 7 closed study days, fine reversed
  Breach --> Pardoned: the month's pardon, 2 study days back at most, fine reversed
  Clean --> [*]
  Flat --> [*]
  Pardoned --> [*]
  Breach --> [*]: 7 closed study days pass
```

- A day is judged once for its first verdict, 7 closed study days back at most; a clean or flat day
  is never judged a breach later, and a pardoned day is final.

## A weakening's horizon

```mermaid
stateDiagram-v2
  [*] --> Pending: a lower stake, 10 at least, landing 4 study days on, before the end
  Pending --> Cancelled: the owner cancels at once
  Pending --> Applied: its landing day, the contract still active
  Pending --> Cancelled: its landing day, the contract revoked or done
  Applied --> [*]
  Cancelled --> [*]
```

- A day before the landing day keeps the old stake: the stake in force on a day is the old stake of
  the earliest applied change that landed after it.

## The panic and the re-arm

```mermaid
stateDiagram-v2
  [*] --> On
  On --> Scheduled: panic, the next study day stored
  Scheduled --> On: the owner cancels before the rollover
  Scheduled --> Off: the first cycle on that study day voids and revokes
  Off --> On: the owner re-arms
```

- Nothing is voided while the panic is only scheduled, so a stake armed on its eve stands for it.
- While off, arming, authoring and a second panic are refused.
- No state here moves real money: every stake is coins (ADR-106).
