# SPEC-615: the review fixture ends its day twelve hours after it is written

- **Issue:** #779. **Context(s):** the adapter's tests and the review fixture's writer
  (`crates/ffi/tests`, `crates/ffi/examples`), the iPhone and iPad app's review tests that read
  the fixture (`ios/AppTests`, `ios/AppUITests`, unchanged), `scripts/mutation-rows.d` and `docs/`.
- **Decided by:** ADR-637 (the review fixture pins its rollover hour half a day ahead, and the
  interval label stays as Anki gives it); SPEC-348 R8 (the review fixture) and its A1, A15, A17
  and A19, which this delivery keeps as they are.
- **Schematic:** `docs/schematics/review-fixture-day.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests.
- **Mutation band:** S61500-S61599.

## 1. The problem, measured

Read at this pull request's base on dev by `git show <base>:<path>` and `grep -n`, and in the
engine at the rev `Cargo.lock` pins by `git show <rev>:<path>`:

- The engine labels a learning interval that reaches the scheduler's next day cutoff in days, as
  Anki does: `IntervalKind::maybe_as_days` (`rslib/src/scheduler/states/interval_kind.rs:12`),
  which `describe_next_states` applies to each of the four choices. `crates/ffi/tests/review_pairs.rs`
  lines 113 to 122 record the rule.
- The review fixture is written by `crates/ffi/examples/review-fixture.rs` (65 lines) in the
  `xcframework` job (`.github/workflows/xcframework.yml` lines 30, 31 and 133) and read by the
  `release-and-app` job, whose `xcodebuild test` runs `ReviewModelTests`, `ReviewSessionTests`
  and `ReviewFlowTests` over it (lines 749 to 751 and 1020 to 1030) and uploads its report as the
  artifact `release-and-app` (lines 1372 to 1380). Both jobs run on the same runner label.
- `crates/ffi/tests/support/review.rs` builds the collection by `CollectionBuilder::new(..).build()`
  (line 98). On create the engine writes the collection's conf with `"schedVer": 2`,
  `"sched2021": true` and a `creationOffset` (`rslib/src/config/schema11.rs` lines 25 to 27, written
  by `rslib/src/storage/sqlite.rs` lines 513 to 525), and the schema 14 upgrade copies that conf
  into the config table (`rslib/src/storage/upgrades/mod.rs` line 22), so `scheduler_version()`
  reads version 2 (`rslib/src/config/mod.rs` lines 213 to 216 fall back to version 1 only when the
  key is absent).
- With a rollover hour and a creation offset the engine takes `sched_timing_today_v2_new`
  (`rslib/src/scheduler/timing.rs` lines 159 to 177), whose next day cutoff is the next rollover
  hour on the reading host's own clock (lines 39 to 45); the creation stamp moves only the days
  elapsed (line 48). The client's offset is the host's local one (`rslib/src/scheduler/mod.rs`
  lines 90 to 107).
- The fixture sets no `rollover`, so the engine writes its default hour on the first reading
  (`rslib/src/scheduler/mod.rs` lines 63 to 71). The fixture's day therefore ends at the engine's
  default hour on the reading host's clock, every day, and a test run in the last ten minutes
  before it reads the ten-minute Good step as `1d`. A15, A17 and A19 of SPEC-348 read so on the
  first destination of one run, and passed on the second destination of the same job once the
  step no longer crossed (#779).
- `review_pairs.rs`'s own test over the review fixture first pins the hour farthest from now and
  selects version 2 itself (lines 125 to 143), so no Rust test read the fixture's day as the app
  reads it.

## 2. Requirements

R1. `crates/ffi/tests/support/day_cutoff.rs` holds one public
    `pin(collection: &Path) -> Result<Pinned, Box<dyn Error>>`, written with the engine's own API as
    `support/review.rs` is: `CollectionBuilder` for every open, `set_config_json("rollover", &hour,
    false)` for every write, `timing_today()` for every reading, and `col.storage.db()` for the one
    read of the stored hour (`select val from config where key = 'rollover'`, read as the bytes of
    the engine's JSON text and parsed as a number). It measures each hour of the day as the
    rollover in a fresh open of its own, because the engine caches its day per open collection,
    keeps the hour whose lead, `next_day_at - now`, lies nearest a target of half a day, sets it,
    and reads it back in one more open. The read-back is an error unless the stored hour equals
    the kept one and the lead is more than the target less one hour and at most the target plus
    half an hour. `Pinned` carries the hour, the engine's instant `t` of the read-back and the
    read-back's `next_day_at`. The target is one named constant, and the window is derived from it
    and one hour. Every failure is returned as an error, never a panic.
R2. The pin writes the `rollover` key alone: no scheduler version and no creation stamp. The
    reopened collection's stored `schedVer` still reads `2`, as `CollectionBuilder` writes it, so
    the engine keeps its version-2 timing.
R3. `crates/ffi/examples/review-fixture.rs` pins the review collection only, once, after it builds
    the review collection and before it builds the parity collection. The other three collections
    are written as before.
R4. The interval label stays the engine's: no Swift test, expected interval or app source changes.
R5. On the pinned fixture the text card (`1_000_007`) reads, through the adapter, the four
    intervals A1 of SPEC-348 pins: `<1m`, `<6m`, `<10m` and `4d`. The fixture's card ids are fixed,
    so these engine-computed words are pinned over a fixture whose seed inputs are fixed.
R6. One mutation row, S61500, in `scripts/mutation-rows.d/S61500-S61599.json`: the pin's target of
    half a day replaced by a quarter of a day, killed by A1.

## 3. Acceptance criteria of SPEC-615

| id | criterion | decided by |
|---|---|---|
| A1 | on the review fixture, whose stored `schedVer` reads `2` and whose day is first set to end within the hour, the pin leaves a stored `rollover` equal to its hour and `schedVer` still `2`, and a fresh reading's day ends more than eleven hours and at most twelve and a half hours ahead, at the pin's own cutoff (R1, R2) | `crates/ffi/tests/fixture_day_cutoff.rs` |
| A2 | the pinned fixture's text card reads, through the adapter, the four intervals A1 of SPEC-348 pins (R5) | `crates/ffi/tests/fixture_day_cutoff.rs` |
| A3 | the writer calls the pin once, after the review collection's build and before the parity collection's, and includes it once (R3) | `crates/ffi/tests/fixture_day_cutoff.rs` |
| A4 | the review screen's A15, A17 and A19 Swift tests pass on both destinations over the pinned fixture (R4) | the `apple / release-and-app` job's review tests |

```acceptance
A1: cargo test -p deck-streak-ffi --test fixture_day_cutoff -- --exact the_review_fixture_day_ends_twelve_hours_after_its_writer
A2: cargo test -p deck-streak-ffi --test fixture_day_cutoff -- --exact the_pinned_review_fixture_keeps_the_text_cards_four_intervals
A3: cargo test -p deck-streak-ffi --test fixture_day_cutoff -- --exact the_fixture_writer_pins_the_review_collection_once
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a15_the_loop_reaches_the_designed_end
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_a17_each_rating_shows_its_interval
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewFlowTests/test_a19_the_ratings_sit_at_the_bottom_named_by_their_titles
```

A1 asserts by its own literals, never the support module's constants, so the row S61500 is killed
by it. A4's three lines are SPEC-348's own, read in CI only: the `apple / release-and-app` job runs
the three review test classes over the fixture the `xcframework` job writes, on an iPhone and an
iPad simulator.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-615-the-review-fixture-ends-its-day-twelve-hours-after-it-is-written.md` | docs | added: this file |
| `docs/decisions/ADR-637-the-review-fixture-pins-its-rollover-hour-half-a-day-ahead-and-the-interval-label-stays-as-anki-gives-it.md` | docs | added: the decision |
| `docs/schematics/review-fixture-day.md` | docs | added: the data flow and where the pin sits |
| `changelog.d/review-fixture-day-615.md` | docs | added: the changelog fragment |
| `docs/red-first/SPEC-615.md` | docs | added: the red-first record |
| `crates/ffi/tests/support/day_cutoff.rs` | tests | added: the day pin (R1, R2) |
| `crates/ffi/tests/fixture_day_cutoff.rs` | tests | added: A1 to A3 |
| `scripts/mutation-rows.d/S61500-S61599.json` | rows | added: S61500 (R6) |
| `crates/ffi/examples/review-fixture.rs` | writer | changed: three insertions, the doc paragraph, the pin's module and its call (R3) |

## 5. What this does NOT cover

- The label rule: a learning step that reaches the next day cutoff reads in days, as Anki labels
  it, and stays so; no expected interval is re-pinned to a day (#779).
- The Swift tests and the app's source, which stay as SPEC-348 pins them (#779).
- The other three collections the writer adds, the parity, sound video and card health
  collections, which no test reads a learning interval from (#779).
- The Rust tests that build their own collections and read the engine's clock, which #747 holds.

## 6. Risks

- **The reading job re-run alone.** A re-run of `release-and-app` by itself, about twelve hours
  after the writing job (sooner on a reader whose clock is ahead of the writer's), meets the
  fixture's cutoff again; a run of the whole workflow writes a fresh fixture (ADR-637). Detected by
  A4's tests, which read `1d` again in that window.
- **A reader far from the writer's offset.** A reader whose offset differs from the writer's by
  eleven hours or more can meet the cutoff during its run. Both jobs run on one runner label;
  detected by the same A4 tests.
- **An engine that stops honouring the hour.** A new engine rev whose timing ignores `rollover`
  reads red in A1, which measures the engine's own timing after the pin.
