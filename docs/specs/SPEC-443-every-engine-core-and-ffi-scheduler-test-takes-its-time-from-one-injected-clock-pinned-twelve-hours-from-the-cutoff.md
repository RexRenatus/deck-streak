# SPEC-443: every engine-core and ffi scheduler test takes its time from one injected clock, with its collection's next cutoff pinned eleven to twelve hours away

- **Wave:** none; a test-reliability fix for the review path's scheduler tests.
  **Issue:** #747 (this is its first part; the pull request says `Refs #747`). **Context(s):**
  engine-core and the native adapter, `ffi`.
- **Decided by:** ADR-457. Read and not changed: SPEC-365 R6 (the adapter's one answer entry),
  SPEC-336 R1 (its one entry point), SPEC-345 A13, A14, A17 and A18, SPEC-348 R1 and SPEC-431 (the
  production clock census, on #897).
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-443.md`.

## 1. The problem, measured

Read at `83846ea3`. Every `git grep` below passes the positive `.` with the excludes
`':(exclude,icase)*drill*'`, `':(exclude,icase)*readings_tree*'`,
`':(exclude,icase)*_standin_checks*'` and `':(exclude,icase)*test_deploy_standin*'`.

- Four tests fail in the minutes before their collection's day cutoff (#747): engine-core
  `exempt.rs` `a_forget_gesture_resets_its_one_card_and_no_other` (queue 1 read for each answered
  card, `exempt.rs:354-358`), engine-core `review_pairs.rs`
  `the_review_pairs_run_natively_and_no_other_pair_joins` (`["<1m", "<6m", "<10m", "4d"]`,
  `review_pairs.rs:80`, `:208`), ffi `exempt.rs` `a_native_forget_tap_resets_its_one_card`
  (`LEARNING_QUEUE`, `exempt.rs:144-152`) and ffi `round_trip.rs`
  `a_native_press_answers_only_its_card_with_its_grade`, whose `timed` assertion brackets each
  review's id between two reads of the test's own clock (`round_trip.rs:455`, `:463`, `:470-472`).
- The engine schedules from its own clock: a learning interval that reaches the next cutoff reads
  in days (`crates/ffi/tests/review_pairs.rs:114-120`, which pins one file's rollover hour for it,
  `:123-141`). The collections these tests build run the engine's version-2 timing: the engine's
  default configuration for a new collection sets `schedVer` 2, `sched2021` and its creation
  offset, and its first timing read writes `rollover` 4, so the next cutoff is the next 04:00 in
  the zone the engine reads and the creation stamp `crt` moves only the day count
  (`review_pairs.rs:117-119` states a version-1 default these collections do not have). A probe
  over the fixtures' own collections, in both crates, under `TZ=UTC` and `TZ=Pacific/Auckland`,
  measured each lever the cutoff has (ADR-457 D1): `rollover` places it on a local hour boundary
  only; `schedVer` 1 selects the engine's version-1 timing, whose next cutoff is `crt` plus whole
  days to the second, and under which the engine answers and refuses the review's bury; clearing
  the creation offset keeps the cutoff on the rollover hour; and a time a test hands the engine
  reaches the review's id only, never the queue.
- `crates/ffi/tests/exempt.rs` reads no clock. Its press (`exempt.rs:97-99`) enters
  `crates/ffi/src/engine.rs:184` `Engine::answer`, which stamps `now_millis()` (`:199`;
  `SystemTime::now()` at `:243-249`) and runs the answer through the core (`:202-204`).
- Under `tests/` of the five members whose manifests name `anki` or `anki_proto` (engine-core,
  ffi, ingest, readings, web-engine: 89 `.rs` files, 0 parked names), `git grep -n -E
  '(SystemTime|Instant|Utc|Local|Date)[[:space:]]*::[[:space:]]*now([^_[:alnum:]]|$)|OffsetDateTime[[:space:]]*::[[:space:]]*now_(utc|local)'`
  finds 32 reads in 26 files: 27 of `SystemTime::now` and 5 of `Instant::now`. In engine-core and
  ffi the 24 `SystemTime::now` reads are 12 times handed to the engine (engine-core `answer.rs:111`,
  `dispatch.rs:74`, `exempt.rs:122`, `one_way.rs:243`, `replay.rs:135`, `review_pairs.rs:132`,
  `undo_answer.rs:79`, `undo_change.rs:131`; ffi `review_actions.rs:60`, `review_pairs.rs:144`,
  `round_trip.rs:142`, `:166`) and 12 stamps that name scratch directories.
- `git grep -n -E 'Timestamp(Secs|Millis)[[:space:]]*::[[:space:]]*now'` finds 7 reads of the
  engine's own clock, all in test code: engine-core `full_sync.rs:56` and `one_way.rs:70`, and
  five in ingest. No production file holds one.
- The core already takes the answer's time as a parameter: `crates/engine-core/src/answer.rs:149-163`
  `answer_request(.., answered_at_millis, ..)`, and the core tests build `CardAnswer` themselves.

## 2. Requirements

R1. **One clock per test crate.** `crates/engine-core/tests/support/clock.rs` and
    `crates/ffi/tests/support/clock.rs` each hold `Clock` (`at_millis`, `at_secs`), `place` and
    `pin`, and the constants `DAY_SECS = 86_400`, `HOUR_SECS = 3_600`, `LEAD_SECS = 43_200`,
    `JUST_BEFORE_SECS = 300` and `JUST_AFTER_SECS = 86_100`. Each operation reads the engine's own
    instant once, `TimestampMillis::now()`, and fixes the clock's instant one millisecond before
    it. `place(collection, start)` selects the engine's version-1 timing on the closed collection
    (`schedVer` 1), writes its creation stamp as `instant_secs + start - DAY_SECS`, closes it,
    reopens it, asserts through the engine's own timing of today that the day count is 0 and the
    next cutoff is `instant_secs + start`, closes it, and answers the clock. `pin(collection)`
    keeps the engine's version-2 timing on the closed collection (`schedVer` 2), reads the engine's
    next cutoff and rollover hour once, sets the rollover hour that makes the next cutoff the last
    local hour boundary at or before `instant_secs + LEAD_SECS`, predicted from that reading in
    whole `HOUR_SECS`, writes the creation stamp as the predicted cutoff less `DAY_SECS`, closes
    it, reopens it, asserts through the engine's own timing of today that the day count is 0 and
    the next cutoff is the predicted one, closes it, and answers the clock. Each module is the one
    reader of time in its crate's scheduler tests.
R2. **Every scheduler collection is pinned.** Engine-core's `support::synthetic` pins the
    collection it builds and carries its `Clock` as `Synthetic::clock`; every engine-core and ffi
    test that builds its own collection for the engine to schedule pins it before an engine opens
    it.
R3. **Every time a scheduler test hands the engine comes from its clock.** In engine-core and ffi
    tests, `CardAnswer`'s answered time, a `DeckTreeRequest`'s `now`, a collection's modified time
    and every review time are the clock's instant or a stated offset from it. No engine-core or ffi
    test reads `SystemTime::now`, `TimestampMillis::now` or `TimestampSecs::now` outside its
    clock module, except the 12 scratch-directory stamps of section 1, which name directories and
    reach no engine.
R4. **The adapter's answer instant.** `Engine` gains a private field holding an optional answer
    instant. `Engine::new` (exported, signature unchanged) leaves it unset, and `Engine::answer` then
    stamps `now_millis()` exactly where it does at the base. `Engine::at_answer_time(message,
    answered_at_millis)`, in an `impl` block the bindings do not export, sets it, and `answer` then
    stamps that instant. `answer` stays the one answer entry and the only place the adapter builds
    an `OwnerAnswer`; `Engine::run` is byte-identical.
R5. **The cutoff proof.** Each named test gains `<name>_on_each_side_of_the_cutoff`, which runs the
    named test's own body twice: on a collection first placed `JUST_BEFORE_SECS` before its cutoff
    and on one first placed `JUST_AFTER_SECS`, each then pinned by the test's own fixture. It asserts
    the named test's exact values on both sides. The native press's body holds each review's id to
    the injected instant, the engine keeping the second id unique, in place of the bracket.
R6. **The engine's rule at the placed start.** In each crate, a collection placed
    `JUST_BEFORE_SECS` before its cutoff and not pinned, after a Good press on its new card, holds
    that card in the day-learning queue due on day 1: `(queue, due) = (3, 1)`, a literal. A placed
    collection that is not pinned serves this case alone: under version-1 timing the engine answers
    but refuses the review's bury.
R7. **No wait and no wall-clock assertion.** No test changed or added here sleeps, waits for a
    time, or asserts on the wall clock or the time of day.
R8. **Production unchanged.** No production file but `crates/ffi/src/engine.rs` changes, and in it
    the one clock read stays `now_millis`, reached only through `answer` with the field unset.

## 3. Acceptance criteria of SPEC-443

| id | criterion | decided by |
|---|---|---|
| A1 | The core's Forget test passes with its collection's cutoff first placed just before and just after the engine's instant (R2, R3, R5) | ADR-457 D1, D2 |
| A2 | The core's review-pairs test passes on each side of the cutoff (R2, R3, R5) | ADR-457 D1, D2 |
| A3 | The adapter's Forget tap test passes on each side of the cutoff, its press on the injected instant (R4, R5) | ADR-457 D1, D3 |
| A4 | The native press stamps each review at the injected instant, the second made unique by the engine, on each side of the cutoff (R4, R5, R7) | ADR-457 D1, D3 |
| A5 | Each crate's fixture, first placed `JUST_BEFORE_SECS` before its cutoff and then pinned, holds the engine's next cutoff more than 39 600 s and at most 43 200 s after its clock's instant, at day 0 (R1, R2) | ADR-457 D1, D2 |
| A6 | Just before an unpinned cutoff, a Good press moves its new card to the day-learning queue due on day 1, in each crate (R6) | ADR-457 D2 |

```acceptance
A1: cargo test -p deck-streak-engine-core --test exempt -- --exact a_forget_gesture_resets_its_one_card_and_no_other_on_each_side_of_the_cutoff
A2: cargo test -p deck-streak-engine-core --test review_pairs -- --exact the_review_pairs_run_natively_and_no_other_pair_joins_on_each_side_of_the_cutoff
A3: cargo test -p deck-streak-ffi --test exempt -- --exact a_native_forget_tap_resets_its_one_card_on_each_side_of_the_cutoff
A4: cargo test -p deck-streak-ffi --test round_trip -- --exact a_native_press_stamps_each_review_at_the_injected_instant_on_each_side_of_the_cutoff
A5: cargo test -p deck-streak-engine-core --test exempt -- --exact the_core_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock
A5: cargo test -p deck-streak-ffi --test exempt -- --exact the_adapter_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock
A6: cargo test -p deck-streak-engine-core --test exempt -- --exact a_core_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow
A6: cargo test -p deck-streak-ffi --test exempt -- --exact an_adapter_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow
```

A1 to A5 are red at the base for a reason that depends on neither the time of day nor the zone:
A1 to A3 by the start placed 300 s before the cutoff by version-1 timing, which the base's fixture
leaves in place; A4 over a constructor stub that keeps its instant and stamps the adapter's own
clock; A5 because the base's fixture leaves its placement, so the engine's next cutoff lies 300 s
after the clock's instant. A6 pins the engine's rule and is green at the base.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/engine-core/tests/support/clock.rs` | 1 | added: the core tests' clock, its version-1 placement and its version-2 pin (R1) |
| `crates/engine-core/tests/support/mod.rs` | 1 | changed: `synthetic` pins and carries its clock (R2) |
| `crates/engine-core/tests/exempt.rs` | 1 | changed: A1, A5 (placed, then pinned), A6; the press on the clock |
| `crates/engine-core/tests/review_pairs.rs` | 1 | changed: A2; its own collection pinned; the tree's `now` on the clock |
| `crates/engine-core/tests/answer.rs` | 1 | changed: answered times on the clock (R3) |
| `crates/engine-core/tests/dispatch.rs` | 1 | changed: answered times on the clock (R3) |
| `crates/engine-core/tests/full_sync.rs` | 1 | changed: answered time on the clock (R3) |
| `crates/engine-core/tests/one_way.rs` | 1 | changed: answered and modified times on the clock; its collections pinned (R2, R3) |
| `crates/engine-core/tests/replay.rs` | 1 | changed: review times on the clock; its collections pinned (R2, R3) |
| `crates/engine-core/tests/undo_answer.rs` | 1 | changed: answered time on the clock (R3) |
| `crates/engine-core/tests/undo_change.rs` | 1 | changed: answered time on the clock (R3) |
| `crates/ffi/src/engine.rs` | 1 | changed: the adapter's optional answer instant and its unexported constructor (R4) |
| `crates/ffi/tests/support/clock.rs` | 1 | added: the adapter tests' clock, its version-1 placement and its version-2 pin (R1) |
| `crates/ffi/tests/exempt.rs` | 1 | changed: A3, A5 (placed, then pinned), A6; pinned, its engine on the clock's instant |
| `crates/ffi/tests/round_trip.rs` | 1 | changed: A4; the native press on the injected instant; the refused answer's time on the clock |
| `crates/ffi/tests/review_actions.rs` | 1 | changed: pinned; the tree's `now` on the clock |
| `crates/ffi/tests/review_pairs.rs` | 1 | changed: the shared pin in place of its own rollover pin; the tree's `now` on the clock |
| `docs/specs/SPEC-443-every-engine-core-and-ffi-scheduler-test-takes-its-time-from-one-injected-clock-pinned-twelve-hours-from-the-cutoff.md` | 1 | added: this SPEC |
| `docs/decisions/ADR-457-scheduler-tests-take-their-time-from-a-clock-read-once-from-the-engine-with-the-cutoff-pinned-and-a-guard-beside-the-clock-census.md` | 1 | added: the decision |
| `docs/schematics/scheduler-tests-take-their-time-from-one-injected-clock.md` | 1 | added: the schematic |
| `docs/red-first/SPEC-443.md` | 1 | added: the red-first record |
| `scripts/mutation-rows.d/S44300-S44399.json` | 1 | added: the band's rows (section 7) |
| `changelog.d/injected-clock-443.md` | 1 | added: the changelog fragment |

## 5. What this does NOT cover

- The production clock reads, which the production clock census lists file by file (#897).
- The skip write's two production reads in `crates/ingest/src/skip_write.rs`, read through the
  kernel's clock by their own delivery (#877).
- The guard that holds every engine member's test files to listed wall-clock reads, beside the
  production census: ADR-457 D4, the second part of this issue, landing after #897 (#747).
- The other engine members' test reads (ingest's eight among them), which that guard lists as
  found, each with its row (#747).
- A clock census for the web client's TypeScript and the native harness's Swift (#879).
- The guard's first judgment of the real tree, which only a run on CI makes (#747).

## 6. Risks

- The engine's timing could read more than the creation stamp for these collections: R1's own
  assertion fails at the first run, naming the day count and the cutoff it read. This fired at the
  build's first run: the collections run version-2 timing, and the assertion read day 1 and the
  next 04:00 cutoff, 85 355 to 85 364 s after the instant, where it asserted day 0 and 300 s. R1
  now places by version-1 timing and pins on version-2 timing, each measured (ADR-457 D1).
- The engine's version-2 cutoff could read more than the rollover hour and the zone's offset at
  the reading, or the zone's offset could change during a run: the pin's own assertion fails at
  that run, naming the cutoff it read and the one it predicted.
- A side case's body could meet an engine refusal under version-1 timing before its pin, at the
  base: a red for the wrong reason, which the build reads at its red commit. The four named bodies
  answer, forget, queue, describe and read the deck tree; the measured refusal is the bury.
- The pin's rollover write is a config row: a test that asserts on the config table sees it, as it
  sees the `rollover` 4 the engine's first timing read writes at the base. Detected by the whole
  suites.
- A collection a test builds and the engine schedules might escape its pin: the guard of part two
  sees no clock read there, so the risk is the base's flake returning in that one test. Detected by
  R5's pattern applied to it, and by the whole suite at a run near a natural cutoff.
- The engine might refuse rather than renumber a second review at one instant: A4 fails at its
  green, and the measured ids are recorded.
- No test asserts the value the adapter's real clock stamps (R7 forbids reading the wall clock). The
  read stays in `now_millis`, which the production census lists, and every press through
  `Engine::new` still runs it.
- The engine's instant stays the wall clock's; only its distance from the cutoff is fixed, to
  within one hour.

## 7. Mutation rows (S44300-S44399)

| id | target | mutant | killer |
|---|---|---|---|
| S44300-CORE-PIN-STAMPS-A-DAY-EARLY | `crates/engine-core/tests/support/clock.rs` | the pin writes the creation stamp two days before its predicted cutoff, not one | `the_core_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |
| S44301-CORE-LEAD-SHORTENED | `crates/engine-core/tests/support/clock.rs` | `LEAD_SECS` 43_200 to 39_600 | `the_core_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |
| S44302-CORE-JUST-BEFORE-WIDENED | `crates/engine-core/tests/support/clock.rs` | `JUST_BEFORE_SECS` 300 to 900 | `a_core_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow` |
| S44303-ADAPTER-PIN-STAMPS-A-DAY-EARLY | `crates/ffi/tests/support/clock.rs` | the pin writes the creation stamp two days before its predicted cutoff, not one | `the_adapter_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |
| S44304-ADAPTER-LEAD-SHORTENED | `crates/ffi/tests/support/clock.rs` | `LEAD_SECS` 43_200 to 39_600 | `the_adapter_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |
| S44305-ADAPTER-JUST-BEFORE-WIDENED | `crates/ffi/tests/support/clock.rs` | `JUST_BEFORE_SECS` 300 to 900 | `an_adapter_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow` |
| S44306-ADAPTER-STAMPS-ITS-OWN-CLOCK | `crates/ffi/src/engine.rs` | `answer` ignores the set instant (`filter(\|_\| false)`, every item kept used) | `a_native_press_answers_only_its_card_with_its_grade` |
| S44307-CONSTRUCTOR-DROPS-ITS-INSTANT | `crates/ffi/src/engine.rs` | `at_answer_time` stores no instant (`filter(\|_\| false)`) | `a_native_press_stamps_each_review_at_the_injected_instant_on_each_side_of_the_cutoff` |
| S44308-CORE-DAY-LENGTHENED | `crates/engine-core/tests/support/clock.rs` | `DAY_SECS` 86_400 to 86_401 | `the_core_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |
| S44309-ADAPTER-DAY-LENGTHENED | `crates/ffi/tests/support/clock.rs` | `DAY_SECS` 86_400 to 86_401 | `the_adapter_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |
| S44310-CORE-PIN-HOUR-ONE-LATE | `crates/engine-core/tests/support/clock.rs` | the pin sets the rollover hour one after the hour it predicted from | `the_core_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |
| S44311-ADAPTER-PIN-HOUR-ONE-LATE | `crates/ffi/tests/support/clock.rs` | the pin sets the rollover hour one after the hour it predicted from | `the_adapter_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |

Each killer observes its mutant at every hour of the day and in every zone: a creation stamp two
days back reads day 1 (measured in both zones, on each branch of the engine's day count); a lead
of 39 600 s puts the cutoff at most 39 600 s after the instant, outside the fixture test's literal
window; a rollover hour one late moves the cutoff one hour past the pin's prediction; a side start
of 900 s leaves the Good step short of the cutoff; a day of 86 401 s moves the placement's cutoff
one second before its assertion. Each fixture test places its collection before it pins it, so
the day-lengthened rows fail at the placement's own assertion.

`JUST_AFTER_SECS` takes no row: once the fixture pins, the start it places changes no outcome, and
its role is the regression guard of R5. `HOUR_SECS` takes no row: a mutant of it meets the pin's
own assertion in every hour but the one in which the pin moves the rollover hour by none, so no
killer observes it at every hour.

## 8. Formal model

Not applicable, decided by surface (ADR-457 D6): one test process per collection, no second actor,
no shared state written by two parties, and no check followed by an act another actor can change;
the clock's arithmetic is checked at every run by the engine's own timing. The one `@phx covers`
line that names a file this delivery edits names `crates/ffi/src/engine.rs` item `run`, which
stays byte-identical.
