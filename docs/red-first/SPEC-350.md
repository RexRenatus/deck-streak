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
A9: red at 7bf7e931: AssertionError: expected [ [ 'en', [ 'en' ] ], …(6) ] to deeply equal [ [ 'en', [ 'en' ] ], …(6) ]
A9: green at 8456c54d
A10: red at 7bf7e931: AssertionError: expected {} to deeply equal { 'src/lib/study/engine.ts': 1 }
A10: green at 8456c54d
A11: red at 7bf7e931: AssertionError: expected [ 'busy', …(1) ] to deeply equal [ 'busy', [ 'card', 'rate 1 3 0' ] ]
A11: green at 8456c54d
A12: red at 7bf7e931: AssertionError: expected [] to deeply equal [ 'show-answer', 'bury', 'flag' ]
A12: green at 8456c54d
A13: red at a5575873: AssertionError: expected [ 'Again', 'Hard', 'Good', 'Easy' ] to deeply equal [ 'Again <1m', 'Hard <6m', …(2) ]
A13: green at 6861ca7f
A14: red at 7bf7e931: AssertionError: expected [ 'again', 'hard', 'good', …(5) ] to deeply equal []
A14: green at 8456c54d
A15: red at 7bf7e931: AssertionError: expected +0 to be 1
A15: green at 8456c54d
A16: red at a5575873: AssertionError: expected [ 1, +0 ] to deeply equal [ +0, +0 ]
A16: green at 6861ca7f
A17: red at a5575873: AssertionError: expected { …(1) } to deeply equal {}
A17: green at 6861ca7f
A18: red at 4485f65a: AssertionError: card card1: expected [] to deeply equal [ 'class' ]
A18: green at f223b824
A19: red at 3528ac25: AssertionError: expected [ '/', '/about', '/badges', …(10) ] to deeply equal [ '/', '/about', '/badges', …(12) ]
A19: green at 965ef25f
A20: red at 2b3bda11: AssertionError: tests-study/study.spec.ts does not exist: expected false to be true
A20: green at a511f110
A21: not red: test_ci_workflows.py runs in CI only, so CI's hygiene job decides the test by name, and the mutation-rows job's kills of S35030 and S35031, each the job without one of its new steps, are its red evidence
A22: red at 2b3bda11: AssertionError: Tuples differ: (0, {'deck_streak_web_engine_bg.wasm': b'\x00asm\x01\x00\x00\x00'}) != (1, {})
A22: green at a511f110
A23: not red: its two tests stand on dev, named as they are and unchanged by this delivery, which must keep them green; both pass at a511f110
```

Two tests decide no criterion and were still seen red first. The engine's English default (R4's
`["en"]` when empty), `crates/web-engine/tests/study.rs`
`the_engine_speaks_english_when_no_language_is_given`, was red at 108a6e53 (`left: [], right:
["en"]`) and green at 4ffc0357. The boundary census of the six new exports (its `OWED` list, grown
insert-only), `crates/web-engine/tests/boundary.rs`, was red at f6784c3f (`fn deck_tree(` occurs 0
times, not once) and green at d1afa325.

## What A9 to A23 disclosed

- **A11 and A14 were each red by their second test.** Each has two lines in the fence. A11's first
  test, `review.test.ts` "the review shows, reveals, rates and moves on", and A14's first,
  `input.test.ts` "every source reaches one action", passed at their stubs at 7bf7e931. They are
  recorded here as passed at the stub, and were not re-shaped to read red. Each criterion's red is
  its second test's: "a gesture during a request fires nothing" (the stub rated again on each
  press) and "the key switch silences single-character keys" (the stub silenced nothing).
- **A13, A16 and A17 were red over the screens' stubs at a5575873.** A13's buttons were named by
  their grade alone; A16's screen held the lock's condition without the gamepad; A17's screen held
  one planted `answer(` call, which the census found and the green removed. At the same red, the
  three `deck-list.test.ts` tests (`Unable to find an accessible element with the role "status"`,
  the stub showing its loading text only) and three `review-screen.test.ts` tests ("the review
  screen shows the card, reveals it and rates it", "keys, the gamepad and a tap on the card reach
  the review", and "a refusal, a card the frame refuses and a done deck are announced", each failing
  for the stub's buttons or its adapter) were red too; they decide no criterion. At the green
  6861ca7f, `deck-list.test.ts`'s retry click changed from an awaited `fireEvent.click` to
  `.click()` and then `flushSync()`, because the awaited click passed through the list's transient
  loading state; no assertion changed.
- **A19 is named, unchanged.** The two study routes joined the route table at 965ef25f, and the
  audit's coverage test, which holds the table equal to the screens on disk, read red at 3528ac25
  with the two route pages on disk and the table not yet grown.
- **A20 holds the study suite's shape; the suite itself runs in CI.** The Playwright suite
  `web/app/tests-study/study.spec.ts` serves the app's build with the module staged beside it, so
  it runs in CI's `web-engine` job, after that job builds the module, and is read there by name.
  A20's test, which runs everywhere, holds the suite to the review loop's seven steps, both
  engines, the persistent profiles, the seed through the build's own Worker, and the audit of both
  screens.
- **A21 is decided in CI.** Its test module runs in CI only. Its red evidence is the
  mutation-rows job's two kills: S35030 and S35031 each remove one of the web-engine job's new
  steps, and the test refuses each.
- **Tests that decide no criterion.** `refusal.test.ts` "each refusal names its own message in
  every locale" was red at 7bf7e931 (`expected { …(8) } to deeply equal { …(8) }`, every status
  mapped to one message) and green at 8456c54d. `review.test.ts` "the frame keeps its card while a
  request is in flight" (`expected null to deeply equal { view: { id: 1n, …(7) }, …(1) }`) and
  `input.test.ts` "the device's storage is the browser's, or none where reading it throws"
  (`expected undefined to be MemoryStorage{ items: Map{} }`) were each red at 72edab88 and green at
  d8806e1d.
- **Mutation coverage, green when written.** `engine.test.ts` "a refused open starts again, and
  the app's engine is the engine's module Worker" was added at the green 8456c54d;
  `review-screen.test.ts` "a gamepad the page already had holds the lock once the page is visible,
  and closing releases it" was green at a5575873; and `src/routes/study.test.ts` was green at
  3528ac25. Each holds behaviour its criterion's test does not reach, so a mutant there dies.
- **R6 and R7 as amended.** SPEC-350 section 10 records them: a collection with no deck shows a
  message only, held by `deck-list.test.ts` "a collection with no deck says so", and a done deck
  shows its designed end with a link back to the deck list and never navigates on its own, held by
  `review-screen.test.ts` "a refusal, a card the frame refuses and a done deck are announced".

## What A1 disclosed

- **A1's green commit grows two oracles, insert-only.** Commit 56f905c, the green of A1 and A2, edits
  `crates/engine-core/tests/table.rs` and `crates/web-engine/tests/study.rs`: the web list in the first
  grows from eight pairs to sixteen, and the study calls in the second gain the same eight rows, each
  an added line and no assertion rewritten. Both oracles pinned the eight pairs the review's pairs now
  join, so they had to grow with the table the commit changes (SPEC-350 R1, M10).

## What the mutation pass added

- **Mutation coverage, green when written.** The tests below were each written after the web
  stage's mutation pass found a survivor in the code they hold, and each passed at the commit that
  added it. `review.test.ts` pins every cell of the review's table, a fresh review's phase and
  side, the one change `start` announces, an undo before any card, a client that fails with no
  code, and a card whose answer alone escapes the frame. `frame-document.test.ts` pins a card's
  frame document with no classes and a body the frame would give attributes. `client.test.ts`
  pins an open that names no languages, `protocol.test.ts` the eighth language and a language
  list nested in a list, and `engine.test.ts` an open refused after the page was hidden and
  after a newer start. `input.test.ts` pins the focus a fresh input takes back, the switch's own
  storage name, a device with no storage and a forgotten gamepad; `deck-list.test.ts` the link
  back to Today; and `review-screen.test.ts` the screen's heading, the one gamepad the page
  already had leaving, a gamepad read afresh after it left, and a pointer after a Tab.
- **The web engine's boundary census grows, mutation coverage.** After dev's table joined the
  branch, the diff's own mutation pass found six survivors in `src/wasm.rs`, a module no native
  test runs. `tests/boundary.rs` reads that module's source, so its owed list gains three rows
  and a statement, each an added line: `deck_json`, `joined` and `undo` as functions, and the
  deck tree's seconds conversion. No assertion is rewritten and none removed.
- **Three rewrites with no change of behaviour.** The card request takes no event, a device with
  no storage is checked outside the try, and no intent resolves to no action; each removed a mutant
  no test could tell from the original.

