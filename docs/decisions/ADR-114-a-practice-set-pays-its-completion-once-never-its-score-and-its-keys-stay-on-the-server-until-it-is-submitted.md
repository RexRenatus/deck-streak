---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A practice set pays its completion once, never its score, and its keys stay on the server until it is submitted

## Context and Problem Statement

#52 asks for timed LSAT and bar practice sets, every choice explained, answers hidden until the owner
attempts, and XP for completion. The pack format carries each item's key inside its heading's
marker, and the model writes the key. XP has one writer, the grant port (SPEC-040), and the day
base feeds the multiplying bonuses (SPEC-072 R17). What is paid, how often, and where do the keys
live until the attempt?

## Decision Drivers

- A model's number is advisory until a deterministic rule accepts it; a key is such a number.
- learning-science's `answers-hidden`: an answer shown before the attempt defeats retrieval.
- CHARTER 10: no imposed penalty, and no unbounded faucet.
- An idempotency guard travels with its feature.

## Considered Options (the alternatives it was chosen against)

- A fixed pay once per completed set: chosen, because no model-written key can move the ledger,
  the key sheet stays on the server until submission so the client never holds an answer early,
  and practice stays out of the day base so the pay cannot multiply other bonuses.
- Pay per correct answer: rejected because a wrong key the model wrote would pay a wrong answer and
  refuse a right one, and it rewards guessing over finishing.
- Send the whole set and hide the keys in the screen: rejected because the keys would travel to the
  client, where any inspector shows them, so `answers-hidden` would hold only on the screen.
- Pay nothing: rejected because #52 asks for XP for completion.
- Allow official items cited by location: rejected because DeckStreak cannot show their text, so a
  set would send the owner to a book mid-attempt.
- Count practice in the day base: rejected because each set would also raise the consistency and
  ascendant bonuses, a faucet that requests could feed.

## Decision Outcome

Chosen option: "pay a fixed amount once per completed set, keep the key sheet on the server until
submission, and leave practice out of the day base", because it is the only option where every
number the ledger sees is DeckStreak's own.

- **The pay.** `xp.bonuses.practice_set` from `economy.json`, source `practice:<set id>`, track
  `law`, scope `once`, on the submission's study day. The amount is the owner's to set; the plan
  recommends 20.
- **The keys.** The engine splits an accepted set into questions and a key sheet; only the
  submission's answer carries the key sheet.
- **The clock.** A pace clock counting up against the limit; a late submission completes and pays.

### Consequences

- Good, because the grant port's `once` scope is the whole guard, and a re-submission writes
  nothing.
- Good, because a set can be completed and paid with the AI route off, since nothing after
  generation needs a model.
- Bad, because a set finished carelessly pays as much as one finished well. The count correct is
  shown to the owner, and the pay rewards finishing, which is what the issue asks.

### Confirmation

SPEC-114's A8, A9, A11 to A13 and A17, and its rows S11406 to S11410.

## What would make this wrong

- An owner who wants accuracy rewarded: a bonus on a verified score would need keys the engine can
  check, such as official items with a licensed key, which is its own decision.
- A request pattern that farms sets: the pay is per completed set, and the caps and one run at a time
  bound generation; W7's XP re-pricing (#281) owns the amount.

## More Information

SPEC-114, SPEC-040 (the grant port), SPEC-072 R17 (the day base), ADR-113 (requested duties), and the
W6 schematic `docs/schematics/w6-duty-run-and-its-degradation.md`.
