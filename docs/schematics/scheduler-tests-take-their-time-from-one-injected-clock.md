# Scheduler tests take their time from one injected clock

- **Kinds:** data flow and sequence, drawn in mermaid.
- **Read at:** commit `83846ea3`; every `path:line` below is a line at that commit. The engine's
  timing on the fixtures' collections was measured by a probe over those collections, in both
  crates, under `TZ=UTC` and `TZ=Pacific/Auckland`.
- **Decided by:** ADR-457. **Specified by:** SPEC-443, the first part of #747.

## 1. Two clocks reach every scheduler test

A scheduler test reaches two clocks. The test's own clock supplies the times it hands the engine:
an answer's time, a deck tree's `now`, a collection's modified time. The engine's own clock decides
the schedule: the engine compares its own reading of now with the collection's next cutoff, and a
learning interval that reaches the cutoff reads in days (`crates/ffi/tests/review_pairs.rs:114-120`).
At the base both are the wall clock and the cutoff sits at the engine's default rollover, the next
04:00 in the zone the engine reads, so the four tests #747 names pass or fail by the time of day
they run. After this delivery every time a test hands the engine comes from one `Clock`, and the
collection's next cutoff is pinned on the last local hour boundary at or before 43 200 s after the
clock's instant, so more than 39 600 s away, whatever the wall clock reads.

In every flowchart below, solid edges are the base's path and dotted edges are what replaces it.

```mermaid
flowchart LR
  wall["the wall clock"]
  testread["base: the test reads SystemTime::now"]
  clk["after: Clock, the engine's TimestampMillis::now read once, minus 1 ms"]
  natural["base: the cutoff at the default rollover, the next 04:00 local"]
  pinned["after: the pin moves the rollover hour, the cutoff 39 600 to 43 200 s after the instant, day count 0"]
  sched["the engine's scheduler: its own now against the next cutoff"]
  asserted["the queue, due day and interval words the test asserts"]
  wall --> testread
  wall --> sched
  wall --> clk
  testread --> sched
  natural --> sched
  clk -.-> sched
  clk -.-> pinned
  pinned -.-> sched
  sched --> asserted
```

## 2. Each named test, from its clock read to the injected clock

### 2.1 engine-core `exempt.rs` `a_forget_gesture_resets_its_one_card_and_no_other`

```mermaid
flowchart TD
  coreread["now_millis, exempt.rs:121-127, SystemTime::now at :122"]
  coreclock["after: synthetic.clock.at_millis()"]
  coreanswer["answer_next, exempt.rs:131-163, CardAnswer.answered_at_millis at :157"]
  corerun["Dispatcher::run_answer, src/dispatch.rs:496"]
  coresched["the engine schedules from its own clock"]
  corepin["after: support::synthetic pins the cutoff 39 600 to 43 200 s away"]
  coreassert["queue 1 for each answered card, exempt.rs:354-358"]
  coreread --> coreanswer
  coreclock -.-> coreanswer
  coreanswer --> corerun
  corerun --> coresched
  corepin -.-> coresched
  coresched --> coreassert
```

| step | at `83846ea3` | after |
|---|---|---|
| time read | `now_millis` (`crates/engine-core/tests/exempt.rs:121-127`), `SystemTime::now()` at `:122` | `synthetic.clock.at_millis()` |
| handed to the engine | `answer_next` (`:131-163`) builds `CardAnswer.answered_at_millis: now_millis()` (`:157`) | the same field, from the clock |
| scheduled by | `Dispatcher::run_answer` (`crates/engine-core/src/dispatch.rs:496`), the engine on its own clock | the same, on the collection `support::synthetic` pinned |
| near the cutoff | the Good step reaches the cutoff: queue 3 | the cutoff is 39 600 to 43 200 s away: queue 1 |
| asserted | queue 1 for each answered card (`exempt.rs:354-358`) | unchanged |

### 2.2 engine-core `review_pairs.rs` `the_review_pairs_run_natively_and_no_other_pair_joins`

```mermaid
flowchart TD
  pairsread["now_secs, review_pairs.rs:131-137, read at :132"]
  pairsclock["after: clock.at_secs()"]
  pairstree["DeckTreeRequest now, review_pairs.rs:163"]
  pairscoll["collection(), review_pairs.rs:84-129, Synthetic literal at :124"]
  pairspin["after: collection() pins its collection"]
  pairssched["DESCRIBE_NEXT_STATES (13, 24), the engine on its own clock"]
  pairswords["NEW_CARD_INTERVALS at :80, asserted at :208"]
  pairsread --> pairstree
  pairsclock -.-> pairstree
  pairstree --> pairssched
  pairscoll --> pairssched
  pairspin -.-> pairscoll
  pairssched --> pairswords
```

| step | at `83846ea3` | after |
|---|---|---|
| time read | `now_secs` (`crates/engine-core/tests/review_pairs.rs:131-137`), read at `:132` | `clock.at_secs()` |
| handed to the engine | `DeckTreeRequest { now: now_secs() }` (`:163`) | `DeckTreeRequest { now: clock.at_secs() }` |
| the collection | its own builder `collection()` (`:84-129`, the `Synthetic` literal at `:124`), cutoff unpinned | pinned in `collection()` |
| scheduled by | `DESCRIBE_NEXT_STATES` (13, 24), the engine on its own clock | the same, on the pinned collection |
| asserted | the words `NEW_CARD_INTERVALS` (`:80`), `["<1m", "<6m", "<10m", "4d"]`, at `:208` | unchanged |

### 2.3 ffi `exempt.rs` `a_native_forget_tap_resets_its_one_card`

The test holds no clock read. It reaches two clocks through `Engine::answer`, and each gets its own
cure (ADR-457 D3): the engine's clock decides the queue the assertion reads, so the test's fixture
pins the synthetic collection's cutoff before the adapter opens it; the adapter's clock
(`crates/ffi/src/engine.rs:199`, `:243-249`) stamps only the review's id, so the test builds its
engine with `Engine::at_answer_time` and the clock's instant.

```mermaid
sequenceDiagram
  participant T as answered(), ffi tests/exempt.rs:80-104
  participant A as Engine::answer, src/engine.rs:184
  participant W as now_millis, src/engine.rs:243-249
  participant D as dispatcher.run_answer, src/engine.rs:202-204
  participant G as the engine's scheduler
  T->>A: engine.answer(..) at exempt.rs:97-99
  alt base and production: Engine::new, the answer instant unset
    A->>W: now_millis() at :199
    W-->>A: SystemTime::now() read at :244
  else after, in this test: Engine::at_answer_time(message, the clock's instant)
    A->>A: stamps the set instant
  end
  A->>D: the one OwnerAnswer, with its stamp
  D->>G: the core runs the answer
  G-->>T: the queue, from the engine's own clock against the collection's cutoff
  Note over T,G: asserted LEARNING_QUEUE, exempt.rs:144-152. After, clock::pin has placed the cutoff 39 600 to 43 200 s away
```

| step | at `83846ea3` | after |
|---|---|---|
| entered by | `answered()` (`crates/ffi/tests/exempt.rs:80-104`), `engine.answer(..)` at `:97-99` | unchanged |
| the review's stamp | `Engine::answer` (`crates/ffi/src/engine.rs:184`) stamps `now_millis()` (`:199`; the fn at `:243-249` reads `SystemTime::now()` at `:244`) | the engine is built by `Engine::at_answer_time`, and `answer` stamps the clock's instant |
| scheduled by | `dispatcher.run_answer` (`engine.rs:202-204`), the engine on its own clock | the same, on the synthetic collection `clock::pin` pinned before the adapter opens it |
| asserted | `LEARNING_QUEUE` (`exempt.rs:144-152`) | unchanged |

### 2.4 ffi `round_trip.rs` `a_native_press_answers_only_its_card_with_its_grade`

```mermaid
flowchart TD
  tripgood["good_answer, answered time at round_trip.rs:142"]
  triprefused["the refused native run, round_trip.rs:417-440"]
  tripclock["after: the refused answer's time from the clock"]
  tripnow["now_millis, round_trip.rs:165-170, read at :166"]
  trippress["the native press, round_trip.rs:442-495"]
  tripbracket["each review's id bracketed between two reads, :455, :463, :470-472"]
  tripinst["after: the engine built by Engine::at_answer_time with the clock's instant"]
  tripids["after: each id equals the instant, the second made unique by the engine"]
  tripgood --> triprefused
  tripclock -.-> triprefused
  tripnow --> tripbracket
  trippress --> tripbracket
  tripinst -.-> trippress
  trippress -.-> tripids
```

| step | at `83846ea3` | after |
|---|---|---|
| the refused answer | `good_answer`'s answered time (`crates/ffi/tests/round_trip.rs:142`), used by the refused native run (`:417-440`) | from the clock |
| the press | `Engine::answer` on the adapter's own clock | the engine built by `Engine::at_answer_time` with the clock's instant |
| asserted | each review's id between two reads of `now_millis` (`:165-170`, read at `:166`): `:455`, `:463`, `:470-472`, an assertion on the wall clock | each review's id equals the injected instant, the second made unique by the engine; the ids' offsets are measured at green and written as literals |

### 2.5 The other reads this delivery moves onto the clock

The same cure reaches every engine-core and ffi scheduler test (SPEC-443 R3). The `SystemTime::now`
reads handed to the engine: engine-core `answer.rs:111`, `dispatch.rs:74`, `exempt.rs:122`,
`one_way.rs:243`, `replay.rs:135`, `review_pairs.rs:132`, `undo_answer.rs:79`, `undo_change.rs:131`;
ffi `review_actions.rs:60`, `review_pairs.rs:144`, `round_trip.rs:142`, `:166`. The engine's own
clock read in engine-core test code: `full_sync.rs:56` and `one_way.rs:70`. Afterwards, under
`crates/engine-core/tests` and `crates/ffi/tests`, those spellings remain only in the 12 stamps that
name scratch directories, which reach no engine, and in the two clock modules.

## 3. The seam

### 3.1 The clock module

One per test crate, `crates/engine-core/tests/support/clock.rs` and
`crates/ffi/tests/support/clock.rs`, each the one reader of time in its crate's scheduler tests. Each
holds `Clock` (`at_millis`, `at_secs`), `place` and `pin`, and these constants (SPEC-443 R1):

| constant | value | what it places |
|---|---|---|
| `DAY_SECS` | `86_400` | one day: the placement's creation stamp a day before its cutoff, and the pin's creation stamp at the cutoff before its predicted one |
| `HOUR_SECS` | `3_600` | one step of the rollover hour, the grain of the engine's version-2 cutoff |
| `LEAD_SECS` | `43_200` | the pin: the next cutoff the last local hour boundary at or before 12 hours after the instant, so 39 600 to 43 200 s after it, and the previous one 43 200 to 46 800 s before it |
| `JUST_BEFORE_SECS` | `300` | a side start, placed to the second: the next cutoff five minutes after the instant |
| `JUST_AFTER_SECS` | `86_100` | a side start, placed to the second: the previous cutoff five minutes before the instant |

`place` sets a start to the second on the engine's version-1 timing:

```mermaid
sequenceDiagram
  participant S as a side case, a rule case or a fixture test
  participant C as the clock module, tests/support/clock.rs
  participant K as the engine's clock
  participant F as the closed collection
  participant E as the engine's timing of today
  S->>C: place(collection, start)
  C->>K: TimestampMillis::now(), read once
  K-->>C: the reading
  C->>C: the instant is the reading minus 1 ms
  C->>F: schedVer 1, the engine's version-1 timing
  C->>F: write crt = instant_secs + start - DAY_SECS, the collection closed
  C->>E: reopen the collection, timing_today()
  E-->>C: the day count and the next cutoff
  C->>C: assert the day count is 0 and the next cutoff is instant_secs + start
  C->>F: close the collection
  C-->>S: the Clock, at_millis and at_secs
```

`pin` keeps the engine's version-2 timing, the one production runs, and moves its rollover hour:

```mermaid
sequenceDiagram
  participant S as a fixture, support::synthetic or a test's own builder
  participant C as the clock module, tests/support/clock.rs
  participant K as the engine's clock
  participant F as the closed collection
  participant E as the engine's timing of today
  S->>C: pin(collection)
  C->>K: TimestampMillis::now(), read once
  K-->>C: the reading
  C->>C: the instant is the reading minus 1 ms
  C->>F: schedVer 2, the engine's version-2 timing, which undoes a placement
  C->>E: timing_today() and the rollover hour r, read once
  E-->>C: the next cutoff c under r
  C->>C: j = floor((instant_secs + LEAD_SECS - c) / HOUR_SECS), predicted = c + j * HOUR_SECS
  C->>F: rollover = (r + j) mod 24, crt = predicted - DAY_SECS, the collection closed
  C->>E: reopen the collection, timing_today()
  E-->>C: the day count and the next cutoff
  C->>C: assert the day count is 0 and the next cutoff is the predicted one
  C->>F: close the collection
  C-->>S: the Clock, at_millis and at_secs
```

Why two timings. The collections these tests build run the engine's version-2 timing: the engine's
default configuration for a new collection sets `schedVer` 2, `sched2021` and its creation offset,
and its first timing read writes `rollover` 4. Their next cutoff is the next rollover hour in the
zone the engine reads, and the creation stamp `crt` moves only the day count (measured: the old
placement read day 1 and the next 04:00 where it asserted day 0 and 300 s). The rollover hour puts
the cutoff only on a local hour boundary (measured: rollover 0, 12 and 23 read 00:00, 12:00 and
23:00 UTC under `TZ=UTC`, and 11:00, 23:00 and 10:00 UTC under `TZ=Pacific/Auckland`), so the pin
holds its lead to within one hour and cannot place a start five minutes from a cutoff. The
engine's version-1 timing, selected by `schedVer` 1, puts the next cutoff at `crt` plus whole days,
to the second in every zone (measured), so `place` uses it for the starts; the engine answers under
it (measured: a Good press 300 s before the cutoff left the card at queue 3, due 1) and refuses the
review's bury (measured), so a placed collection is a start only, and every body runs on a pinned
one. `crt` at the cutoff before the predicted one keeps the day count 0 on both branches of the
engine's day count (measured in both zones). Each module reads its own result back through the
engine's `timing_today()` at every run, so its arithmetic is checked by the engine itself and is
never a transcription of it. The instant sits one millisecond before the engine's reading, so no
later read of any clock in the run equals it; on the pin the engine's own now lay 0 to 1 s after
it (measured).

Where the pin is applied: engine-core's `support::synthetic` (`crates/engine-core/tests/support/mod.rs`,
`pub struct Synthetic` at `:54`) pins the collection it builds and carries its clock as
`Synthetic::clock`; engine-core `review_pairs.rs` pins in its own `collection()`; the ffi tests pin
the synthetic collection (`crates/ffi/tests/support/synthetic.rs`) with `clock::pin` before the
adapter opens it, and `crates/ffi/tests/review_pairs.rs` takes the shared pin in place of its own
rollover pin (`:123-141`). The time a core test supplies already had a seam: `CardAnswer`'s
`answered_at_millis`, which `crates/engine-core/src/answer.rs:149-163` `answer_request` takes as a
parameter.

### 3.2 The adapter's answer instant

`Engine` gains a private field holding an optional answer instant, the one production seam
(SPEC-443 R4). `Engine::run` (`crates/ffi/src/engine.rs:104`) is byte-identical, and no exported
signature changes.

```mermaid
flowchart TD
  newctor["Engine::new, exported, signature unchanged"]
  testctor["Engine::at_answer_time(message, answered_at_millis), in an impl block the bindings do not export"]
  unset["the answer instant: unset"]
  isset["the answer instant: set"]
  stampwall["Engine::answer stamps now_millis(), exactly as at the base"]
  stampset["Engine::answer stamps the set instant"]
  buildanswer["the one OwnerAnswer, built only in Engine::answer"]
  core["dispatcher.run_answer, engine.rs:202-204"]
  newctor --> unset
  unset --> stampwall
  testctor -.-> isset
  isset -.-> stampset
  stampwall --> buildanswer
  stampset -.-> buildanswer
  buildanswer --> core
```

## 4. The cutoff cases

| start | next cutoff after the instant | previous cutoff before the instant | a Good step on a new card |
|---|---|---|---|
| placed `JUST_BEFORE_SECS` (300 s), version-1 timing | 300 s | 86 100 s | reaches the cutoff: the day-learning queue, due on day 1 |
| placed `JUST_AFTER_SECS` (86 100 s), version-1 timing | 86 100 s | 300 s | stays within the day |
| pinned, `LEAD_SECS` (43 200 s) to the hour, version-2 timing | 39 600 to 43 200 s | 43 200 to 46 800 s | stays within the day, with 11 hours of margin or more on each side |

Each named test gains `<name>_on_each_side_of_the_cutoff`, which runs the named test's own body
twice: on a collection first placed `JUST_BEFORE_SECS` before its cutoff and on one first placed
`JUST_AFTER_SECS`, each placed by version-1 timing and then pinned by the test's own fixture. It
asserts the named test's exact values on both sides. Red at the base: the base's fixture pins
nothing, so the start placed 300 s before the cutoff stays, the Good step reaches it, and the test
fails whatever the time of day or the zone.

```mermaid
sequenceDiagram
  participant N as the on_each_side_of_the_cutoff case
  participant P as the test's own fixture
  participant C as the clock module
  participant B as the named test's own body
  loop start JUST_BEFORE_SECS (300 s), then JUST_AFTER_SECS (86 100 s)
    N->>P: build the collection at this start
    P->>C: place(collection, start)
    P->>C: pin(collection)
    C-->>P: the Clock, the next cutoff 39 600 to 43 200 s after the instant, day 0
    N->>B: run the body on this collection and clock
    B-->>N: the named test's exact values
  end
```

A rule case per crate pins what the engine does at the placed start, as a literal, so the side
cases cannot pass over a placement that did nothing. Each crate's fixture test places its
collection at `JUST_BEFORE_SECS` and then pins it, and asserts day 0 and a next cutoff more than
39 600 s and at most 43 200 s after the instant, against literals: at the base the placement stays,
so the cutoff lies 300 s after the instant at every hour.

```mermaid
sequenceDiagram
  participant R as the rule case, one per crate
  participant C as the clock module
  participant G as the engine's scheduler
  R->>C: place(collection, JUST_BEFORE_SECS), version-1 timing, not pinned
  R->>G: a Good press on its new card
  G-->>R: queue 3, due 1, the day-learning queue due on day 1
```

| criterion | case |
|---|---|
| A1 | `a_forget_gesture_resets_its_one_card_and_no_other_on_each_side_of_the_cutoff` |
| A2 | `the_review_pairs_run_natively_and_no_other_pair_joins_on_each_side_of_the_cutoff` |
| A3 | `a_native_forget_tap_resets_its_one_card_on_each_side_of_the_cutoff` |
| A4 | `a_native_press_stamps_each_review_at_the_injected_instant_on_each_side_of_the_cutoff` |
| A5 | `the_core_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock`, `the_adapter_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock` |
| A6 | `a_core_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow`, `an_adapter_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow` |

## 5. What this does NOT cover

- The production clock reads, which the production clock census lists file by file (#897).
- The skip write's two production reads in `crates/ingest/src/skip_write.rs`, read through the kernel's clock by their own delivery (#877).
- The guard that holds every engine member's test files to listed wall-clock reads, beside the production census: ADR-457 D4, the second part of this issue, landing after #897 (#747).
- The other engine members' test reads, ingest's among them (#747).
- A clock census for the web client's TypeScript and the native harness's Swift (#879).
