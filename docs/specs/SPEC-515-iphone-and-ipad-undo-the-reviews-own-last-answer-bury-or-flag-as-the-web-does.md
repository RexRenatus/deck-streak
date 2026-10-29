# SPEC-515: iPhone and iPad undo the review's own last answer, bury or flag, as the web does

- **Context(s):** `deck-streak-engine-core` (`crates/engine-core`), `deck-streak-ffi`
  (`crates/ffi`), the app (`ios/App`).
- **Decided by:** ADR-533 (this SPEC's own), under ADR-382 (Undo is an exempt write behind the
  owner's gesture, checked at the write, after the review shows what it changes), ADR-397 (a bury
  or a flag is restored through the same exempt Undo, by the record's kind) and ADR-521 (the native
  column and the adapter's facade).
- **Delivers:** SPEC-358 section 7's native undo, moved out of its part a (#714).
- **Beside:** SPEC-371 and SPEC-383 (the web's undo of an answer, a bury and a flag), SPEC-358
  (the native bury and flag), SPEC-503 (the native sync, after which no undo is offered).
- **Schematic:** `docs/schematics/native-undo.md` (the status after open, answer, bury, flag and
  undo; the exempt Undo (3,8); the next queued card).
- **Mutation band:** S51500-S51599. **Model:** `formal/tla/UndoOwnAnswer`, re-read with the native
  actor, its covers gaining the adapter's two entries (section 8).

## 1. The problem, measured

At the base (`92755ef0`), the web undoes the review's own last answer, its last bury and its last
flag through the core's two rules, `undo_answer::judge` and `undo_change::judge_change`, and the
exempt Undo (3,8), which only an owner's gesture reaches (`table.rs:336-342`; `dispatch.rs`
`run_undo` and `run_restore`). The iPhone and iPad app cannot:

| id | measured | figure | command |
|---|---|---|---|
| M1 | The native column lacks the undo status | (3,7) `CollectionService.GetUndoStatus` is `native: false, web: true` (`table.rs:151-157`); the adapter's allow-list holds 11 pairs without it (`allow_list.rs:32`) | `git show 92755ef0:crates/engine-core/src/table.rs \| sed -n '151,157p'` |
| M2 | The adapter has no undo | it exports `run_exempt` (`engine.rs:144`), whose taps name six writes and no Undo, `answer` (`:194`), `bury` (`:224`) and `flag` (`:236`) | `git grep -n -E 'pub fn (answer\|bury\|flag\|run_exempt)\(' 92755ef0 -- crates/ffi/src/engine.rs` |
| M3 | The adapter reads no protocol message | its dependencies are the core, UniFFI, reqwest and tokio (`crates/ffi/Cargo.toml`): it can neither decode an undo status nor encode the record the exempt Undo decodes | `git show 92755ef0:crates/ffi/Cargo.toml` |
| M4 | The app names no undo | 0 lines of `ios/App`, `ios/AppTests` and `ios/AppUITests` match `undo` | `git grep -c -i undo 92755ef0 -- ios/App ios/AppTests ios/AppUITests` |
| M5 | The app's budgets | `ReviewModel.swift` holds 6 decisions (the model ceiling), `EngineSession.swift` 4 (the session ceiling), `ReviewSession.swift` 3, `ReviewChrome.swift` and `AnswerBar.swift` 3 each (the view ceiling) | `ios/swift-roles.json` |

## 2. Requirements

- **R1. The column.** (3,7) `CollectionService.GetUndoStatus` is `native: true`, and the
  adapter's allow-list carries it (twelve pairs), held equal to the column by the parity test.
  (3,8) stays an exempt write on both transports.
- **R2. One slot, in the adapter.** The adapter's `Engine` keeps the review's last recorded step,
  an answer, a bury or a flag, and never two. `answer` records only when the newest review is the
  answered card's; `bury` records only when the card's mark shows it user-buried; `flag` records
  only when the mark shows the flag the toggle set. A successful undo empties the slot; the native
  engine opens one collection, so no step of another opening is kept. A step the engine's reads
  cannot record is not kept, and the press, bury or flag itself stands. `bury` takes the states the
  card was shown with, as `answer` does, so a bury's record names the state its card returns to.
  The core's new `native_undo` module reads each fact
  through the dispatcher's fixed reads and the status the adapter read through `run`, and keeps the
  record the exempt Undo decodes; the adapter names no protocol type.
- **R3. The offer.** `undo_offer()` reads (3,7) through `run`, has the core judge the slot's record
  now (`judge` for an answer, `judge_change` for a bury or a flag), and answers `UndoOffered`: an
  `UndoOffer` naming the card, the step, the kind and the dialog's words, or none with the sentence
  that says why. The card is named by its question as one line of text: the question's nodes
  joined, its AV tags stripped and its HTML made one line with media names kept, as the web's offer
  reads it. The words are the web's English ones (SPEC-371 R10, SPEC-383 R11): the title, `Card:`
  with the text, `Your answer:` with Again or Good, `Goes back to:` with the state and "in today's
  queue", the line of what the undo changes, `Keep it` and the confirmation's title.
- **R4. The undo.** `undo(card, step)` refuses unless the slot names that card and step, has the
  core judge the record again, mints `OwnerGesture::from_tap(ExemptWrite::Undo,
  Target::Card(card))` and runs `run_exempt` with the record, which the core judges once more at
  the write. On success the slot empties and the next queued card is the undone one.
- **R5. A refused undo says why.** `UndoOffered.said` and `UndoDone.said` hold the web's sentence:
  for a synced record, "Your last answer has synced, so it can no longer be undone." or "Your last
  change has synced, so it can no longer be undone."; for every other refusal, "This answer can no
  longer be undone: something changed after it." or "This change can no longer be undone:
  something changed after it.", by the kind the confirmation's offer named. An engine error stays
  `EngineRefusal::Engine`.
- **R6. The app.** `ReviewAction` gains undo, keep and the confirmation. The undo control sits in
  the review's top row, enabled only from a card's question or answer while the adapter's offer
  names a step it would undo now, that step's confirmation title ("Undo answer", "Undo bury" or
  "Undo flag") as its accessibility value; the offer is read after the review opens and after
  every answer, bury, flag and undo, and the done screen offers none. (3,7)'s raw label does not
  gate it: choosing the review's deck is itself an engine step with a label, which the review did
  not make. A press asks for the offer and writes nothing; the dialog shows the offer's words as
  text, with the focus on Keep; Keep writes nothing; the confirmation runs the undo, then the
  review shows the next queued card; a refusal shows its sentence. Swift builds no record and
  judges nothing. Every new Swift file is registered with its role and exact decision count, and
  no ceiling rises.
- **R7. The app's terms.** The app holds an offer as its own `UndoDialog`, field for field; the
  session alone names the adapter's records, as it does the sync choice's.
- **R8. The model.** `UndoOwnAnswer` is re-read with the native actor: its covers gain
  `crates/ffi/src/engine.rs` `undo` and `undo_offer`, and its four witnesses still violate.
- **R9. Containment.** Outside the core, `.undo(` is held at the adapter's own tests' lines, and
  the gesture on the exempt Undo is minted once in each of the two review doors (`wasm.rs` and the
  adapter's `engine.rs`), nowhere else. In the app, the adapter's undo is reached only from the
  session, the session's only from the review's undo model, and the undo's two gestures only from
  the undo control.

## 3. Acceptance criteria

```acceptance
A1: cargo test -p deck-streak-engine-core --test table
A1: cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_table_equals_its_transport_column
A1: cargo test -p deck-streak-ffi --test review_pairs -- --exact the_allow_list_carries_the_review_pairs
A2: cargo test -p deck-streak-ffi --test undo -- --exact an_answer_is_offered_by_its_card_answer_and_state_then_undone
A2: cargo test -p deck-streak-ffi --test undo -- --exact a_bury_is_offered_with_its_line_then_undone
A2: cargo test -p deck-streak-ffi --test undo -- --exact a_flag_is_offered_with_its_line_then_undone
A2: cargo test -p deck-streak-ffi --test review_actions -- --exact a_bury_buries_the_card_as_the_users_bury
A3: cargo test -p deck-streak-ffi --test undo -- --exact a_confirmation_after_a_later_change_is_refused_and_says_why
A3: cargo test -p deck-streak-ffi --test undo -- --exact with_nothing_kept_no_undo_is_offered
A3: cargo test -p deck-streak-ffi --test sync -- --exact after_a_normal_sync_the_undo_says_the_answer_has_synced
A4: cargo test -p deck-streak-engine-core --test containment
A4: python3 -m unittest discover -s scripts/tests -p test_ios_undo_reach.py
A5: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py
A5: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests -only-testing:DeckStreakUITests/ReviewActionsFlowTests
A6: phxd formal check --root . --base <the base> --kind tla --gate --format json
```

A1 holds R1. A2 holds R2, R3 and R4 at the adapter: an answer's, a bury's and a flag's offer, each
with its words and the card named as text, and the undo the confirmation runs, after which the card
is back. A3 holds R5: a confirmation after a later change, an offer and a confirmation with nothing
kept, and an answer that has synced. A4 holds R9. A5 holds R6 and R7 in the app: the census of the
Swift budgets runs everywhere; the XCTest classes, on one iPhone and one iPad simulator, run on the
macOS lane only, beside the review's other classes, with the review fixture. A6 holds R8.

## 4. File manifest

- `crates/engine-core/src/table.rs`: (3,7) `native: true`.
- `crates/engine-core/src/native_undo.rs` (new) and `crates/engine-core/src/lib.rs`: the slot's
  record, its facts and its judge for the native adapter.
- `crates/engine-core/src/dispatch.rs` and `crates/engine-core/src/face.rs`: a card's question as
  one line of text.
- `crates/ffi/src/allow_list.rs`: (3,7) joins the allow-list.
- `crates/ffi/src/engine.rs`: the slot, `undo_offer`, `undo`, and `answer`, `bury` and `flag`
  recording their step; `crates/ffi/src/review_undo.rs` (new) and `crates/ffi/src/lib.rs`: the
  offer's records and words.
- `crates/ffi/tests/undo.rs` (new), `crates/ffi/tests/sync.rs`, `review_pairs.rs` and
  `review_actions.rs`; `crates/engine-core/tests/table.rs` and `containment.rs`.
- `ios/App/Sources/ReviewUndo.swift`, `UndoControl.swift` and `UndoDialogView.swift` (new),
  `ReviewModel.swift`, `ReviewSession.swift`, `EngineSession.swift`, `ReviewChrome.swift`,
  `AnswerBar.swift`; `ios/AppTests/ReviewModelTests.swift`,
  `ios/AppUITests/ReviewActionsFlowTests.swift`; `ios/swift-roles.json`.
- `scripts/tests/test_ios_undo_reach.py` (new).
- `formal/tla/UndoOwnAnswer/UndoOwnAnswer.tla`.
- `docs/decisions/ADR-533-*.md` (new), `docs/schematics/native-undo.md` (new),
  `changelog.d/native-undo-515.md` (new).
- This SPEC (new) and its red-first record, `docs/red-first/SPEC-515.md` (new).
- `scripts/mutation-rows.d/S51500-S51599.json` (new): the band's rows (section 7);
  `scripts/mutation-rows.d/S35000-S35099.json`: S35019's find reads (3,7)'s native mark, now
  `native: true`.

## 5. What this does NOT cover

- The hardware keyboard's and the game controller's undo keys: they reach the same gesture through
  the review's one entry when the keyboard and controller parity is built.
- Words in any language but English: the app's other review words are English today.
- Redo, and an undo of anything but the review's own last answer, bury or flag.

## 6. Risks

- The control reads the adapter's offer after every step, one render of the kept step's card each
  time; a review's step is a press, so the render runs at the press's pace.
- The dialog's words live in the adapter beside the sync choice's sentences, so a change of them is
  a Rust change.

## 7. Mutation rows

Band S51500-S51599: S51500 (3,7)'s native mark; S51501 (3,7)'s allow-list entry; S51502 the
offer's judge; S51503 the card the offer names; S51504 the kind a late confirmation's refusal
names. Each row's killer is named in the acceptance block above. S35019 keeps its id and property;
its find reads (3,7)'s `native: true`.

## 8. Formal model

`UndoOwnAnswer` models one review's undo with an actor that offers, keeps and confirms; the native
adapter is that actor on iPhone and iPad, in the same order: `undo_offer` judges the record against
the engine's undo queue and writes nothing, and `undo` checks the confirmation's card and step,
judges again and runs the exempt Undo, which the core's door judges at the write. The adapter's
`answer`, `bury` and `flag` record as the web's `rate`, `bury` and `flag` do. Its covers gain
`crates/ffi/src/engine.rs` `undo` and `undo_offer`, and every property and witness stands (A6).

## 9. What only a device or a person proves

A5's XCTest on the iPhone and iPad simulators, and an undo by touch on both devices.
