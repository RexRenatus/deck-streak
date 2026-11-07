---
status: accepted
decision-makers: "the owner, the DeckStreak architect"
---

# Scheduler tests take their time from a clock read once from the engine, with the cutoff pinned, and a guard beside the clock census

## Context and Problem Statement

Four tests named by #747 fail when they run in the minutes before their collection's day cutoff
and pass at every other time of day: `crates/engine-core/tests/exempt.rs`
`a_forget_gesture_resets_its_one_card_and_no_other`, `crates/engine-core/tests/review_pairs.rs`
`the_review_pairs_run_natively_and_no_other_pair_joins`, `crates/ffi/tests/exempt.rs`
`a_native_forget_tap_resets_its_one_card` and one assertion of `crates/ffi/tests/round_trip.rs`
`a_native_press_answers_only_its_card_with_its_grade`. Read at `83846ea3`:

- Two clocks reach each of them. The test's own clock: `exempt.rs:121-127` and `review_pairs.rs:131-137`
  in engine-core and `round_trip.rs:142` and `:165-170` in ffi call `SystemTime::now()` for the
  answer time and the deck tree's `now`. The engine's own clock: the engine schedules an answer from
  its own reading of now against the collection's next cutoff, and a learning interval that reaches
  the cutoff reads in days. `crates/ffi/tests/review_pairs.rs:114-120` states it and cures it for
  that one file by pinning a rollover hour far from now; `crates/engine-core/src/dispatch.rs:595-613`
  reads the engine's day from the engine's own timing of today. So a 10-minute Good step taken in
  the ten minutes before the cutoff becomes a day-learning card (queue 3), and the assertions that
  read queue 1 or the words `<10m` fail.
- `crates/ffi/tests/exempt.rs` holds no clock read. Its press, `exempt.rs:97-99`, enters
  `crates/ffi/src/engine.rs:184` `Engine::answer`, which stamps `now_millis()` (`:199`, reading
  `SystemTime::now()` at `:243-249`) and runs the answer through the core (`:202-204`), where the
  engine's own clock decides the queue.
- `round_trip.rs`'s assertion brackets each review's id between two reads of the test's own clock
  (`:455`, `:463`, `:470-472`): an assertion on the wall clock.
- No DeckStreak seam fixes the engine's own instant: the engine is a pinned dependency, and its
  scheduler reads its own clock. The collection's next cutoff, by contrast, is the collection's
  data. The collections these tests build run the engine's version-2 timing with its new cutoff
  handling: the engine's default configuration for a new collection sets `schedVer` 2, `sched2021`
  and the creation offset, and its first timing read writes `rollover` 4. So their next cutoff is
  the next 04:00 in the zone the engine reads, and the creation stamp `crt` moves only the day
  count. A collection that selects `schedVer` 1 runs the engine's version-1 timing instead, whose
  next cutoff is `crt` plus whole days. Tests already move `crt`
  (`crates/engine-core/tests/late.rs:182-186`) and set the rollover hour
  (`crates/ffi/tests/review_pairs.rs:123-141`, whose comment at `:117-119` states a version-1
  default these collections do not have). Each lever below was measured by a probe over the
  fixtures' own collections, in both crates, under `TZ=UTC` and under `TZ=Pacific/Auckland`.
- The time a core test supplies already has a seam: `CardAnswer.answered_at_millis`, which
  `crates/engine-core/src/answer.rs:149-163` `answer_request` takes as a parameter and the core
  tests build themselves.
- The class is wider than four tests. Under `tests/` of the five members whose manifests name the
  engine (`engine-core`, `ffi`, `ingest`, `readings`, `web-engine`; 89 files), the clock census's
  spellings find 32 reads: 27 of `SystemTime::now` (15 times handed to the engine, 12 stamps that
  name scratch directories) and 5 of `Instant::now`. The engine's own `TimestampMillis::now` and
  `TimestampSecs::now` add 7 more, which the census's spellings do not see.
- The production clock census (`scripts/tests/test_clock_reads.py`, `config/clock-reads.json`,
  SPEC-431 and ADR-445 on #897) sets every file under a member's `tests/` aside, listed and never
  opened (SPEC-431 R12; ADR-445 D3), and lists `crates/ffi/src/engine.rs` at one read.

## Decision Drivers

- A scheduler test passes at every time of day, and its assertions stay exact.
- No production caller reads the clock anywhere but where it does today.
- No signature the Swift client calls changes.
- The class cannot come back unseen: a guard holds every scheduler test file's wall-clock reads.
- Every red is red at the base for a reason that does not depend on the time of day it ran.

## Considered Options (the alternatives each decision was chosen against)

### D1, the clock seam

- Chosen: one clock per test crate, `crates/engine-core/tests/support/clock.rs` and
  `crates/ffi/tests/support/clock.rs`, with two operations on the closed collection, because the
  engine reads its own clock, only the collection's cutoff can be fixed against it, and no one
  engine setting places that cutoff both to the second and on the timing production runs. Each
  operation reads the engine's own instant once, `anki::timestamp::TimestampMillis::now()`, the
  clock the engine's scheduler reads, and fixes the clock's instant one millisecond before that
  reading, so no later read of any clock in the run equals it.
  - `place(collection, start)`: it sets a start to the second. It selects the engine's version-1 timing
    (`schedVer` 1), writes `crt = instant_secs + start - DAY_SECS`, reopens the collection and
    asserts, through the engine's own timing of today, that the day count is 0 and the next cutoff
    is `instant_secs + start`. Measured exact in both crates and both zones. Under it the engine
    answers (a Good press on a new card 300 s before the cutoff left it at queue 3, due 1) and
    refuses the review's bury ("Your collection needs to be upgraded to the v3 scheduler"), so a
    placed collection is only a start: the side cases pin it before the named test's body runs,
    and only the rule cases answer on it unpinned.
  - `pin(collection)`: it keeps the engine's version-2 timing, the one production runs. It sets
    `schedVer` 2 (the fixture's own value, which undoes a placement), reads the engine's next
    cutoff and rollover hour once, sets the rollover hour that makes the next cutoff the last local
    hour boundary at or before `instant_secs + LEAD_SECS`, predicted from that reading in whole
    `HOUR_SECS`, and writes `crt` at the cutoff before it (the predicted cutoff less `DAY_SECS`), so
    the day count is 0. It then reopens the collection and asserts, through the engine's own timing
    of today, day 0 and the predicted cutoff. Measured in both crates and both zones, from an
    unplaced fixture and from both placements: the cutoff the engine read equalled the prediction,
    39 988 to 40 016 s after the instant, at day 0; a Good press then stayed in the learning queue,
    and the review's bury buried its card.
  - Every time a scheduler test hands the engine comes from the instant: the core tests'
    `answered_at_millis` (the existing seam, `answer.rs:149-163`), the deck tree's `now`, a
    collection's modified time.
- The adapter's answer time is the one production seam, and the smallest: `Engine` gains a private
  field holding an optional answer instant. `Engine::new`, the exported constructor, leaves it unset,
  and `answer` then stamps `now_millis()` exactly as it does at the base. A constructor the bindings
  do not export, `Engine::at_answer_time(message, answered_at_millis)`, sets it for the adapter's
  tests. No exported signature changes, `answer` stays the adapter's one answer entry and the only
  place it builds an `OwnerAnswer` (SPEC-365 R6), and `Engine::run` is byte-identical.

Chosen against:

- The rollover hour alone: lost, because measured it puts the cutoff only on a local hour boundary
  of the zone the test process runs in (rollover 0, 12 and 23 read 00:00, 12:00 and 23:00 UTC
  under `TZ=UTC`, and 11:00, 23:00 and 10:00 UTC under `TZ=Pacific/Auckland`), so it cannot place
  a start 300 s before a cutoff and the cutoff proof of D2 could not be red. The pin uses it, for
  a lead it can hold to within one hour.
- Version-1 timing alone, for the pin as well as the starts: lost, because measured the engine
  refuses the review's bury under it, in both crates, and the bury and undo tests run on pinned
  collections; and every scheduler test would then run a timing production never runs.
- The creation offset cleared: lost, because measured the engine then runs its legacy version-2
  timing, whose cutoff still follows the rollover hour in the process's zone (the next 04:00, the
  day count 1 from a moved `crt`), so it places nothing the rollover hour does not.
- The clock pinned to the engine's natural cutoff, the collection's timing untouched: lost, because
  measured the pinned instant reaches only the review's id (id minus the pinned instant 0), never
  the scheduler: a Good press at the pinned instant 300 s before the cutoff stayed in the learning
  queue in both zones, so the base's flake at the natural 04:00 cutoff stays and no side case could
  be red. The engine's cutoff did stay fixed while the clock moved, and the pin keeps that fact.
- A literal instant for every time a test supplies: lost, because the engine schedules from its own
  clock, so a literal moves only the review ids, cures no failure, and puts a deck tree's `now`
  before the collection's creation on a pinned collection.
- A clock seam inside the engine: lost, because the engine is a pinned dependency built from source
  this repository does not hold, and moving the pin re-pins every member that builds the engine.
- The engine's own test mode read from the environment: lost, because it is process-wide, shifts
  the clock only around one rollover hour, and is not observable from this tree.
- A clock trait object on the adapter: lost, because a fixed instant is all injection needs, and a
  trait object adds a production type no caller uses.
- An `answer_at` method beside `Engine::answer`: lost, because it is a second answer entry that
  builds an `OwnerAnswer`, against SPEC-365 R6.

### D2, the fixed instant and the cutoff proof

- Chosen: the instant is fixed by its place in its day, because its absolute value is the engine's
  own and the engine's version-2 cutoff falls only on a local hour: every pinned collection's next
  cutoff lies more than 39 600 s (11 hours) and at most 43 200 s (12 hours) after the instant, its
  previous cutoff 86 400 s before that, so between 12 and 13 hours before the instant, and the day
  count at the instant is 0, as on a fresh collection at the base. The cutoff hour of every
  collection these tests build is the rollover hour the pin sets, in the zone the engine reads
  (measured for an instant near 04:50 UTC: hour 16 under `TZ=UTC` and hour 5 under
  `TZ=Pacific/Auckland`, each cutoff at 16:00 UTC). The constants: `DAY_SECS = 86_400`,
  `HOUR_SECS = 3_600`, `LEAD_SECS = 43_200`, `JUST_BEFORE_SECS = 300`, `JUST_AFTER_SECS = 86_100`.
- The proof: one case per named test, `<name>_on_each_side_of_the_cutoff`, which runs the test's
  own body twice: on a collection whose cutoff was first placed `JUST_BEFORE_SECS = 300` after the
  instant (the engine's instant five minutes before its cutoff, as on the wall clock five minutes
  before the cutoff) and `JUST_AFTER_SECS = 86_100` after it (the cutoff passed five minutes ago),
  each placed by version-1 timing and then pinned by the test's own fixture. It passes on both
  sides. Red at the base: the base's fixture pins nothing, so the start placed 300 s before the
  cutoff stays, the Good step reaches it, and the test fails whatever the time of day or the zone.
- A rule case per crate pins what the engine does at the placed start, as a literal: on a
  collection placed 300 s before its cutoff and not pinned, a Good press on a new card leaves it in
  the day-learning queue due on day 1 (`queue = 3`, `due = 1`; measured in both crates and both
  zones). It keeps the side cases from passing over a placement that did nothing.
- Each crate's fixture test: it places its collection first at `JUST_BEFORE_SECS`, then pins it, and
  asserts the pin's window against the literals 39 600 and 43 200 and day 0, so the base's
  fixture, which leaves the placement, reads its cutoff 300 s after the instant at every hour.

Chosen against:

- The rollover hour alone: lost, because measured it cannot place a start 300 s from an arbitrary
  instant, so no side case or rule case could be placed by it.
- Version-1 timing alone, an exact 12-hour lead for every collection: lost, because measured the
  engine refuses the review's bury under it, so the pin must stay on version-2 timing, whose
  cutoff the rollover hour sets to within one hour.
- The creation offset cleared: lost, because measured the cutoff then still follows the rollover
  hour in the process's zone, so it moves the instant's place in its day no better.
- The clock pinned to the engine's natural cutoff, the instant derived as `cutoff - LEAD_SECS`: lost, because
  measured the pinned instant never reaches the scheduler, and on the chosen pin the engine's own
  now lay 0 to 1 s after the instant, where a derived instant would trail it by up to an hour, so a
  deck tree read at the instant would count a time the engine has passed.
- The rollover hour farthest from now, as `crates/ffi/tests/review_pairs.rs:123-141` sets it: lost,
  because it can leave the previous cutoff minutes before the instant, so a time a test states back
  from the instant could cross it.
- An instant a few minutes from the cutoff: lost, because a slow run would reach the cutoff, and 11
  hours leaves most of the day's margin on each side.
- A side case that sets the wall clock or waits for the cutoff: lost, because no test waits on time
  or asserts the time of day.
- A side case whose body loosens its assertion to admit both outcomes: lost, because a widened
  assertion also passes the wrong value.
- Side cases with no rule case: lost, because a placement that silently did nothing would leave
  both sides passing for the reason the base passes.

### D3, the path in `crates/ffi/tests/exempt.rs`

- Chosen: the test reaches two clocks through `Engine::answer`, and each gets its own cure, because
  they decide different things. The engine's clock decides the queue the assertion reads: the
  test's fixture pins the synthetic collection's cutoff before the adapter opens it. The adapter's
  clock (`engine.rs:199`, `:243-249`) stamps only the review's id: the test builds its engine with
  `Engine::at_answer_time` and the clock's instant (D1). No public signature the Swift client calls
  changes, so no question is owed.

Chosen against:

- An `answered_at_millis` parameter on the exported `Engine::answer`: lost, because it changes the
  signature the Swift client calls and makes the client the clock.
- Pinning the cutoff alone and leaving the press on the adapter's clock: lost, because the test
  would still take a time from the wall clock, and the native press test could then not drop its
  wall-clock bracket.
- A test-side entry point through `Engine::run`: lost, because the native `run` refuses AnswerCard
  by design (SPEC-365 R6).

### D4, the guard, and the open sibling #897

- Chosen: the guard STANDS BESIDE the production clock census, as its own check sharing its
  matcher, because ADR-445 D4 gives each census its own module and list so each fails, is listed
  and is measured alone, and because a local run may judge only fixture trees, which a module of its
  own can shape.
  - Home: `scripts/tests/test_scheduler_test_clock.py` with `config/scheduler-test-clock-reads.json`.
  - Population: every `.rs` under `tests/` of every workspace member, read through the census's own
    `members`, whose `Cargo.toml` names `anki` or `anki_proto` as a key of `dependencies`,
    `dev-dependencies`, `build-dependencies` or any `target.<cfg>` table of those. Every such file
    is read; none is set aside by name. At `83846ea3`: 5 members and 89 files.
  - Matcher: the census's own lexer (`lex`: comments, nested block and doc comments and the contents
    of every literal blanked, newlines kept), alias grammar (`USE`, `ALIAS`, `TYPE_ALIAS`) and
    pattern builder (`read_pattern`), imported from `test_clock_reads`, over a wall-clock type set:
    `SystemTime`, `Utc`, `Local`, `Date`, the engine's `TimestampSecs` and `TimestampMillis`, and
    `OffsetDateTime`'s `now_utc` and `now_local`. `Instant` is monotonic and places no day, so it is
    not a wall-clock read. Aliases are collected over the population to a fixed point. No
    `#[cfg(test)]` item is removed: test code is the population.
  - Counts: it prints the engine members, the files it read and the reads it found; 0 members or 0
    files fails as VOID; the reads and rows may be empty and are listed, not refused.
  - Allow-list: `{"sites": [{"path", "count", "adr", "why"}]}`, the census's row shape, sorted by
    path, no path twice, each file's reads equal to its row in both directions, and each row's ADR
    naming its path in backticks. The rows at its landing, after this ADR's first part:
    `crates/engine-core/tests/support/clock.rs` 1 and `crates/ffi/tests/support/clock.rs` 1 (the
    one reader of the engine's instant per crate); the scratch-directory stamps
    `crates/engine-core/tests/support/mod.rs` 1, `crates/engine-core/tests/support/sync_server.rs` 1,
    `crates/ffi/tests/face.rs` 3, `crates/ffi/tests/fixture.rs` 1,
    `crates/ffi/tests/review_actions.rs` 1, `crates/ffi/tests/review_pairs.rs` 1,
    `crates/ffi/tests/seam.rs` 1, `crates/ffi/tests/support/mod.rs` 1,
    `crates/ffi/tests/support/sync_server.rs` 1 and `crates/ffi/tests/voices.rs` 1, which name
    directories and reach no engine; and ingest's reads, listed as found and owed to their own
    delivery: `crates/ingest/tests/skip_write.rs` 1, `crates/ingest/tests/skip_zero_upload.rs` 1,
    `crates/ingest/tests/support/mod.rs` 2, `crates/ingest/tests/support/synthetic.rs` 3 and
    `crates/ingest/tests/undo_and_full_sync.rs` 1. Seventeen rows, 22 reads.
  - A finding names `<path>:<line>: <spelling>`, never the line's text. The real tree is judged in
    one test method, run in CI; the planted defects are refused in others, over fixture trees only.
  - SPEC-431 is untouched: its census still sets test code aside, and this guard reads that code
    under its own SPEC. Land order: after #897, whose module it imports, and after this ADR's first
    part, whose clock modules and converted reads its rows describe.

JOIN's delta. Joining would put a second population into `test_clock_reads.py` and a second list
into `config/clock-reads.json`, and land after #897 as BESIDE does. It would amend SPEC-431 R10 (a
second list), R12 (test-dir files opened for the second population), R13 (a third printed
population), R15 (the base listing) and R16 (a real-tree test split from its planted tests so a local
run can name the planted half), and ADR-445 D3's "(never opened)", each insert-only against #897's
head and applying only after it lands; and it would edit lines that #897's rows S43126 to S43132
anchor on.

Chosen against:

- JOIN #897's census: lost, because it amends five requirements and one decision of a delivery not
  yet on dev and edits the lines its rows anchor on, for one shared lexer the import already shares.
- BESIDE with its own copy of the lexer, landing in either order: lost, because two lexers drift,
  and the population and spellings would no longer agree with the census's.
- Every file under every member's `tests/` as the population: lost, because it holds 14 parked
  test files no local run may open, whose reads no reader here can measure, and the other members' days
  are the kernel's study day, not the engine's cutoff.
- `#897`'s full type set, `Instant` included: lost, because a monotonic timer places no day, and
  five budget timers would need rows that admit nothing about the cutoff.
- A clippy `disallowed-methods` rule: lost, because every admitted stamp would carry an `expect` in
  test code, and clippy reads no list and names no ADR per file.

### D5, shape and split

- Chosen: two deliveries, the clock and then the guard, each under its own SPEC, ADR and band,
  because the cure can land as soon as dev admits it while the guard must wait for #897.
  - Part one, SPEC-443 with this ADR, band `S44300-S44399`: both clocks, the adapter's answer
    instant, every engine-core and ffi scheduler test on its collection's clock, the cutoff proof
    and the rule cases. It changes one production file, `crates/ffi/src/engine.rs`, and test files
    of two crates. Its pull request says `Refs #747`.
  - Part two, its own SPEC, ADR and band numbered when it is cut: the guard of D4. Its pull request
    says `Closes #747`.
  - No SPEC or ADR on dev is amended: SPEC-365 R6's text still holds word for word, and SPEC-336's
    one entry point, `Engine::run`, is unchanged.

Chosen against:

- One delivery: lost, because the guard imports a module that is not on dev, so the cure would
  wait for #897.
- The four named tests first and every other scheduler test with the guard: lost, because the
  other engine-core and ffi tests answer at the base's wall clock too, and the guard would then
  land with a dozen owed rows for code part one already cures.

### D6, tests and rows

- Chosen: red-first per acceptance criterion, each red independent of the time of day and of the
  zone, because the base's reds are what the delivery cures. The side cases are red at the base by
  their start placed 300 s before the cutoff by version-1 timing; the clock's fixture tests are red
  at the base because each places its collection first and the base's fixture leaves that
  placement, so the cutoff lies 300 s after the instant where the pin's window is 39 600 to 43 200
  s; the native press test is red at the base over a constructor stub that keeps its instant and
  stamps the adapter's own clock, so its ids are later than the instant. The rule cases are green
  at the base and recorded `not red`. Rows from `S44300`, each with a killer that observes its
  mutant at every hour of the day: the pin's creation stamp a day early (day 1 where 0, measured in
  both zones), the lead shortened to 11 hours (the fixture test's literal window), the rollover
  hour one late (the pin's own assertion), the side start widened (the rule case), the day
  lengthened (the placement's own assertion), and the adapter's two answer-instant rows. FORMAL:
  not applicable by surface (no second actor, no shared state written, no check followed by an act
  another actor can change; the clock's arithmetic is checked at every run by the engine's own
  timing; the one `@phx covers` line on a file this delivery edits names
  `crates/ffi/src/engine.rs` item `run`, which stays byte-identical).

Chosen against:

- Rows and reds over the rollover hour alone: lost, because measured it cannot place the 300 s
  start, so no side case could be red at the base.
- Rows and reds over version-1 timing alone: lost, because measured the engine refuses the
  review's bury under it, so the pinned bury and undo tests would fail at green.
- Rows and reds over the creation offset cleared: lost, because measured its cutoff follows the
  rollover hour as the base's does, so its reds would turn on the time of day.
- Rows and reds over the clock pinned to the natural cutoff: lost, because measured the pinned
  instant changes no queue, so a red there would be the base's own flake.
- A row on the pin's version-2 selection: lost, because its mutant leaves version-1 timing, under
  which the engine refuses to report a rollover hour, so it fails on that refusal and not on a
  property a test states.
- A row on the pin's creation stamp kept as the collection's own: lost, because the engine's day
  count then turns on the local date and on whether today's rollover passed, so no killer observes
  it at every hour.
- A red read from a run near the real cutoff: lost, because it depends on the time of day.
- A red from a missing constructor: lost, because a test that does not compile is not red for its
  criterion's reason.
- A Lean proof of the cutoff arithmetic: lost, because it would prove a transcription of the
  engine's timing, while the clock's own assertion reads the engine's timing itself at every run.

### D7, what this does not do

- Chosen: the production clock reads, other members' tests, the engine's own clock and anything
  only a run on CI can measure stay out, each with its own issue, because each is its own class.

Chosen against:

- Curing ingest's test reads here: lost, because two of them sit beside the skip write's own
  production reads (#877) and the rest build ingest's sync fixtures, a different suite and a
  different review path.

## Decision Outcome

Part one gives every engine-core and ffi scheduler test one clock, read once from the engine, with
its collection's next cutoff pinned between 11 and 12 hours away on the engine's own version-2
timing, and proves the four named tests on both sides of a cutoff placed to the second by the
engine's version-1 timing. Part two holds the engine members' test files to listed wall-clock
reads beside the production census.

### Consequences

- Good: the four tests and their siblings pass at every time of day with exact assertions; no
  production caller's clock moves; no exported signature changes.
- Good: the native press test asserts each review's id equals the injected instant, an exact value
  where the base had a bracket.
- Bad: no test asserts the value the adapter's real clock stamps, because no test may read the wall
  clock. `now_millis` stays the one read `config/clock-reads.json` lists for
  `crates/ffi/src/engine.rs`, and every press through `Engine::new` still runs it.
- Bad: the engine's instant is still the wall clock's; only its distance from the cutoff is fixed,
  and only to within one hour, the engine's rollover granularity.
- Bad: the clock module holds two timings: the version-1 placement serves only the cutoff proof's
  starts and the rule cases, because the engine refuses the review's bury under it.
- Neutral: `crates/ffi/tests/review_pairs.rs`' own rollover pin gives way to the shared pin, which
  moves the same setting.

### Confirmation

SPEC-443's acceptance fence, its red-first record, the band's rows proved by hand, and the whole
suites of `deck-streak-engine-core` and `deck-streak-ffi` at each committed step.

### What would make this wrong

- The engine's timing at its pin reads anything but the creation stamp for a collection that sets
  no scheduler version: the clock's own assertion then fails at its first run. This fired at the
  build's first run. The fixtures' collections set `schedVer` 2 through the engine's default
  configuration, so six of the eight fence runs stopped in the clock's own assertion: it read day 1
  and the next 04:00 cutoff, between 85 355 and 85 364 s after the clock's instant, where it
  asserted day 0 and 300 s. The seam above is the measured cure.
- The engine's version-2 cutoff reads more than the rollover hour and the zone's offset at the
  reading, or the zone's offset changes during a run: the pin's own assertion then fails at that
  run, naming the cutoff it read and the one it predicted.
- The engine stops answering under version-1 timing, or starts refusing an operation a side case's
  body runs before its pin: the rule cases, or the side cases at the base, then fail on the
  refusal and not on their criterion.
- A scheduler test that hands the engine a time it did not take from its clock: the guard of part
  two finds it once it lands.
- An engine change that lets a test fix the engine's own instant: then the instant itself should be
  a literal, and this ADR is superseded.

## More Information

#747 (the issue), #897 (the production clock census), #877 (the skip write's production reads),
#879 (a clock census for the web client and the native harness).
