---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# A fine is one economy port keyed by its reference, recorded beside its coins and refunded once

## Context and Problem Statement

Four W5 features take coins as a fine or give them back: the doomscroll rail and its grace (#110),
the confession (#111), a contract's breach and its pardon (#113, #114), and the smoke bomb that
cancels a night's fines (#279). The discipline wave also re-judges a fine when its evidence fails
(ADR-104). SPEC-082 built the wallet's capped debit and its refund, and left the fine's record to
this wave.

The predecessor records each fine in `penalty_ledger`, unique on (study day, kind, reference),
through `database.py:GamifyStore.insert_penalty`, and reverses it with `reverse_penalty`, which
answers true only for the call that reversed it; `pipeline_layers/economy.py:EconomyLayer._debit_fine`
clips the request and writes the coin movement after the penalty row, in two statements (predecessor
`27ee2bc`). Every kind is `tripwire`, so the column carries nothing the reference does not.

Where does a fine live, what is its key, and what does a reversal give back?

## Decision Drivers

- A false fine is worse than a missed fine: a reversal must be exact and automatic, and never
  mint coins beyond what was taken.
- Coins are the only confiscable stake, and they move only through SPEC-082's wallet.
- An idempotency guard travels with its feature: a retried verdict, on any study day, fines once.
- The smoke bomb needs every standing fine of a study day, whichever feature raised it.
- ADR-071: a settled study day's value never falls without a recorded reason.

## Considered Options (the alternatives it was chosen against)

- One economy port, `penalty_ledger` beside `coin_ledger`, keyed by the reference alone, written with its coin movement in one transaction, and a reversal that refunds the recorded amount once — chosen, because it is the only option in which every fine has one record, one key and one exact refund.
- The predecessor's key of (study day, kind, reference) — rejected because a verdict retried on a later study day would write a second row for one reference, and the kind column repeats what the reference's prefix already says.
- Each context keeps its own fines (the rail's events, a contract's days) and refunds by a credit — rejected because nothing would then prove that a refund equals what was taken, and the smoke bomb would have to read every feature's table to list a night's fines.
- The coin ledger alone, a fine being a movement of source `fine` and a reversal a matching credit — rejected because a fine the loss cap forgives in full writes no movement, so it would leave no record, and a listing of standing fines would have to pair movements by reference.
- A reversal refunds the requested amount — rejected because a fine clipped by the cap or the floor would then pay back coins that were never taken.
- A reversal deletes the fine's row — rejected because a retried verdict could then levy the fine again, and the export would lose the fine's history.
- The coin movement written after the penalty row in two statements, as the predecessor does — rejected because a crash between them leaves coins taken with no record to refund, or a record of coins never taken.

## Decision Outcome

Chosen option: "one economy port, keyed by the reference", built by SPEC-103.

- `penalty_ledger` is economy's, `STRICT`, unique on the reference. Its reference is opaque to
  economy and carries its feature's prefix (`tw:<event>`, `confess:<event>`, `contract:<id>:<day>`).
- `fine(day, reference, requested)` clips the request as SPEC-082's capped debit does and writes the
  movement and the row in one `BEGIN IMMEDIATE` write; an existing reference writes nothing.
- `reverse_fine(reference, refund_day, reason)` refunds the recorded amount once, under the
  reason's own coin source, and marks the row; a reversed fine is never levied again.
- The reasons are a closed set: `grace`, `pardon`, `revision` and `smoke_bomb`. Which verdict
  reverses, and when, is its feature's (ADR-104).

### Consequences

- Good, because every fine has one record, and a reversal can never give back more than was taken.
- Good, because a retried verdict, on any study day, fines once.
- Good, because the smoke bomb and the digest read one listing for every feature's fines.
- Bad, because the capped debit and the refund must accept their caller's transaction, a change to
  two of SPEC-082's ports.
- Bad, because a reference's prefix is a convention between features: economy cannot check it.

### Confirmation

SPEC-103's A1 to A13: the goldens of `_debit_fine` and the penalty store's functions; one fine per
reference; the movement and the row together; a forgiven fine refunding nothing; a second reversal
refunding nothing; concurrent fines under the cap and the floor; the census that only economy
writes the table. The hand-proved rows `S10301` to `S10307` prove the key, the recorded amount and
the single refund.

## What would make this wrong

- A fine that takes something other than coins: the ten rules forbid it, and this port would have
  to be redesigned rather than widened.
- A feature that needs a partial reversal: the port reverses whole fines only, and a partial one
  would need its own reason and a recorded remainder.

## More Information

Cites ADR-012 (the parity oracle), ADR-071 (a settled day never falls without a recorded reason),
SPEC-082 (the wallet) and ADR-104 (the revision). The schematic is
`docs/schematics/fine-verdict-and-refund.md`.
