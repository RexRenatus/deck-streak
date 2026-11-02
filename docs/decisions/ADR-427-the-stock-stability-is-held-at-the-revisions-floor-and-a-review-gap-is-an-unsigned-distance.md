# ADR-427: the stock stability is held at the revision's floor, and a review gap is an unsigned distance

| field | value |
|---|---|
| status | accepted, with SPEC-413 |
| spec | SPEC-413 |
| issue | #823 |
| builds on | ADR-338, ADR-353, ADR-400 |
| amends | ADR-400 D2 ("The stock stability is the interval at which the revision's forgetting curve reads 0.9"), and SPEC-386 R6: below the floor, the stock stability is the floor |
| measured at | dev `c74b6b96`; D6 and D7 after the merge of dev `e25c0c45` |
| related | #874 (lean warnings as errors and an escape-hatch guard), #897 (formal parity in the fitness checks), `crates/engine-core/src/scheduler.rs::day_queue` (SPEC-494 R6-R7, a second builder of the stock queue with no production caller, not ported here) |

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

The replay of `lean/Fsrs7Schedule`'s vectors through the engine core's `scheduled` and `day_of` was owed by #823 when
this decision was first written; D6 delivers it after the merge of dev.

## D6. The pure seam for the schedule's vectors, and what it was chosen against

Since the scheduling port (SPEC-494, ADR-512), `crates/engine-core/src/replay.rs` reaches a review card's schedule in
two steps: the port's `interval_days` returns the whole days of the projected interval, and the private `scheduled`
holds them to the preset's maximum and to at least one day, due that many days after the private `day_of` of the last
kept review. Together they compute what the fork's single `scheduled` did, so `lean/Fsrs7Schedule` was re-ported over
the two steps and its claims held unchanged (SPEC-413 A5 is "not red" for that reason).

The vectors test `crates/engine-core/tests/formal_vectors_fsrs7_schedule.rs` reaches the steps through three
`#[doc(hidden)] pub` functions of `replay.rs`, each a pure function of its arguments:

- `whole_days(f32) -> u32`, the rounding step itself, moved out of `interval_days`, which now returns
  `whole_days(interval)`. The text `let whole = interval.round() as u32;` lives only there, so the S49400 row on it
  (S49415, `S49400-S49499.json`) still finds it once and its killer still reaches it through the port.
- `schedule_of`, which calls `scheduled` and reads its refusal as `None`, and `engine_day_of`, which calls `day_of`.
  Neither copies a line of the function it calls, so `scheduled`'s `let ivl = rounded.min(max_ivl).max(1);` (rows
  S38609 and S38614, `S38600-S38699.json`) stays present and unique, and every other mutation row on `replay.rs`
  (21 rows, counted by a census of `scripts/mutation-rows.d/`) still finds its text exactly once.

Considered options:

- Three hidden pure seams, chosen because the vectors hold `f32` intervals no review history yields (NaN, both
  infinities, values past `u32::MAX`), and each seam is the code the replay runs, not a copy of it.
- Driving the vectors through `Dispatcher::replay`, rejected because it takes review rows and a collection: no row set
  produces a NaN or an infinite interval, and `interval_days` takes a folded state, so the rounding step cannot be fed
  a vector's interval.
- Making `scheduled` and `day_of` themselves public, rejected because it moves the spans `lean/Fsrs7Schedule` covers
  for no gain, and puts two engine helpers in the crate's public surface without the hidden marker.
- A copy of the rounding in the test, rejected because a test that recomputes the code's answer proves nothing about
  the code, and a second copy of the text in `replay.rs` would leave S49415's find text no longer unique.

## D7. The update's bounds, the port's state, their witnesses, and what they were chosen against

After the merge of dev, #823 owed two more claims (SPEC-413 R10, R11), each over code that already holds them.

**The update's bounds (L12, `lean/Fsrs7MemoryState`).** The pinned revision's `stability_for_set_scalar`
(`src/model_v7.rs:120-144`) is ported over its operations, with `exp`, `pow` and the weights abstract and held to
stated laws: `exp` positive, monotone and 1 at 0, so `exp x >= 1` for `x >= 0`; `pow` positive on a positive base;
every weight at least 0, the floor `FSRS::new`'s clip gives all 34 (`src/parameter_clipper_v7.rs:31-47`). Without
`exp 0 = 1` the success bound does not follow: it needs `exp((1-r)w) - 1 >= 0` (the check's T6). The lapse bound
needs no law. The 540 update lines are replayed through upstream's public `FSRS::next_states_with_elapsed_days`.

**The port's state (L13, `lean/Fsrs7PortState`).** TLA+ was declined (main's Q1: one caller, a local state), so the
state's discipline is proved in Lean over a port of `new_state`, `fold`, `same_model`, `check_event`, `checkpoint`
and `restore`:

- an event's fields are their bit patterns, below a power of two, and a key is read signed, as Rust's `i64` order
  reads it; the fold makes every check before its first write;
- the state's `OnceCell` memo is left out: every fold resets it, and it is read only through `replayed`;
- the checkpoint is a list of bytes: `to_le_bytes` is `le`, an integer's low bytes first, and `from_le_bytes` is
  `readLe`, so the round trip is proved over the naturals (`readLe (le w n) = n % 256^w`), with no bit-vector
  decision procedure and no native evaluation (the check's T9);
- `restore` keeps the code's order of checks and the reader's `take`, `array` and `event`; the `usize` arithmetic of
  the expected length (`try_from`, `checked_mul`, `checked_add`) runs under a parameter `usizeMax`, 64 bits on a
  desktop target and 32 on wasm, and the claim asks only that the checkpoint fits;
- the round trip is per model (the check's T5): the checkpoint holds no parameters, `restore` folds into the model's
  own new state, so a state made under other parameters does not round-trip.

The entry covers every item the round trip and the refusal rest on (the check's T8), so an edit to the reader stales
the claim. Its 38 vectors are replayed through `Fsrs7`'s public `SchedulerPort` methods.

**The witnesses (main's Q2).** No claim here is violated by the merge-base, so each witness is a planted variant named
for the defect class it stands for, and each is recorded here:

| claim | witness | the planted variant |
|---|---|---|
| `a_success_never_lowers_the_stability` | `a_growth_factor_without_its_one_lets_a_success_lower_the_stability` | the growth factor with no `+ 1.0` |
| `a_lapse_never_raises_the_stability` | `a_lapse_that_keeps_the_larger_stability_raises_it` | a lapse that takes the larger stability, not the smaller |
| `a_refused_event_leaves_the_state_unchanged` | `a_fold_that_writes_before_it_checks_keeps_a_refused_event` | a fold that writes the event before its checks |
| `the_folded_keys_strictly_increase` | `a_check_that_admits_an_equal_key_folds_it_twice` | a check that refuses only a key before the last |
| `restoring_a_checkpoint_gives_the_state_back` | `a_restore_reading_another_format_refuses_the_new_states_checkpoint` | a restore that expects format byte 2 |

The plant control for the formal stage: the format witness switched to a positive statement reads
`FORMAL FAIL WITNESS_VACUOUS`, exit 1; the witness's restore switched to the port's own fails the module's build,
`FORMAL VOID MODEL_ERROR`, exit 3 (`docs/red-first/SPEC-413.md`).

Considered options:

- Planted-variant witnesses recorded here, chosen because main ruled them for a verification issue whose code
  already holds (Q2 A), and each variant is the defect a later edit would bring.
- Only the claims the merge-base violates, rejected because it would drop every claim of #823 but the floor (L4),
  and an issue that asks for a formal pass would close with most of its families unproved.
- Each claim marked NOT APPLICABLE for its witness, rejected because the formal checker would then judge the claims
  with no planted defect at all, and a claim that no variant breaks may be vacuous.
- TLA+ for the port's state, rejected by main (Q1): one caller holds the state, nothing interleaves, and the TLA+
  pack declines a strictly sequential carry-over.
- The checkpoint's codec over core bit vectors and `bv_decide`, rejected because `bv_decide` closes by native
  evaluation and adds an axiom (the check's T9), which the formal checker refuses.

## Consequences

- A card whose 90-percent interval is below `0.0001` days reports and is written with a stock stability of `0.0001`,
  which the store keeps and the stock curve reads finite. Every other value is unchanged.
- `histories` is total over every row set: no build panics and none reads a wrapped gap.
- Five Lean entries hold the scheduler's projection, replay and update, schedule, selection and port state, each
  replayed by a Rust vectors test: the fsrs7 crate's for the projection, the replay and the selection, and the
  engine core's for the schedule and the port state.
- `replay.rs` carries three hidden public functions (D6) that no production caller uses.
- #823's "the interval it derives" is partly proved: the whole-day mapping is proved, and the revision's Newton and
  bisection derivation (L14, the interval monotone in desired retention) stays a recorded gap, accepted by main in
  writing.
- The witnesses of D7 are planted variants, which phx-formal-lean's witness rule does not yet name; main filed that
  as skill feedback.

## Confirmation

SPEC-413 A1 to A7, the mutation rows of section 9, and the formal checker's per-entry and ratchet checks at the
delivery's head.

## What would make this wrong

- A revision that lowers its least stability below the store's grid: `STABILITY_MIN` would then be above the model's
  floor, and the floor would need its own reason.
- An engine that stores the stability at fewer places: the floor would no longer be a value the store keeps.
- A checkpoint format that carries the parameters, or a `restore` that folds into a state it was handed: D7's per-model
  round trip would then be weaker than the code, and the claim would need restating.
