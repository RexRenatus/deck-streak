# SPEC-507: The card health panel on the web shows a card's own record, and no figure the scheduler did not give

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
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
R4. The panel's capped total equals the analytics crate's capped seconds over the same rows, times 1000, and
    its answer count equals the analytics crate's review count.
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
R8. No verdict: the panel's strings hold no judgement of the card ("hard", "difficult", "ambiguous", "leech
    risk", "problem", "struggling", "weak"); it shows facts and the scheduler's labelled estimate only.
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
| A1 | a fixture card's reps, lapses, capped mean and median, rows newest first and provenance, through the two fixed reads; a 3,600,000 ms row counts as 60,000 (R1, R2, R3, R7) | `cargo test -p deck-streak-engine-core --test health -- --exact the_panel_reads_a_fixture_cards_figures_through_the_two_fixed_reads` |
| A2 | the capped total and the answer count equal the analytics crate's over generated rows, and after real answers through the engine the card's reps and lapses equal the counts over its rows (R4, R5) | `cargo test -p deck-streak-engine-core --test health -- --exact the_panels_capped_time_equals_the_analytics_functions_over_generated_rows` and `... --exact the_cards_reps_and_lapses_equal_the_counts_of_its_review_rows` |
| A3 | outside Ready, the recall rows name their state and no number; the stored FSRS state never reaches them (R6) | `cargo test -p deck-streak-engine-core --test health -- --exact no_scheduler_figure_reaches_the_recall_rows_outside_ready` and the Vitest test `the recall rows read the state's line in every state but Ready` |
| A4 | the panel's strings say "The scheduler estimates" and hold no verdict word, and the panel names the collection and the created date (R7, R8) | Vitest `the panel's strings label the estimate and name no verdict` and `the panel says where the card came from` |
| A5 | the answer clock excludes every overlay's time over random sequences and the panel pauses it, and the panel's module opens no fetch and adds no route (R9, R10) | Vitest `the answer clock excludes every overlay's time`, `the card info panel pauses the answer clock` and `the panel's module opens no fetch and adds no route` |
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
- The panel on iPhone and iPad: the native record and its sheet are their own delivery.
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
