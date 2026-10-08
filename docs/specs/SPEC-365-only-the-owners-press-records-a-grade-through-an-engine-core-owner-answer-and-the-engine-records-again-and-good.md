# SPEC-365: only the owner's press records a grade, through an engine-core `OwnerAnswer`, and the engine records two grades, Again and Good

- **Issue:** `#711`. **Decision:** the owner's grading decision, which settles the question
  issue #175 raised: grading is two buttons. It settles three things. The two-button rule: every
  surface that grades a card offers exactly two grades. The owner-press rule: only the owner's own
  tap or press grades a card, held by an engine-core token. The late-review rule: a late review is
  told plainly that it does not count toward the streak. This SPEC builds the owner-press rule, the
  engine's half of the two-button rule and the native client's half. The web's half of the
  two-button rule and the late-review copy are section 7's.
- **Context(s):** `deck-streak-engine-core` (`crates/engine-core`), `deck-streak-ffi`
  (`crates/ffi`), `deck-streak-web-engine` (`crates/web-engine`), `miniapp`
  (`web/app/src/lib/engine`, `web/app/tests-engine`), the native harness (`ios/Harness`), the
  native app (`ios/App`) and the native codec (`ios/HarnessWire`).
- **Decided by:** ADR-376 (this SPEC's own). It amends ADR-356 D2 (the table) and D4
  (containment), ADR-361 D1 (its rejection of a token on every grade), ADR-348's JS boundary
  (the `answer` export, and the wire's four ratings) and ADR-359 D5 (which side picks a native
  answer's next state). Each amended ADR gains an insert-only
  amendment section at its end that names ADR-376, and no existing line of any of them changes. It works under ADR-301 (the never-list, which it leaves untouched), ADR-356
  D5 (the gesture, its precedent) and ADR-361 D2 (a rating reaches only the shown card, which it
  keeps).
- **Schematic:** `docs/schematics/owner-press-to-recorded-grade.md` (the data flow from a press to the recorded grade, on
  both clients; this delivery adds it).
- **Status:** delivery D1 of three, which lands second (section 7). **Mutation band:** `S36500-S36599` (section 9). **Model:** none
  (section 8).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `dee337bc` by `git grep -n` and `git show`. Nothing was
run: every figure is a read.

### 1a. Where a grade is recorded today

| door | path | what it admits | who can reach it |
|---|---|---|---|
| the core's `run` | `crates/engine-core/src/table.rs:184-190` (AnswerCard (13,4), an ordinary row, `native: true, web: true`), `dispatch.rs:112-130` | any card, any of the engine's four ratings | every adapter caller, on both transports |
| the native `run` | `crates/ffi/src/allow_list.rs:54-58`, `crates/ffi/src/engine.rs:94-103` | the same, from the native caller's bytes | any native caller |
| the web `run_method` | `crates/web-engine/src/study.rs:88` (`STUDY_CALLS` holds (13,4)), `wasm.rs:656-659` | the same, from the page's bytes | any script in the page's Worker |
| the web `answer` | `crates/web-engine/src/wasm.rs:251-270` | the queue's head card, which the engine chooses rather than a press, with any of four wire ratings | 0 product callers; tests: `engine.spec.ts` 5 calls, `session.test.ts` 14 lines, `client.test.ts` 4, `protocol.test.ts` 1, `credential.test.ts` 1 |
| the web `rate` | `crates/web-engine/src/wasm.rs:485-502`, `study.rs:28-36` | only the kept card (ADR-361 D2), with any of four wire ratings | the review screen's press, through `review.ts:279-280`, `client.ts:109-110` and `session.ts:264-265` |
| the native harness | `ios/Harness/Sources/EngineSession.swift:106-120` (`answerGood`, through `engine.run` (13,4) at `:118` and `:129`) | Good only, from its one button (`ReviewView.swift:17`) | the harness's button |

So three doors record a grade with no press behind it: the core's and the native `run`, the web
`run_method`, and the web `answer`. Five engine entry points admit four grades: the core's `run`,
the native `run`, `run_method`, `answer` and `rate`. `ios/App` records no grade: its pairs are
(3,0), (7,13) and (1,3) (`ios/App/Sources/EngineSession.swift:8-10`).

At this delivery's cut, `99702eb9`, the native review screen is on `dev`, and
it answers a card through the native `run` with AnswerCard (13,4). `git grep -n -E '\(13, *4\)' --
ios` reads `ios/App/Sources/EngineSession.swift:13`, `ios/App/Sources/EngineSession.swift:117` and `ios/Harness/Sources/EngineSession.swift:11`, and the screen offers `case again, hard, good, easy` at `ios/App/Sources/ReviewView.swift:12`. R12 moves each caller.

Test fixtures outside the core call the engine's own `answer_card` at four lines: `col.answer_card(`
at `crates/ingest/tests/support/mod.rs:346` and `:393` and at
`crates/ingest/tests/undo_and_full_sync.rs:95`, plus the string literal `".answer_card(",` at
`crates/ingest/tests/skip_census.rs:49`. The containment census does not name `answer_card` today
(`crates/engine-core/tests/containment.rs:27-50`, 22 names), so nothing holds a fifth.

### 1b. Why the answer is not the gesture

The owner's gesture (`crates/engine-core/src/gesture.rs`, 167 lines) is the precedent. It has
private fields, is neither `Clone` nor `Copy`, has one constructor (`from_tap`), and is consumed by
`Dispatcher::run_exempt` (`dispatch.rs:142-151`), which decodes, checks and encodes the request
again. It is held by the crate graph and by the census (`containment.rs`: `GESTURE_NAMES` `:53`,
`ENTRY_FILES` `:56`, `ENTRY_CALLS` `:59`, 20 held lines). It guards `EXEMPT` (`table.rs:240-289`,
6 rows): the never-list writes ADR-301 (`:105-119`) lets the owner tap.

ADR-361 D1 (`:37-55`) rejected "a token on every grade" (`:50-52`). Its "What would make this
wrong" (`:345-346`) said grades, if they became owner gestures, would move into `EXEMPT` with the
token. Answering is not a never-list write, so this SPEC gives the answer a token of its own and a
row of its own (ADR-376 D2). Nothing joins `EXEMPT`.

### 1c. The state the change meets

The three crates' `src` hold no new shared state after this delivery. The existing state is
`wasm.rs:65-70` (`thread_local!`: `DISPATCHER`, `POOL`, `LAST_PANIC`, `SHOWN`), a `Mutex` at
`dispatch.rs:50` (the media folder) and a `Mutex` at `crates/ffi/src/voices.rs:63`. The token is a
value made and consumed inside one call. None of the edited files carries a formal cover: the
covers under `formal/` that name `crates/engine-core` are `src/full_sync.rs` (7) and
`src/credential.rs` (5).

## 2. Requirements

R1. **The two grades.** A new module, `crates/engine-core/src/answer.rs`, declares
    `pub enum Grade { Again, Good }`. `Grade::rating` answers the engine's rating numbers, 0 for
    Again and 2 for Good, and `Grade::pick(again, good)` selects the grade's next state. No type in
    the core names a third grade.

R2. **The owner's answer.** `pub struct OwnerAnswer` holds one card id and one `Grade` in private
    fields. It is neither `Clone`, `Copy` nor `Default`, and `OwnerAnswer::from_press(card, grade)`
    is its one constructor.

R3. **The one door that records a grade.** `Dispatcher::run_answer(answer: OwnerAnswer, input:
    &[u8]) -> Result<Vec<u8>, AnswerRefusal>` consumes the answer. It decodes `input` as the engine's
    `CardAnswer`, refusing `AnswerRefusal::Undecodable` when it cannot. It refuses
    `AnswerRefusal::NotTheCard { pressed, named }` when the request names another card, and
    `AnswerRefusal::NotTheGrade { pressed, named }` when the request's rating is not the grade's.
    Otherwise it runs AnswerCard on the checked message, encoded again, never on the caller's
    bytes, and passes on an engine error as `AnswerRefusal::Engine { error }`.

R4. **The table.** AnswerCard (13,4) leaves `ORDINARY` (18 rows to 17) and becomes the one row of
    a new set, `ANSWERED`. `decide` answers `Decision::NeedsAnswer` for it on both transports, and
    every other pair's decision is unchanged. `EXEMPT` keeps its 6 rows.

R5. **`run` holds an answer for the owner.** `Dispatcher::run(13, 4, …)` refuses with
    `Refusal::NeedsAnswer { service: 13, method: 4 }` before the engine sees the call, and the card
    is not answered.

R6. **The native door.** `ALLOW_LIST` drops AnswerCard (10 rows to 9), so the native `run` refuses
    it as not allowed. `Engine::answer(card, grade: PressedGrade, states, milliseconds_taken) ->
    Result<Vec<u8>, PressRefusal>` is the adapter's one answer entry and the only place it builds an
    `OwnerAnswer`. It decodes `states` as the engine's `SchedulingStates`, the states the queue gave
    the card when it was shown, and refuses `PressRefusal::Undecodable` when it cannot, before the
    engine sees anything. It builds the `CardAnswer` from them: the card, the current state,
    `grade.pick(states.again, states.good)` as the next state, the grade's own rating, the time the
    adapter's clock reads, and `milliseconds_taken`. It then mints
    `OwnerAnswer::from_press(card, grade)` and calls `run_answer`. `PressedGrade { Again, Good }`
    and `PressRefusal` are its foreign types, beside `ExemptTap` and `ExemptRefusal`.
    `EngineRefusal` is unchanged.

R7. **The web door.** The wire names two grades: 1 is Again and 3 is Good. 2 and 4 are refused by
    name as `StudyError::NotAGrade(n)`, and any other number as `RatingOutOfRange`. `STUDY_CALLS`
    drops (13,4) (16 rows to 15), so `run_method` refuses it. `rate` builds the request from the
    kept card's states with `grade.pick(states.again, states.good)`, mints
    `OwnerAnswer::from_press(shown.card, grade)` and calls `run_answer`. The queue-head `answer`
    export is removed. `call()` refuses `Refusal::NeedsAnswer` as it refuses `NeedsGesture`.

R8. **Containment.** Outside the core, only the two entry files (`crates/ffi/src/engine.rs` and
    `crates/web-engine/src/wasm.rs`) name `OwnerAnswer`, `from_press` or `run_answer`, and each of
    them builds the answer from a press and runs it. The engine's `answer_card` appears outside the
    core only at its four held lines (section 1a). The census plants a caller of each new name and
    refuses each by name.

R9. **The page.** The Worker protocol drops its `answer` operation (`protocol.ts:13`, `:46`;
    `client.ts:72-73`; `session.ts:33`, `:248-249`), and refuses that op as `parseRequest` refuses any op `OPS` does not name
    (`unknown operation answer`, code `bad-request`). Each of the engine suite's five queue-head answers becomes the review's own pair:
    show the next card, then rate it.

R10. **The native harness.** `answerGood` answers through
    `engine.answer(card:grade:states:millisecondsTaken:)` with `.good` and the head card's encoded
    states, and a `PressRefusal` reads as its own sentence beside the existing `EngineRefusal`
    sentences (`EngineSession.swift:20-38`).

R11. **The words.** `docs/LEXICON.md`'s glossary gains `grade` and `owner answer` (engine-core). No
    fence line is added: the core declares the engine's own `rating`, which a lock would refuse.

R12. **The native callers on `dev`.** Every native caller that answers a card through the native
    `run` with AnswerCard (13,4) at this delivery's cut answers through
    `Engine::answer(card, grade, states, milliseconds_taken)` instead, naming the press's card, its
    grade, Again or Good, and the states the card was shown with. No Swift source under `ios/`
    writes the pair `(13, 4)`, and a planted source that writes
    it is refused by name.

R13. **The native client's two grades.** The codec's `Rating`
    (`ios/HarnessWire/Sources/HarnessWire/Messages.swift`) names two cases, `again = 0` and
    `good = 2`, and the app's own rating type names Again and Good alone. The review screen's
    answer bar shows two grade buttons, Again then Good, Good the visually primary one, each with
    its interval under its title. A press answers through `engine.answer` with the shown card, its
    grade and the encoded `SchedulingStates` the queue gave it, so no Swift source sends the engine
    a next state of its own choosing. No Swift source under `ios/` names a third grade. Code that
    no caller reaches after the move keeps its tests, narrowed to two grades: nothing is deleted
    for being unused.

## 3. Acceptance criteria

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | AnswerCard decides `NeedsAnswer` on both transports, and every other pair's decision is unchanged | the red stub adds `Decision::NeedsAnswer`, but no row returns it: `decide(Native, 13, 4)` reads `Admit` | `crates/engine-core/tests/table.rs` `every_pair_is_admitted_held_or_refused_by_its_transport` (name kept; a fourth arm and an `ANSWERED` set) |
| A2 | `run` refuses an answer with `NeedsAnswer { 13, 4 }`, and the card's `reps` stays 0 | `run` still admits the pair: `Ok`, and `reps` reads 1 | `crates/engine-core/tests/dispatch.rs` `an_answer_through_run_is_held_for_the_owner_and_leaves_the_card` |
| A3 | A press answers its one card with its grade: on two new cards, Again and Good each raise `reps` from 0 to 1 and leave the card in the next state the engine itself gave for that grade | not red: the red stub runs the request unchecked, and that is already this criterion's answer. A4 to A6 and rows `S36503` to `S36509` hold the check | `crates/engine-core/tests/answer.rs` `a_press_answers_its_one_card_with_its_grade` |
| A4 | A press for one card cannot answer another: `NotTheCard { pressed, named }`, and both cards' `reps` stay 0 | the unchecked stub answers the other card: `Ok`, and its `reps` reads 1 | `answer.rs` `a_press_for_one_card_cannot_answer_another` |
| A5 | A press records only its own grade: Again against ratings 1, 2 and 3, and Good against 0, 1 and 3, each `NotTheGrade`, with `reps` at 0 (examined 6 of 6 pairs). No token can carry Hard or Easy | the unchecked stub records each request's rating: `Ok` 6 times | `answer.rs` `a_press_records_only_its_own_grade` |
| A6 | A request that is not a `CardAnswer` is refused `Undecodable` before the engine sees it | the unchecked stub hands the bytes to the engine, which answers `Engine { .. }` | `answer.rs` `a_request_that_is_not_a_card_answer_is_refused` |
| A7 | `OwnerAnswer` is neither `Clone`, `Copy` nor `Default`; the controls read as each type is | the red stub derives `Clone` and `Copy` (the planted forbidden shape): it reads `(true, true, false)` | `crates/engine-core/tests/containment.rs` `the_owner_answer_is_neither_clone_nor_copy` |
| A8 | Outside the core, only the two entry files name the answer, and each builds it from a press and runs it. `answer_card` appears only at its four held lines, and every plant is refused by name | committed before either adapter's door, the census reads its entries as `[]` against both entry files | `containment.rs` `no_non_ui_caller_reaches_an_exempt_function` (name kept; SPEC-345 A15) |
| A9 | The wire names two grades: 1 picks Again's state and 3 Good's; 2 and 4 are `NotAGrade`; 0 and 5 are out of range | the red stub admits every wire rating from 1 to 4: `grade(2)` reads `Ok(Again)` | `crates/web-engine/tests/study.rs` `a_rating_on_the_wire_picks_its_answer_and_its_next_state` (name kept; SPEC-338 A1) |
| A10 | `run_method` admits only the study calls, and AnswerCard is not one | `STUDY_CALLS` still holds (13,4) | `crates/web-engine/tests/study.rs` `run_method_admits_only_the_study_calls` (name kept) |
| A11 | `rate`'s body mints the answer from the kept card, picks one of two states and runs `run_answer`; it calls AnswerCard nowhere through `call`, and no `answer` export remains | `rate`'s body still reads `call(service::SCHEDULER, 4, …)` | `crates/web-engine/tests/boundary.rs` `each_boundary_function_reaches_the_engine_through_the_dispatcher` (name kept) |
| A12 | The native `run` refuses AnswerCard as not allowed, and the card's `reps` stays 0 | `ALLOW_LIST` still admits it: `Ok`, and `reps` reads 1 | `crates/ffi/tests/round_trip.rs` `a_native_run_refuses_an_answer_and_leaves_the_card` |
| A13 | A native press answers its card with its grade's own next state: on two new cards, Again and Good each raise `reps` from 0 to 1 and leave the card in the next state the engine itself gave for that grade, and the other card's `reps` stays 0; states that are not a `SchedulingStates` are refused `Undecodable` before the engine sees them | the red adapter picks Good's state for either grade and decodes with `unwrap_or_default`: Again's card reads Good's next state, and the undecodable states read `Engine { .. }` | `round_trip.rs` `a_native_press_answers_only_its_card_with_its_grade` |
| A14 | The Worker refuses an `answer` request as an op it does not name: the reply is `ok: false`, code `bad-request`, message `unknown operation answer` (`protocol.ts:202`, `parseRequest`) | the session still answers it with the head card's id | `web/app/src/lib/engine/session.test.ts` "the session refuses the queue-head answer as an unknown operation" |
| A15 | No Swift source under `ios/` writes the pair `(13, 4)`; a planted tuple, with or without spaces, is refused by name, and the same text in a comment or a string is not counted | the harness writes it (`ios/Harness/Sources/EngineSession.swift:11`) | `scripts/tests/test_ios_answer_door.py` `test_no_swift_source_answers_through_run` (added) |
| A16 | No Swift source under `ios/` names a third grade: no `case` declares `hard` or `easy`, and no expression names `.hard` or `.easy`; a planted case and a planted member are each refused by name, and the same words in a comment or a string are not counted | at the cut, the review screen's rating type and the codec's `Rating` declare `hard` and `easy` (`ios/App/Sources/AnswerBar.swift:23`, `:25`, `ios/App/Sources/ReviewSession.swift:34`, `ios/App/Sources/ReviewView.swift:12`, `ios/AppTests/ReviewModelTests.swift:25`, `:126`, `ios/HarnessWire/Sources/HarnessWire/Messages.swift:9`, `:11`, `ios/HarnessWire/Tests/HarnessWireTests/RequestBytesTests.swift:109`, `:111`, `:118`, `:119`, `ios/HarnessWire/Tests/HarnessWireTests/ResponseDecodingTests.swift:152`, `:153`) | `scripts/tests/test_ios_answer_door.py` `test_no_swift_source_names_a_third_grade` (added) |

```acceptance
A1: cargo test -p deck-streak-engine-core --test table -- --exact every_pair_is_admitted_held_or_refused_by_its_transport
A2: cargo test -p deck-streak-engine-core --test dispatch -- --exact an_answer_through_run_is_held_for_the_owner_and_leaves_the_card
A3: cargo test -p deck-streak-engine-core --test answer -- --exact a_press_answers_its_one_card_with_its_grade
A4: cargo test -p deck-streak-engine-core --test answer -- --exact a_press_for_one_card_cannot_answer_another
A5: cargo test -p deck-streak-engine-core --test answer -- --exact a_press_records_only_its_own_grade
A6: cargo test -p deck-streak-engine-core --test answer -- --exact a_request_that_is_not_a_card_answer_is_refused
A7: cargo test -p deck-streak-engine-core --test containment -- --exact the_owner_answer_is_neither_clone_nor_copy
A8: cargo test -p deck-streak-engine-core --test containment -- --exact no_non_ui_caller_reaches_an_exempt_function
A9: cargo test -p deck-streak-web-engine --test study -- --exact a_rating_on_the_wire_picks_its_answer_and_its_next_state
A10: cargo test -p deck-streak-web-engine --test study -- --exact run_method_admits_only_the_study_calls
A11: cargo test -p deck-streak-web-engine --test boundary -- --exact each_boundary_function_reaches_the_engine_through_the_dispatcher
A12: cargo test -p deck-streak-ffi --test round_trip -- --exact a_native_run_refuses_an_answer_and_leaves_the_card
A13: cargo test -p deck-streak-ffi --test round_trip -- --exact a_native_press_answers_only_its_card_with_its_grade
A14: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "the session refuses the queue-head answer as an unknown operation"
A15: python3 -m unittest discover -s scripts/tests -p test_ios_answer_door.py -k test_no_swift_source_answers_through_run
A16: python3 -m unittest discover -s scripts/tests -p test_ios_answer_door.py -k test_no_swift_source_names_a_third_grade
```

Shipped tests whose names this delivery keeps and whose bodies it changes, so that their SPECs'
fence lines still resolve:
- SPEC-345 A3: `crates/engine-core/tests/dispatch.rs` `an_ordinary_call_reaches_the_engine_and_an_exempt_one_does_not` (`:75-139`). Its ordinary call becomes another admitted pair, because an answer no longer goes through `run`.
- SPEC-338 A1 (A9 above) and A11. SPEC-338 A11's tests keep their titles and answer by show and rate: the engine suite's, including "opens, answers and undoes over OPFS", and `session.test.ts`'s "the session opens, answers and undoes through the engine".
- The native `a4_answers_the_card` (`crates/ffi/tests/round_trip.rs:214`). It answers through `Engine::answer`.
- SPEC-348 A14 (`scripts/tests/test_ios_review_screen.py` `the_answer_bar_is_named_placed_and_felt`), A17, A19 and A20 (the review screen's unit and UI tests, as SPEC-348 names them): each reads two grades, Again and Good, and keeps its name. Every kept native test that names Hard or Easy (`ios/AppTests/ReviewModelTests.swift:25`, `:126`, `ios/HarnessWire/Tests/HarnessWireTests/RequestBytesTests.swift:109`, `:111`, `:118`, `:119`, `ios/HarnessWire/Tests/HarnessWireTests/ResponseDecodingTests.swift:152`, `:153`) names Again or Good instead.

The helpers that answer through `run` move to `run_answer`:
- `crates/engine-core/tests/exempt.rs` `answer_next` (`:129-159`)
- `crates/ffi/tests/exempt.rs` `answered` (`:86`)
- `crates/ffi/tests/round_trip.rs` `good_answer` (`:122`)

Both `review_pairs.rs` lists drop (13,4): the core's at `:49`, the native one at `:56`.

Shipped requirements this delivery changes, by their own SPECs (their text is not edited; this
SPEC and ADR-376 record the change, and their tests keep their names):

| shipped requirement | what changes |
|---|---|
| SPEC-338 R1 (`:48-56`), A1 (`:108`, `:131`) and A11 (`:119`, `:142`) | the wire names two grades; the engine suite answers by show and rate |
| SPEC-345 A3 (`:128`), R10 (`ENGINE_NAMES`), A15 and A16 | an answer no longer goes through `run`; the census gains names, entry calls and held lines |
| SPEC-350 R2 | `rate`'s wire carries two grades |
| SPEC-336 R2 (`allow_list.rs:23-28`) | the allow-list drops AnswerCard |
| SPEC-348 R10 and R11, and A14, A17, A19 and A20 | the review screen offers two grades, Again and Good, each with its interval, and a press answers through `engine.answer` |
| SPEC-339 R9 (the codec) | the codec's `Rating` names two grades |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-365-only-the-owners-press-records-a-grade-through-an-engine-core-owner-answer-and-the-engine-records-again-and-good.md` | docs | added |
| `docs/decisions/ADR-376-the-owner-answer-token-holds-a-grade-to-a-press-and-the-engine-records-again-and-good.md` | docs | added |
| `docs/decisions/ADR-356-the-engine-core-holds-the-engine-for-both-clients-behind-a-per-transport-table.md` | docs | an insert-only amendment section appended (D2, D4) |
| `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md` | docs | an insert-only amendment section appended (D1) |
| `docs/decisions/ADR-348-the-web-engine-pins-the-forks-wasm32-patches-by-tag-and-crosses-into-a-worker-through-wasm-bindgen.md` | docs | an insert-only amendment section appended (the JS boundary) |
| `docs/decisions/ADR-359-the-review-screens-media-face-voice-choice-deck-tree-rating-and-test-seam.md` | docs | an insert-only amendment section appended (D5) |
| `docs/schematics/owner-press-to-recorded-grade.md` | docs | added |
| `docs/red-first/SPEC-365.md` | docs | added |
| `docs/LEXICON.md` | docs | two glossary rows (R11) |
| `changelog.d/grade-token-365.md` | docs | added |
| `crates/engine-core/src/answer.rs` | engine-core | added (R1 to R3), with a native press's two codec steps (R6; ADR-376 D15) |
| `crates/engine-core/src/lib.rs` | engine-core | `pub mod answer` and its line in the module list |
| `crates/engine-core/src/table.rs` | engine-core | AnswerCard leaves `ORDINARY` and becomes `ANSWERED`'s one row; `Decision::NeedsAnswer`; `decide` (R4) |
| `crates/engine-core/src/dispatch.rs` | engine-core | `Refusal::NeedsAnswer`, its arm in `run`, and `run_answer` (R3, R5) |
| `crates/engine-core/tests/answer.rs` | engine-core | added (A3 to A6) |
| `crates/engine-core/tests/table.rs` | engine-core | `NATIVE` and `WEB` drop (13,4); an `ANSWERED` set and a fourth arm (A1) |
| `crates/engine-core/tests/dispatch.rs` | engine-core | A2; SPEC-345 A3's ordinary call |
| `crates/engine-core/tests/exempt.rs` | engine-core | `answer_next` answers through `run_answer` |
| `crates/engine-core/tests/review_pairs.rs` | engine-core | `NATIVE` drops (13,4) |
| `crates/engine-core/tests/containment.rs` | engine-core | the answer's names and entry calls, `answer_card`, its held lines, the plants, and A7 (R8) |
| `crates/engine-core/tests/full_sync.rs` | engine-core | `engine_error` reads `Refusal::NeedsAnswer` as a refusal made before the engine (R5) |
| `crates/ffi/src/allow_list.rs` | ffi | AnswerCard leaves `ALLOW_LIST` (R6) |
| `crates/ffi/src/engine.rs` | ffi | `Engine::answer`, `PressedGrade` and `PressRefusal`; `NeedsAnswer` in `refusal()` (R6) |
| `crates/ffi/tests/round_trip.rs` | ffi | A12 and A13; `good_answer` and `a4_answers_the_card` answer through `Engine::answer` |
| `crates/ffi/tests/exempt.rs` | ffi | `answered` answers through `Engine::answer` |
| `crates/ffi/tests/review_pairs.rs` | ffi | the native list drops (13,4) |
| `crates/web-engine/src/study.rs` | web-engine | `grade`, `StudyError::NotAGrade`; `Answer` removed; `STUDY_CALLS` drops (13,4) (R7) |
| `crates/web-engine/src/wasm.rs` | web-engine | `rate` through `run_answer`; `answer` removed; `NeedsAnswer` in `call()` (R7) |
| `crates/web-engine/tests/study.rs` | web-engine | A9 and A10; the review's pairs drop (13,4) (`:68`, `:156`) |
| `crates/web-engine/tests/boundary.rs` | web-engine | `rate`'s owed statements and a `pressed` entry (`OWED` 29 rows to 30; the base holds no `answer` entry), and the retired texts with their planted control (A11) |
| `scripts/mutation-equivalent.d/deck-streak-web-engine.json` | scripts | `answer`'s three records retire with the function (`:159`, `:168`, `:177`); `rate`'s re-anchor if its first lines move |
| `scripts/mutation-rows.d/S36500-S36599.json` | scripts | added (section 9) |
| `web/app/src/lib/engine/protocol.ts` | miniapp | the `answer` op leaves the op list and the request union (R9) |
| `web/app/src/lib/engine/client.ts` | miniapp | `answer` removed |
| `web/app/src/lib/engine/session.ts` | miniapp | the engine's `answer` and its case removed |
| `web/app/src/lib/engine/session.test.ts` | miniapp | A14; the `answer` cases removed |
| `web/app/src/lib/engine/client.test.ts` | miniapp | the `answer` cases removed |
| `web/app/src/lib/engine/protocol.test.ts` | miniapp | `answer` leaves the op list |
| `web/app/src/lib/engine/credential.test.ts` | miniapp | its `answer` request becomes a `rate` |
| `web/app/tests-engine/engine.spec.ts` | miniapp | five queue-head answers become show and rate; titles kept |
| `ios/Harness/Sources/EngineSession.swift` | native harness | `answerGood` through `engine.answer`; the `PressRefusal` sentence (R10) |
| `ios/App/Sources/EngineSession.swift` | native app | the answer path through `engine.answer` (R12) |
| `scripts/tests/test_ios_answer_door.py` | scripts | added (A15, A16) |
| `ios/swift-roles.json` | native | a moved file's decision count, only where it changes (`scripts/tests/test_ios_thin_swift.py`) |
| `ios/HarnessWire/Sources/HarnessWire/Messages.swift` | native codec | `Rating` names two cases, Again and Good (R13) |
| `ios/HarnessWire/Tests/HarnessWireTests/RequestBytesTests.swift` | native codec | a Hard or Easy case reads Again or Good; names kept (R13) |
| `ios/HarnessWire/Tests/HarnessWireTests/ResponseDecodingTests.swift` | native codec | a Hard or Easy case reads Again or Good; names kept (R13) |
| `ios/App/Sources/AnswerBar.swift` | native app | the app's rating type and the answer bar name two grades; tests' names kept (R13) |
| `ios/App/Sources/ReviewSession.swift` | native app | the app's rating type and the answer bar name two grades; tests' names kept (R13) |
| `ios/App/Sources/ReviewView.swift` | native app | the app's rating type and the answer bar name two grades; tests' names kept (R13) |
| `ios/AppTests/ReviewModelTests.swift` | native app | the app's rating type and the answer bar name two grades; tests' names kept (R13) |
| `scripts/tests/test_ios_review_screen.py` | scripts | SPEC-348 A14's bar names Again and Good; name kept (R13) |

## 5. What this does NOT do

- It changes no web grade surface: the web's buttons, keys, remote and stick mappings, the stored
  mapping and the grade strings in the seven locales are delivery D2's (section 7, `#712`).
- It writes no late-review copy, and it does not let a late review rejoin the streak. The copy is
  delivery 3's, and rejoining is later work under the charter's revisable verdicts (`#713`).
- It leaves Undo unchanged (`#714`).
- It leaves every sync path unchanged (`#715`).
- It moves no row into `EXEMPT` and adds no never-list exemption. ADR-301 and the owner-taps ruling
  are untouched (`#716`).
- It adds no native key, remote or stick mapping: the native client maps none at this delivery's
  cut, and ADR-342's list for it stands as written (`#717`).

## 6. Risks

- **A Hard or Easy press is refused if this delivery lands before the web's buttons are two.** The
  engine now refuses wire ratings 2 and 4 by name, so a web still sending them would show the
  review's refusal instead of recording a grade. The delivery order puts the web half of D2 first
  (section 7), and the build re-measures at its cut that the web sends only 1 and 3, and stops if
  it does not.
- **The native review screen answers through the native `run`, with four grades.** It lands on
  `dev` before this delivery, and its answer path is the native `run` with AnswerCard (SPEC-348
  R11), offering Again, Hard, Good and Easy. This delivery removes that pair from the allow-list,
  narrows the screen to Again and Good (R13) and moves its answer path to `Engine::answer` (R12)
  in the same pull request, so no commit on `dev` leaves the screen answering through a refused
  pair or offering a grade the door cannot name. From the screen's merge to this delivery's, `dev`
  offers four native grades, which the engine still records through the native `run`. A15, A16
  and the review screen's flow tests (C2, C3) hold it.
- **A forged next state.** The token checks the card and the rating, not the next state. Both
  clients now pick it beside the token by the same two-armed pick: the web's `rate` from the kept
  states, which the boundary census holds (A11), and the native `Engine::answer` from the states
  the caller passes (A13, rows `S36515` and `S36516`). A native caller that passes states the
  queue did not give could still have one recorded; the engine refuses a stale current state, and
  the review log shows the rest.
- **Swift is built only in CI.** The narrowed screen, codec and tests compile only in the native
  workflow, so the build's own checks of them are the Python censuses (A15, A16) and SPEC-348's
  A14; C2 and C3 read the rest on the pull request.
- **A renamed shipped test orphans a fence line.** SPEC-338, SPEC-345 and the native round trip
  name tests this delivery edits, so each keeps its name (section 3).
- **A census that refuses an unrelated identifier.** The six new names (`OwnerAnswer`,
  `from_press`, `run_answer`, `NeedsAnswer`, `AnswerRefusal`, `PressedGrade`) match 0 lines at
  `dev` today. The census's own run shows any new match by name.
- **A mutant the native build cannot see.** `wasm.rs` compiles only for the browser, so its
  mutants are equivalent natively. The boundary census is their one native killer (rows
  `S36512` and `S36513`). The three equivalence records on `answer` retire with it.

## 7. Delivered by the other pull requests

Delivery D2 makes the web's grade surfaces two buttons: the review's buttons, keys, remote and
stick, the stored mapping, the protocol's `Rating`, the messages, and a census proving that only
the review's grade cells send `rate`. It stands alone on the engine as it is (the engine accepts
the wire's four ratings and the web then sends two), so it lands first. The native client's half
lands in this delivery (R13), after the open native review pull request, and no separate native
pull request follows. Delivery D3 writes the late-review copy and is independent. Each is outlined
in the design's split, with its own SPEC, ADR and criteria; `#717`.

## 8. Formal model

None, decided by surface.
- **TLA+ is not applicable.** The doors that reach AnswerCard shrink to the owner's presses: the
  web's `rate` on the Worker's one thread, and the native `Engine::answer`. No new actor is added.
  The token is a value made and consumed inside one call, and the delivery adds no `static`,
  `OnceLock`, `LazyLock` or `thread_local!` item. The check compares two values the call owns,
  which nothing else can change between the check and the act. The engine's own refusal of a stale
  current state is unchanged.
- **Lean is not applicable.** The rule is two comparisons and one table row, which unit tests and
  mutation rows hold, as ADR-374 D12 decided for four comparisons.

A builder may raise either decision, and says why.

## 9. Mutation rows

Each row names a behaviour of R1 to R7, its mutant and the one test that kills it. Rows live in
`scripts/mutation-rows.d/S36500-S36599.json`, table `MUTATIONS`.

| row | file | mutant | killer |
|---|---|---|---|
| `S36501-ANSWER-CARD-NEEDS-AN-ANSWER` | `engine-core` `src/table.rs` | the `ANSWERED` arm of `decide` never matches (`… && false`) | `table::every_pair_is_admitted_held_or_refused_by_its_transport` |
| `S36502-RUN-HOLDS-AN-ANSWER-FOR-THE-OWNER` | `engine-core` `src/dispatch.rs` | `run`'s `NeedsAnswer` arm refuses `NotAllowed` | `dispatch::an_answer_through_run_is_held_for_the_owner_and_leaves_the_card` |
| `S36503-A-PRESS-ANSWERS-ONLY-ITS-CARD` | `engine-core` `src/answer.rs` | `request.card_id != card` becomes `request.card_id != request.card_id` | `answer::a_press_for_one_card_cannot_answer_another` |
| `S36504-A-PRESS-RECORDS-ONLY-ITS-GRADE` | `engine-core` `src/answer.rs` | `request.rating != grade.rating()` becomes `request.rating != request.rating` | `answer::a_press_records_only_its_own_grade` |
| `S36505-GOOD-IS-THE-ENGINES-GOOD` | `engine-core` `src/answer.rs` | `Self::Good => 2,` becomes `Self::Good => 1,` | `answer::a_press_answers_its_one_card_with_its_grade` |
| `S36506-AGAIN-IS-THE-ENGINES-AGAIN` | `engine-core` `src/answer.rs` | `Self::Again => 0,` becomes `Self::Again => 2,` | `answer::a_press_answers_its_one_card_with_its_grade` |
| `S36507-GOOD-PICKS-GOODS-STATE` | `engine-core` `src/answer.rs` | `Self::Good => good,` becomes `Self::Good => again,` | `answer::a_press_answers_its_one_card_with_its_grade` |
| `S36508-AN-UNDECODABLE-REQUEST-IS-REFUSED` | `engine-core` `src/answer.rs` | the decode's refusal becomes `.unwrap_or_default()` | `answer::a_request_that_is_not_a_card_answer_is_refused` |
| `S36509-THE-ENGINE-RUNS-ONLY-THE-CHECKED-ANSWER` | `engine-core` `src/dispatch.rs` | `run_answer` falls back to the caller's bytes when the check refuses | `answer::a_press_for_one_card_cannot_answer_another` |
| `S36510-THE-ANSWER-ROW-IS-ANSWER-CARD` | `engine-core` `src/table.rs` | the `ANSWERED` row's method 4 becomes 5 (a literal in a `const`, which the tool never mutates) | `table::every_pair_is_admitted_held_or_refused_by_its_transport` |
| `S36511-THE-WIRE-REFUSES-HARD-AND-EASY` | `web-engine` `src/study.rs` | `2 \| 4 => Err(StudyError::NotAGrade(rating)),` becomes `2 \| 4 => Ok(Grade::Good),` | `study::a_rating_on_the_wire_picks_its_answer_and_its_next_state` |
| `S36512-RATE-RECORDS-THROUGH-THE-ANSWER` | `web-engine` `src/wasm.rs` | `rate` calls `call(service::SCHEDULER, 4, …)` in place of `run_answer` | `boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher` |
| `S36513-RATE-PICKS-ITS-GRADES-STATE` | `web-engine` `src/wasm.rs` | `grade.pick(states.again, states.good)` swaps its arguments | `boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher` |
| `S36514-THE-NATIVE-PRESS-KEEPS-ITS-GRADE` | `ffi` `src/engine.rs` | `PressedGrade::Again => Grade::Again,` becomes `PressedGrade::Again => Grade::Good,` | `round_trip::a_native_press_answers_only_its_card_with_its_grade` |
| `S36515-THE-NATIVE-ANSWER-PICKS-ITS-GRADES-STATE` | `ffi` `src/engine.rs` | `grade.pick(states.again, states.good)` swaps its arguments | `round_trip::a_native_press_answers_only_its_card_with_its_grade` |
| `S36516-THE-NATIVE-ANSWER-REFUSES-UNDECODABLE-STATES` | `ffi` `src/engine.rs` | the states' decode refusal becomes `.unwrap_or_default()` | `round_trip::a_native_press_answers_only_its_card_with_its_grade` |

Removing (13,4) from `ALLOW_LIST` and `STUDY_CALLS` leaves no text a row could anchor on. Putting
it back would break each array's declared length, so the mutant would not compile. The parity test
(`crates/engine-core/tests/parity.rs:70`) and A10 hold those removals by enumeration. Encoding the
checked request again matches the caller's bytes under the same decoder on both sides, so it has
no row; it is kept as the gesture keeps it.

## 10. What only CI or a device proves

| id | criterion | who, and when |
|---|---|---|
| C1 | The native harness's Good button answers its card through `engine.answer`, and the deck's count falls by one | `HarnessUITests/HarnessFlowTests/test_a7_answers_the_card_good` (shipped) in the xcframework workflow's harness test step, on both simulators, on the pull request; the native code is built only there |
| C2 | The native review screen answers its card through `engine.answer`, and the deck's count falls by one | `ios/AppUITests/ReviewFlowTests.swift:51` `test_a20_show_reveal_rate_next` (shipped) in the xcframework workflow, on the pull request |
| C3 | The review screen's bar shows two grade buttons, Again then Good, each named by its title with its interval as its value, and Show Answer reveals the two | SPEC-348's UI tests `test_a19_the_ratings_sit_at_the_bottom_named_by_their_titles` and `test_a20_show_reveal_rate_next` (names kept) in the native workflow, on the pull request |
| V1 | On the device, a Good press in the harness answers the card it shows | the owner, in the acceptance session (`#718`) |
| V2 | On the device, the review screen offers Again and Good alone, and each press answers the card it shows | the owner, in the acceptance session (`#718`) |
