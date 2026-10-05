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
