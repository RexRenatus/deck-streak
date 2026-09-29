---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# A lost market position is judged again for 7 closed study days, and paid when its evidence now holds

## Context and Problem Statement

The self-prediction markets (#119) sell the owner a position on the next study day: at least T
reviews, retention at 85 percent, a kept window, a streak alive through Sunday, a touch of the law
track. The stake is coins, escrowed at the trade, and the position settles at the first sync cycle
after its outcome day: won pays its payout, lost keeps the stake, and a mercy voids it with a
refund.

The predecessor settles each position once and never again
(`pipeline_layers/markets.py:MarketsLayer._settle_markets`, predecessor `27ee2bc`), under a sync
every few minutes, so the outcome day's reviews were nearly always read before the settlement. Its
issue carries that rule as "settle once".

DeckStreak reads reviews once a study day, at the rollover, plus the owner's own `/sync` (ADR-037).
The first cycle after the outcome day can run before another client's reviews of that day reach
the collection, and a settled day's rollup changes when they arrive (ADR-071). A freeze consumed on
the day after a gap, which keeps a `streak_sun` position alive, is written when the streak settles
that day, which can be a cycle later. Settled once, such a position is lost on evidence DeckStreak
had not read, and the owner's stake is kept for a forecast that came true.

ADR-104 made every discipline verdict provisional for 7 closed study days, and refunds a fine when
its evidence fails. A market position is not a fine: the owner chose the stake, and the loss is
the price of a wrong forecast. What happens to a lost position when its evidence later says it came
true?

## Decision Drivers

- A false fine is worse than a missed fine: keeping a stake the evidence no longer supports is the
  same harm as a fine that should not stand.
- ADR-037's cadence is the owner's decision at cutover (#164): the markets must be right at any
  cadence.
- ADR-071: a settled day's rollup changes only with its reviews, so the evidence can change, and
  only in a known way.
- The Oracle's rank scores a forecast against what happened, so the stored outcome should be what
  happened.
- A payout already made is never taken back.

## Considered Options (the alternatives it was chosen against)

- Judge a lost position again for 7 closed study days and make it won, paid once, when its truth now holds — chosen, because it keeps no stake for a forecast the evidence says came true, at any cadence.
- Settle once, as the predecessor and its issue do — rejected because under ADR-037's one daily sync a loss can rest on reviews another client had not yet uploaded, and the stake is then kept wrongly.
- Delay every settlement by 7 closed study days — rejected because the owner would wait a week to see a forecast resolved, and the digest's Oracle block (#129) would report stale positions.
- Revise a lost position to voided with a refund, as a lost wager is — rejected because the evidence says the forecast came true, and a void would drop it from the Brier, which would then score the owner on a forecast that never happened.
- Revise in both directions, a won position to lost as well — rejected because taking back a payout the owner may already have spent is a confiscation outside any stake or fine.
- Revise for as long as a position exists — rejected because the Oracle's rank and the weekly line read a settled set, and ADR-104's window of 7 closed study days is the one the discipline verdicts already use.

## Decision Outcome

Chosen option: "judge a lost position again for 7 closed study days and pay it when its truth now
holds".

- **Settlement.** A position settles at the first sync cycle after its outcome day (SPEC-023 R12),
  as the predecessor's does: the mercy voids first, then the truth, and an unresolvable truth voids.
- **Revision.** At each sync cycle, each `lost` position whose outcome day is one of the 7 closed
  study days before the current study day is judged again from the same inputs as they now stand.
  When its truth now holds it becomes `won`, its revision day is recorded, and its payout is
  credited once, in the same transaction as the status. The mercy is not judged again: a position
  lost on its truth is revised on its truth alone.
- **Finality.** `won` and `voided` are final, and a `lost` position is final once its outcome day
  leaves the 7 closed study days.
- **The celebrations.** A revised long shot celebrates as a settled one does, and a rank that rises
  with a revision celebrates once through SPEC-084's ladder; neither moves a coin.

### Consequences

- Good, because no stake is kept for a forecast the evidence says came true.
- Good, because the stored outcome and the Brier follow the evidence, so the rank measures the
  owner's forecasting rather than the sync's timing.
- Good, because the rule holds unchanged if the owner raises the cadence at cutover.
- Bad, because a lost line in the digest can become a win days later; the revised position is
  reported by the weekly line and the Oracle's summary, not by a message of its own.
- Bad, because it departs from the issue's "settle once", which the owner may prefer to keep.

### Confirmation

SPEC-107's A18 and A29, and the hand-proved rows `S10718` and `S10719`.

## What would make this wrong

- The owner rules that a market settles once, as its issue states: the revision is then dropped,
  and a late review leaves the stake kept.
- The owner raises the sync cadence to minutes at cutover: a revision would then almost never fire,
  though the rule would still hold.
- A market whose evidence can arrive later than 7 closed study days: no such market exists at W5.

## More Information

Cites ADR-037 (the cadence), ADR-071 (a settled day's rollup), ADR-104 (a discipline verdict is
provisional) and SPEC-082 (the wallet's `credit`). SPEC-107 builds it; its schematic is
`docs/schematics/market-position-lifecycle.md`.
