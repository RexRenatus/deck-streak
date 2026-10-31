# SPEC-507: The card health panel on the web shows a card's own record, and no figure the scheduler did not give

| field | value |
|---|---|
| status | part a (sections 1 to 7) delivered by the pull request that adds this file; part b (section 8, the panel on iPhone and iPad) by the pull request that adds section 8 |
| issue | none filed for the panel; #759 asks where a card came from, and section 2 R7 answers what phase 1 can |
| decided by | ADR-525, under ADR-356 (the engine core holds every read) and ADR-348 (the web engine's boundary) |
| measured at | the delivery's base (every file this SPEC cites is read there) |

## 1. The problem, measured

During a review the learner cannot see a card's own record on the web: how often it was answered, how often
it lapsed, how long its answers took, what the scheduler expects of it, or where it came from. Anki's desktop
shows this as Card Info. The web client has no such view:

- `git grep -n -E 'CardHealth|card_health|CARD_REVIEWS_SQL'` over `crates` and `web/app/src` prints 0 lines
  at the base.
- The engine core holds one fixed read of a card, `SNAPSHOT_SQL` (`crates/engine-core/src/dispatch.rs`):
  `select id, queue, type, due, ivl, reps, lapses from cards where id = ?`. No fixed read returns a card's
  review rows.
- The analytics crate caps each answer at 60 seconds before it sums time (`crates/analytics/src/constants.rs`
  `ANSWER_TIME_CAP_SECONDS = 60.0`; `crates/analytics/src/metrics.rs` `seconds_of`). A panel that summed raw
  times would disagree with every other time figure the product shows.
- The review's answer clock (`web/app/src/lib/study/review.ts`) sends `now - shownAt` with each grade, so any
  time spent in an overlay on the review would count as answering time.
- No readout of recall or of days to 90% exists on the scheduler's port in this build, so the panel has no
  scheduler figure to show yet.

## 2. Requirements

R1. The engine core holds a second fixed read of a card, `CARD_REVIEWS_SQL = "select id, ease, time, type, ivl
    from revlog where cid = ? order by id"`, beside `SNAPSHOT_SQL`. `Dispatcher::card_health(card, scheduler)`
    reads the card by `SNAPSHOT_SQL` and its review rows by `CARD_REVIEWS_SQL`, writes nothing, and answers
    `None` for an id no card has. The web adapter exports `card_health(card)`, which answers the record as JSON
    or `null`. No adapter passes SQL.
R2. The record (`crates/engine-core/src/health.rs`, `CardHealth`) holds integers and closed enums only: the
    card's `reps` and `lapses` counters, the counts of answers and lapses over its review rows, the number of
    rows, the newest 200 rows newest first (each with its instant, button, type, interval and its time capped),
    the capped total, mean and median, the scheduler's readout and the provenance.
R3. Each answer's time counts as `min(time, 60000)` milliseconds, a negative time as 0. An answer is a row of
    type 0 to 3 with a button of 1 or more, as the analytics crate's study event is. The total is the sum of the
    capped answers; the mean is the total over the count, rounded half up to a millisecond; the median is the
    middle capped answer, or the floor of the two middle ones' mean. With no answer, the mean and the median
    are absent.
R4. Over rows whose times are not negative, the panel's capped total equals the analytics crate's capped seconds
    over the same rows, times 1000, and its answer count equals the analytics crate's review count. That is the
    differential's domain: the analytics crate's function keeps a negative time negative, where the panel reads it
    as 0 (R3), so a negative time is held by A1's fixture and never by the differential.
R5. A lapse in the rows is an answer of type 1 (review) with button 1 (Again). The record carries the card's
    counters and the rows' counts side by side; when they differ the panel says what the log holds, as a fact.
R6. The recall rows show a figure only from the scheduler's readout in its Ready state: the recall chance in
    parts per million and the days until the scheduler's estimate falls to 90%, labelled "The scheduler
    estimates". In every other state (absent, fixture, unready, stale) they read that state's line and no
    number. The record holds no stored FSRS memory state, so none can stand in. This build's web adapter has no
    readout, so it answers the absent state: "Not available in this build."
R7. Provenance: every card in phase 1 came through the collection, so the panel says so with the card's created
    time, which is the card id read as a millisecond instant. A collection records no package of origin for a
    note; the panel claims none.
R8. No verdict: the panel's own strings hold no judgement of the card ("hard", "difficult", "ambiguous",
    "leech", "risk", "problem", "struggling", "weak", "trouble"); it shows facts and the scheduler's labelled
    estimate only. A review row names the button the learner pressed by the review's own answer names, which
    record what was pressed and are not the panel's words.
R9. The panel is a `role="dialog"` opened from a "Card info" control in the review's chrome. Focus moves into
    it, Tab and Shift+Tab stay inside it, Escape and its Close button close it, and focus returns to the review
    region. While it is open, the answer clock stops: the time a grade sends excludes every moment an overlay was
    open, and a grade after it closes applies to the card shown.
R10. The panel's module and the review's clock open no fetch and add no route: the record crosses the Worker's
    protocol as one new operation, `card-health`, over the engine already open.
R11. The panel passes axe-core's WCAG 2.2 A and AA rules in both colour schemes on the review, with the dialog
    open on a card whose recall rows are not available.

## 3. Acceptance criteria of SPEC-507

| id | criterion | decided by |
|---|---|---|
| A1 | a fixture card's reps, lapses, capped mean and median, rows newest first and provenance, through the two fixed reads; a 3,600,000 ms row counts as 60,000 and a negative time as 0, and the record keeps the newest 200 of 207 rows (R1, R2, R3, R5, R7) | `cargo test -p deck-streak-engine-core --test health -- --exact the_panel_reads_a_fixture_cards_figures_through_the_two_fixed_reads` |
| A2 | the capped total and the answer count equal the analytics crate's over generated rows whose times are not negative, and after real answers through the engine the card's reps and lapses equal the counts over its rows (R4, R5) | `cargo test -p deck-streak-engine-core --test health -- --exact the_panels_capped_time_equals_the_analytics_functions_over_generated_rows` and `... --exact the_cards_reps_and_lapses_equal_the_counts_of_its_review_rows` |
| A3 | outside Ready, the recall rows name their state and no number; the stored FSRS state never reaches them (R6) | `cargo test -p deck-streak-engine-core --test health -- --exact no_scheduler_figure_reaches_the_recall_rows_outside_ready` and the Vitest test `the recall rows read the state's line in every state but Ready` |
| A4 | the panel's strings say "The scheduler estimates" and hold no verdict word, and the panel names the collection and the created date (R7, R8) | Vitest `the panel's strings label the estimate and name no verdict` and `the panel says where the card came from` |
| A5 | the answer clock excludes every overlay's time over random sequences, grades under an open overlay among them, and the panel pauses it; the panel's two modules reach only what a default-deny census of their imports, names, properties and markup lists, and the base's route files stand frozen (R9, R10) | Vitest `the answer clock excludes every overlay's time`, `the card info panel pauses the answer clock` and `the panel's module opens no fetch and adds no route` |
| A6 | axe passes on the review with the panel open, in both colour schemes (R11) | the study suite's `the card info panel passes axe in both colour schemes` (`web/app/tests-study/study.spec.ts`), held by the Vitest census `the study suite covers the review loop in both engines` |

```acceptance
A1: cargo test -p deck-streak-engine-core --test health -- --exact the_panel_reads_a_fixture_cards_figures_through_the_two_fixed_reads
A2: cargo test -p deck-streak-engine-core --test health -- --exact the_panels_capped_time_equals_the_analytics_functions_over_generated_rows
A2: cargo test -p deck-streak-engine-core --test health -- --exact the_cards_reps_and_lapses_equal_the_counts_of_its_review_rows
A3: cargo test -p deck-streak-engine-core --test health -- --exact no_scheduler_figure_reaches_the_recall_rows_outside_ready
A3: pnpm exec vitest run web/app/src/lib/study/card-health.test.ts -t "the recall rows read the state's line in every state but Ready"
A4: pnpm exec vitest run web/app/src/lib/study/card-health.test.ts -t "the panel's strings label the estimate and name no verdict"
A4: pnpm exec vitest run web/app/src/lib/study/card-health.test.ts -t "the panel says where the card came from"
A5: pnpm exec vitest run web/app/src/lib/study/review-clock.test.ts -t "the answer clock excludes every overlay's time"
A5: pnpm exec vitest run web/app/src/lib/study/card-health.test.ts -t "the card info panel pauses the answer clock"
A5: pnpm exec vitest run web/app/src/lib/study/card-health.test.ts -t "the panel's module opens no fetch and adds no route"
A6: pnpm exec vitest run web/app/src/lib/study/study-coverage.test.ts -t "the study suite covers the review loop in both engines"
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/engine-core/src/health.rs` | `deck-streak-engine-core` | added: the record, its readout and its JSON |
| `crates/engine-core/src/dispatch.rs` | `deck-streak-engine-core` | changed: `CARD_REVIEWS_SQL` and `Dispatcher::card_health` |
| `crates/engine-core/src/lib.rs` | `deck-streak-engine-core` | changed: the `health` module |
| `crates/engine-core/Cargo.toml` | `deck-streak-engine-core` | changed: three dev-dependencies for the differential test |
| `crates/engine-core/tests/health.rs` | `deck-streak-engine-core` | added: A1 to A3 |
| `crates/web-engine/src/wasm.rs` | `deck-streak-web-engine` | changed: the `card_health` export |
| `web/app/src/lib/engine/protocol.ts` | web | changed: the `card-health` operation and the record's type |
| `web/app/src/lib/engine/session.ts` | web | changed: the Worker serves `card-health` |
| `web/app/src/lib/engine/client.ts` | web | changed: `cardHealth` |
| `web/app/src/lib/engine/protocol.test.ts` | web | changed: the operations' census names `card-health` |
| `web/app/src/lib/engine/session.test.ts` | web | changed: its stand-in module answers `card_health` |
| `web/app/src/lib/engine/credential-stand-in.test.support.ts` | web | changed: its stand-in module answers `card_health` |
| `web/app/src/lib/study/review.ts` | web | changed: the clock's overlay pause and the panel's read |
| `web/app/src/lib/study/card-health.ts` | web | added: the panel's lines |
| `web/app/src/lib/study/CardHealthPanel.svelte` | web | added: the dialog |
| `web/app/src/lib/study/ReviewScreen.svelte` | web | changed: the "Card info" control and the dialog |
| `web/app/src/lib/study/card-health.test.ts` | web | added: A3 to A5 |
| `web/app/src/lib/study/review-clock.test.ts` | web | added: A5's property |
| `web/app/src/lib/study/study-coverage.test.ts` | web | changed: the study suite's new step |
| `web/app/tests-study/study.spec.ts` | web | changed: A6 |
| `web/app/messages/en.json` | web | changed: the panel's strings |
| `docs/specs/SPEC-507-...md`, `docs/decisions/ADR-525-...md`, `docs/schematics/card-health-panel-507.md`, `docs/red-first/SPEC-507.md`, `changelog.d/card-health-panel-web-507.md` | docs | added |

Tests that already census the Worker's operations may change with R10's one operation; each such change is
named in the red-first record.

## 5. What this does NOT cover

- The scheduler's live readout (recall chance and days to 90%): the port gains its read in its own delivery; until
  then every build answers the absent state.
- The panel on iPhone and iPad: the native record and its sheet are their own delivery, part b (section 8).
- The due date, the preset's leech threshold, the note type and the deck path: each needs a read beyond the two
  fixed reads, and each is a later amendment.
- Translations of the panel's strings: they ship in English, which every locale falls back to, until translated.

## 6. Risks

- A figure the scheduler did not give could appear as its estimate. A3 and A4 hold the record and the strings.
- The panel's time could drift from the analytics crate's. A2 holds them equal over generated rows.
- An overlay could leak time into a grade. A5's property holds the clock over random sequences.

## 7. What only CI proves

- The `card_health` export compiles only for `wasm32-unknown-unknown`, in CI's web-engine job.
- A6 runs in the study suite (Chromium and WebKit over the built module), which CI runs.

## 8. Part b: the card health panel on iPhone and iPad

Part b gives the native app the panel part a gave the web: the same record, read by the same two fixed reads, in
a sheet over the review. ADR-525's decisions hold for it unchanged, so it takes no ADR of its own, and its data
flow is part a's schematic with the native adapter in the web adapter's place.

### 8.1 The problem, measured

- `git grep -n -E 'card_health|CardHealth|cardHealth'` over `crates/ffi` and `ios` prints 0 lines at the base:
  the native adapter exports no record, and the app has no panel.
- The app's answer clock (`ios/App/Sources/ReviewSession.swift`) sends `Date().timeIntervalSince(shownAt)` with
  each grade, so time spent in a sheet over the review would count as answering time.
- The app's Swift is held thin (SPEC-347 R11, `scripts/tests/test_ios_thin_swift.py`): only a `session` file
  imports the adapter, a `view` file holds at most 3 decisions and a `model` file at most 6. The register
  (`ios/swift-roles.json`) records `ReviewChrome.swift` at 3 and `ReviewModel.swift` at 6, both at their ceilings,
  and `EngineSession.swift` at 4, the `session` ceiling. The panel's words and their branches therefore sit in
  the adapter, beside its other sentences (`ChoiceStep.said`), and the app shows them as they are.
- The umbrella crate's build closure counts its dev tables (`closure_of` in
  `scripts/tests/test_one_static_library.py`), and the Apple change caller must watch every crate in it
  (`test_ci_workflows.py`). A dev edge from the adapter to the analytics crate would widen what the Apple lane
  watches, so part b's differential compares the native record with the core's record, which A2 already holds
  equal to the analytics crate's.

### 8.2 Requirements

R12. `Engine::card_health(card_id)` (`crates/ffi/src/engine.rs`) answers the core's record of the card by the
     core's two fixed reads (R1), field for field, as the UniFFI record `CardHealth` (`crates/ffi/src/health.rs`)
     with `HealthLogged`, `HealthReview`, `HealthReadout`, `HealthState` and `HealthProvenance`, or `None` for an
     id no card has. It writes nothing and passes no SQL. This build's scheduler port gives no readout, so the
     recall part is the absent state, as the web adapter's is (R6).
R13. `card_health_lines(health)` answers the panel's sections, each a title and its lines, in the web panel's
     English words: "Counts and times" (the card's reviews and lapses, the capped total, mean and median in
     seconds to one decimal rounded half up, "No answers yet" with no answer, and the log's counts when they
     differ from the card's, R5); "Recall" (under "The scheduler estimates", the chance of recall in percent to
     one decimal and the days to 90%, in Ready only; the state's line alone in every other state, R6); "Where it
     came from" (the collection and the created date, R7); and, when the card has rows, "Newest reviews" (how
     many of how many when the record holds fewer rows than the card, then each row newest first as its date,
     the name of its button and its capped time, R2). Dates are UTC `yyyy-mm-dd`. With no record it answers one
     section, "Card info", whose one line is "This card's record could not be read." No line judges the card
     (R8).
R14. The review's chrome (`ios/App/Sources/ReviewChrome.swift`) shows a "Card info" control in its counts row,
     enabled where bury and flag are. It opens a sheet over the review, on the iPhone and the iPad, that shows
     the sections as the adapter answers them, with a Close button. The app adds no word and no decision to
     them: the session's file turns each section into the app's own type and does nothing else.
R15. Opening the sheet stops the answer clock before the card is read, and closing it, by Close or by a swipe,
     starts it again once. A grade sends the time since the card was shown less every moment an overlay held
     the clock, one still open at the grade included. An overlay open as a new card shows holds from that show
     on; a second open keeps the first one's instant; a close with none open changes nothing; the time never
     reads below 0.
R16. VoiceOver reads the control as "Card info", though it shows its icon alone, and the sheet's Close button as
     "Close". Each section title is a heading, and each line is one element whose label is its text as shown.
R17. The panel's XCTest cases and its UI test run on the iPhone and then the iPad simulator. The card health
     collection they open is written as `health/` beside the review fixture, by the same example and the same
     builder the adapter's test reads (`crates/ffi/tests/support/health.rs`): one card, fixed at a 2023-11-14
     instant, with part a's 200 entries set by hand and seven rows, counters that differ from its log, and a
     stored FSRS memory state.

### 8.3 Acceptance criteria of part b

| id | criterion | decided by |
|---|---|---|
| B1 | T-SS-17 through the native record: the health card reads reps 8 and lapses 1 beside the log's 6 answers and 1 lapse, 207 rows of which the newest 200 newest first, the hour's row at 60,000 ms and the negative one at 0, a capped total of 129,999, mean 21,667 and median 16,000, created at its id, and the absent state; its deck's queue shows it; an id no card has reads `None` (R12, R17) | `cargo test -p deck-streak-ffi --test card_health -- --exact the_native_record_reads_the_health_cards_figures` |
| B2 | PO-SS-8 through the native record: over generated rows, times past the cap and the 200-row bound among them, and every readout, the native record equals the core's field for field, so A2's equality with the analytics crate holds for the record the app shows (R12) | `cargo test -p deck-streak-ffi --test card_health -- --exact the_native_record_carries_the_cores_figures_over_generated_rows` |
| B3 | T-SS-18: the card's stored FSRS state reaches no part of the native record; in every state but Ready the recall section reads the state's line alone, and in Ready the label and the two figures (R6, R12, R13) | `cargo test -p deck-streak-ffi --test card_health -- --exact no_scheduler_figure_reaches_the_native_recall_lines_outside_ready` |
| B4 | the health card's sections word by word, a card with no answer, no record, and over generated records in every state no line of the panel's own words holding a verdict word (R8, R13, R16) | `cargo test -p deck-streak-ffi --test card_health -- --exact the_panels_lines_word_the_record_and_name_no_verdict` |
| B5 | T-SS-17 and T-SS-18 in XCTest on both simulators: the health card's sections through the app's session, and the recall section's one line beside the stored state (R13, R14, R17) | `ReviewSessionTests/test_the_card_info_panel_shows_the_health_cards_record` and `ReviewSessionTests/test_no_figure_of_the_stored_state_reaches_the_recall_lines` |
| B6 | the clock in XCTest: over sequences of shows, opens, closes and grades the clock sends each grade's time less every overlay's, and the text card held 30 of 40 seconds in the sheet records 10 (R15) | `AnswerClockTests/test_the_answer_clock_excludes_every_overlays_time` and `ReviewSessionTests/test_the_card_info_panel_pauses_the_answer_clock` |
| B7 | the sheet from the chrome on the iPhone and the iPad: the control and Close read their titles, the text card's lines read as shown, and Close returns to the review (R14, R16, R17) | `ReviewActionsFlowTests/test_the_card_info_sheet_opens_from_the_chrome_with_its_voiceover_labels` |

```acceptance
B1: cargo test -p deck-streak-ffi --test card_health -- --exact the_native_record_reads_the_health_cards_figures
B2: cargo test -p deck-streak-ffi --test card_health -- --exact the_native_record_carries_the_cores_figures_over_generated_rows
B3: cargo test -p deck-streak-ffi --test card_health -- --exact no_scheduler_figure_reaches_the_native_recall_lines_outside_ready
B4: cargo test -p deck-streak-ffi --test card_health -- --exact the_panels_lines_word_the_record_and_name_no_verdict
B5: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_the_card_info_panel_shows_the_health_cards_record -only-testing:DeckStreakTests/ReviewSessionTests/test_no_figure_of_the_stored_state_reaches_the_recall_lines
B6: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/AnswerClockTests/test_the_answer_clock_excludes_every_overlays_time -only-testing:DeckStreakTests/ReviewSessionTests/test_the_card_info_panel_pauses_the_answer_clock
B7: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewActionsFlowTests/test_the_card_info_sheet_opens_from_the_chrome_with_its_voiceover_labels
```

B5 to B7 need the review fixture named to the test runner, as the review steps of `xcframework.yml` name it
(`TEST_RUNNER_DS_REVIEW_FIXTURE`); B6's `AnswerClockTests` needs none. The cases sit in classes those steps
already run, `ReviewSessionTests` and `ReviewActionsFlowTests`, so no workflow changes, and `AnswerClockTests`
runs in the app's own step, which skips only the fixture's classes.

### 8.4 File manifest of part b

| file | context | change |
|---|---|---|
| `crates/ffi/src/health.rs` | `deck-streak-ffi` | added: the native record, its conversion from the core's, and the panel's words |
| `crates/ffi/src/engine.rs` | `deck-streak-ffi` | changed: `Engine::card_health` |
| `crates/ffi/src/lib.rs` | `deck-streak-ffi` | changed: the `health` module |
| `crates/ffi/tests/card_health.rs` | `deck-streak-ffi` | added: B1 to B4 |
| `crates/ffi/tests/support/health.rs` | `deck-streak-ffi` | added: the card health collection's builder |
| `crates/ffi/examples/review-fixture.rs` | `deck-streak-ffi` | changed: it writes `health/` beside the review fixture |
| `ios/App/Sources/AnswerClock.swift` | app | added: the clock and its pause |
| `ios/App/Sources/CardHealthModel.swift` | app | added: the sheet's state |
| `ios/App/Sources/CardHealthView.swift` | app | added: the sheet |
| `ios/App/Sources/EngineSession.swift` | app | changed: `cardHealth` and `CardInfoSection` |
| `ios/App/Sources/ReviewSession.swift` | app | changed: the clock, `health`, `pause` and `resume` |
| `ios/App/Sources/ReviewModel.swift` | app | changed: the panel's model |
| `ios/App/Sources/ReviewChrome.swift` | app | changed: the "Card info" control and the sheet |
| `ios/AppTests/AnswerClockTests.swift` | app tests | added: B6's sequences |
| `ios/AppTests/ReviewSessionTests.swift` | app tests | changed: B5, and B6's text card |
| `ios/AppUITests/ReviewActionsFlowTests.swift` | app UI tests | changed: B7 |
| `ios/swift-roles.json` | app | changed: the three app files and the test file, each with its role and its decisions |
| this SPEC's section 8, `docs/red-first/SPEC-507.md`, `changelog.d/card-health-panel-iphone-507.md` | docs | changed, changed, added |

### 8.5 What part b does NOT cover

- The scheduler's live readout, as section 5 says: every build answers the absent state until the port gains it.
- Translations: the sheet's words are the adapter's, in English, as its other sentences are.
- The web panel, which part b leaves as it is.

### 8.6 What only the Apple lane proves

- B5 to B7 run only on macOS, with Xcode and the iPhone and iPad simulators. On Linux the Swift is neither built
  nor run, and the UniFFI bindings for the new record are generated by the `xcframework` job.
