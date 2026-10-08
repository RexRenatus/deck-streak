# Red-first record: SPEC-371

The SPEC (cd6243f3), the schematic (8a4bba18) and ADR-382 with its amendments (6c8a8d03) were
committed first. Then the censuses alone (68bc0e0e), then the tests that compile at the base
(f2076d70), then the core's rule over a compiling stub with its tests (5f0afd53), all before the
code that turns them green: the core and the native adapter (46e416bd). Each red below is quoted
from the run of that criterion's fence line at its red commit, and each green is measured at the
commit named.

## The fence, line by line

Each of the 28 lines of SPEC-371 section 3's fence resolves to one test, named by its exact name or
its `-t` filter.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/engine-core/tests/table.rs` `every_pair_is_admitted_held_or_refused_by_its_transport` | named: body changed |
| 2 | A2 | `crates/engine-core/tests/table.rs` `each_exempt_write_names_its_engine_call_and_its_target_kind` | named: body changed (its data lists seven exempt rows) |
| 3 | A3 | `crates/engine-core/tests/dispatch.rs` `an_undo_through_run_is_held_for_the_owner_and_leaves_the_answer` | added |
| 4 | A4 | `crates/engine-core/tests/undo_answer.rs` `an_undo_reverts_the_offered_answer` | added |
| 5 | A5 | `crates/engine-core/tests/undo_answer.rs` `an_undo_after_a_later_change_is_refused_and_changes_nothing` | added |
| 6 | A6 | `crates/engine-core/tests/undo_answer.rs` `an_undo_aimed_at_another_card_is_refused` | added |
| 7 | A7 | `crates/engine-core/tests/undo_answer.rs` `a_record_whose_review_is_gone_is_refused` | added |
| 8 | A8 | `crates/engine-core/tests/undo_answer.rs` `a_record_for_another_card_is_refused` | added |
| 9 | A9 | `crates/engine-core/tests/undo_answer.rs` `a_synced_answer_is_refused` | added |
| 10 | A10 | `crates/engine-core/tests/undo_answer.rs` `an_empty_undo_queue_is_refused` | added |
| 11 | A11 | `crates/engine-core/tests/undo_answer.rs` `a_later_step_is_refused` | added |
| 12 | A12 | `crates/engine-core/tests/undo_answer.rs` `another_undo_label_is_refused` | added |
| 13 | A13 | `crates/engine-core/tests/undo_answer.rs` `a_record_that_is_not_one_is_refused` | added |
| 14 | A14 | `crates/engine-core/tests/undo_answer.rs` `an_undone_answer_returns_its_card_to_the_state_it_left` | added |
| 15 | A15 | `crates/engine-core/tests/dispatch.rs` `the_web_reads_a_card_as_one_line_of_text` | added |
| 16 | A16 | `crates/web-engine/tests/study.rs` `run_method_admits_only_the_study_calls` | named: body changed |
| 17 | A17 | `crates/web-engine/tests/boundary.rs` `each_boundary_function_reaches_the_engine_through_the_dispatcher` | named: `undo`'s and `rate`'s owed statements, `undo_offer` owed, one retired text |
| 18 | A18 | `crates/ffi/tests/round_trip.rs` `a5_undoes_the_answer` | named: body changed (SPEC-336 A5) |
| 19 | A19 | `crates/engine-core/tests/containment.rs` `a_planted_undo_outside_the_entry_files_is_refused_by_name` | added |
| 20 | A20 | `web/app/src/lib/study/undo-reach.test.ts` "an undo reaches the engine only from the review and the session" | added |
| 21 | A21 | `web/app/src/lib/study/undo-reach.test.ts` "only the confirmation holds a cell whose effect is undo" | added |
| 22 | A22 | `web/app/src/lib/study/undo-reach.test.ts` "every locale holds the undo dialog's fifteen messages" | added |
| 23 | A23 | `web/app/src/lib/study/review.test.ts` "an undo press asks first and writes nothing until confirmed" | added |
| 24 | A24 | `web/app/src/lib/study/review.test.ts` "while it asks, any other action keeps the answer and is not carried out" | added |
| 25 | A25 | `web/app/src/lib/study/review.test.ts` "a synced answer is not offered and says so" | added |
| 26 | A26 | `web/app/src/lib/study/review.test.ts` "a refused confirmation reloads the card and says why" | added |
| 27 | A27 | `web/app/src/lib/study/review-screen.test.ts` "the confirmation names the card, the answer and the state it returns to" | added |
| 28 | A28 | `web/app/src/lib/engine/session.test.ts` "a refused undo reads as its own error code" | added |

## The censuses first

`containment.rs`'s A19 test and its plant, `boundary.rs`'s owed statements for `undo` and
`undo_offer`, and `undo-reach.test.ts` were committed alone at 68bc0e0e, with no other code. Each
was red for its own reason, by assertion: the plant passed the census, because `ENGINE_NAMES` did
not yet name `undo`; `undo`'s body still read `call(service::COLLECTION, 8, &[])`; and `.undoOffer(`
read 0 sites, its positive site asserted ahead of the `examined()` count.

## The reds and greens

Each line's command is the criterion's line in SPEC-371 section 3's fence, run at the commit named.

```red-first
A1: red at f2076d70: the pairs Native admits: left holds (3, 8), right does not
A1: green at 46e416bd
A2: red at f2076d70: left 6 exempt rows, right 7 (+("Undo", 3, 8, "CollectionService.Undo", Card))
A2: green at 5f0afd53
A3: red at f2076d70: left (Ok(()), Some(0), Err(Engine{CollectionNotOpen})) right (Err(NeedsGesture{3,8}), Some(1), Err(NeedsGesture{3,8}))
A3: green at 5f0afd53
A4: not red: the stub runs the undo unchecked, which is already this criterion's answer
A5: red at 5f0afd53: left (Ok(()), Some(0), Some(1)) right (Err(NotTheTarget{write: Undo, target: Card(A)}), Some(-3), Some(1))
A5: green at 46e416bd
A6: red at 5f0afd53: left (Ok(()), Some(0)) right (Err(NotTheTarget{write: Undo, target: Card(B)}), Some(1))
A6: green at 46e416bd
A7: red at 5f0afd53: left (Ok(()), Ok(())) right (Err(Gone), Ok(()))
A7: green at 46e416bd
A8: red at 5f0afd53: left (Ok(()), Ok(())) right (Err(NotTheCard), Ok(()))
A8: green at 46e416bd
A9: red at 5f0afd53: left (Ok(()), Ok(())) right (Err(Synced), Ok(()))
A9: green at 46e416bd
A10: red at 5f0afd53: left (Ok(()), Ok(())) right (Err(Gone), Ok(()))
A10: green at 46e416bd
A11: red at 5f0afd53: left (Ok(()), Ok(())) right (Err(Changed), Ok(()))
A11: green at 46e416bd
A12: red at 5f0afd53: left (Ok(()), Ok(())) right (Err(Changed), Ok(()))
A12: green at 46e416bd
A13: red at 5f0afd53: left (Ok(()), Some(0)) right (Err(Undecodable{write: Undo}), Some(1))
A13: green at 46e416bd
A14: red at 5f0afd53: learning: left New right Learning
A14: green at 46e416bd
A15: red at f2076d70: left Err(NotAllowed{service: 27, method: 14}) right Ok("Hello world  cat.jpg  & a.mp3")
A15: green at 46e416bd
A16: red at f2076d70: left (Ok("undo"), Err(CallRefused{27,14})) right (Err(CallRefused{3,8}), Ok("html_to_text_line"))
A17: red at 68bc0e0e: undo's body lacks its owed statements, src/wasm.rs holds `call(service::COLLECTION, 8,`, undo_offer occurs 0 times
A18: red at f2076d70: left (Ok([]), Ok(()), Ok(()), Ok(1)) right (Ok([]), Ok(()), Err(NotAllowed{3,8}), Ok(0))
A18: green at 46e416bd
A19: red at 68bc0e0e: left [] right ["crates/coordination/src/lib.rs: `col.undo();` found 1, held 0"]
A19: green at 46e416bd
A20: red at 68bc0e0e: expected {} to deeply equal { 'src/lib/study/review.ts': 1 }
A21: red at 68bc0e0e: expected [ 'answer', 'question' ] to deeply equal [ 'confirming' ]
A22: red at 68bc0e0e: messages/en.json lacks an undo message: expected [ 'study_undo_answer', …(14) ] to deeply equal []
A23: red at f2076d70: expected ['question', ['card', 'undo', 'card']] to deeply equal ['confirming', ['card', 'undo-offer']]
A24: red at f2076d70: again: expected 'question' to be 'confirming'
A25: red at f2076d70: expected ['question', null, ['card', 'undo', 'card']] to deeply equal ['question', 'undo-synced', ['card']]
A26: red at f2076d70: expected ['question', 3n, null, ['card', 'undo', 'card']] to deeply equal ['question', 3n, 'not-undoable', ['card', 'undo-offer', 'undo 1 7', 'card']]
A27: red at f2076d70: TestingLibraryElementError: Unable to find an accessible element with the role "alertdialog"
A28: red at f2076d70: code bad-request, message "undo takes no card", where undo-synced was expected
```

## What the record discloses

- **A4 is not red, by construction.** The stub at 5f0afd53 ran the undo of (3,8) unchecked, and an
  unchecked undo of the recorded answer is this criterion's answer. A5 to A13 are the reds the
  check owes, and rows `S37103` to `S37110` pin it.
- **A15's test changed after its red, admitted as changed and not a weakening.** At 46e416bd the
  web dispatcher opens a synthetic collection before (27,14), because the engine renders through
  the open collection; its red at f2076d70, `NotAllowed` decided before the engine sees the call,
  does not depend on that collection.
- **A28's red is the base's request shape.** At f2076d70 neither `undo-synced` nor `not-undoable`
  exists, and the base's `undo` takes no card, so the session refused the request as
  `bad-request`. Its green shows both codes by name, each from its own request.
- **Test files changed at 46e416bd, between reds and their greens, in census data and fixtures
  only.** `containment.rs` gained `undo` in `ENGINE_NAMES` and four `HELD` entries with their
  reasons (R13), which is the census this delivery grows, A19's own test body unchanged;
  `dispatch.rs`'s A15 test opens the synthetic collection described above; and the native
  `review_pairs.rs` drops (3,8) from `EXPECTED` (R15). The other criteria's own test bodies are
  unchanged between their red and their green.
- **Two tests that are not fence lines went red beside the reds.** `study.rs`'s
  `the_study_calls_are_the_reviews_pairs` went red at f2076d70, when its oracle dropped (3,8) and
  gained (27,14) with A16; and `boundary.rs`'s
  `a_boundary_function_that_answers_a_constant_is_refused_by_name` went red at 68bc0e0e, because
  `undo_offer` had no body to plant into. Each is green with the web engine.
- **The parity test went red with the core.** `crates/engine-core/tests/parity.rs`
  `each_adapter_table_equals_its_transport_column` compares the web `STUDY_CALLS` with the ordinary
  table's web column, so from 46e416bd, where (27,14) joined that column and (3,8) left it, it read
  red until `STUDY_CALLS` followed in the web engine.
- **`exempt.rs`'s Undo arm names its target "its target".** Each per-write synthetic collection
  numbers its own cards, and the fixture holds no answer, so the Undo request's record names no
  row and is refused before the engine.
