# Red-first record: SPEC-365

The SPEC, ADR-376 and the schematic were committed first (61095013), then the native census alone
(89b6f83d), then the core's tests against two planted stubs (c4f66f70) and the adapters' tests
against theirs (6963576d), all before the code that turns them green: the core and the native
adapter (ae4e458a), the web (37b69926), the native harness (46738d68) and the native app
(c107bd84). Each red below is quoted from the run of that criterion's fence line at its red commit,
and each green is measured at the commit named. Every fence test's file is unchanged from its
green commit to the head but one: `crates/engine-core/tests/answer.rs` gained one test after the
code, at a23c05ca, insert-only, and A3 to A6 are as they were at their green.

## The fence, line by line

Each of the 16 lines of SPEC-365 section 3's fence resolves to one test, named by its exact name or
its `-t` or `-k` filter.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/engine-core/tests/table.rs` `every_pair_is_admitted_held_or_refused_by_its_transport` | named: body changed (an `ANSWERED` set and a fourth arm) |
| 2 | A2 | `crates/engine-core/tests/dispatch.rs` `an_answer_through_run_is_held_for_the_owner_and_leaves_the_card` | added |
| 3 | A3 | `crates/engine-core/tests/answer.rs` `a_press_answers_its_one_card_with_its_grade` | added |
| 4 | A4 | `crates/engine-core/tests/answer.rs` `a_press_for_one_card_cannot_answer_another` | added |
| 5 | A5 | `crates/engine-core/tests/answer.rs` `a_press_records_only_its_own_grade` | added |
| 6 | A6 | `crates/engine-core/tests/answer.rs` `a_request_that_is_not_a_card_answer_is_refused` | added |
| 7 | A7 | `crates/engine-core/tests/containment.rs` `the_owner_answer_is_neither_clone_nor_copy` | added |
| 8 | A8 | `crates/engine-core/tests/containment.rs` `no_non_ui_caller_reaches_an_exempt_function` | named: the census's lists grown, insert-only |
| 9 | A9 | `crates/web-engine/tests/study.rs` `a_rating_on_the_wire_picks_its_answer_and_its_next_state` | named: body changed |
| 10 | A10 | `crates/web-engine/tests/study.rs` `run_method_admits_only_the_study_calls` | named: body changed |
| 11 | A11 | `crates/web-engine/tests/boundary.rs` `each_boundary_function_reaches_the_engine_through_the_dispatcher` | named: `rate`'s owed statements and the retired texts |
| 12 | A12 | `crates/ffi/tests/round_trip.rs` `a_native_run_refuses_an_answer_and_leaves_the_card` | added |
| 13 | A13 | `crates/ffi/tests/round_trip.rs` `a_native_press_answers_only_its_card_with_its_grade` | added |
| 14 | A14 | `web/app/src/lib/engine/session.test.ts` "the session refuses the queue-head answer as an unknown operation" | added |
| 15 | A15 | `scripts/tests/test_ios_answer_door.py` `test_no_swift_source_answers_through_run` | added |
| 16 | A16 | `scripts/tests/test_ios_answer_door.py` `test_no_swift_source_names_a_third_grade` | added |

## The census first

`test_ios_answer_door.py` was committed alone at 89b6f83d, with every Swift source unchanged. The
native harness and the app still wrote the pair `(13, 4)`, and the codec's and the app's rating
types still declared Hard and Easy, so A15 and A16 were each red for their own reason, by
assertion, with the behaviour's assertion ahead of the `examined()` count.

## The reds and greens

Each line's command is the criterion's line in SPEC-365 section 3's fence, run at the commit named.

```red-first
A1: red at c4f66f70: table.rs:90 `the pairs Native admits` left {.. (13, 3), (13, 4), (13, 24) ..} right {.. (13, 3), (13, 24) ..}
A1: green at ae4e458a
A2: red at c4f66f70: dispatch.rs:175 left: (Ok(()), Some(1)) right: (Err(NeedsAnswer { service: 13, method: 4 }), Some(0))
A2: green at ae4e458a
A3: not red: at c4f66f70 the stub's check runs the request as given, which is already this criterion's answer (Again and Good each raise reps from 0 to 1 and leave the card in the engine's own next state for that grade); A4 to A6 and rows S36503 to S36509 hold the check
A4: red at c4f66f70: answer.rs:249 left: (Ok(()), Some(1), Some(0)) right: (Err(NotTheCard { pressed: .., named: .. }), Some(0), Some(0))
A4: green at ae4e458a
A5: red at c4f66f70: answer.rs:290 left: [(Again, 1, Ok(()), Some(1)), ... (Good, 3, Ok(()), Some(1))] (6 Ok)
A5: green at ae4e458a
A6: red at c4f66f70: answer.rs:315 left: (Err(Engine { error: [..] }), Some(0)) right: (Err(Undecodable), Some(0))
A6: green at ae4e458a
A7: red at c4f66f70: containment.rs:862 left: [("OwnerAnswer", (true, true, false)), ...]
A7: green at ae4e458a
A8: red at c4f66f70: containment.rs:635 left: [] right: ["crates/ffi/src/engine.rs", "crates/web-engine/src/wasm.rs"]
A8: green at 37b69926
A9: red at 6963576d: study.rs:32 wire rating 2: left: Ok(Again) right: Err(NotAGrade(2))
A9: green at 37b69926
A10: red at 6963576d: study.rs:72 AnswerCard: left: Ok("answer_card") right: Err(CallRefused { service: 13, method: 4 })
A10: green at ae4e458a
A11: red at 6963576d: boundary.rs:392 left: (["rate records only the kept card's press, ... and its body lacks `grade(rating)`", ...], [...]) right: ([], [])
A11: green at 37b69926
A12: red at 6963576d: round_trip.rs left: (Ok([]), Ok([8, 1, 24, 1, ...]), [[1, 1, 1]], [[.., 3, 4000, ..]]) right: (Ok([]), Err(NotAllowed { service: 13, method: 4 }), [[0, 0, 0]], [])
A12: green at ae4e458a
A13: red at 6963576d: round_trip.rs rows left [[1, 1, 1], [1, 2, 1]] right [[1, 2, 1], [1, 1, 1]]; undecodable left Err(Engine { "card was modified: ..." }) right Err(Undecodable)
A13: green at ae4e458a
A14: red at 6963576d: session.test.ts:357 AssertionError: expected { id: 3, ok: true, value: 1001n } to deeply equal { id: 3, ok: false, …(2) }
A14: green at 37b69926
A15: red at 89b6f83d: AssertionError: Lists differ: ['ios/App/Sources/EngineSession.swift:13: [91 chars] 4)'] != []
A15: green at c107bd84
A16: red at 89b6f83d: AssertionError: Lists differ: ['ios/App/Sources/AnswerBar.swift:23: name[1069 chars]asy'] != []
A16: green at c107bd84
```

## What the record discloses

- **A10 turned green with the core, not with the web.** `crates/engine-core/tests/parity.rs`
  includes `crates/web-engine/src/study.rs` by path and compares `STUDY_CALLS` with the ordinary
  table's web column, so AnswerCard left `STUDY_CALLS` in the same commit that took it out of the
  ordinary table (ae4e458a), or the parity test would have read red there.
- **A3 is not red, by construction.** The stub at c4f66f70 ran the caller's request unchecked, and
  an unchecked run of a press's own request is this criterion's answer. A4 to A6 are the reds the
  check owes, and rows `S36503` to `S36509` pin it.
- **A4's red is the criterion's.** Its first run before the commit read the engine's own refusal of
  a card not at the head of the queue, because it pressed the head and asked for the second card;
  it was swapped to press the second card and ask for the head before c4f66f70, so the committed
  red is the other card answered.
- **Test files changed between a red and its green, in other tests or shared data only.**
  `containment.rs` gained, insert-only, the two literal lines of `boundary.rs` that name the answer
  in its held list at 6963576d, before A8's green. At ae4e458a, `dispatch.rs`'s SPEC-345 A3 test
  (`an_ordinary_call_reaches_the_engine_and_an_exempt_one_does_not`) and `round_trip.rs`'s
  `good_answer`, `a4_answers_the_card` and `a5_undoes_the_answer` moved their answers onto the
  answer door, names kept. At 37b69926, `session.test.ts`'s "the session opens, answers and undoes
  through the engine" shows the card and then rates it. A2's, A12's, A13's and A14's own test
  bodies are unchanged between their red and their green.
- **A11's companion test was red beside it.** `boundary.rs`'s
  `a_boundary_function_that_answers_a_constant_is_refused_by_name` panicked at boundary.rs:427 at
  6963576d (`` `fn pressed(` occurs 0 times, not once ``) and is green at 37b69926; it is not a fence
  line, and its red is its own.
- **A control is not a red-first line.** `boundary.rs`'s `a_retired_text_planted_back_is_refused_by_name`
  was green at 6963576d, as a planted control is: it proves the retired-text census can refuse.
- **The Swift is read on the pull request.** A15 and A16 read the Swift sources as text on this
  box; the native harness, the app and their unit and UI tests build only in the native workflow,
  where SPEC-365 section 10's C1 to C3 read them.
- **Four rows of SPEC-338 moved with the code they guarded.** `S33800` to `S33802` anchored on the
  four-answer type this delivery removes; each keeps its id and killer and now anchors on the
  two-grade rule that holds its property. `S33803` keeps its anchor, and its mutant picks Again's
  state, because no Easy state remains for it to pick.
- **Two rows are killed by A5's test, not A3's.** `S36505` and `S36506` change `Grade::rating`, and
  A3's request takes its rating from that function, so the request moved with each mutant and A3
  passed under both. A5 spells every other rating literally: under `Self::Good => 1,` a Good press
  recorded rating 1, and under `Self::Again => 2,` an Again press recorded rating 2, each `Ok(())`
  where A5 expects `NotTheGrade`. SPEC-365 section 9 names A5's test for both rows (be6e3b8a).

## What the mutation pass added

The diff's own mutants were listed and run before the push, with `cargo mutants --in-diff` over
`git diff origin/dev...HEAD`: 48 listed and 48 tested, of which 38 were caught, 2 missed, 0 timed
out and 8 unviable. The 8 unviable are `Default::default()` replacements that do not compile; they
are recorded as unviable, never as caught.

The five mutants listed in `crates/web-engine/src/wasm.rs` (`call` three times, `rate` and
`pressed`) were caught on the native build. That module is built only for `wasm32`, and A11's
boundary census reads each function's body as text, so a replaced body fails the census by name.

The two missed mutants each replace a refusal's `Display` with empty text: `AnswerRefusal`'s in
`crates/engine-core/src/answer.rs` and `PressRefusal`'s in `crates/ffi/src/engine.rs`. Two tests
added after the code kill them, MUTATION COVERAGE and not red first, at a23c05ca:
`answer::each_answer_refusal_reads_as_its_own_sentence` and
`refusal_text::each_press_refusal_reads_as_its_own_sentence`, each holding every variant's whole
sentence. Run again by name at that commit, both mutants were caught.

A11's census changed, so every record in `scripts/mutation-equivalent.d/deck-streak-web-engine.json`
was tested again with a whole-file run of `wasm.rs`: 86 listed and 86 tested, 62 caught and 24
missed, and the 24 missed are exactly the file's 24 records, each read by name. No record was
refuted, and none was added.

StrykerJS mutated the three changed production files of the page, each whole: `client.ts` 44
mutants, `protocol.ts` 142 and `session.ts` 182, all 368 killed, none surviving, timed out or
uncovered.

The rows `S36501` to `S36516` each read KILLED, and the 75 rows already on the files this delivery
touches were proved again: 91 of 91 killed, `S36505` and `S36506` once A5's test was their
killer. The census examined 2173 rows and reported nothing.
