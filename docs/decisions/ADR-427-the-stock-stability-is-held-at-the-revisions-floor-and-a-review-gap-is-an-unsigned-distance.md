# ADR-427: the stock stability is held at the revision's floor, and a review gap is an unsigned distance

| field | value |
|---|---|
| status | accepted, with SPEC-413 |
| spec | SPEC-413 |
| issue | #823 |
| builds on | ADR-338, ADR-353, ADR-400 |
| amends | ADR-400 D2 ("The stock stability is the interval at which the revision's forgetting curve reads 0.9"), and SPEC-386 R6: below the floor, the stock stability is the floor |
| measured at | dev `c74b6b96` |

## Context

A formal pass over the isolated FSRS-7 scheduler (#823) ported the crate's pure functions and the engine core's
mapping to Lean, proved what holds, and replayed the proofs' vectors through the real crate. It found:

- **D1, a wrapper defect.** `stock::project` writes the pinned revision's 90-percent interval as the stock stability
  with no floor. A first review rated Again under the default parameters has an interval of `3.827_562_6e-5` days,
  below the revision's own least stability, `0.0001`. The engine stores a stability rounded to four decimal places, so
  it is stored as 0, and the released scheduler the engine runs reads retrievability by dividing by it: NaN at the
  review, 0 after. The formal pass measured 385 of 4800 random default histories stored as 0, and all of them under
  the all-low and all-zero parameter sets (SPEC-413 M1 to M6).
- **D2, a wrapper defect.** `convert::histories` subtracts two `i64` review ids. A difference past `i64::MAX` panics in
  a test build and wraps in a release build, where the scheduler reads it as a gap of 0. Only a corrupt log reaches it
  (SPEC-413 M7).
- **Two upstream properties that do not hold as first stated**, and one observation (SPEC-413 M8 to M10).

The pass touched no product code. This ADR decides the fixes and what stays as it is.

## D1. The floor, and what it was chosen against

`stock::STABILITY_MIN` is `0.0001`, the pinned revision's own least stability (its `S_MIN`, private to the revision's
simulation module at `src/simulation.rs:199`, so it is stated again here). `stock::stock_stability(interval)` returns
`interval.max(STABILITY_MIN)`, and `project` writes `stock_stability` of the 90-percent interval. By `f32::max`, NaN and
negative infinity read the floor, positive infinity stays, and every interval at or above the floor is kept bit for bit.
Only the stability is floored: the interval at the preset's retention already becomes at least 1 day in the engine
core's schedule.

Considered options:

- The revision's own least stability, `0.0001`, chosen because it is the least stability the model that produced the
  state ever holds, and the least positive value the engine's four-place store keeps: the fix moves only values the
  store would read as 0, and nothing else.
- The released scheduler's least stability, `0.001`, rejected because that scheduler already lifts any last stability
  below it to it at the next review (its `src/analytic.rs:219`), so writing it here duplicates a rule the engine owns
  and moves values between `0.0001` and `0.001` that the store keeps and the stock curve reads finite.
- Refusing the card, or the whole replay, rejected because a first Again is the commonest first answer there is, and
  a refusal would leave its card out of the switch.
- Floor in the engine core's `stock_fields`, rejected because `project` is the one place a stock value is made
  (SPEC-386 R6), and `CardReplay::stability` reports the value before any write.
- Rounding up to the store's grid, rejected because it would move every value, not only those below the floor, and
  the store's rounding is the engine's.
- A private floor inside `project`, rejected because the formal vectors could then reach the floor only through a
  memory state, and the Lean port names the floor's own function: `stock_stability` is public so the vectors test
  replays it bit for bit.

## D2. The gap, and what it was chosen against

`histories` reads the distance between two kept reviews as `row.id.abs_diff(at)`, a `u64` of milliseconds, and `days`
reads a `u64`. The rows are sorted by id first, so each id is at least the one before it and the distance is their
difference, exact for every pair of `i64` ids: up to `u64::MAX` milliseconds, which `days` reads as about
`2.1e11` days.

Considered options:

- `abs_diff`, chosen because it is exact for every pair, has no branch and no failure, and leaves the function's
  signature as it is.
- `checked_sub` with a refusal, rejected because `histories` returns no error today, and a refusal would drop a card
  whose distance is merely large rather than wrong.
- `saturating_sub`, rejected because it saturates at `i64::MAX` milliseconds, a distance shorter than the true one.
- Widening to `i128`, rejected because it gives the same value as `abs_diff` with two casts more.

## D3. The formal surface, and what it was chosen against

- **Lean: REQUIRED.** `lean/Fsrs7Stock` gains `the_stock_stability_is_at_least_the_floor_and_stored_above_zero`:
  for every `f32` the interval can be, NaN and the infinities included, the stock stability is positive infinity or a
  value at least `STABILITY_MIN` that the store's four-place rounding keeps above zero, and every interval at or above
  the floor is kept. Its witness `an_unfloored_stability_stores_as_zero` proves the merge-base's projection breaks it
  at the Again interval. The floor is read as its exact `f32` value. The store's rounding is ported over exact
  rationals: the `f32` product inside it is not ported, and A1 asserts the store's rounding on the real `f32`.
- `lean/ReplayHistory` ports the distance as the natural distance of the two ids, and is stamped again. Lean's
  integers do not overflow, so the overflow itself is not a Lean goal: A2 closes it on the real `i64`.
- **TLA+: NOT APPLICABLE.** The replay reads one statement and writes nothing; `stock_fields` writes a card its caller
  owns. No second actor shares state with any span this delivery changes, and the existing full-sync and undo models
  cover paths it does not change.

Considered options:

- One more theorem in `lean/Fsrs7Stock`, chosen because the floor is part of the same projection the entry already
  ports, and its witness is the merge-base's own `project`.
- A new entry for the floor, rejected because it would cover `project` a second time beside the entry that ports it.
- A TLA+ model of the replay, rejected because the replay has one actor and no shared state; a model would explore one
  path.

## D4. The upstream findings, and what it was chosen against

- **The curve never reads 1** (SPEC-413 M8): its last step maps the mixture onto `1e-5` to `1 - 1e-5`. The proved
  claim is that band, with `1 - 1e-5` exactly at a gap of 0 or less.
- **The interval is not monotone in stability** (M9): the slow component's weight grows with stability, so a larger
  stability can move the mixture towards a slower curve. Measured on the default and the all-high parameter sets;
  whether real histories reach those states is not established.
- **The retention is passed unchecked** (M10): the revision clamps it, so every review card's interval is at least a
  day and at most the preset's maximum.

Considered options:

- Record them in SPEC-413 and here, and change nothing upstream, chosen because each is the revision's arithmetic or
  its stated design, and DeckStreak's wrapper is not where a guard for them belongs.
- A wrapper rule that makes the interval monotone in stability, rejected because it would invent a schedule the
  revision does not compute.
- Validating the retention in the replay, rejected because the retention is the caller's contract (ADR-400), and the
  revision's clamp already bounds every output.

## D5. The tests, and what they were chosen against

Red first. A1 runs the projection of the Again state over a stub of `stock_stability` that returns its input, and
reads the stored stability as 0. A2 runs `histories` over the extreme ids and reads the overflow. A3 replays the new
stability vectors over the same stub. The fix then lands alone. SPEC-386 A7 and A8 keep their tests: their Again row
now expects the floor.

Considered options:

- Typed expectations, chosen because the floor's bits and the store's rounding of them are written by hand, never read
  from the crate.
- Asserting only that the stored stability is not 0, rejected because a mutant that moves the floor to another
  positive value would survive it.

The replay of `lean/Fsrs7Schedule`'s vectors through the engine core's `scheduled` and `day_of` is owed by #823: this
delivery adds no engine-core test.

## Consequences

- A card whose 90-percent interval is below `0.0001` days reports and is written with a stock stability of `0.0001`,
  which the store keeps and the stock curve reads finite. Every other value is unchanged.
- `histories` is total over every row set: no build panics and none reads a wrapped gap.
- Four Lean entries hold the scheduler's projection, replay, schedule and selection, each replayed by the fsrs7
  crate's tests but the schedule's.

## Confirmation

SPEC-413 A1 to A4, the mutation rows of section 9, and the formal checker's per-entry and ratchet checks at the
delivery's head.

## What would make this wrong

- A revision that lowers its least stability below the store's grid: `STABILITY_MIN` would then be above the model's
  floor, and the floor would need its own reason.
- An engine that stores the stability at fewer places: the floor would no longer be a value the store keeps.
