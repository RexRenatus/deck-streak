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
A16: green at b6030dd5
A17: red at 68bc0e0e: undo's body lacks its owed statements, src/wasm.rs holds `call(service::COLLECTION, 8,`, undo_offer occurs 0 times
A17: green at b6030dd5
A18: red at f2076d70: left (Ok([]), Ok(()), Ok(()), Ok(1)) right (Ok([]), Ok(()), Err(NotAllowed{3,8}), Ok(0))
A18: green at 46e416bd
A19: red at 68bc0e0e: left [] right ["crates/coordination/src/lib.rs: `col.undo();` found 1, held 0"]
A19: green at 46e416bd
A20: red at 68bc0e0e: expected {} to deeply equal { 'src/lib/study/review.ts': 1 }
A20: green at 2c68b183
A21: red at 68bc0e0e: expected [ 'answer', 'question' ] to deeply equal [ 'confirming' ]
A21: green at 2c68b183
A22: red at 68bc0e0e: messages/en.json lacks an undo message: expected [ 'study_undo_answer', …(14) ] to deeply equal []
A22: green at 2c68b183
A23: red at f2076d70: expected ['question', ['card', 'undo', 'card']] to deeply equal ['confirming', ['card', 'undo-offer']]
A23: green at 2c68b183
A24: red at f2076d70: again: expected 'question' to be 'confirming'
A24: green at 2c68b183
A25: red at f2076d70: expected ['question', null, ['card', 'undo', 'card']] to deeply equal ['question', 'undo-synced', ['card']]
A25: green at 2c68b183
A26: red at f2076d70: expected ['question', 3n, null, ['card', 'undo', 'card']] to deeply equal ['question', 3n, 'not-undoable', ['card', 'undo-offer', 'undo 1 7', 'card']]
A26: green at 2c68b183
A27: red at f2076d70: TestingLibraryElementError: Unable to find an accessible element with the role "alertdialog"
A27: green at 2c68b183
A28: red at f2076d70: code bad-request, message "undo takes no card", where undo-synced was expected
A28: green at 2c68b183
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
- **A green line names the commit before the one that writes it.** No commit can hold its own
  sha, so the commit that turns a criterion green also changes this record, naming what it greens,
  and the commit after it writes the `green at` line. The web engine's commit, b6030dd5, greens A16
  and A17, `the_study_calls_are_the_reviews_pairs`,
  `a_boundary_function_that_answers_a_constant_is_refused_by_name` and the parity test, each
  measured at b6030dd5, and it changes `crates/web-engine/tests/study.rs` only by adding the tests
  of R6 and R7's mirrors.
- **`exempt.rs`'s Undo arm names its target "its target".** Each per-write synthetic collection
  numbers its own cards, and the fixture holds no answer, so the Undo request's record names no
  row and is refused before the engine.
- **The page's commit, 2c68b183, greens A20 to A28.** It carries the review's
  machine, the Worker protocol, the dialog and the fifteen messages in every locale; each of A20
  to A28 is measured green at that commit, and the commit after it writes their `green at` lines.
  A28's green shows both codes by name, `undo-synced` and `not-undoable`, each from its own
  request.
- **A27's test changed after its red, adding a step and no assertion.** It reveals the answer
  before its first `u`, because the remote's reader (`resolve`) turns no key but show answer into
  an action on the question side. Its red at f2076d70, no `alertdialog` because no confirmation
  state existed, does not depend on the side.
- **The keyboard and the dialog.** While the dialog asks, the focus is on "Keep it", and the key
  reader leaves a key aimed at a button to that button, so key `u` does not confirm: the keyboard
  confirms through the dialog's "Undo answer" button, and remote button 4 confirms from the answer
  side; on the question side the bar's "Undo answer" button asks for the offer.
- **Tests of the old undo, changed to the offer and its confirmation, each still asserting what it
  covered.** `review.test.ts` "the review shows, reveals, rates and moves on" undoes through the
  offer and the confirmation, and "the table has these cells and no others" names the new cells,
  the question's and the answer's `undo` now asking for the offer; `review-screen.test.ts` "undo,
  bury and flag act on the shown card" presses "Undo answer" and confirms in the dialog;
  `session.test.ts`'s undo requests carry `card` and `step`, and "the session opens, answers and
  undoes through the engine" asks for the offer first; `client.test.ts` sends `undo(card, step)`
  and the offer; `protocol.test.ts` lists `undo-offer` after `undo` and parses `undo`'s card and
  step; and `tests-engine/engine.spec.ts` undoes through the offer, its titles kept.
- **Files outside the manifest's first draft, each changed because the new undo shape broke it,
  each now a manifest row.**
  - `web/app/src/lib/study/refusal.ts`: `STATUS_KEYS` names the two new statuses' messages.
  - `web/app/src/lib/study/refusal.test.ts`: its status table gains `undo-synced` and
    `not-undoable`.
  - `web/app/src/lib/engine/credential.test.ts`: a bare `{ op: 'undo' }` is now refused as
    `undo's card is malformed`, so its study list carries `card` and `step` and asks for the offer.
  - `web/app/src/lib/engine/credential-stand-in.test.support.ts`: its study module answers
    `undo_offer`.
  - `web/app/src/routes/study.test.ts`: its card's `undo` reads `null`, the new type.
  - `web/app/tests-study/study.spec.ts`: "undo returns the rated card" presses "Undo answer",
    confirms in the dialog and still reaches the rated card, title kept.
  - `web/app/src/lib/study/audio.test.ts`: its card's `undo` reads `null`, the new type, and its
    fake client answers `undoOffer` with no offer; no assertion changed.
  - `web/app/src/lib/study/voice.test.ts`: its card's `undo` reads `null`, the new type, and its
    fake client answers `undoOffer` with no offer; no assertion changed.

## What the mutation pass added

- **Seventeen rows, `S37101` to `S37117`, each proved by hand before the commit that writes them,
  f4356c31.** For each row the target's sha256 was read, its find counted once, the replacement
  installed once and only its killer run, one test selected: the control passed, the mutant failed
  by the killer's name for the row's reason, and the restored target matched its sha256. Rows
  `S37101`, `S37112` and `S37117` pin `const` literals, which the generator never mutates. Rows
  `S37113` to `S37116` mutate `src/wasm.rs`, which only the browser target compiles; their native
  killer is the boundary census,
  `boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher`, which reads that
  file's text and names the owed statement each mutant removes or changes.
- **A4's code path has no hand row.** Its success path is held by the generated
  `Dispatcher::run_undo` body replacements, which A4's test kills: the `Ok(vec![])` body, installed
  by hand, failed `undo_answer::an_undo_reverts_the_offered_answer` by name, the card left answered
  and its review row still there, and the restored file matched. `S37109`, with A5's test its
  killer, holds the check side.
- **F5 has no row.** The retired text `call(service::COLLECTION, 8,` is already an entry of
  `boundary.rs`'s `RETIRED`, killed by its own planted control,
  `boundary::a_retired_text_planted_back_is_refused_by_name`; SPEC-371 section 9 lists seventeen
  rows, and an eighteenth would go beyond it.
- **Every row the gate would select dies.** A plan built the way the gate builds one over this
  branch selected 95 rows, this band's seventeen with the rows already on the files and killers it
  touches, and proving them read 95 killed, 0 survived, 0 void.
- **`src/wasm.rs` was run whole, natively.** All 100 of its generated mutants ran: 65 caught, 35
  missed. The 35 missed are exactly the 35 records of
  `scripts/mutation-equivalent.d/deck-streak-web-engine.json`, one mutant to a record: 24 kept and
  11 new, each new one a private helper of `undo`, `undo_offer` or `rate` (`no_offer`,
  `engine_record`, `review_of`, `mirrored`, `returned` and `newest_review`) whose body no native
  test reaches and whose text the census does not own. The kept `close` record is re-anchored on
  `close`'s new body, which first clears the kept answer; its mutant is still missed natively. No
  kept record was refuted.
- **The diff's own mutants.** A run over this branch's diff generated 66: 48 caught, 6 unviable and
  12 missed. All 12 missed are in `src/wasm.rs`, each one of the records above (`close` and the 11
  new). The 6 unviable replace a body with a `Default::default()` value of a type that derives no
  `Default`. The engine core's 26 viable mutants and the 13 viable in `src/study.rs` were all
  caught, as were 9 in `src/wasm.rs`.
- **The page's StrykerJS survivors, killed by nine tests (MUTATION COVERAGE: each is green at the
  round's base, so none is red first).** Each line names the test, the mutants it kills as file:line
  and mutator, and the test that StrykerJS's report names in `killedBy`.
  - `review.test.ts` "a confirmation refused as synced reloads the card and says so": `review.ts:387`
    ConditionalExpression and StringLiteral on the `undo-synced` code; `killedBy` reads this test.
  - `review.test.ts` "an offer the Worker declines for another reason says the answer can no longer
    be undone": `review.ts:372` ConditionalExpression and the `not-undoable` StringLiteral;
    `killedBy` reads this test.
  - `review.test.ts` "a synced press on the answer side announces itself and tells the screen":
    `review.ts:288` ConditionalExpression, EqualityOperator and StringLiteral on the second operand,
    and `review.ts:290` the removed `onChange` call; `killedBy` reads this test.
  - `review.test.ts` "a synced press while a request is in flight announces nothing":
    `review.ts:288` ConditionalExpression on the whole guard; `killedBy` reads this test.
  - `review-screen.test.ts` "the confirmation names every state the card goes back to":
    `ReviewScreen.svelte:65`, `:67`, `:68` and `:69` ArrowFunction; `killedBy` reads this test.
  - `review-screen.test.ts` "the confirmation says so for a card with no text, and names an Again
    answer": `ReviewScreen.svelte:209` and `:214` ConditionalExpression, StringLiteral and
    CallExpression; `killedBy` reads this test.
  - `review-screen.test.ts` "keeping the answer, by its button or by Escape, gives the focus back to
    the review": `ReviewScreen.svelte:126` the removed `region.focus()`; `killedBy` reads this test.
  - `review-screen.test.ts` "while it asks, the undo key confirms rather than keeps":
    `ReviewScreen.svelte:132` LogicalOperator and the ConditionalExpression on the `Escape` operand;
    `killedBy` reads this test.
  - `review-screen.test.ts` "outside the confirmation, Escape keeps a notice standing and every
    other key is the input's": `ReviewScreen.svelte:132` ConditionalExpression on the `confirming`
    operand; `killedBy` reads this test.
- **Five mutants were equivalent, so two lines were rewritten, none excluded.**
  `ReviewScreen.svelte:197` read `shown.phase === 'confirming' && shown.offer !== null`, but
  `Review.offer` is non-null exactly while the phase is `confirming`, so each operand, and the `||`
  form, changed nothing a reader could see. It now reads `shown.offer !== null`. `review.ts:253`
  read `action === 'undo' && this.#state.phase !== 'confirming'`, but the view says `answer` for
  every confirmation (an offer is asked only from that state), so the second operand and its
  string-literal mutant changed nothing. It now reads `action === 'undo'`. After the rewrites the
  run read no survivor and no uncovered mutant, and `killedBy` names a test for each mutant of
  these two lines.
- The nine tests above and the two rewrites are at 3b226c6d7af01f5f9d18b7124e738f49e0d7387b.
