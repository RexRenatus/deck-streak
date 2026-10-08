# SPEC-371: Undo reverts only the review's own last answer, through the owner's gesture, after the review shows what it changes

- **Issue:** #714. **Decision:** the owner-taps ruling, whose three conditions every exempt tap
  meets: it is reachable only from the UI, and that is proven; it makes one gesture's own write on
  the card it names; and the write is shown before it is made, with what it changes. #714 measured
  that today's Undo meets none of them. This SPEC holds the undo of the review's own last answer,
  while that answer has not synced, to all three.
- **Context(s):** `deck-streak-engine-core` (`crates/engine-core`), `deck-streak-ffi`
  (`crates/ffi`), `deck-streak-web-engine` (`crates/web-engine`), and `miniapp` (`web/app`).
- **Decided by:** ADR-382 (this SPEC's own). It amends ADR-356 D2 (the table), D4 (containment)
  and D6 (the exempt table, now seven rows), ADR-361 D1 for undo only, and ADR-337's Decision
  Outcome. Each amended ADR gains an insert-only amendment section at its end, in the form
  SPEC-365's sections take (`## Amendment: <title> (SPEC-371)`, opening "ADR-382 amends"), and no
  existing line of any of them changes. ADR-301 gains a dated note at `:107` only if the build's
  base holds none; `dev` `2438c4c6` holds none (section 7). It works under ADR-348 (the new
  exports sit inside its boundary, as SPEC-350's did) and ADR-376 (its `Grade`, reused unchanged).
- **Schematic:** `docs/schematics/undo-the-reviews-own-last-answer.md` (the press to the confirmed
  write, and every refused path; this delivery adds it).
- **Status:** one delivery, built on `dev` with SPEC-365 (#711) and SPEC-366 (#712) both landed
  (section 7). **Mutation band:** `S37100-S37199` (section 9). **Model:**
  `formal/tla/UndoOwnAnswer`, required (section 8).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `2438c4c6`, which holds SPEC-365 and SPEC-366, by
`git show` and `git ls-tree`; the page's file counts and the five census lines were read at `dev`
`119e4476` by `git grep -n` and carried to `2438c4c6` by the diff between the two, in which no
changed line moves them. Each engine figure was read in the engine's pinned source, which the two
share. Nothing was run: every figure is a read.

### 1a. Where an undo is made today

| door | path | what it reverts | who can reach it |
|---|---|---|---|
| the core's `run` | `crates/engine-core/src/table.rs:144-150` (Undo (3,8), an ordinary row, `native: true, web: true`), `dispatch.rs:122-141` | the front of the engine's undo queue, whatever it is | every adapter caller, on both transports |
| the native `run` | `crates/ffi/src/allow_list.rs:61-65` (`ALLOW_LIST`, 9 rows) | the same | any native caller. No product caller: `Messages.swift:103-104` in HarnessWire builds the bytes, tested only at `RequestBytesTests.swift:60-61`, and 0 Swift lines run (3,8) |
| the web `run_method` | `crates/web-engine/src/study.rs:82` (`STUDY_CALLS`, 15 rows, holds (3,8)), `wasm.rs:650` | the same | any script in the page's Worker |
| the web `undo` | `crates/web-engine/src/wasm.rs:252-259`: `call(service::COLLECTION, 8, &[])` at `:256`, and it clears `SHOWN` | the same, from a call that names nothing | the review's press: the `UNDO` cell (`review.ts:42`), reached from the question (`:55`) and the answer (`:56`), sends `client.undo()` on the press (`:283-284`) |

So four doors reach (3,8), none of them names what it reverts, and the review's own control writes
on the press. `EXEMPT` (`table.rs:269-312`, 6 rows) holds no Undo, and the web's `run_exempt`
export maps its indices 0 to 5 to the six exempt writes (`wasm.rs:272-289`, the indices at
`:279-284`).

### 1b. What Undo writes

- It reverts the front of the engine's undo queue, whatever that is: an answer, a bury or a flag
  (`rslib/src/undo/mod.rs` `can_undo`, `:104-106`).
- Every begun operation advances a step counter (`:84-86`), and `undo_status` (`:195-200`) reports
  it as `last_step`, beside the label of the change at the front (an answer's label is the
  engine's op label, `ops.rs:60`).
- A normal sync discards the queue (`rslib/src/sync/collection/normal.rs:86`), so a synced answer
  reads `UndoEmpty` (`crates/ingest/tests/undo_and_full_sync.rs:359`).
- Undoing an answer removes its review row and restores the card: "the card is new and first
  again" (`crates/ffi/tests/round_trip.rs:381`, SPEC-336 A5).
- The page reads the label only as a boolean: `current_card` reads (3,7) at `wasm.rs:437` and
  ships it at `:448` as `CardView.undo` (`protocol.ts:105`), which `review.ts:177` and `:200` test
  for truth.

So a press of today's Undo after a bury reverts the bury, and nothing on the screen says which
change the press will revert.

### 1c. The state the change meets

- The web engine's state is four `thread_local!` items, `DISPATCHER`, `POOL`, `LAST_PANIC` and
  `SHOWN` (`wasm.rs:66-72`). This delivery adds one, `LAST_ANSWER` (R6). SPEC-365 section 8 says
  its delivery adds no `thread_local!` item. That sentence describes SPEC-365's own delivery and
  binds no later one; this delivery's item, with the offer and the confirmation it sits between,
  is why section 8 models it.
- No cover under `formal/` names a web-engine file today. The covers this delivery adds are the
  first.
- No UI calls an exempt write: `run_exempt`, `runExempt` and `exempt` match 0 lines in
  `web/app/src` and `ios`. Undo is the first exempt tap built, and the owner-taps ruling and
  ADR-337's order of work want a dated note at ADR-301 `:107` when the first one is built.
- The web cannot normal-sync: the only sync-service row in `ORDINARY` is SyncLogin
  (`table.rs:119`). The "already synced" arm is reached on the web only once a web sync lands. It
  is built and unit-tested now.
- The census (`crates/engine-core/tests/containment.rs`): `ENGINE_NAMES` (`:31-55`, 23 names,
  SPEC-365's `answer_card` among them) names no undo. `names()` (`:420`) counts an engine name
  that is called (`.name(`), pathed (`::name`) or in a use tree, with comments stripped and string
  literals not excluded, and a gesture name (`GESTURE_NAMES`, `:59-66`) as any identifier. So
  `HELD` (`:103`, 25 entries) holds the boundary census's owed literals that name `from_tap` and
  `run_exempt` (`:208`, `:214`, `:220`) and SPEC-365's two (`:226`, `:232`) under
  `BOUNDARY_CENSUS_LITERAL`. Once `undo` joins, it counts five lines outside the core: the bot's
  `commands.rs:657`, `one_router.rs:670`, and `crates/ingest/tests/undo_and_full_sync.rs:318`,
  `:359` and `:382`.
- The page: 9 product files under `web/app` mention undo, beside 7 locale files and 19 test files.

### 1d. Which never-list entry this is

The owner-taps ruling's table maps the never-list's entry 1 to undoing an answer after it has
synced, and ADR-361 D1 says the never-list does not name undo. This delivery builds to the
stricter reading. It holds the undo of an unsynced answer to all three of the ruling's
conditions, as an exempt write behind the owner's gesture, and it meets all three whichever
reading holds. The undo of a synced answer is not built (section 5).

## 2. Requirements

R1. **The scope.** Undo reverts only the review's own last answer, and only while that answer has
    not synced. After a bury or a flag, Undo is not offered (a flag toggles back by itself).

R2. **The table.** Undo (3,8) "CollectionService.Undo" leaves `ORDINARY` and joins `EXEMPT` as
    `ExemptWrite::Undo`, `TargetKind::Card`, under the existing `OwnerGesture`. `decide` answers
    `NeedsGesture` for it on both transports. HtmlToTextLine (27,14) joins `ORDINARY`, web only.
    `ORDINARY` keeps 17 rows (SPEC-365's 17, less Undo, plus HtmlToTextLine), `EXEMPT` grows from 6
    rows to 7, and every other pair's decision is unchanged.

R3. **The rule.** A new pure module, `crates/engine-core/src/undo_answer.rs`, named so that no
    `::undo` path enters a use tree once the census names `undo`. It declares `Recorded` (a prost
    message: `status: UndoStatus`, tag 1; `review: i64`, tag 2), `Review { cid, usn }`,
    `UndoRefusal { Gone, NotTheCard, Synced, Changed }`, `Returns { New, Learning, Review,
    Relearning, Preview }`, `judge(recorded, now: &UndoStatus, review: Option<Review>, card)` and
    `returns_to(&SchedulingState)`. `judge` decides in this order: no review row is `Gone`; a row
    for another card is `NotTheCard`; a row whose `usn` is not -1 is `Synced`; an empty undo label
    is `Gone`; a `last_step` or a label that differs from the record's is `Changed`; otherwise it
    admits. `returns_to` answers the kind of state the answer left: a filtered card's rescheduling
    state answers its original's kind, and a filtered preview answers `Preview`.

R4. **Two fixed reads.** `Read` (`dispatch.rs:89-94`) gains `Read::NewestReview` (`select id, cid
    from revlog order by id desc limit 1`) and `Read::Review(i64)` (`select cid, usn from revlog
    where id = ?`).

R5. **The check at the write.** `OwnerGesture::checked` answers an enum: `Run(service, method,
    request)` for the six existing writes, or `Undo { card, recorded }`, whose arm decodes
    `Recorded` and refuses `Undecodable` when it cannot. `Dispatcher::run_exempt` hands `Undo` to a
    private `run_undo`, which reads (3,7) and `Review(recorded.review)` through the backend, runs
    `judge`, and only then runs (3,8) with an EMPTY request. A refusal from `judge` is
    `GestureRefusal::NotTheTarget { write: Undo, target }`, and no `GestureRefusal` variant is
    added. `run(3, 8, …)` refuses `NeedsGesture` before the engine sees the call, and the answer
    stands.

R6. **The record.** `wasm.rs` holds one new `thread_local!`, `LAST_ANSWER: RefCell<Option<LastAnswer>>`.
    `LastAnswer { card, grade, returns, recorded }` lives in `study.rs`, and its `grade` is the study
    rule's `Grade` (`study.rs:14`, SPEC-365 R7). `rate`, once `run_answer` succeeds, reads (3,7)
    and `NewestReview`, and records only when the newest review's card is the rated card. A
    successful undo, `open` and `close` clear it.

R7. **The web exports.** `undo_offer() -> String` answers JSON, either
    `{"offer":{"card":"<id>","step":n,"text":"…","grade":"again"|"good","returns":"new"|"learning"|"review"|"relearning"|"preview"}}`
    or `{"offer":null,"why":"synced"|"none"}`. `undo(card: i64, step: u32)` refuses
    `StudyError::NotUndoable(Changed)` unless `(card, step)` equal the record's, runs `judge`,
    mints `OwnerGesture::from_tap(ExemptWrite::Undo, Target::Card(card))`, calls
    `run_exempt(gesture, &recorded.encode_to_vec())`, and then clears `LAST_ANSWER` and `SHOWN`.
    `StudyError::NotUndoable(UndoRefusal)` is new. `current_card`'s `undo` becomes `"answer"`,
    `"synced"` or `null`, by `judge`. The `run_exempt` export keeps its indices 0 to 5 and gains no
    sixth. `STUDY_CALLS` drops (3,8) and gains (27,14), so it keeps 15 rows, and `run_method`
    refuses (3,8).

R8. **The card's text.** The offer's `text` is the card's question as one line: the question
    nodes of (27,6) joined, then (27,9) StripAvTags, then (27,14) HtmlToTextLine with
    `preserve_media_filenames: true`. The index 14 is inferred, not read from a build: the engine's
    `card_rendering.proto` (`:14-31`) puts HtmlToTextLine 11th counting from 0, and (27,6)
    RenderExistingCard (rpc 3) and (27,9) StripAvTags (rpc 6) show that the backend service's
    three rpcs come first. The build measures it from a build before it writes anything, and A15
    pins it.

R9. **The review's machine** (`review.ts`). The offer and its confirmation are how the ruling's
    third condition is met: the press asks for the offer, the dialog shows the write with what it
    changes, and only the confirmation makes it. From the question or the answer, `undo` goes to busy
    with the effect `offer`. From busy, `offered` goes to `confirming`, and `not-offered` goes back
    to the side the review was on, with its notice. In `confirming`, `undo` goes to busy with the
    effect `undo`; `keep` and every other action go back to the side with no effect, and that
    other action is not carried out. From busy, `undo-refused` goes to the next card (a reload),
    with its notice. `act()` on `undo` when `view.undo` is `'synced'` announces `undo-synced` and
    sends nothing; when it is `null` it does nothing. The confirmation is the Undo action again
    (remote button 4, key `u`) or the dialog's "Undo answer" button. A key repeat is already
    dropped (`keys.ts:47`), and a press while the offer loads finds no cell in busy.

R10. **The screen** (`ReviewScreen.svelte`). In `confirming` the control bar becomes a
    `role="alertdialog"` region with `aria-labelledby` and `aria-describedby`, and focus moves to
    "Keep it". It shows "Card:" and the text line, rendered as text and never through `{@html}`;
    "Your answer:" and Again or Good (the existing `study_again` and `study_good`); "Goes back
    to:" and New, Learning, Review, Relearning or Preview, "in today's queue"; and "This answer
    has not synced yet. Undoing it removes it from your review history; nothing else changes."
    Two `<button>`s, each `min-h-11`, read "Undo answer" and "Keep it", and Escape keeps. The bar's
    button reads "Undo answer" (a new `study_undo_answer`; `study_undo` stays, for the mapping
    screen). For a synced answer the bar's button is disabled, and a press announces "Your last
    answer has synced, so it can no longer be undone." A confirmation that comes too late reads
    "This answer can no longer be undone: something changed after it." No text shames the choice
    to keep.

R11. **The messages.** 15 keys in each of the 7 locales, 105 strings: `study_undo_answer`,
    `undo_title`, `undo_card`, `undo_answer`, `undo_returns`, `undo_returns_new`,
    `undo_returns_learning`, `undo_returns_review`, `undo_returns_relearning`,
    `undo_returns_preview`, `undo_unsynced`, `undo_keep`, `undo_synced`, `undo_gone` and
    `undo_no_text`.

R12. **The Worker protocol.** `protocol.ts` gains an operation for the offer; the `undo` request
    carries `card` and `step`; `CardView.undo` becomes `'answer' | 'synced' | null`; and the error
    codes gain `undo-synced` (for `Synced`) and `not-undoable` (for every other refusal), which
    `session.ts` maps from `NotUndoable`. `client.ts` offers `undoOffer()` and `undo(card, step)`,
    and `session.ts` calls the engine's `undo_offer` and `undo(card, step)`.

R13. **Containment in the core.** `ENGINE_NAMES` gains `undo` (23 names at `2438c4c6`, 24 after).
    `HELD` (25 entries at `2438c4c6`) gains four entries, each with its reason, in the shape the
    sync probe's holds use: the bot's `commands.rs:657` `Some("undo") => self.undo().await,` (the
    bot's own habit undo); `one_router.rs:670` `("Commands::undo", "send"),` (a string literal);
    `undo_and_full_sync.rs`'s `col.undo().expect("the answer is undone");`, count 2 (`:318`,
    `:382`); and its `matches!(col.undo(), Err(AnkiError::UndoEmpty)),`, count 1 (`:359`). Each
    owed statement of `undo` in `crates/web-engine/tests/boundary.rs` that names `from_tap` or
    `run_exempt` is held under `BOUNDARY_CENSUS_LITERAL`, as the `run_exempt` export's and
    `rate`'s are. A planted `col.undo();` in a non-UI source is refused by name, as the census's
    plants of `from_tap` are. The gesture for Undo is minted only in `wasm.rs`'s `undo`.

R14. **Containment in the page.** A new census, `web/app/src/lib/study/undo-reach.test.ts`, holds
    `.undo(` to exactly 2 sites (`review.ts`, `session.ts`), `.undoOffer(` to 1 (`review.ts`) and
    `.undo_offer(` to 1 (`session.ts`), and holds the cells whose effect is `undo` to `confirming`
    alone. Each population prints its examined count and refuses zero, and each plant is refused
    by name, in SPEC-366 R9's shape. A locale census holds the 15 keys in every locale.

R15. **The native door.** `ALLOW_LIST` drops Undo (SPEC-365's 9 rows to 8; the docs at `:5` and
    `:23` follow), so the native `run` refuses (3,8) as not allowed, and
    `crates/ffi/tests/review_pairs.rs:67` drops it. `ExemptTap` (`engine.rs:315-329`) gains no
    variant, no Swift line changes, and HarnessWire's `Requests.undo()` stays a bytes builder.

## 3. Acceptance criteria

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | Undo decides `NeedsGesture` on both transports; HtmlToTextLine (27,14) is admitted on the web and refused natively; every other pair's decision is unchanged | `ORDINARY` admits Undo: `decide(Native, 3, 8)` reads `Admit` | `crates/engine-core/tests/table.rs` `every_pair_is_admitted_held_or_refused_by_its_transport` (name kept) |
| A2 | The Undo exempt row names (3,8), "CollectionService.Undo" and `TargetKind::Card` | `EXEMPT` holds no Undo row: 6 rows read where the test's data lists 7 | `table.rs` `each_exempt_write_names_its_engine_call_and_its_target_kind` (name kept) |
| A3 | `run` refuses (3,8) with `NeedsGesture`, and the answered card stays answered | `run` admits the pair: `Ok`, and the card reads new again | `crates/engine-core/tests/dispatch.rs` `an_undo_through_run_is_held_for_the_owner_and_leaves_the_answer` |
| A4 | A confirmed undo of the recorded answer removes its review row and returns the card to the state it left | not red: the red stub runs (3,8) unchecked, and that is already this criterion's answer. A5 to A13 and rows `S37103` to `S37110` hold the check | `crates/engine-core/tests/undo_answer.rs` `an_undo_reverts_the_offered_answer` |
| A5 | Answer A, then bury B: an undo with A's record is refused `NotTheTarget`, B stays buried and A stays answered | the stub reverts the front of the queue: `Ok`, and B is no longer buried | `undo_answer.rs` `an_undo_after_a_later_change_is_refused_and_changes_nothing` |
| A6 | An undo that targets card B with A's record is refused, and A stays answered | the stub runs the undo: `Ok`, and A's answer is reverted | `undo_answer.rs` `an_undo_aimed_at_another_card_is_refused` |
| A7 | `judge` refuses `Gone` when the record's review row is absent, and admits the same inputs with the row present | the stub's `judge` admits every input: `Ok` | `undo_answer.rs` `a_record_whose_review_is_gone_is_refused` |
| A8 | `judge` refuses `NotTheCard` when the review row names another card, and nothing else differs | the stub admits: `Ok` | `undo_answer.rs` `a_record_for_another_card_is_refused` |
| A9 | `judge` refuses `Synced` when the row's `usn` is 0, and nothing else differs | the stub ignores the `usn`: `Ok` | `undo_answer.rs` `a_synced_answer_is_refused` |
| A10 | `judge` refuses `Gone` when the engine's undo label is empty, and nothing else differs | the stub admits: `Ok` | `undo_answer.rs` `an_empty_undo_queue_is_refused` |
| A11 | `judge` refuses `Changed` when `last_step` differs and the label is the same | the stub ignores the step: `Ok` | `undo_answer.rs` `a_later_step_is_refused` |
| A12 | `judge` refuses `Changed` when the label differs and `last_step` is the same | the stub admits: `Ok` | `undo_answer.rs` `another_undo_label_is_refused` |
| A13 | An exempt Undo whose request is not a `Recorded` is refused `Undecodable`, and the answer stands | the stub decodes with `unwrap_or_default()` and runs the undo: `Ok` | `undo_answer.rs` `a_record_that_is_not_one_is_refused` |
| A14 | `returns_to` answers each kind the answer left: New, Learning, Review, Relearning, a filtered card's original kind, and Preview for a filtered preview (examined 6 of 6) | the stub answers `New` for every kind | `undo_answer.rs` `an_undone_answer_returns_its_card_to_the_state_it_left` |
| A15 | On the web, (27,14) run on known HTML answers that HTML's known text line, media file names kept; natively it is refused | `decide(Web, 27, 14)` reads `NotAllowed`: the call is refused | `dispatch.rs` `the_web_reads_a_card_as_one_line_of_text` |
| A16 | `run_method` admits only the study calls: (3,8) is refused `CallRefused`, and (27,14) is admitted | `STUDY_CALLS` still holds (3,8): it is admitted | `crates/web-engine/tests/study.rs` `run_method_admits_only_the_study_calls` (name kept) |
| A17 | `undo` checks `(card, step)` against `LAST_ANSWER`, runs `judge`, mints `Target::Card(card)`, calls `run_exempt` and clears `LAST_ANSWER` and `SHOWN`; `undo_offer` is owed (`OWED` 30 rows to 31); `rate` records only when the newest review's card is the rated card; no `call(service::COLLECTION, 8, …)` remains | `undo`'s body still reads `call(service::COLLECTION, 8, &[])` | `crates/web-engine/tests/boundary.rs` `each_boundary_function_reaches_the_engine_through_the_dispatcher` (name kept) |
| A18 | The native `run` refuses (3,8) with `EngineRefusal::NotAllowed { 3, 8 }`, and the card stays answered | `ALLOW_LIST` admits it: `Ok(ANSWER_OPERATION)` | `crates/ffi/tests/round_trip.rs` `a5_undoes_the_answer` (name kept; SPEC-336 A5) |
| A19 | A planted `col.undo();` in a non-UI source is refused by name, and the four held entries are the only `undo` lines counted outside the core | committed alone, before any code: `ENGINE_NAMES` lacks `undo`, so the plant passes the census | `crates/engine-core/tests/containment.rs` `a_planted_undo_outside_the_entry_files_is_refused_by_name` |
| A20 | `.undoOffer(` is called at 1 site (`review.ts`), `.undo_offer(` at 1 (`session.ts`) and `.undo(` at 2 (`review.ts`, `session.ts`); each population prints its examined count and refuses zero; each plant is refused by name | committed alone, before any code: `.undoOffer(` sites read 0 (the positive site is asserted before `examined()`) | `web/app/src/lib/study/undo-reach.test.ts` "an undo reaches the engine only from the review and the session" |
| A21 | Only `confirming` holds a cell whose effect is `undo` | the question's and the answer's cells hold it (`review.ts:42`, `:55-56`) | `undo-reach.test.ts` "only the confirmation holds a cell whose effect is undo" |
| A22 | Every locale holds the 15 undo keys (examined 7 of 7 locales) | no locale holds `undo_title` | `undo-reach.test.ts` "every locale holds the undo dialog's fifteen messages" |
| A23 | An undo press asks for the offer and writes nothing until the confirmation | the `UNDO` cell sends `client.undo()` on the press (`review.ts:42`, `:283-284`) | `web/app/src/lib/study/review.test.ts` "an undo press asks first and writes nothing until confirmed" |
| A24 | While the review asks, any other action keeps the answer and is not carried out | there is no `confirming` state: the press has already sent the undo, and the action runs | `review.test.ts` "while it asks, any other action keeps the answer and is not carried out" |
| A25 | A synced answer is not offered: the press announces `undo-synced` and sends nothing | there is no synced arm: the press sends the undo | `review.test.ts` "a synced answer is not offered and says so" |
| A26 | A confirmation the engine refuses reloads the next card and shows its notice | there is no `undo-refused` event: the review stays busy | `review.test.ts` "a refused confirmation reloads the card and says why" |
| A27 | The confirmation is an `alertdialog` that names the card as text, the answer and the state it returns to, with focus on "Keep it" | no dialog renders | `web/app/src/lib/study/review-screen.test.ts` "the confirmation names the card, the answer and the state it returns to" |
| A28 | A refused undo reaches the page as `undo-synced` for `Synced` and `not-undoable` for every other refusal | neither code exists | `web/app/src/lib/engine/session.test.ts` "a refused undo reads as its own error code" |

```acceptance
A1: cargo test -p deck-streak-engine-core --test table -- --exact every_pair_is_admitted_held_or_refused_by_its_transport
A2: cargo test -p deck-streak-engine-core --test table -- --exact each_exempt_write_names_its_engine_call_and_its_target_kind
A3: cargo test -p deck-streak-engine-core --test dispatch -- --exact an_undo_through_run_is_held_for_the_owner_and_leaves_the_answer
A4: cargo test -p deck-streak-engine-core --test undo_answer -- --exact an_undo_reverts_the_offered_answer
A5: cargo test -p deck-streak-engine-core --test undo_answer -- --exact an_undo_after_a_later_change_is_refused_and_changes_nothing
A6: cargo test -p deck-streak-engine-core --test undo_answer -- --exact an_undo_aimed_at_another_card_is_refused
A7: cargo test -p deck-streak-engine-core --test undo_answer -- --exact a_record_whose_review_is_gone_is_refused
A8: cargo test -p deck-streak-engine-core --test undo_answer -- --exact a_record_for_another_card_is_refused
A9: cargo test -p deck-streak-engine-core --test undo_answer -- --exact a_synced_answer_is_refused
A10: cargo test -p deck-streak-engine-core --test undo_answer -- --exact an_empty_undo_queue_is_refused
A11: cargo test -p deck-streak-engine-core --test undo_answer -- --exact a_later_step_is_refused
A12: cargo test -p deck-streak-engine-core --test undo_answer -- --exact another_undo_label_is_refused
A13: cargo test -p deck-streak-engine-core --test undo_answer -- --exact a_record_that_is_not_one_is_refused
A14: cargo test -p deck-streak-engine-core --test undo_answer -- --exact an_undone_answer_returns_its_card_to_the_state_it_left
A15: cargo test -p deck-streak-engine-core --test dispatch -- --exact the_web_reads_a_card_as_one_line_of_text
A16: cargo test -p deck-streak-web-engine --test study -- --exact run_method_admits_only_the_study_calls
A17: cargo test -p deck-streak-web-engine --test boundary -- --exact each_boundary_function_reaches_the_engine_through_the_dispatcher
A18: cargo test -p deck-streak-ffi --test round_trip -- --exact a5_undoes_the_answer
A19: cargo test -p deck-streak-engine-core --test containment -- --exact a_planted_undo_outside_the_entry_files_is_refused_by_name
A20: pnpm exec vitest run web/app/src/lib/study/undo-reach.test.ts -t "an undo reaches the engine only from the review and the session"
A21: pnpm exec vitest run web/app/src/lib/study/undo-reach.test.ts -t "only the confirmation holds a cell whose effect is undo"
A22: pnpm exec vitest run web/app/src/lib/study/undo-reach.test.ts -t "every locale holds the undo dialog's fifteen messages"
A23: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "an undo press asks first and writes nothing until confirmed"
A24: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "while it asks, any other action keeps the answer and is not carried out"
A25: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "a synced answer is not offered and says so"
A26: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "a refused confirmation reloads the card and says why"
A27: pnpm exec vitest run web/app/src/lib/study/review-screen.test.ts -t "the confirmation names the card, the answer and the state it returns to"
A28: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "a refused undo reads as its own error code"
```

Each of A7 to A12 holds one operand of `judge`: the test first reads `Ok` with every operand
matching, then flips that one operand alone and reads its refusal, so that operand alone decides
the result.

Shipped tests whose names this delivery keeps and whose bodies it changes, so that their SPECs'
fence lines still resolve:
- SPEC-345 A15: `containment.rs` `no_non_ui_caller_reaches_an_exempt_function` (`:645`). Its
  `HELD` gains the entries of R13.
- SPEC-338 A11: the engine suite's "opens, answers and undoes over OPFS" (`engine.spec.ts:55`) and
  `session.test.ts`'s "the session opens, answers and undoes through the engine" (`:298`) keep
  their titles and undo through the offer and the confirmation.
- SPEC-336 A5: `a5_undoes_the_answer` (`round_trip.rs:363`) becomes the refusal of A18.
- The core's `tests/gesture.rs` (`TAKEN` 6 to 7 at `:21`, taps 18 to 21 at `:64`),
  `tests/table.rs` (`NATIVE` 9 to 8 at `:23`, `WEB` keeps 15 at `:36` with (3,8) out and (27,14)
  in, `HELD` 6 to 7 at `:56`), `tests/review_pairs.rs` (`NATIVE` 9 to 8 at `:42`, dropping (3,8)
  at `:45`; `HELD` 6 to 7 at `:54`), and `tests/exempt.rs` (its per-write arms gain Undo).
- The native `tests/review_pairs.rs` (`EXPECTED`, 9 rows at `:64`) drops (3,8) at `:67`, and the
  web engine's `tests/study.rs` `the_study_calls_are_the_reviews_pairs` (`:166`) drops (3,8) at
  `:173` and gains (27,14).
- The engine suite's other `client.undo()` calls (`engine.spec.ts:71`, `:260`, `:311`) and
  `session.test.ts`'s other `op: 'undo'` requests (`:217`, `:230`, `:325`, `:372`, `:424`) carry
  `card` and `step` (R12).

Shipped requirements this delivery changes, by their own SPECs (their text is not edited; this
SPEC and ADR-382 record the change):

| shipped requirement | what changes |
|---|---|
| SPEC-350 R2 and R7 (undo at `:85`, `:92`, `:112-113`, `:142`, `:283-284`) | `undo` takes `(card, step)` and has an offer before it; `CardView.undo` names `'answer'`, `'synced'` or nothing |
| SPEC-345 M15 (`:39`, `:45`, `:206-208`) | Undo is an exempt write held to the gesture, no longer an ordinary call |
| SPEC-338 A11 | its tests undo through the offer and the confirmation, titles kept |
| SPEC-336 A5 | a refusal (A18) |
| SPEC-365 section 5 (`:300`, "It leaves Undo unchanged") | true of SPEC-365's delivery; this delivery changes Undo after it lands |
| SPEC-366 (`:210`, "It leaves Undo, bury, flag and replay, and their keys and buttons, unchanged") | true of SPEC-366's delivery; this delivery, built after it, relabels the bar's Undo button and makes its key and button ask first |
| SPEC-342 A14 to A16 | unchanged |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-371-undo-reverts-only-the-reviews-own-last-answer-through-the-owners-gesture-after-the-review-shows-what-it-changes.md` | docs | added |
| `docs/decisions/ADR-382-undo-reverts-only-the-reviews-own-last-answer-as-an-exempt-write-behind-the-owners-gesture-checked-at-the-write-after-the-review-shows-what-it-changes.md` | docs | added |
| `docs/decisions/ADR-356-the-engine-core-holds-the-engine-for-both-clients-behind-a-per-transport-table.md` | docs | an insert-only amendment section appended (D2, D4, D6) |
| `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md` | docs | an insert-only amendment section appended (D1, for undo only) |
| `docs/decisions/ADR-337-the-owners-own-taps-are-exempt-from-the-never-list-and-every-other-path-stays-bound.md` | docs | an insert-only amendment section appended (its Decision Outcome) |
| `docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md` | docs | a dated note at `:107`, insert-only, only if the base holds none (section 7) |
| `docs/schematics/undo-the-reviews-own-last-answer.md` | docs | added |
| `docs/schematics/web-study-screens.md` | docs | its undo lines follow R9 and R10 |
| `docs/red-first/SPEC-371.md` | docs | added |
| `changelog.d/undo-own-answer-371.md` | docs | added |
| `crates/engine-core/src/undo_answer.rs` | engine-core | added (R3) |
| `crates/engine-core/src/lib.rs` | engine-core | `pub mod undo_answer` and its line in the module list |
| `crates/engine-core/src/table.rs` | engine-core | Undo leaves `ORDINARY` and joins `EXEMPT`; (27,14) joins `ORDINARY`, web only; the module doc (`:1-8`) follows (R2) |
| `crates/engine-core/src/gesture.rs` | engine-core | `ExemptWrite::Undo`; `checked` answers `Run` or `Undo` (R5) |
| `crates/engine-core/src/dispatch.rs` | engine-core | `Read` gains `NewestReview` and `Review(i64)` (R4); `run_exempt`'s `Undo` arm and the private `run_undo` (R5) |
| `crates/engine-core/tests/undo_answer.rs` | engine-core | added (A4 to A14) |
| `crates/engine-core/tests/table.rs` | engine-core | A1 and A2; `NATIVE` 9 to 8, `WEB` 15 kept with (3,8) out and (27,14) in, `HELD` 6 to 7 |
| `crates/engine-core/tests/dispatch.rs` | engine-core | A3 and A15 |
| `crates/engine-core/tests/exempt.rs` | engine-core | the per-write arms gain Undo |
| `crates/engine-core/tests/gesture.rs` | engine-core | `TAKEN` 6 to 7, taps 18 to 21 |
| `crates/engine-core/tests/review_pairs.rs` | engine-core | `NATIVE` 9 to 8 (drops (3,8)); `HELD` 6 to 7 |
| `crates/engine-core/tests/containment.rs` | engine-core | `undo` in `ENGINE_NAMES`; R13's `HELD` entries; the plant and A19 (R13) |
| `crates/ffi/src/allow_list.rs` | ffi | Undo leaves `ALLOW_LIST`; the docs at `:5` and `:23` (R15) |
| `crates/ffi/tests/review_pairs.rs` | ffi | the native list drops (3,8) |
| `crates/ffi/tests/round_trip.rs` | ffi | A18 |
| `crates/web-engine/src/study.rs` | web-engine | `LastAnswer`, `StudyError::NotUndoable`; `STUDY_CALLS` drops (3,8) and gains (27,14) (R6, R7) |
| `crates/web-engine/src/wasm.rs` | web-engine | `LAST_ANSWER`; `undo_offer`; `undo(card, step)`; `rate` records; `open` and `close` clear; `current_card`'s `undo` (R6 to R8) |
| `crates/web-engine/tests/study.rs` | web-engine | A16; `the_study_calls_are_the_reviews_pairs` drops (3,8) and gains (27,14) |
| `crates/web-engine/tests/boundary.rs` | web-engine | `undo`'s owed statements; `undo_offer` owed (`OWED` 30 rows to 31) (A17) |
| `scripts/mutation-rows.d/S37100-S37199.json` | scripts | added (section 9) |
| `scripts/mutation-equivalent.d/deck-streak-web-engine.json` | scripts | records for `undo`, `undo_offer` and `rate`'s new lines |
| `formal/tla/UndoOwnAnswer/UndoOwnAnswer.tla` | formal | added (section 8) |
| `formal/tla/UndoOwnAnswer/MCUndoOwnAnswer.cfg` | formal | added |
| `formal/tla/UndoOwnAnswer/witness/an-undo-reverts-only-the-offered-answer.cfg` | formal | added (named `a-confirm-with-no-re-check.cfg` in the draft) |
| `formal/tla/UndoOwnAnswer/witness/an-undo-runs-only-on-a-confirm.cfg` | formal | added (named `an-undo-on-the-press.cfg` in the draft) |
| `web/app/src/lib/engine/protocol.ts` | miniapp | the offer operation, `undo`'s arguments, `CardView.undo`, the two error codes (R12) |
| `web/app/src/lib/engine/client.ts` | miniapp | `undoOffer()`, `undo(card, step)` |
| `web/app/src/lib/engine/session.ts` | miniapp | `EngineModule`'s `undo_offer` and `undo`; the error mapping (R12) |
| `web/app/src/lib/engine/session.test.ts` | miniapp | A28; SPEC-338 A11's test through the offer and the confirmation; its other undo requests carry `card` and `step` |
| `web/app/src/lib/engine/client.test.ts` | miniapp | its undo case takes `(card, step)`; the offer |
| `web/app/src/lib/engine/protocol.test.ts` | miniapp | the op list gains the offer |
| `web/app/src/lib/study/review.ts` | miniapp | the machine of R9 |
| `web/app/src/lib/study/review.test.ts` | miniapp | A23 to A26 |
| `web/app/src/lib/study/ReviewScreen.svelte` | miniapp | the dialog of R10 |
| `web/app/src/lib/study/review-screen.test.ts` | miniapp | A27 |
| `web/app/src/lib/study/undo-reach.test.ts` | miniapp | added (A20 to A22) |
| `web/app/messages/en.json` | miniapp | 15 keys (R11) |
| `web/app/messages/es.json` | miniapp | 15 keys (R11) |
| `web/app/messages/fr.json` | miniapp | 15 keys (R11) |
| `web/app/messages/ja.json` | miniapp | 15 keys (R11) |
| `web/app/messages/ko.json` | miniapp | 15 keys (R11) |
| `web/app/messages/zh-Hans.json` | miniapp | 15 keys (R11) |
| `web/app/messages/zh-Hant.json` | miniapp | 15 keys (R11) |
| `web/app/tests-engine/engine.spec.ts` | miniapp | SPEC-338 A11's undo through the offer and the confirmation, title kept; its other `client.undo()` calls take `(card, step)` |
| `web/app/src/lib/study/refusal.ts` | miniapp | `STATUS_KEYS` names the messages of `undo-synced` and `not-undoable` (R10, R12) |
| `web/app/src/lib/study/refusal.test.ts` | miniapp | its status table gains `undo-synced` and `not-undoable` |
| `web/app/src/lib/engine/credential.test.ts` | miniapp | its study requests ask for the offer, and `undo` carries `card` and `step` |
| `web/app/src/lib/engine/credential-stand-in.test.support.ts` | miniapp | its study module answers `undo_offer` |
| `web/app/src/routes/study.test.ts` | miniapp | its card's `undo` reads `null` |
| `web/app/tests-study/study.spec.ts` | miniapp | "undo returns the rated card" confirms in the dialog, title kept |
| `web/app/src/lib/study/audio.test.ts` | miniapp | its client answers `undoOffer`, and its card's `undo` reads `null` |
| `web/app/src/lib/study/voice.test.ts` | miniapp | its client answers `undoOffer`, and its card's `undo` reads `null` |

## 5. What this does NOT do

- It does not undo a bury or a flag. Undo reverts only the review's own last answer; after a bury
  or a flag it is not offered, and a flag toggles back by itself. Undoing either needs a record
  that names its kind (#714).
- It does not undo an answer that has synced. The control stays disabled and says "Your last
  answer has synced, so it can no longer be undone." The engine discards its undo queue at a
  normal sync, so that undo needs a review-history write of its own, as ADR-356 anticipated
  (#714).
- It offers no undo from the done screen: the review's `done` state holds no cell (`review.ts:59`)
  (#714).
- It adds no native undo. `ExemptTap` gains no variant, no Swift line changes, and HarnessWire's
  `Requests.undo()` stays a bytes builder whose (3,8) the native `run` now refuses (#714).
- It adds no web sync. The synced arm is reached on the web only once a web sync lands (#714).
- It changes no never-list entry and no line of the owner-taps ruling or of ADR-301; at most it
  adds ADR-301's dated note (#714).
- It changes none of the six other exempt writes, and the `run_exempt` export keeps its indices 0
  to 5 (#714).
- It leaves the bot's own habit undo (`commands.rs:657`) as it is; the census holds that line by
  name (#714).
- It changes no grade: SPEC-365's `Grade` is reused as it landed (#711), and the review's two grade
  buttons are SPEC-366's (#712).

## 6. Risks

- **The text index is inferred.** (27,14) is read from the proto's order, not from a build (R8).
  If it is not HtmlToTextLine, the offer shows another method's answer, or none. The build
  measures the index from a build before it writes anything and stops if it is not 14, and A15
  pins it on known HTML.
- **The census's new name meets lines that are not the page's undo.** `names()` does not exclude
  string literals, so `undo` counts the bot's habit undo, the bot router's literal and three
  ingest fixture lines. Each is held by name with its reason (R13), and the census's run prints
  any new match by name. A new `.undo(` outside the entry files is refused, which is the census
  working.
- **A mutant the native build cannot see.** `wasm.rs` compiles only for the browser, so its
  mutants are equivalent natively. The boundary census is their one native killer (rows
  `S37113` to `S37116`), and their equivalence records name the lines. The browser target's
  own verdict is section 10's.
- **The synced arm is not reachable on the web yet.** The web cannot normal-sync today. The arm is
  built now and held by A9 and A25, so a web sync that lands later meets a control that already
  refuses.
- **The Undo key now asks.** A learner used to one press undoing now meets a dialog that names what
  the undo changes. The same key confirms, so a quick second press writes, but it writes only the
  answer the dialog named; Escape and "Keep it", which holds the focus, keep the answer.
- **A renamed shipped test orphans a fence line.** SPEC-336 A5, SPEC-338 A11 and SPEC-345 A15 name
  tests this delivery edits, so each keeps its name (section 3).
- **Two pull requests edit the census.** `containment.rs` is a shared path with #631 (section 7).

## 7. The other pull requests

- SPEC-365's delivery (#711) adds the owner's answer token and `Grade`, and SPEC-366's (#712)
  makes the review's grades two buttons. Both are on `dev` at `2438c4c6`, this SPEC's base,
  merged by pull requests #725 and #719: `LAST_ANSWER` is set after `run_answer`, and the dialog
  names Again or Good.
- #631 shares `crates/engine-core/tests/containment.rs`. Whichever of the two lands second rebases
  onto the other and keeps both deliveries' names, held lines and plants.
- #631 may build an exempt tap before this delivery does. If ADR-301 already holds a dated note at
  `:107` on the build's base, this delivery adds none, and its manifest row for ADR-301 is dropped
  before the SPEC is committed. At `2438c4c6` it holds none.

## 8. Formal model

**Required.** The surface is an interleaving: the offer is a check and the confirmed write is an
act, and between them the owner, the review's other controls and a future web sync can each change
the engine's undo queue. The delivery also adds a `thread_local!` item, `LAST_ANSWER`.

The model is `formal/tla/UndoOwnAnswer/`, after the precedent of `formal/tla/FullSyncChoice`
(covers, cites, `ramp=report`, `witness/*.cfg`).
- **Covers:** `crates/engine-core/src/undo_answer.rs` anchor `judge`;
  `crates/engine-core/src/dispatch.rs` anchor `run_undo`; `crates/web-engine/src/wasm.rs` anchors
  `undo`, `undo_offer` and `rate`.
- **Cites:** #714.
- **Variables:** `ops` (a sequence of a kind, a card and a synced flag), `counter`, `record`,
  `offers`, `confirmed`, `reverted`.
- **Actions:** `Answer(c)`, `OtherOp(c)`, `Sync`, `Offer`, `Confirm(o)`, `StaleConfirm(o)`.
- **Properties,** each `ramp=report`: `AnUndoRevertsOnlyTheOfferedAnswer` (whatever is reverted is
  the answer a confirmed offer named, and nothing reverted is a bury, a flag or a synced answer)
  and `AnUndoRunsOnlyOnAConfirm` (nothing is reverted that no confirmation of an offer preceded).
- **Witnesses,** each named for the property it holds, never for the defect, which stays its
  body (old -> new: `witness/a-confirm-with-no-re-check.cfg` ->
  `witness/an-undo-reverts-only-the-offered-answer.cfg`; `witness/an-undo-on-the-press.cfg` ->
  `witness/an-undo-runs-only-on-a-confirm.cfg`):
  `witness/an-undo-reverts-only-the-offered-answer.cfg` ports `dev`'s `wasm.rs:256`, an undo with
  no check, and kills `AnUndoRevertsOnlyTheOfferedAnswer`;
  `witness/an-undo-runs-only-on-a-confirm.cfg` ports `dev`'s `review.ts:42`, `:55-56` and
  `:283-284`, the write on the press, and kills `AnUndoRunsOnlyOnAConfirm`. Each witness must
  read VIOLATION before the model reads clean: the control runs forward.
- **Stamping:** every `digest=` is committed as 64 zeros, the entry is checked, and each `STALE`
  line's current digest is copied in, after the last edit to the entry and to the covered code.

**Lean is not applicable.** `judge` is five comparisons in a fixed order, which A7 to A12 and rows
`S37103` to `S37108` hold one operand at a time, as ADR-374 D12 decided for four comparisons
and SPEC-365 section 8 for two.

A builder may raise the Lean decision, and says why.

## 9. Mutation rows

Each row names a behaviour of R2 to R7 or R13, its mutant and the one test that kills it. Rows live
in `scripts/mutation-rows.d/S37100-S37199.json`, table `MUTATIONS`, and each killer is in the
row's own crate.

| row | file | mutant | killer |
|---|---|---|---|
| `S37101-THE-UNDO-ROW-IS-UNDO` | `engine-core` `src/table.rs` | the Undo exempt row's `method: 8` becomes `method: 9` (a literal in a `const`, which the tool never mutates: a hand row) | `table::each_exempt_write_names_its_engine_call_and_its_target_kind` |
| `S37102-AN-UNDO-TARGETS-A-CARD` | `engine-core` `src/table.rs` | the Undo row's `kind: TargetKind::Card` becomes `kind: TargetKind::Note` | `gesture::a_gesture_takes_one_target_of_its_writes_kind` |
| `S37103-A-GONE-REVIEW-IS-REFUSED` | `engine-core` `src/undo_answer.rs` | `judge`'s no-review arm admits | `undo_answer::a_record_whose_review_is_gone_is_refused` |
| `S37104-AN-UNDO-NAMES-ITS-CARD` | `engine-core` `src/undo_answer.rs` | `review.cid != card` becomes `review.cid != review.cid` | `undo_answer::a_record_for_another_card_is_refused` |
| `S37105-A-SYNCED-ANSWER-IS-REFUSED` | `engine-core` `src/undo_answer.rs` | `review.usn != -1` becomes `review.usn != review.usn` | `undo_answer::a_synced_answer_is_refused` |
| `S37106-AN-EMPTY-QUEUE-IS-REFUSED` | `engine-core` `src/undo_answer.rs` | `now.undo.is_empty()` becomes `false` | `undo_answer::an_empty_undo_queue_is_refused` |
| `S37107-A-LATER-STEP-IS-REFUSED` | `engine-core` `src/undo_answer.rs` | the `last_step` comparison compares the current step with itself | `undo_answer::a_later_step_is_refused` |
| `S37108-ANOTHER-LABEL-IS-REFUSED` | `engine-core` `src/undo_answer.rs` | the label comparison compares the current label with itself | `undo_answer::another_undo_label_is_refused` |
| `S37109-THE-WRITE-WAITS-FOR-THE-CHECK` | `engine-core` `src/dispatch.rs` | `run_undo` discards `judge`'s refusal (`let _ = …`) and runs (3,8) | `undo_answer::an_undo_after_a_later_change_is_refused_and_changes_nothing` |
| `S37110-A-RECORD-THAT-IS-NOT-ONE-IS-REFUSED` | `engine-core` `src/gesture.rs` | the Undo arm's decode `.map_err(undecodable)?` becomes `.unwrap_or_default()` | `undo_answer::a_record_that_is_not_one_is_refused` |
| `S37111-A-REVIEW-RETURNS-TO-REVIEW` | `engine-core` `src/undo_answer.rs` | `returns_to`'s Review arm answers `Learning` | `undo_answer::an_undone_answer_returns_its_card_to_the_state_it_left` |
| `S37112-THE-TEXT-ROW-IS-HTML-TO-TEXT-LINE` | `engine-core` `src/table.rs` | the (27,14) row's `method: 14` becomes `method: 13` (a hand row) | `table::every_pair_is_admitted_held_or_refused_by_its_transport` |
| `S37113-AN-UNDO-MATCHES-ITS-OFFER` | `web-engine` `src/wasm.rs` | `undo`'s `(card, step)` comparison with the record is dropped | `boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher` |
| `S37114-AN-UNDO-TARGETS-ITS-CARD` | `web-engine` `src/wasm.rs` | `undo` mints a target other than `Target::Card(card)` | `boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher` |
| `S37115-RATE-RECORDS-ONLY-ITS-OWN-REVIEW` | `web-engine` `src/wasm.rs` | `rate` records without the newest review's card check | `boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher` |
| `S37116-AN-UNDO-CLEARS-ITS-RECORD` | `web-engine` `src/wasm.rs` | `undo` leaves `LAST_ANSWER` set | `boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher` |
| `S37117-THE-CENSUS-NAMES-UNDO` | `engine-core` `tests/containment.rs` | `ENGINE_NAMES`' `"undo"` becomes `"undone"` (a hand row) | `containment::a_planted_undo_outside_the_entry_files_is_refused_by_name` |

Rows 01, 12 and 17 change a literal inside a `const`, which the tool never mutates, so each is a
hand row planted by the build. Rows 13 to 16 sit in code only the browser target compiles: the
boundary census is their native killer, and their equivalence records go in
`scripts/mutation-equivalent.d/deck-streak-web-engine.json`. Rows 07 and 08 are the two operands
of one `||`, so each has a test in which it alone decides (A11, A12). The page's code is mutated by
StrykerJS's `mutation-web` job, which mutates each changed `web/app` production file whole; a
mutant it shows equivalent is recorded in `scripts/mutation-equivalent.d/miniapp.json`, which then
joins the manifest by amendment.

The killers of rows 03 to 11 and 17 are tests this delivery adds, so none exists at the
merge-base. That does not change how the pull request is judged: CI checks out the merge ref and
diffs `HEAD^1...HEAD` (`.github/workflows/ci.yml:390`), the plan selects every row whose id the
base's rows lack (`scripts/mutation-verdict.py:659`, in `selected_rows`, `:636-666`), and
`mutation_rows.py prove --rows-from plan.json` proves each selected row in the merge ref's tree
(`ci.yml:628`; `scripts/mutation_rows.py:32-40`), where its killer exists. No killer is run at the
base.

## 10. What only CI or a device proves

| id | criterion | who, and when |
|---|---|---|
| C1 | The browser build of the web engine exports `undo_offer` and `undo(card, step)`, and the engine suite's "opens, answers and undoes over OPFS" undoes an answer through the offer and the confirmation | the browser engine suite in CI, on the pull request; `wasm.rs` is built only there |
| C2 | The browser target's mutation verdict for `wasm.rs`'s `undo`, `undo_offer` and `rate` | CI's mutation job for the web engine, read by row stem and record |
| C3 | StrykerJS's verdict over `review.ts`, `ReviewScreen.svelte`, `session.ts`, `client.ts` and `protocol.ts` | CI's `mutation-web` job, on the pull request |
| C4 | The native harness and its `RequestBytesTests` are unchanged and green | the native workflow's tests, on the pull request; the native code is built only there |
| V1 | On a device: answer a card, press Undo, read the card, the answer and the state it returns to, confirm, and see the card again; then bury a card and see Undo offer nothing | the owner, in the acceptance session (#714) |
