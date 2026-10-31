# Red-first record: SPEC-443

SPEC-443 (R1 to R8, A1 to A6). The SPEC, ADR-457 and the schematic were committed first
(`405c709`) and amended to the measured clock seam (`994609a`) before the red commit. A first red
commit, `81dfa1f`, is superseded and records no red: its placement wrote the creation stamp for the
engine's version-1 timing, but the collections these tests build run version-2 timing, so six of
its eight fence runs panicked in the clock module's own self-check rather than on their criteria.

The red commit is `b3158bd`. It holds the two clock modules with `place` real (it selects the
engine's version-1 timing in the same open as its creation-stamp write, then reads the day count and
the next cutoff back through the engine's own timing of today and asserts them) and `pin` a stub that
reads the instant once and answers the clock, writing and checking nothing; the adapter's
`Engine::at_answer_time`, which keeps its instant parameter and stores none, so `answer` stamps the
adapter's own clock; and the criterion tests under the SPEC's names. A1 to A5 each read red there on
its criterion's own assertion, and A6 passes there. Every red below was measured over that commit,
the tree clean, each fence line selecting one test.

```text
A1: cargo test -p deck-streak-engine-core --test exempt -- --exact a_forget_gesture_resets_its_one_card_and_no_other_on_each_side_of_the_cutoff
A2: cargo test -p deck-streak-engine-core --test review_pairs -- --exact the_review_pairs_run_natively_and_no_other_pair_joins_on_each_side_of_the_cutoff
A3: cargo test -p deck-streak-ffi --test exempt -- --exact a_native_forget_tap_resets_its_one_card_on_each_side_of_the_cutoff
A4: cargo test -p deck-streak-ffi --test round_trip -- --exact a_native_press_stamps_each_review_at_the_injected_instant_on_each_side_of_the_cutoff
A5: cargo test -p deck-streak-engine-core --test exempt -- --exact the_core_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock
A5: cargo test -p deck-streak-ffi --test exempt -- --exact the_adapter_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock
A6: cargo test -p deck-streak-engine-core --test exempt -- --exact a_core_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow
A6: cargo test -p deck-streak-ffi --test exempt -- --exact an_adapter_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow
```

```red-first
A1: red at b3158bded1f5777fcb604f1fe023daa92aa28add: assertion `left == right` failed: both cards were in learning before the gesture
A2: red at b3158bded1f5777fcb604f1fe023daa92aa28add: assertion `left == right` failed: a new card's intervals
A3: red at b3158bded1f5777fcb604f1fe023daa92aa28add: assertion `left == right` failed: the answer took the card out of the new queue before the tap
A4: red at b3158bded1f5777fcb604f1fe023daa92aa28add: assertion `left == right` failed: A13: Again and Good each answer their own card once, into the engine's own next state for the grade, each review's id the injected instant and the second made unique by the engine; Again's press leaves the other card new at the queue's head; undecodable states are refused before the engine
A5: red at b3158bded1f5777fcb604f1fe023daa92aa28add: assertion `left == right` failed: the engine's day count and next cutoff on the core's fixture: day 0, the cutoff more than 39 600 s and at most 43 200 s after the clock's instant
A6: not red: pins the engine's rule at the placed start, green at the base (MUTATION COVERAGE)
A1: green at 00521097a8c40b5b9aa73bf593978c4990351209
A2: green at 00521097a8c40b5b9aa73bf593978c4990351209
A3: green at 00521097a8c40b5b9aa73bf593978c4990351209
A4: green at 00521097a8c40b5b9aa73bf593978c4990351209
A5: green at 00521097a8c40b5b9aa73bf593978c4990351209
```

## What each red shows

Each side case runs its criterion's body on the start placed `JUST_BEFORE_SECS` (300 s) before the
cutoff, then on the start placed `JUST_AFTER_SECS` after it; every red below is on the first.

- A1: the core's Forget test found both cards in the day-learning queue before the gesture, left
  `(Some(3), Some(3))` against right `(Some(1), Some(1))`: the Good step crossed the placed cutoff.
- A2: a new card's four intervals read `["<1m", "1d", "1d", "4d"]` against the pinned
  `["<1m", "<6m", "<10m", "4d"]`: the learning steps crossed the placed cutoff and read in days.
- A3: the adapter's head after the answer was empty, `Head { card_id: 0, queue: 0, new: 0,
  learning: 0 }`, where the answered card at the head of the learning queue (`queue: 1, new: 0,
  learning: 1`) is expected.
- A4: the one tuple differed in the review ids alone: their offsets from the injected instant read
  `[24, 28]` against `[0, 1]`, because the stub left the adapter stamping its own clock.
- A5: each crate's fixture, placed 300 s before its cutoff and then handed to the stubbed pin, read
  day 0 with the next cutoff 300 s after the clock's instant, so `(day, more than 39 600 s, at most
  43 200 s)` read `(0, false, true)` against `(0, true, true)`, in the core's test and in the
  adapter's alike.
- A6: both rule cases pass: just before an unpinned cutoff, a Good press moves its new card to the
  day-learning queue, due on day 1.

The whole suites over `b3158bd`: the core's 156 tests, 153 passed and 3 failed (A1, A2 and A5's
core test); the adapter's 50, 46 passed and 4 failed (A3, A4, A5's adapter test, and
`a_native_press_answers_only_its_card_with_its_grade`, whose shared body holds each id to the
instant and failed on the ids alone, `[30, 33]` against `[0, 1]`). The formatter and the lints of
both crates were green.

## The green

The green took three commits after the red, and each edits tests:

- `016e626` un-stubs `pin` in both modules: one timing read of the next cutoff and the rollover
  hour, the rollover moved by whole hours to the last hour boundary at or before twelve hours after
  the instant, the creation stamp written one day before the predicted cutoff, then the day count
  and the cutoff read back and asserted. It lets `Engine::at_answer_time` store its instant, and it
  moves the four named tests onto the clock: their collections pinned, and every time they hand the
  engine the clock's instant. Every fence line passed there.
- `778e096` moves every other scheduler test of the two crates onto its clock: the core's
  `answer.rs`, `dispatch.rs`, `full_sync.rs`, `one_way.rs`, `replay.rs`, `undo_answer.rs` and
  `undo_change.rs`, and the adapter's `review_actions.rs` and `review_pairs.rs`.
- `0052109` gives each clock module one private `read` of the engine's instant, which `place` and
  `pin` both call, and rewords the module's comment to name it. Before it, each module read the
  instant in two places, so moving the tests onto the clock took two commits, `778e096` and
  `0052109`.

`0052109` is the green, the last commit that changes a Rust file. Over it the eight fence lines each
pass, one test selected each, and the whole suites pass: the core's 156 tests and the adapter's 50.
The formatter and the lints of both crates are green, and the censuses that read the edited files
pass: the core's containment and graph targets, the notifications allow-list, and the one-review
rule, which examined 735 Rust files.

## The time reading (R3)

At `0052109`, a search of the tree for `SystemTime`, `TimestampMillis` or `TimestampSecs` followed
by `::now` finds 14 lines under the two crates' test directories, and no other there: the 12 that
stamp a scratch directory's name (the core's `tests/support/mod.rs` and
`tests/support/sync_server.rs`, one each; the adapter's `tests/face.rs`, three, and its
`tests/fixture.rs`, `tests/review_actions.rs`, `tests/review_pairs.rs`, `tests/seam.rs`,
`tests/support/mod.rs`, `tests/support/sync_server.rs` and `tests/voices.rs`, one each), and one in
each clock module, its `read`.

## The native press's ids (R4)

Run once over the green, each review's id is the injected instant, and the second review's is the
instant plus one, made unique by the engine. The literal `[0, 1]` holds those offsets on both sides
of the cutoff.

## Removed assertions

| file | removed | replaced by | class |
|---|---|---|---|
| `crates/ffi/tests/round_trip.rs` | the bracket holding each review's id between two readings of the wall clock | each id's offset from the injected instant equal to the literal `[0, 1]` | replaced-by-stronger |
| `crates/ffi/tests/review_pairs.rs` | `pinned >= MIN_LEAD_SECS`, with the file's own rollover pin | the shared pin's exact check of the day count and the next cutoff (ADR-457 D1) | replaced-by-stronger |

No test is ignored or skipped.

## Rows

The band `S44300` to `S44311` (SPEC-443 section 7) holds twelve rows, each proved by hand over
`0052109`: its mutant installed alone by a one-occurrence replace, its killer run by exact name, and
the source restored and checked before the next. Each was killed at the assertion its row names:
S44300, S44303, S44310 and S44311 at the pin's own check; S44301 and S44304 at the fixture test's
literal window; S44302 and S44305 at the rule case's queue and due; S44306 and S44307 at the native
press's ids; S44308 and S44309 at the placement's own check. Over the adapter's `src/engine.rs`
changes, the generated mutants number 4: 3 caught, and 1 unviable (`at_answer_time`'s body replaced
by a default, which does not compile because `Engine` has no default; S44307 holds the instant it
stores).
