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
- **Two mutation-coverage cases gain a positive assertion.** The tdd probe read `input.test.ts`
  "the switch is kept under its own name, and a device with none keeps no switch and throws
  nothing" and `review-screen.test.ts` "the screen is headed Review" as asserting only absences.
  The first now also reads the stored `off` back from the storage it wrote, an added line; the
  second reads the heading's text instead of its presence, a stronger assertion on the same
  element.
- **Three rewrites with no change of behaviour.** The card request takes no event, a device with
  no storage is checked outside the try, and no intent resolves to no action; each removed a mutant
  no test could tell from the original.


## Part 2: the fence, line by line

Part 2's criteria are A24 to A28 and A30, in SPEC-350 section 11; A29 is #685's. Each of the 20
lines of section 11's fence resolves to a test this delivery adds, or to a test that stands on
`dev` and is named as it is.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A24 | `crates/web-engine/tests/study.rs` `each_name_the_core_asks_for_is_wanted_once` | added (step 1) |
| 2 | A24 | `crates/web-engine/tests/study.rs` `a_file_is_read_no_further_than_its_limit` | added (step 1) |
| 3 | A24 | `crates/web-engine/tests/boundary.rs` `each_boundary_function_reaches_the_engine_through_the_dispatcher` | named: its `OWED` grows by `faces` (step 2) |
| 4 | A24 | `web/app/src/lib/engine/session.test.ts` "faces asks the engine twice, the second time with the files it named" | added (step 3) |
| 5 | A24 | `web/app/src/lib/engine/media.test.ts` "the worker reads each name the engine asks for, no further than its limit" | added (step 3) |
| 6 | A24 | `web/app/src/lib/engine/media.test.ts` "the media rules have one copy" | added (step 3) |
| 7 | A24 | `web/app/src/lib/study/review.test.ts` "the frame shows the faces the engine completed" | added (step 5) |
| 8 | A24 | `web/app/src/lib/card/frame-document.test.ts` "a data: media source reaches the frame unchanged" | added (step 4) |
| 9 | A25 | `web/app/src/lib/card/policy.test.ts` "the card frame's policy fetches only data and runs no script" | named: unchanged |
| 10 | A25 | `web/app/src/lib/card/card-sinks.test.ts` "card HTML reaches the page only through the card frame" | named: SPEC-341's, unchanged |
| 11 | A25 | `web/app/src/lib/card/card-frame.test.ts` "the card frame is a sandboxed srcdoc frame with no token" | named: unchanged |
| 12 | A26 | `web/app/src/lib/study/audio.test.ts` "the face's clips play in order on show and reveal, and replay replays them" | added (step 5) |
| 13 | A26 | `web/app/src/lib/study/audio.test.ts` "a blocked play leaves the replay control" | added (step 5) |
| 14 | A26 | `web/app/src/lib/study/input.test.ts` "the remote's replay fires on either side" | added (step 5) |
| 15 | A27 | `web/app/src/lib/study/voice.test.ts` "a speech clip's voice is the stored choice, else the language's default" | added (step 6) |
| 16 | A27 | `web/app/src/lib/study/voice.test.ts` "a speech clip is spoken in its language at the web's rate" | added (step 6) |
| 17 | A27 | `web/app/src/lib/study/voice.test.ts` "the picker stores the choice per language" | added (step 6) |
| 18 | A28 | `web/app/src/lib/manifest.test.ts` "the manifest meets the install criteria" | added (step 7) |
| 19 | A28 | `web/app/src/lib/csp.test.ts` "the page policy admits WebAssembly compilation and nothing else new" | named: unchanged |
| 20 | A30 | `web/app/src/lib/study/mapping-store.test.ts` "a stored mapping drives the review, and each mode keeps its default" | added (step 8) |

## Part 2: the reds and greens

Each line's command is the criterion's line in SPEC-350 section 11's fence, run at the commit named.

```red-first
A24: red at 803693bb: assertion `left == right` failed: left: [("cat.mp3", 9), ("cat.mp3", 9), ("dog.ogg", 7)] right: [("cat.mp3", 9), ("dog.ogg", 7)]
A24: green at f8ae12cd
A25: not red: its three tests stand on dev, named as they are and unchanged by this delivery, which must keep them green
A26: red at e4935e82: AssertionError: expected [] to deeply equal [ true ]
A26: green at f8ae12cd
A27: red at 45de2676: AssertionError: expected null to be { voiceURI: 'markus', …(2) } // Object.is equality
A27: green at f10a2333
A28: red at d97ce3ce: AssertionError: expected null to deeply equal { name: 'DeckStreak', …(4) }
A28: green at e8723819
A30: red at 876861a9: AssertionError: expected [ 'good', 'good', 'again' ] to deeply equal [ 'good', 'undo', 'good', 'again' ]
A30: green at aef11216
```

## Part 2: what the reds and greens disclosed

- **A24 is red by its first test, and green where its last turned green.** The native tests were
  red at 803693bb. Its TypeScript lines were each committed before the code they hold:
  `session.test.ts` and `media.test.ts` at ac6a31d0, and `review.test.ts` "the frame shows the
  faces the engine completed" at e4935e82, the last to turn green, at f8ae12cd. Its eighth line,
  `frame-document.test.ts` "a data: media source reaches the frame unchanged", passed when it was
  written at 003f7d76: the frame already passed a `data:` source through, so the test pins that
  and proves no new behaviour.
- **A26's other two lines were red at e4935e82 too**, re-measured in a checkout of that commit:
  "a blocked play leaves the replay control" (`expected [] to deeply equal [ false ]`) and
  `input.test.ts` "the remote's replay fires on either side" (`expected [ 'replay', 'replay' ] to
  deeply equal [ 'replay', 'replay', 'replay', …(1) ]`).
- **A27's other two titles were red at 45de2676 by assertion**: `expected [] to deeply equal [
  …(2) ]` and `expected [] to deeply equal [ 'de-DE', 'ja-JP' ]`.
- **A30's other tests were red at 876861a9.** The route's red is `a11y-coverage.test.ts`
  (`expected [ '/', '/about', '/badges', …(12) ] to deeply equal [ '/', '/about', '/badges',
  …(13) ]`), with the route page on disk and the route table not yet grown. The store's other
  tests were red by assertion, and the screen's three by `TestingLibraryElementError` over the
  stub screen, which held no heading, button or checkbox. All are green at aef11216.
- **Commits between a red and its green that touch a test file:**
  - f9269f2d added a third native test, `a_sound_takes_the_type_the_table_gives_its_name`, beyond
    the two the plan named; it decides no criterion, and it is S35043's killer.
  - 7167420c, step 2's green, changed one `OWED` statement in `boundary.rs` that f9269f2d added,
    because rustfmt put a trailing comma after the `Files::new(` argument; no assertion changed.
  - ac6a31d0 grew part 1's `OPS` list in `protocol.test.ts` by `'faces'` (GROWN), and inserted a
    `wants` field and a `faces` method into `session.test.ts`'s `FakeEngine`.
  - 20b1b3c8 edited the `OWED` statement in `boundary.rs` in the same commit that made `wasm.rs`
    consume the media contents by value.
  - ced6a00d, step 3's green, made one type-only edit to `media.test.ts`'s helper: `FakeFolder`'s
    map became `Map<string, Uint8Array<ArrayBuffer>>`, for svelte-check. No case was edited.
  - 003f7d76 is step 4's new test, above; e4935e82 is step 5's tests.
  - 45de2676 added a test outside the fence to `review-screen.test.ts`, "the review screen plays
    the face, and offers its replay and its voices", and an insert-only second import line,
    `import type { Clip, Faces }`.
  - 876861a9, step 8's red, gave `StudyInput` a stub third constructor parameter, the mapping,
    which it ignored, so the red test reads red by assertion rather than by a type error.
  - aef11216, step 8's green, grew `startapp.test.ts`'s `BY_PATH` by `/study/mapping` (GROWN, with
    a SPEC-350 R18 comment), and typed the two expected-pair lists in `mapping-store.test.ts` as
    `[string, string][]` and `[number, string][]` for svelte-check; no assertion changed. It also
    added `review-screen.test.ts` "the review reads the mapping this device stores, and links to
    its screen", outside the fence, which read red on the uncommitted tree before the review
    screen passed the mapping on (`expected [ 'card', 'rate 1 3 0', 'card' ] to deeply equal [
    'card' ]`).
- **A27's fence title is quoted so a static reader resolves it.** The tdd probe's
  `acceptance-has-a-test` found that `voice.test.ts` wrote the title "a speech clip is spoken in
  its language at the web's rate" with an escaped quote, which it reads as no match. The title is
  now in double quotes; the string vitest matches is unchanged.
- **The study suite gains no test.** A replay control needs a card with a sound, which the suite's
  synthetic notes do not hold and no seeding route may add; the mapping screen holds no deck, and
  its behaviour is held by the render tests above. `tests/a11y.spec.ts` audits every path of the
  route table, so `/study/mapping` is audited in both colour schemes once it joins the table.
- **The icons were made by this standard-library script**, run from the repository root:

  ```python
  import struct, zlib
  def chunk(kind, data):
      return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
  for n, name in ((192, 'icon-192.png'), (512, 'icon-512.png'), (180, 'apple-touch-icon.png')):
      rows = (b'\x00' + bytes((15, 23, 42)) * n) * n
      header = struct.pack('>IIBBBBB', n, n, 8, 2, 0, 0, 0)
      with open('web/app/static/' + name, 'wb') as out:
          out.write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header) + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b''))
  ```

- **Rows S35040 to S35043** hold the media reader: a name asked twice is wanted once, a file is
  read no further than its limit, a file not read is absent, and an extension is read without
  case. Each was proved KILLED by its killer.

## Part 2: what the mutation pass added

- **The mapping screen, mutation coverage, green when written.** The web stage's mutation pass
  found six survivors in `MappingScreen.svelte`, and two tests in `mapping-store.test.ts` kill
  them; each passed at the commit that added it. "the mapping screen says what it does, names its
  columns and links back to the review" pins the screen's introduction, its key and gamepad
  columns' headings and the back link's name. "a key a capture takes does nothing else on the
  page, and Cancel shows only while a capture waits" pins the prevented default of the key a
  capture takes, a key the page keeps while no capture waits and while a button's capture waits,
  and no Cancel while no capture waits.
- **The Worker's media and the voices, mutation coverage, green when written.** The same pass
  found a survivor in `worker.ts` and one in `voice.ts`, each killed by a test that passed at the
  commit that added it. `worker.test.ts` "a scope with no navigator reads no media, and throws
  nothing" pins the media read in a Worker scope with no navigator, and `voice.test.ts` "the voice
  choices are kept under their own name on this device" pins the one storage key every language's
  choice is kept under, written in the test.
- **One rewrite with no change of behaviour.** The player's turn in `audio.ts` was a counter
  compared only for equality, so counting down in place of up was a mutant no test could tell
  from the original. The turn is now a token compared by identity; `audio.test.ts`, which holds a
  later sequence stopping the last, is unchanged and green.

## Part 2: the census reports what it examined

- **The media census reports its count through the examined contract.** "the media rules have one
  copy" in `media.test.ts` printed how many source files it read but asserted nothing on the count,
  so an empty walk would have passed it. It now reports through a local `examined` helper that
  prints the same line and fails when the count is zero; no assertion was removed or loosened, and
  the test is green.

## Part 2: the rebuild

- **The icons are built from text, and the branch was rebuilt so that no commit holds a binary
  file.** The public scrub refuses every binary file in the tree and in its history, and the first
  cut committed the three icons as PNG files at step 7's green. Before anything was pushed, the
  branch was rebuilt locally from f10a2333: step 7's red and green were made again, the icons now
  built at build time by an endpoint at each icon's path that the build prerenders, and every later
  commit was taken from its first-cut commit by path, with the same subject. The script in "what the
  reds and greens disclosed" above made the first cut's icons; no commit on this branch holds them,
  and ADR-361 D16's amendment records the change.
- **A28's test was rebuilt too, red first.** It had read each icon from `static/`; at d97ce3ce it
  reads each icon from the endpoint at its path, checks the endpoint is prerendered and answers
  `image/png`, and checks every chunk's checksum and the pixels the image data inflates to. Its red
  at d97ce3ce is the manifest's assertion, as before: `AssertionError: expected null to deeply
  equal { name: 'DeckStreak', …(4) }`. It is green at e8723819, where StrykerJS killed all 42
  mutants of `web/app/src/lib/icon.ts` and the three endpoints with it. A30's red was re-measured at
  876861a9 and reads as its line above.
- **The red-first lines above name the rebuilt commits.** Commits 9edb096a to f10a2333 are
  unchanged. Each first-cut commit after them, and the commit that replaced it:
  - 1ea03704 -> d97ce3ce, step 7's red;
  - 5307dd94 -> e8723819, step 7's green;
  - 55398e36 -> 876861a9, step 8's red;
  - 0e47c89b -> aef11216, step 8's green;
  - 40e4d4f5 -> 2867f22d, the rows;
  - 74ba7cd1 -> 046974dd, this record's reds and greens, written with the rebuilt commits' names;
  - 882a4c11 -> ac35bd23, the mapping screen's mutation coverage;
  - d6760976 -> 49b4c61c, the player's turn;
  - 19c55fac -> 1df3f5ed, the media reader's and the voices' mutation coverage;
  - ad2e0f13 -> ee12bee7, the media census's examined count.
- **Paths.** SPEC-350 section 10's path bullet names `web/app/static/icon-192.png`,
  `web/app/static/icon-512.png` and `web/app/static/apple-touch-icon.png`, which no commit on this
  branch touches. In their place the branch touches `web/app/src/lib/icon.ts`,
  `web/app/src/routes/icon-192.png/+server.ts`, `web/app/src/routes/icon-512.png/+server.ts` and
  `web/app/src/routes/apple-touch-icon.png/+server.ts`.
- **The icons name no content type.** At 7290f87b the web stage's whole vitest run was red on A24's
  census, "the media rules have one copy": `AssertionError: expected [ Array(1) ] to deeply equal
  []`, with `+ "web/app/src/lib/icon.ts: image/png"`. The endpoint named the PNG media type, a type
  of the core's closed table, so the app held a copy of a media rule. The name entered at e8723819,
  so the census was red from there to 7290f87b; the first cut's icons were files under `static/`,
  outside the census's population, and named no type. At 9ef7bc7e the answer names no content type:
  the build writes an answer of 200 to the file at its path whatever its type, and the host types
  the file by its extension. A28's test changed with it, disclosed here: its endpoint reader checked
  the answer's `image/png` content type, and now checks the answer's 200, the status the build
  writes a file for. No other line of the test changed, and its red at d97ce3ce is still the
  manifest's assertion, which runs first. At 9ef7bc7e the census and A28 are green, the build writes
  the three files at 192, 512 and 180 pixels as before, and StrykerJS killed all 39 mutants of
  `web/app/src/lib/icon.ts` and the three endpoints (33, 2, 2 and 2), with none surviving.

## Issue 685: the fence, line by line

SPEC-350 section 14's fence holds three lines for A29, the criterion #685 delivers. Each resolves to a test this delivery adds.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A29 | `scripts/tests/test_release_workflow.py` `test_the_release_carries_the_module_at_web_engine` | added |
| 2 | A29 | `scripts/tests/test_release_workflow.py` `test_the_release_builds_and_gates_the_module_as_ci_does` | added |
| 3 | A29 | `scripts/tests/test_release_workflow.py` `test_an_over_budget_module_stops_the_release_before_the_draft` | added |
