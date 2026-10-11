# SPEC-413: the FSRS-7 stock stability is never below the revision's floor, and a review gap is read whole

- **Wave:** the scheduler's formal pass. **Issue:** #823. **Context(s):** `deck-streak-fsrs7` (`crates/fsrs7`), with
  one comment in `deck-streak-engine-core` (`crates/engine-core/src/replay.rs`).
- **Decided by:** ADR-427 (this SPEC's own: D1 the stability floor, D2 the gap, D3 the formal surface, D4 the
  upstream findings, D5 the tests), under ADR-338, ADR-353 and ADR-400. It amends SPEC-386 R6 and ADR-400 D2 (the
  stock stability below the floor).
- **Schematic:** `docs/schematics/fsrs-7-replay-undo-probe-and-full-sync-choice.md` section 9 (this delivery
  appends it).
- **Status:** delivered by the pull request that adds this file. **Mutation band:** `S41300-S41399`.
  **Changelog:** `changelog.d/formal-fsrs7-verify.md`. **Red-first record:** `docs/red-first/SPEC-413.md`.

Refs read: `DEV` is DeckStreak `dev` `c74b6b9684bf24dc56a83c7005afd52064d2632c`. The pinned scheduler is the
revision the workspace names for `fsrs7`; the released scheduler is the one the engine locks. Every fact below is
read at DEV, or at the pinned revision, at the lines beside it.

## 1. The problem, measured

A formal pass over the isolated scheduler (#823) ported its pure functions to Lean and proved what holds (section
8). It found two defects in DeckStreak's own wrapper, two properties of the pinned revision that do not hold as
first stated, and one observation.

| id | fact | where |
|---|---|---|
| M1 | `project` writes the revision's 90-percent interval as the stock stability, unclamped | `crates/fsrs7/src/stock.rs:34-36` |
| M2 | A first review rated Again under the default parameters has a 90-percent interval of `3.827_562_6e-5` days, the revision's own value | `crates/fsrs7/tests/stock.rs:11`, from the pinned revision's `src/inference.rs:346` |
| M3 | The revision's least stability is `0.0001`, private to its simulation module | pinned `src/simulation.rs:199` |
| M4 | The engine stores a card's stability rounded to four decimal places, so `3.827_562_6e-5` is stored as 0 | the engine's `rslib/src/storage/card/data.rs:94-95`, `:110-113` |
| M5 | The released scheduler divides by the stability to read retrievability, so a stored 0 reads NaN at the review and 0 after it | released `src/inference.rs:60-63` |
| M6 | Of 4800 random histories under the default parameters, 385 project a stock stability that is stored as 0, the least `1.2259376e-8`; under the all-low and the all-zero parameter sets, all 4800 do | the formal pass's probe, not committed |
| M7 | `histories` subtracts two review ids, each an `i64`: a difference past `i64::MAX` panics in a test build, `attempt to subtract with overflow` at `convert.rs:68:66`, and wraps in a release build, which the scheduler then reads as a gap of 0 | `crates/fsrs7/src/convert.rs:68`; the workspace sets no release overflow checks |
| M8 | Upstream: the curve never reads 1, not even at a gap of 0: it reads `1 - 1e-5`, by its last step `mul_add(1.0 - 2e-5, 1e-5)` | pinned `src/model_v7.rs:219-243` |
| M9 | Upstream: the interval is not monotone in stability. Default parameters, difficulty `4.109511`, fast stability `12170.385`, desired retention 0.9: a stability of `0.00015386242` gives `1.5077906` days and `0.00016771005` gives `1.4922663` | pinned `src/model.rs:168` over `src/model_v7.rs` |
| M10 | Observation: the engine core passes the preset's retention to the revision unchecked; the revision clamps it to 0.0001 to 0.9999, so every review card gets an interval of at least 1 day and at most the preset's maximum, never 0 and never NaN | `crates/engine-core/src/replay.rs` `Dispatcher::replay`; pinned `src/model.rs:168-170` |

M7 needs a corrupt review log: an id is an answer's time in milliseconds, so no real collection reaches it. M1 to
M6 are reached by the commonest first answer there is.

## 2. Requirements

R1. **The floor.** `stock::STABILITY_MIN` is `0.0001`, the pinned revision's least stability (M3), stated again
    because the revision keeps its own private. `stock::stock_stability(interval)` is `interval.max(STABILITY_MIN)`:
    an interval at or above the floor is kept bit for bit, one below it, negative infinity and NaN read the floor,
    and positive infinity stays.
R2. **The projection.** `project`'s stability is `stock_stability` of the revision's 90-percent interval. Its
    difficulty and its interval at the preset's retention are unchanged; the engine core's schedule holds that
    interval at 1 day or more, as before.
R3. **The gap.** `histories` reads the distance between two kept reviews as the unsigned distance of their ids,
    `abs_diff`, in milliseconds, and `days` reads an unsigned distance. The rows are in id order, so the distance is
    their difference, exact for every pair of `i64` ids: no build panics and none wraps.
R4. **The proof of the floor.** The Lean entry `lean/Fsrs7Stock` gains the theorem
    `the_stock_stability_is_at_least_the_floor_and_stored_above_zero` over a port of `stock_stability`, and the
    witness `an_unfloored_stability_stores_as_zero` over the merge-base's projection, at M2's interval. It covers
    `stock_stability` and `STABILITY_MIN`, and its vectors carry one line per interval with the port's stock
    stability.
R5. **The replay of the floor.** The stock vectors test replays every stability line through
    `stock::stock_stability`, bit for bit, NaN and the infinities included.
R6. **The selection's port.** `lean/ReplayHistory` reads a review's distance as the natural distance of the two ids,
    as `abs_diff` does, and is stamped again over the changed `histories`.
R7. **The formal pass's entries.** `lean/Fsrs7Schedule`, `lean/Fsrs7MemoryState` and `lean/Fsrs7Stock` land with
    their vectors; the memory-state and stock vectors are replayed by the fsrs7 crate's own tests.
R8. **Upstream stays upstream.** M8, M9 and M10 are recorded here and in ADR-427 D4. Nothing in the pinned revision is
    changed, and nothing is filed anywhere else.

## 3. Acceptance criteria

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1, R2: a first review rated Again keeps a stock stability at the floor, which the store keeps above 0 | the stock stability is `3.827_562_6e-5`, stored as 0 | ADR-427 D1 |
| A2 | R3: a gap past the signed range is read whole | the subtraction overflows | ADR-427 D2 |
| A3 | R4, R5: the stock projection and the curve hold the Lean claims on every vector, the stability lines included | the stability lines are not floored | ADR-427 D3 |
| A4 | R7: the replay holds the Lean claims on every memory-state vector | not red: it replays the formal pass's own vectors over code this delivery does not change | ADR-427 D3 |

```acceptance
A1: cargo test -p deck-streak-fsrs7 --test stock -- --exact a_first_review_rated_again_keeps_a_stock_stability_the_store_holds
A2: cargo test -p deck-streak-fsrs7 --test convert -- --exact a_gap_past_the_signed_range_is_read_whole
A3: cargo test -p deck-streak-fsrs7 --test formal_vectors_fsrs7_stock -- --exact the_stock_projection_and_the_curve_hold_the_lean_claims_on_every_vector
A4: cargo test -p deck-streak-fsrs7 --test formal_vectors_fsrs7_memory_state -- --exact the_replay_holds_the_lean_claims_on_every_vector
```

A1's expected values are typed: the floor's bits and the store's four-place rounding of it, never read from the
crate. A3 prints its examined count and refuses zero. SPEC-386 A7 and A8 keep their tests, whose Again row now expects
the floor as the stock stability (R2), and whose Hard, Good and Easy rows are unchanged.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-413-the-fsrs-7-stock-stability-is-never-below-the-revisions-floor-and-a-review-gap-is-read-whole.md` | document | new: this SPEC |
| `docs/decisions/ADR-427-the-stock-stability-is-held-at-the-revisions-floor-and-a-review-gap-is-an-unsigned-distance.md` | document | new |
| `docs/schematics/fsrs-7-replay-undo-probe-and-full-sync-choice.md` | document | section 9 appended |
| `docs/red-first/SPEC-413.md` | document | new: red-first record |
| `changelog.d/formal-fsrs7-verify.md` | document | new: fragment |
| `crates/fsrs7/src/stock.rs` | `deck-streak-fsrs7` | `STABILITY_MIN`, `stock_stability`; `project` floors its stability |
| `crates/fsrs7/src/convert.rs` | `deck-streak-fsrs7` | the gap is `abs_diff`; `days` reads a `u64` |
| `crates/engine-core/src/replay.rs` | `deck-streak-engine-core` | the comment on `CardReplay::stability` names the floor; no code changes |
| `crates/fsrs7/tests/stock.rs` | `deck-streak-fsrs7` | A1; SPEC-386 A7 and A8's Again row |
| `crates/fsrs7/tests/convert.rs` | `deck-streak-fsrs7` | A2 |
| `crates/fsrs7/tests/formal_vectors_fsrs7_stock.rs` | `deck-streak-fsrs7` | new (formal pass); A3's stability lines |
| `crates/fsrs7/tests/formal_vectors_fsrs7_memory_state.rs` | `deck-streak-fsrs7` | new (formal pass); A4 |
| `formal/lean/Formal/Fsrs7Stock.lean` | formal | new (formal pass); R4's theorem, witness and covers |
| `formal/lean/Formal/Fsrs7StockVectors.lean` | formal | new (formal pass); R4's stability lines |
| `formal/lean/Formal/Fsrs7MemoryState.lean` | formal | new (formal pass) |
| `formal/lean/Formal/Fsrs7MemoryStateVectors.lean` | formal | new (formal pass) |
| `formal/lean/Formal/Fsrs7Schedule.lean` | formal | new (formal pass) |
| `formal/lean/Formal/Fsrs7ScheduleVectors.lean` | formal | new (formal pass) |
| `formal/lean/Formal/ReplayHistory.lean` | formal | R6: the distance's port, the stamp |
| `formal/lean/Formal/Vectors.lean` | formal | three writer arms (formal pass) |
| `formal/lean/Formal.lean` | formal | three imports (formal pass) |
| `formal/vectors/fsrs7-stock.jsonl` | formal | new; written by the checker |
| `formal/vectors/fsrs7-memory-state.jsonl` | formal | new; written by the checker |
| `formal/vectors/fsrs7-schedule.jsonl` | formal | new; written by the checker |
| `formal/vectors/replay-history.jsonl` | formal | the header's digest; written by the checker |
| `scripts/mutation-rows.d/S41300-S41399.json` | rows | new: section 9 |

## 5. What this does NOT do

- It changes nothing in the pinned revision: M8, the curve's ceiling below 1, is the revision's design (#823).
- It does not make the interval monotone in stability (M9): the interval is the revision's own search over its own
  mixture, and a wrapper rule would invent a schedule the revision does not compute (#823).
- It does not validate the preset's retention (M10): that is the caller's contract (ADR-400), and the scheduler
  switch that will call it validates its own input (#611).
- It adds no test in the engine core: the replay of `lean/Fsrs7Schedule`'s vectors through `scheduled` and `day_of`
  is owed by #823, which stays open for it.
- It does not prove the interval monotone in desired retention: the revision's Newton and bisection search is not
  ported, and the property was measured clean, not proved (#823).
- It writes nothing into a collection: the scheduler switch owns the write (#611).
- It does not change the replay's arithmetic, its methods or its parameters (#641).

## 6. Risks

- **The floor moves a value a client reads.** Only intervals below `0.0001` days move, to `0.0001`; A1 and A3 pin
  the floor and the bits of every other value, and SPEC-386 A7 and A8 still pin Hard, Good and Easy exactly.
- **The released scheduler reads a floor below its own.** Its next review lifts any stability below its own least to
  that least (ADR-427 D1), and its retrievability at `0.0001` is finite at every gap.
- **The port drifts from the code.** Each covered item is stamped; an edit to it reads `STALE` at the next check.

## 7. What only CI proves

- The engine core's suites, which this delivery touches by one comment only.

## 8. Formal model

- **Lean: REQUIRED.** The stock projection, the replay's bounds, the schedule and the selection are total pure
  functions. Four entries: `lean/Fsrs7Stock` (the difficulty clamp, the curve's band and fall, and R4's floor),
  `lean/Fsrs7MemoryState` (a replayed state lies in the scheduler's bounds; the batch equals the single replay; the
  clamped rating and gap), `lean/Fsrs7Schedule` (the interval is a day or more and never past the maximum; the due;
  the write is idempotent) and `lean/ReplayHistory` (R6). Each theorem has a witness over the merge-base's code.
- **TLA+: NOT APPLICABLE by surface.** The replay reads one statement and computes in memory; `stock_fields` writes
  a card its caller owns, and nothing writes the collection. No second actor shares state with it. The existing
  models `tla/FullSyncChoice` and `tla/UndoOwnAnswer` cover the full-sync choice and the undo paths, which this
  delivery does not change; the one-preset switch (#611) is not built.

## 9. Mutation rows

Band `scripts/mutation-rows.d/S41300-S41399.json`, from S41300, in the `MUTATIONS` table. Each `find` is written
after `cargo fmt` and occurs exactly once at its target.

| stem | file | mutant | killer |
|---|---|---|---|
| S41300-STABILITY-MIN | `crates/fsrs7/src/stock.rs` | the floor moved off `0.0001` | `stock::a_first_review_rated_again_keeps_a_stock_stability_the_store_holds` |
| S41301-STABILITY-FLOOR | `crates/fsrs7/src/stock.rs` | `project` writes the interval with no floor | `stock::a_first_review_rated_again_keeps_a_stock_stability_the_store_holds` |
| S41302-GAP-WHOLE | `crates/fsrs7/src/convert.rs` | the gap wraps at the signed range | `convert::a_gap_past_the_signed_range_is_read_whole` |
