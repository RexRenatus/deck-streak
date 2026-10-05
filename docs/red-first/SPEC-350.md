# Red-first record: SPEC-350

The SPEC, its schematic, ADR-361, this record and the changelog fragment were committed first. Each
criterion's test was then committed before the code that turns it green, and each red below is
quoted from the run at the red commit. This delivery is part 1 of 2: its criteria are A1 to A23;
section 7's A24 to A30 are the next pull request's.

## The fence, line by line

Each of the 27 lines of SPEC-350 section 3's fence resolves to a test this delivery adds, or to a
test that stands on `dev` and is named as it is.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/web-engine/tests/study.rs` `the_study_calls_are_the_reviews_pairs` | added (step 2) |
| 2 | A2 | `crates/engine-core/tests/table.rs` `the_review_pairs_are_ordinary_on_the_web` | added (step 1) |
| 3 | A2 | `crates/engine-core/tests/parity.rs` `each_adapter_table_equals_its_transport_column` | named: SPEC-345's, unchanged |
| 4 | A3 | `crates/web-engine/tests/study.rs` `only_the_shown_card_is_rated_buried_or_flagged` | added (step 3) |
| 5 | A4 | `crates/web-engine/tests/study.rs` `the_flag_toggles_red` | added (step 3) |
| 6 | A5 | `crates/web-engine/tests/study.rs` `bury_is_the_users_bury_of_the_shown_card` | added (step 3) |
| 7 | A6 | `web/app/src/lib/engine/protocol.test.ts` "each study operation parses its arguments and refuses any other" | added (step 5) |
| 8 | A7 | `web/app/src/lib/engine/session.test.ts` "each study operation reaches its engine call" | added (step 5) |
| 9 | A8 | `web/app/src/lib/engine/client.test.ts` "the client posts each study operation and settles its answer" | added (step 5) |
| 10 | A9 | `web/app/src/lib/study/locale.test.ts` "the engine's languages follow the app's locale" | added (step 7) |
| 11 | A10 | `web/app/src/lib/study/engine.test.ts` "the app starts the engine's Worker in one place" | added (step 7) |
| 12 | A11 | `web/app/src/lib/study/review.test.ts` "the review shows, reveals, rates and moves on" | added (step 7) |
| 13 | A11 | `web/app/src/lib/study/review.test.ts` "a gesture during a request fires nothing" | added (step 7) |
| 14 | A12 | `web/app/src/lib/study/review.test.ts` "a refused card keeps its answer controls" | added (step 7) |
| 15 | A13 | `web/app/src/lib/study/answer-buttons.test.ts` "each answer button names its grade and its interval" | added (step 8) |
| 16 | A14 | `web/app/src/lib/study/input.test.ts` "every source reaches one action" | added (step 7) |
| 17 | A14 | `web/app/src/lib/study/input.test.ts` "the key switch silences single-character keys" | added (step 7) |
| 18 | A15 | `web/app/src/lib/study/input.test.ts` "focus returns to the review" | added (step 7) |
| 19 | A16 | `web/app/src/lib/study/review-screen.test.ts` "the screen lock follows the review, the gamepad and the page" | added (step 8) |
| 20 | A17 | `web/app/src/lib/study/study-calls.test.ts` "the study screens call only the study operations" | added (step 8) |
| 21 | A18 | `web/app/src/lib/card/frame-document.test.ts` "the frame body carries the card's classes and nothing else" | added (step 6) |
| 22 | A19 | `web/app/src/lib/a11y-coverage.test.ts` "the accessibility audit covers every route in both colour schemes" | named: unchanged, red at step 9 |
| 23 | A20 | `web/app/src/lib/study/study-coverage.test.ts` "the study suite covers the review loop in both engines" | added (step 10) |
| 24 | A21 | `scripts/tests/test_ci_workflows.py` `test_the_web_engine_job_runs_the_study_suite` | added (step 10) |
| 25 | A22 | `scripts/tests/test_web_engine_stage.py` `test_the_stage_refuses_a_missing_module` | added (step 10) |
| 26 | A23 | `web/app/src/lib/card/card-sinks.test.ts` "card HTML reaches the page only through the card frame" | named: SPEC-341's, unchanged |
| 27 | A23 | `web/app/src/lib/csp.test.ts` "the page policy admits WebAssembly compilation and nothing else new" | named: unchanged |

## The reds and greens

Each line's command is the criterion's line in SPEC-350 section 3's fence, run at the commit named.

```red-first
A1: red at 686e0132: assertion `left == right` failed: left holds the eight study calls, right the review's sixteen
A1: green at 56f905c0
A2: red at 2131fda2: assertion `left == right` failed: DecksService.DeckTree (7, 4) on the web, left: NotAllowed, right: Admit
A2: green at 56f905c0
A3: red at 108a6e53: assertion `left == right` failed: left: Ok(Shown { card: 42, states: "the states shown with card 42", flag: 0 }), right: Err(NotShown)
A3: green at 4ffc0357
A4: red at 108a6e53: assertion `left == right` failed: left: 0, right: 1
A4: green at 4ffc0357
A5: red at 108a6e53: assertion `left == right` failed: left: BuryOf { card_ids: [42], note_ids: [], mode: 1 }, right: BuryOf { card_ids: [42], note_ids: [], mode: 2 }
A5: green at 4ffc0357
A6: red at cb6f38a3: AssertionError: expected { id: 1, …(1) } to deeply equal { request: { id: 1, op: 'decks' } }
A6: green at ab7d5aea
A7: red at cb6f38a3: AssertionError: expected { id: 1, ok: false, …(2) } to deeply equal { id: 1, ok: true, value: { …(2) } }
A7: green at ab7d5aea
A8: red at cb6f38a3: TypeError: client.decks is not a function
A8: green at ab7d5aea
```

Two tests decide no criterion and were still seen red first. The engine's English default (R4's
`["en"]` when empty), `crates/web-engine/tests/study.rs`
`the_engine_speaks_english_when_no_language_is_given`, was red at 108a6e53 (`left: [], right:
["en"]`) and green at 4ffc0357. The boundary census of the six new exports (its `OWED` list, grown
insert-only), `crates/web-engine/tests/boundary.rs`, was red at f6784c3f (`fn deck_tree(` occurs 0
times, not once) and green at d1afa325.
