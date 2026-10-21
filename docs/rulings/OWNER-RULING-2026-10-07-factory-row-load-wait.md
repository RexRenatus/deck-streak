The owner's decision, carried by the owner's signature on the commit that adds this file: the card factory rows of #682 are judged by the load wait each row already asserts, one more fix round is admitted beyond the round budget, and one test-only keeper view is admitted under the warm-up bound.

# OWNER RULING 2026-10-07: the card factory rows are gated by their own load wait

Cited by SPEC-361 and ADR-372. Card scripts stay switched off on iOS (SPEC-361); this ruling changes no switch. Nothing that depends on it lands before the signed commit of this file is on `dev`.

## What was held

The land gate of #682 read the `apple / harness` job by destination and held every factory-row reading (`took=`) other than the two warm-ups under 3 s on both destinations. That bound was set while the factory built every row's view before it timed the first, so twelve of its thirteen rows read about zero.

## What was measured

The factory now builds each row's view just before that row's own wait (ADR-372 D11), so every row measures its own load. At head `a6234fc4` the job passed, 16 tests and 0 failures per destination, and read:

| destination (xcodebuild order) | factory rows | at or above 3 s | slowest |
|---|---|---|---|
| first, iPhone | 13 | 1 | 3.50 s |
| second, iPad | 13 | 6 | 10.15 s |

The 10.15 s row ran past its own 10 s wait (`loadSeconds`) and read loaded only by the wait's final check after the bound.

## What it closes

Whether the factory rows are judged by a fixed 3 s bound. Once every row measures its own load, the 26 loads ran 0.54 s to 10.15 s, and a fixed 3 s bound judges each destination's start-up as much as the card. The wait each row already asserts judges the card.

## What replaces it

- Every factory row loads within its own `loadSeconds` wait (10 s, unchanged) by that wait. None may read loaded only by the final check after the bound.
- It holds on both destinations, on two `apple / harness` runs of the same head.
- Unchanged: `loadSeconds` and every other wait, timeout and bound; no skip, retry or expected-failure mark; every row's assertions (ran, verdict, loaded); the warm-up; the planted rows, the layer map and the blind set.
- Admitted by name: one keeper card view, test-only. The factory builds it with scripts off. It loads before the first row under the existing warm-up bound, stays open across every row and is removed after the last. It is not a row and is not timed as one, and it changes no row's wait, reading or assertions. Its own load is asserted, so it only adds a way to fail.
- The iPad row at 10.15 s fails this bound too. After one read-only look at why it is slow, one more fix round, with the verify that judges it, is admitted beyond the round budget. A cure that changes anything in the list above needs a new signed ruling.

## Why it is admitted

It is a weakening by the letter, because the bound on the readings moves from 3 s to the 10 s wait. In effect the gate becomes the assertion each row already makes, read strictly: a row that loads only after its wait fails, as it must. The rejected alternatives:

- **Keeping 3 s.** The build that met it measured nothing for twelve rows, and an honest per-row load does not meet it on the iPad.
- **Raising `loadSeconds`.** It moves the wait itself, so a slower load would pass.
- **Timing fewer rows.** It leaves rows unjudged.
- **A bound per destination.** It sets the bar by a reading, not by what each row asserts.
- **Making the timing an advisory.** It drops the gate.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
