# SPEC-383: Undo reaches the review's last bury or flag, from one slot, through the owner's gesture, checked at the write

- **Issue:** #753. **Decision:** the owner-taps ruling's three conditions, as SPEC-371 holds them for
  an answer: the undo is reachable only from the review, and that is proven; it makes one gesture's
  own write on the card it names; and the write is shown before it is made, with what it changes.
  This SPEC holds the undo of the review's last bury or flag, while that change has not synced, to
  all three, through the same door the undo of an answer uses.
- **Context(s):** `deck-streak-engine-core` (`crates/engine-core`), `deck-streak-web-engine`
  (`crates/web-engine`), and `miniapp` (`web/app`). No native file changes.
- **Decided by:** ADR-397 (this SPEC's own). It amends ADR-382 (D2 and its Consequences), ADR-361 D1
  for undo only, and ADR-337's Decision Outcome. Each amended ADR gains an insert-only amendment
  section at its end, after SPEC-371's, in the form SPEC-371's sections take
  (`## Amendment: <title> (SPEC-383)`, opening "ADR-397 amends"), and no existing line of any of them
  changes. It works under ADR-356 (no table row and no exempt row is added) and ADR-348 (no export
  is added; three exports change their bodies).
- **Schematic:** `docs/schematics/undo-the-reviews-own-last-answer.md`, amended insert-only by a new
  section 7 (the bury and the flag, from the press to the restored card, and every refused path);
  `docs/schematics/web-study-screens.md:130`, `:131`, `:133` and `:136` follow R10.
- **Status:** one delivery, built on `dev` at or after `e7ecf10d`. **Mutation band:**
  `S38300-S38399` (section 9). **Model:** `formal/tla/UndoOwnAnswer`, amended, required (section 8).

## 1. The problem, measured

Every `path:line` below was read at DeckStreak `dev` `e7ecf10d`.

### 1a. Where bury and flag are written today

| client | the write | engine call | what it changes | review row |
|---|---|---|---|---|
| web review | `crates/web-engine/src/wasm.rs` `bury` (`:771-784`) | (13,14) BuryOrSuspendCards with `bury_of(card)` (`crates/engine-core/src/review.rs:39`): one card, no notes, mode `BURY_USER` = 2 (`review.rs:23-24`) | the card's queue becomes user-buried (-3); the kept card is cleared | none |
| web review | `wasm.rs` `flag` (`:789-805`) | (5,4) SetFlag with `toggled_red(flag)` (`review.rs:19-21`) | the card's flag: red becomes none, and any other flag, none included, becomes red; the kept card keeps its new flag, and the export answers it | none |
| native review | `crates/ffi/src/engine.rs` `bury` (`:214-218`) and `flag` (`:226-231`), through the allow-list, with `bury_request` (`review.rs:50`) and `flag_request` (`review.rs:62`) | the same two calls | the same | none |
| server | no crate writes either (`git grep` over `crates/` names only the four functions above) | none | none | none |

Each change marks the card unsynced (`usn` -1), and the next normal sync sends it. A normal sync
discards the engine's undo queue (SPEC-371 section 1b). The web review holds a normal sync today:
SyncCollection (1,5) is an ordinary pair the web transport admits and the native one does not
(`crates/engine-core/src/table.rs:130-136`).

### 1b. What the shipped Undo records and checks

- `LAST_ANSWER` (`wasm.rs:82`) is the one record. `rate` (`wasm.rs:719-756`) writes it only when
  the newest review is of the rated card; `open` (`:169`), `close` (`:193`) and a successful `undo`
  (`:328-348`) clear it. `bury` and `flag` leave it as it is.
- `Recorded` (`crates/engine-core/src/undo_answer.rs:15-23`) holds the engine's undo status and the
  review row's id: two fields, and no kind.
- `judge` (`undo_answer.rs:71-98`) refuses, in order: no review row, `Gone`; another card,
  `NotTheCard`; a synced row, `Synced`; an empty undo queue, `Gone`; another last step or another
  undo label, `Changed`.
- After a bury or a flag, the engine's last step is no longer the record's, so `judge` reads
  `Changed` and the control offers nothing. SPEC-371 R1 (`:98-99`) states it: "After a bury or a
  flag, Undo is not offered (a flag toggles back by itself)."

### 1c. What the learner loses today

- **A bury made by mistake takes a due card out of today's review.** The review holds no unbury:
  none of the 18 ordinary pairs (`table.rs:122`) unburies a card, and `review.rs:23` records that
  the next day does not undo the user's own bury alone.
- **A flag pressed on a card that carried another flag loses that flag.** `toggled_red` turns flags
  2 to 7 into red, and the toggle back turns red into none (`review.rs:19-21`). "A flag toggles back
  by itself" holds only for a card with no flag or a red one.
- **Either change also takes away the undo of the answer before it,** because `judge` then reads
  `Changed`.

### 1d. What a restore needs, and what the engine already offers

- (3,8) "CollectionService.Undo" reverts the engine's last step, whatever its kind, and returns the
  card to the row it had before that step (SPEC-371 section 1b, `:45-62`). The exempt Undo row
  (`table.rs:337-340`) already reaches it, behind the owner's gesture, through the private
  `run_undo` (`crates/engine-core/src/dispatch.rs:260-282`). No forward write restores a flag
  exactly: SetFlag back would need the earlier flag and would add a step of its own.
- Three things are missing: a record that names its kind and its card; a read of the card's queue,
  flag and sync mark (`Read` holds four fixed reads, `dispatch.rs:139-148`, and none reads the
  flag); and a check for each kind.

## 2. Requirements

- **R1. One slot.** The review's undo slot holds its last recorded action: an answer, in the
  shipped `LAST_ANSWER`, or a bury or a flag, in a new `thread_local!` item `LAST_MARK` in
  `wasm.rs`, never both. `rate` clears `LAST_MARK` when it records an answer; `bury` and `flag`
  clear `LAST_ANSWER` when they record their change; `open`, `close` and a successful undo of
  either kind clear both.
- **R2. What a bury or a flag records.** `bury` records only after its write, and only when the
  card's `CardMark` read shows it user-buried (queue -3). `flag` records only after its write, and
  only when the `CardMark` read shows the flag the toggle set; otherwise it answers an error and
  records nothing. The record holds the card, the kind, the engine's undo status read through (3,7)
  after the write, and: for a bury, the state the card returns to, `returns_to` of the kept card's
  current state (as `rate` computes it for an answer); for a flag, the flag before and the flag the
  change left.
- **R3. The record names its kind.** `Recorded` gains `kind` (tag 3: 0 an answer, 1 a bury, 2 a
  flag), `flag` (tag 4: the flag the change left) and `card` (tag 5: the changed card). A record
  with no kind decodes as an answer, so every record the shipped code writes reads as before; a kind
  the engine does not name is refused.
- **R4. One new fixed read.** `Read::CardMark(card)` runs
  `select queue, flags, usn from cards where id = ?` and answers the card's queue, flags and sync
  mark, or no row.
- **R5. The check for a bury or a flag.** A new pure `judge_change(recorded, now, mark, card)`, in
  `crates/engine-core/src/undo_change.rs`, refuses, in order: no card row, `Gone`; a record of
  another card, `NotTheCard`; a bury whose card is not user-buried, or a flag whose card's user flag
  (the low three bits of `flags`) is not the recorded flag, `Changed`; a card that has synced
  (`usn` not -1), `Synced`; an empty undo queue, `Gone`; another last step, `Changed`; another undo
  label, `Changed`. It reuses `UndoRefusal` unchanged. `judge` and every line of it are unchanged.
- **R6. The same door.** `run_undo` reads the record's kind first. An answer takes the shipped path,
  unchanged. A bury or a flag takes a new private `run_restore`, which reads (3,7) and
  `CardMark(card)`, runs `judge_change`, and runs (3,8) with an empty request only when it admits.
  Every refusal reaches the caller as `GestureRefusal::NotTheTarget`, as shipped. No exempt row, no
  `ExemptWrite`, no `GestureRefusal` variant and no table pair is added; `OwnerGesture` is still
  minted only in `wasm.rs` `undo`.
- **R7. The restore is exact, and nothing else moves.** A confirmed undo of a bury or a flag
  returns the card to the row it had before the change, queue, flag and sync mark included. It
  writes and removes no review row, and changes no grade, no schedule and no memory state.
- **R8. Sync.** A change that has synced is refused `Synced`, and the control says so. A normal
  sync discards the engine's undo queue, so an undo after it is refused. An undo of an unsynced
  change leaves the card as it was before the change, so the next sync has nothing to send for it.
- **R9. The web engine's answers.** `undo_offer` answers the slot's offer with its `kind`: an
  answer's offer as shipped; a bury's with the card, the step, the card's text and the state it
  returns to; a flag's with the card, the step, the card's text and what the undo does to the flag
  (`added`, `removed` or `replaced`). `undo(card, step)` undoes the slot's record when its card and
  step match the offer. `current_card`'s `undo` names `'answer'`, `'synced'`, `'bury'`, `'flag'`,
  `'change-synced'` or nothing; the shipped two values keep their meaning. `flag` keeps its
  arguments and its answer, the card's new flag.
- **R10. The control.** The bar's undo button reads "Undo answer", "Undo bury" or "Undo flag" by
  the view, and key `u` and the remote's button 4 stay its only other doors. For a synced change it
  is disabled and says "Your last change has synced, so it can no longer be undone." A press asks
  for the offer, the dialog shows what the undo changes, and the write is made only on
  confirmation, as shipped. After a flag the control offers the flag's undo; after a bury the next
  card's view offers the bury's undo. The done screen offers none.
- **R11. The dialog.** A bury: the title "Undo this bury?", "Card:" with the card's text, "Goes
  back to:" with the state, the line "This bury has not synced yet. Undoing it puts the card back
  in today's queue; nothing else changes.", and the buttons "Keep it" (focused) and "Undo bury". A
  flag: the title "Undo this flag?", "Card:" with the card's text, one line by what the undo does
  ("You added a red flag. Undoing removes it; nothing else changes.", "You removed the red flag.
  Undoing puts it back; nothing else changes." or "You replaced this card's flag with a red flag.
  Undoing puts its earlier flag back; nothing else changes."), and "Keep it" (focused) and "Undo
  flag".
- **R12. The words, in every locale.** Ten keys join each of the seven locale files (en, es, fr,
  ja, ko, zh-Hans, zh-Hant), seventy strings: `study_undo_bury`, `study_undo_flag`,
  `undo_bury_title`, `undo_flag_title`, `undo_bury_unsynced`, `undo_flag_added`,
  `undo_flag_removed`, `undo_flag_replaced`, `undo_change_synced` and `undo_change_gone`, in English
  as R10, R11 and R13 give them. `undo_card`, `undo_returns`, every `undo_returns_*`, `undo_keep`
  and `undo_no_text` are reused unchanged.
- **R13. A refused undo of a change says why.** The page picks the notice by the kind it pressed:
  `undo-synced` reads "Your last change has synced, so it can no longer be undone." and
  `not-undoable` reads "This change can no longer be undone: something changed after it." The
  answer's notices are unchanged, and no error code is added.
- **R14. The censuses.** The web engine's boundary census holds each slot fact of R1 and R2 in
  `wasm.rs` by function, with plants it refuses by name; the locale census holds R12's ten keys in
  every locale, with a plant it refuses by name.
- **R15. The model.** `formal/tla/UndoOwnAnswer` gains the bury and the flag, two new properties at
  `ramp=report` with a witness each, and covers for the new and changed spans (section 8).
- **R16. The records.** `docs/red-first/SPEC-383.md`, the band `scripts/mutation-rows.d/S38300-S38399.json`
  and `changelog.d/undo-bury-flag-383.md` are added.

## 3. Acceptance criteria of the undo of the review's last bury or flag

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | A confirmed undo of the review's bury returns the card whole: every column of `CardSnapshot` and `CardMark` reads as before the bury | the record's kind is not read: the undo takes the answer path, is refused `NotTheTarget`, and the card stays buried | `crates/engine-core/tests/undo_change.rs` `a_confirmed_undo_of_a_bury_restores_the_card_whole` |
| A2 | A card flagged 4, then toggled red by the review, reads flag 4 after a confirmed undo, every column of both reads as before the toggle | refused `NotTheTarget`; the flag stays red | `undo_change.rs` `a_confirmed_undo_of_a_flag_puts_back_the_flag_the_card_had` |
| A3 | Answer A, then bury B: a confirmed undo of the bury returns B, and A's review row and state stay | refused; B stays buried | `undo_change.rs` `an_undo_of_a_bury_after_an_answer_leaves_the_answer` |
| A4 | Bury B, then answer C: an undo with B's record is refused, B stays buried and C stays answered | the unchecked restore reverts the front: `Ok`, and C's answer is gone | `undo_change.rs` `an_undo_of_a_bury_after_a_later_change_is_refused_and_changes_nothing` |
| A5 | An undo that targets card C with B's bury record is refused, and B stays buried | the unchecked restore runs: `Ok`, and B is no longer buried | `undo_change.rs` `an_undo_of_a_change_aimed_at_another_card_is_refused` |
| A6 | `judge_change` refuses `Gone` when the card row is absent, and admits the same inputs with it present | the stub admits every input | `undo_change.rs` `a_change_whose_card_is_gone_is_refused` |
| A7 | `judge_change` refuses `NotTheCard` for a record of another card | the stub admits | `undo_change.rs` `a_change_recorded_for_another_card_is_refused` |
| A8 | `judge_change` refuses `Changed` for a bury whose card is no longer user-buried | the stub admits | `undo_change.rs` `a_bury_no_longer_buried_is_refused` |
| A9 | `judge_change` refuses `Changed` for a flag whose card's user flag is not the recorded flag | the stub admits | `undo_change.rs` `a_flag_since_changed_is_refused` |
| A10 | `judge_change` reads only the user flag's three bits: `flags` 9 against a recorded red admits, `flags` 12 against a recorded none refuses | the stub admits both | `undo_change.rs` `a_flag_is_judged_by_the_users_three_bits` |
| A11 | `judge_change` refuses `Synced` for a card whose `usn` is not -1 | the stub admits | `undo_change.rs` `a_synced_change_is_refused` |
| A12 | `judge_change` refuses `Gone` for an empty undo queue | the stub admits | `undo_change.rs` `an_empty_undo_queue_refuses_a_change` |
| A13 | `judge_change` refuses `Changed` for another last step | the stub admits | `undo_change.rs` `a_later_step_refuses_a_change` |
| A14 | `judge_change` refuses `Changed` for another undo label | the stub admits | `undo_change.rs` `another_undo_label_refuses_a_change` |
| A15 | The bytes of a shipped two-field record decode as an answer, with flag 0 and card 0 | not red: it pins the meaning of the shipped bytes, which the stub's new fields already keep; row `S38312` holds it | `undo_change.rs` `an_old_record_decodes_as_an_answer` |
| A16 | A real answer's record carrying kind 7 is refused through the door, and the answer stays | the kind is not read: the answer path admits, and the answer is undone | `undo_change.rs` `a_record_of_an_unknown_kind_is_refused_and_leaves_the_answer` |
| A17 | `mark_view` names `bury` or `flag` for an admitted record, `change-synced` for `Synced`, and nothing for every other refusal | the stub answers nothing | `crates/web-engine/tests/study.rs` `a_mark_view_names_its_kind` |
| A18 | `last_mark_for` answers the record only when both the card and the step match | the stub answers nothing | `study.rs` `a_mark_record_matches_only_its_card_and_step` |
| A19 | `flag_change` reads `added` for none to red, `removed` for red to none, and `replaced` for any other flag to red | the stub answers `added` | `study.rs` `a_flag_offer_says_what_the_undo_puts_back` |
| A20 | The census reads in `wasm.rs`: `bury` and `flag` record `LAST_MARK` after their `CardMark` read and clear `LAST_ANSWER`; `rate` clears `LAST_MARK`; `open`, `close` and a successful undo clear both; and it refuses each plant by name | `wasm.rs` holds no `LAST_MARK` | `crates/web-engine/tests/boundary.rs` `the_review_records_one_change_at_a_time` |
| A21 | The bar's undo control reads "Undo answer", "Undo bury" or "Undo flag" by the view | a `bury` or `flag` view shows no undo control | `web/app/src/lib/study/review.test.ts` "the undo control names the change it would undo" |
| A22 | After a flag the view's undo is the flag's | the view keeps the undo it had before the flag | `review.test.ts` "after a flag the undo control offers the flag" |
| A23 | A press on the undo of a bury or a flag sends only the offer request, and a confirmation sends the undo with the offer's card and step | a `bury` view's press sends nothing | `review.test.ts` "a pressed undo of a bury or a flag asks before it writes" |
| A24 | A refused undo of a bury or a flag reads the change's notice for `undo-synced` and for `not-undoable` | the answer's notices are shown | `review.test.ts` "a refused undo of a change reads its own notice" |
| A25 | The confirmation of a bury is an `alertdialog` naming the card as text and the state it returns to, with focus on "Keep it" and the button "Undo bury" | no dialog renders for a bury offer | `web/app/src/lib/study/review-screen.test.ts` "the confirmation of a bury names the card and the state it returns to" |
| A26 | The confirmation of a flag names the card and reads the added, removed or replaced line by the offer | no dialog renders for a flag offer | `review-screen.test.ts` "the confirmation of a flag says what the undo puts back" |
| A27 | The session maps a bury offer's `returns` and a flag offer's `flag`, each with its `kind` | the offer carries no kind | `web/app/src/lib/engine/session.test.ts` "an offer of a bury or a flag reads its kind" |
| A28 | Every locale holds R12's ten keys, and the census refuses a locale that lacks one, by name | the ten keys are absent | `web/app/src/lib/study/undo-reach.test.ts` "every locale holds the bury and flag undo messages" |

```acceptance
A1: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_confirmed_undo_of_a_bury_restores_the_card_whole
A2: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_confirmed_undo_of_a_flag_puts_back_the_flag_the_card_had
A3: cargo test -p deck-streak-engine-core --test undo_change -- --exact an_undo_of_a_bury_after_an_answer_leaves_the_answer
A4: cargo test -p deck-streak-engine-core --test undo_change -- --exact an_undo_of_a_bury_after_a_later_change_is_refused_and_changes_nothing
A5: cargo test -p deck-streak-engine-core --test undo_change -- --exact an_undo_of_a_change_aimed_at_another_card_is_refused
A6: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_change_whose_card_is_gone_is_refused
A7: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_change_recorded_for_another_card_is_refused
A8: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_bury_no_longer_buried_is_refused
A9: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_flag_since_changed_is_refused
A10: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_flag_is_judged_by_the_users_three_bits
A11: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_synced_change_is_refused
A12: cargo test -p deck-streak-engine-core --test undo_change -- --exact an_empty_undo_queue_refuses_a_change
A13: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_later_step_refuses_a_change
A14: cargo test -p deck-streak-engine-core --test undo_change -- --exact another_undo_label_refuses_a_change
A15: cargo test -p deck-streak-engine-core --test undo_change -- --exact an_old_record_decodes_as_an_answer
A16: cargo test -p deck-streak-engine-core --test undo_change -- --exact a_record_of_an_unknown_kind_is_refused_and_leaves_the_answer
A17: cargo test -p deck-streak-web-engine --test study -- --exact a_mark_view_names_its_kind
A18: cargo test -p deck-streak-web-engine --test study -- --exact a_mark_record_matches_only_its_card_and_step
A19: cargo test -p deck-streak-web-engine --test study -- --exact a_flag_offer_says_what_the_undo_puts_back
A20: cargo test -p deck-streak-web-engine --test boundary -- --exact the_review_records_one_change_at_a_time
A21: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "the undo control names the change it would undo"
A22: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "after a flag the undo control offers the flag"
A23: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "a pressed undo of a bury or a flag asks before it writes"
A24: pnpm exec vitest run web/app/src/lib/study/review.test.ts -t "a refused undo of a change reads its own notice"
A25: pnpm exec vitest run web/app/src/lib/study/review-screen.test.ts -t "the confirmation of a bury names the card and the state it returns to"
A26: pnpm exec vitest run web/app/src/lib/study/review-screen.test.ts -t "the confirmation of a flag says what the undo puts back"
A27: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "an offer of a bury or a flag reads its kind"
A28: pnpm exec vitest run web/app/src/lib/study/undo-reach.test.ts -t "every locale holds the bury and flag undo messages"
```

Each of A6 to A14 holds one operand of `judge_change`: the test first reads `Ok` with every operand
matching, then flips that one operand alone and reads its refusal, so that operand alone decides
the result. A1 to A3, A6 to A14 and A16 are red at the first stub, which keeps the shipped
`run_undo` and whose `judge_change` admits every input; A4 and A5 are red at the second, which
reads the kind and restores without the check; both stubs are commits of the build, named in the
red-first record. A15 and A16 read the kind through `Kind::try_from` on the decoded number, never
through a getter that turns an unknown number into the default kind.

Shipped tests whose names this delivery keeps and whose bodies it changes, so that their SPECs'
fence lines still resolve:
- `review.test.ts`'s step test for `'flagged'` (`:392`) and its fake client's `flag` (`:85`): the
  flagged effect also sets the view's undo to the flag's.
- `undo-reach.test.ts` keeps "every locale holds the undo dialog's fifteen messages" unchanged; the
  ten keys get their own test (A28).
- Every test in `crates/engine-core/tests/undo_answer.rs` and `crates/engine-core/tests/exempt.rs` is
  kept by name, input and assertion: an answer's record decodes and is judged as shipped. The only
  body change is that each helper literal of `Recorded` (`undo_answer.rs`'s `answer_head` and
  `admitted`, `exempt.rs`'s `undo`) gains `..Recorded::default()`, so the record it builds still
  carries kind 0, an answer.

Shipped requirements this delivery changes, by their own SPECs (their text is not edited; this
SPEC and ADR-397 record the change):

| shipped requirement | what changes |
|---|---|
| SPEC-371 R1 (`:98-99`, "After a bury or a flag, Undo is not offered") | after a bury or a flag, Undo offers that bury or flag, while it has not synced |
| SPEC-371 section 5 (`:385-387`, "It does not undo a bury or a flag") | true of SPEC-371's delivery; this delivery adds that undo with a record that names its kind |
| SPEC-371 V1 (bury a card and see Undo offer nothing) | after this delivery Undo offers the bury; V1 below replaces that step |
| SPEC-350 R7 (`CardView.undo` names `'answer'`, `'synced'` or nothing, as SPEC-371 changed it) | it also names `'bury'`, `'flag'` and `'change-synced'` |
| SPEC-358 R1 to R3 (native bury and flag) | unchanged: the native clients keep bury and flag with no undo |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-383-undo-reaches-the-reviews-last-bury-or-flag-from-one-slot-through-the-owners-gesture-checked-at-the-write.md` | docs | added |
| `docs/decisions/ADR-397-undo-restores-the-reviews-last-bury-or-flag-through-the-shipped-exempt-undo-checked-at-the-write-by-the-records-kind.md` | docs | added |
| `docs/decisions/ADR-382-undo-reverts-only-the-reviews-own-last-answer-as-an-exempt-write-behind-the-owners-gesture-checked-at-the-write-after-the-review-shows-what-it-changes.md` | docs | an insert-only amendment section appended (D2 and its Consequences) |
| `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md` | docs | an insert-only amendment section appended (D1, for undo only) |
| `docs/decisions/ADR-337-the-owners-own-taps-are-exempt-from-the-never-list-and-every-other-path-stays-bound.md` | docs | an insert-only amendment section appended (its Decision Outcome) |
| `docs/schematics/undo-the-reviews-own-last-answer.md` | docs | section 7 appended, insert-only |
| `docs/schematics/web-study-screens.md` | docs | `:130`, `:131`, `:133` and `:136` follow R10 |
| `docs/red-first/SPEC-383.md` | docs | added |
| `changelog.d/undo-bury-flag-383.md` | docs | added |
| `crates/engine-core/src/undo_answer.rs` | engine-core | `Recorded` gains `kind`, `flag` and `card`; `Kind` added beside it (R3); `judge` unchanged |
| `crates/engine-core/src/undo_change.rs` | engine-core | added: `Mark`, `judge_change` (R5) |
| `crates/engine-core/src/lib.rs` | engine-core | declares `undo_change` |
| `crates/engine-core/src/dispatch.rs` | engine-core | `CARD_MARK_SQL`, `Read::CardMark` and its arm in `query` (R4); `run_undo` reads the kind and the private `run_restore` (R6) |
| `crates/engine-core/tests/undo_change.rs` | engine-core | added (A1 to A16) |
| `crates/engine-core/tests/undo_answer.rs` | engine-core | the helper literals of `Recorded` in `answer_head` and `admitted` gain `..Recorded::default()`; every test keeps its name, input and assertion |
| `crates/engine-core/tests/exempt.rs` | engine-core | the helper literal of `Recorded` in `undo` gains `..Recorded::default()`; every test keeps its name, input and assertion |
| `crates/web-engine/src/study.rs` | web-engine | `LastMark`, `last_mark_for`, `mark_view`, `flag_change` (R2, R9) |
| `crates/web-engine/src/wasm.rs` | web-engine | `LAST_MARK`; `bury`, `flag` and `rate` keep the slot (R1, R2); `undo_offer`, `undo`, `current_card`, `open` and `close` read or clear it (R9) |
| `crates/web-engine/tests/study.rs` | web-engine | A17 to A19 |
| `crates/web-engine/tests/boundary.rs` | web-engine | A20 |
| `web/app/src/lib/engine/protocol.ts` | miniapp | `CardView.undo`'s three new values; `UndoOffer` gains `kind`, and a bury's `returns` or a flag's `flag` |
| `web/app/src/lib/engine/session.ts` | miniapp | `toOffer` maps the kind (R9) |
| `web/app/src/lib/engine/session.test.ts` | miniapp | A27 |
| `web/app/src/lib/study/review.ts` | miniapp | the control's label by kind, the flagged effect's undo, the notices by kind (R10, R13) |
| `web/app/src/lib/study/review.test.ts` | miniapp | A21 to A24, and the shipped bodies section 3 names |
| `web/app/src/lib/study/ReviewScreen.svelte` | miniapp | the dialog by kind (R11) |
| `web/app/src/lib/study/review-screen.test.ts` | miniapp | A25, A26 |
| `web/app/src/lib/study/undo-reach.test.ts` | miniapp | A28 |
| `web/app/messages/en.json` | miniapp | ten keys (R12) |
| `web/app/messages/es.json` | miniapp | ten keys (R12) |
| `web/app/messages/fr.json` | miniapp | ten keys (R12) |
| `web/app/messages/ja.json` | miniapp | ten keys (R12) |
| `web/app/messages/ko.json` | miniapp | ten keys (R12) |
| `web/app/messages/zh-Hans.json` | miniapp | ten keys (R12) |
| `web/app/messages/zh-Hant.json` | miniapp | ten keys (R12) |
| `web/app/tests-engine/engine.spec.ts` | miniapp | the browser engine suite buries, flags and undoes each over OPFS (C1) |
| `formal/tla/UndoOwnAnswer/UndoOwnAnswer.tla` | formal | the bury and the flag, two properties, new covers (section 8) |
| `formal/tla/UndoOwnAnswer/MCUndoOwnAnswer.cfg` | formal | the two properties and each one's `_base` |
| `formal/tla/UndoOwnAnswer/witness/an-undo-restores-only-the-offered-change.cfg` | formal | added |
| `formal/tla/UndoOwnAnswer/witness/a-restore-runs-only-on-a-confirm.cfg` | formal | added |
| `config/formal.json` | config | `budgets.entries` gains `tla/UndoOwnAnswer` between `tla/SyncSnapshotWindow` and `tla/WalletFloor`, its value measured whole (RULED 1086 Q-H, Q-J) |
| `scripts/tests/test_formal_config.py` | scripts | `EXPECTED` gains the same `budgets.entries` row in the same place, so SPEC-295 A1 still pins the committed file whole (RULED 1086 Q-H) |
| `scripts/mutation-rows.d/S38300-S38399.json` | scripts | added (section 9) |
| `scripts/mutation-equivalent.d/deck-streak-web-engine.json` | scripts | changed only if the browser target's mutation verdict shows a `wasm.rs` row of section 9 equivalent, each record with its reason; and the shipped `engine_record` record's `anchor` follows that function's new text, its mutant, reason and evidence kept as they stand |
| `scripts/mutation-equivalent.d/miniapp.json` | scripts | changed only if StrykerJS shows a mutant equivalent, each record with its reason |

## 5. What this does NOT cover

- It adds no native undo. The native clients keep bury and flag as SPEC-358 built them, with no
  undo of any kind; the native `run` still refuses (3,8), and no native file changes (#714).
- It does not undo a bury or a flag that has synced. The control stays disabled and says so; that
  undo needs a write of its own after the engine's undo queue is gone (#714).
- It undoes only the review's last action, one level. Undoing an answer, then the bury before it,
  is not offered; an undo advances the engine's step, so a deeper record would need a stack (#714).
- It offers no undo from the done screen, where the review holds no cell, so burying the last due
  card cannot be undone there (#714).
- It does not undo a bury, a suspend or a flag made anywhere but the review: the deck browser, the
  bot and the native clients keep their own paths (#714).
- It leaves the bot's own habit undo (`commands.rs:657`) as it is (#714).
- It changes none of the other exempt writes, adds no exempt row, and the `run_exempt` export keeps
  its indices 0 to 5 (#714).
- It changes no grade, no schedule and no memory state: an undone bury or flag writes no review
  row (#714).

## 6. Risks

- **The flag's bits are inferred.** R5 reads the user flag as the low three bits of `flags`. If the
  engine keeps the user flag elsewhere, A2 and A10 read it wrong. The build measures how SetFlag
  writes the column before it writes any code, and stops if it is not the low three bits.
- **A shipped row's anchor can be duplicated.** `run_restore` and the mark path of `undo` sit beside
  lines rows `S37109` and `S37113` to `S37116` anchor on. A new line that copies an anchored line
  makes that row's find ambiguous. The build checks that every `S37100-S37199` row on an edited file
  finds exactly once, and rewrites its own new line, never the row.
- **`wasm.rs` is built only by the browser target.** A slot fact the census reads but the browser
  build breaks shows only in CI's browser suite (C1) and its mutation verdict (C2).
- **The census names meet other lines.** `LAST_MARK` and the census's function names may appear in
  comments; the census reads code with comments removed, as the shipped census does, and its plants
  prove it is not blind.
- **The six translated locales are unverified by the build.** A wrong translation shows the learner
  a wrong promise in the dialog. A fluent reader reviews the sixty strings before release (V2).
- **An undo after a flag re-shows the card from its question side.** A successful undo clears the
  kept card, as the undo of an answer does, so the review asks for its card again. The flag's
  confirmation says nothing else changes; that the card is shown again from its front is the
  shipped undo's behaviour, kept.

## 7. The other pull requests

Two pull requests were open at `e7ecf10d`. #748 shares `web/app/messages/*.json` (all seven),
`web/app/src/lib/engine/session.ts`, `session.test.ts`, `protocol.ts`, `crates/web-engine/src/wasm.rs`,
`crates/web-engine/tests/boundary.rs`, `crates/engine-core/src/dispatch.rs` and
`crates/engine-core/src/lib.rs` with this manifest. #749 shares none of them. The build re-measures
each shared path at its cut, and a fact section 1 states whose content changed is a stop, never a
rebase.

## 8. Formal model

**Required.** The surface is an interleaving: the offer is a check and the confirmed write is an
act, and between them the learner, the review's other controls and the web review's sync can each
change the engine's undo queue and the card. The delivery also adds a `thread_local!` item,
`LAST_MARK`, and a new path through the covered `run_undo`.

The model stays `formal/tla/UndoOwnAnswer/`, amended in place, because its covered spans are the
ones this delivery changes.
- **Kept:** both properties, `AnUndoRevertsOnlyTheOfferedAnswer` and `AnUndoRunsOnlyOnAConfirm`,
  their text and their witnesses, `witness/an-undo-reverts-only-the-offered-answer.cfg` and
  `witness/an-undo-runs-only-on-a-confirm.cfg`, unchanged; every `TypeOK` conjunct unchanged, and no
  existing variable's domain widened.
- **New variables:** `mark` (none, or a bury or a flag with its card and step), `markOffers`,
  `markConfirmed` and `restored`, each with its own new `TypeOK` conjunct.
- **New actions:** `Bury(c)` and `Flag(c)` begin a step, set `mark` and clear `record`; `Answer(c)`
  also clears `mark`; `Forget` (`open`, `close`) clears both; `MarkOffer`, `MarkConfirm(o)` and
  `MarkStaleConfirm(o)` mirror the answer's offer and confirmation; a restore at the write judges
  by the existing `Checked` switch. Every other existing action leaves the new variables unchanged.
- **New properties,** each `ramp=report`: `AnUndoRestoresOnlyTheOfferedChange` (whatever is
  restored is the bury or flag a confirmed offer named, on its own card, unsynced, with no step
  begun after it) and `ARestoreRunsOnlyOnAConfirm` (nothing is restored that no confirmation of a
  mark offer preceded).
- **New witnesses,** each named for the property it holds, and each reusing a shipped switch so no
  shipped witness file changes: `witness/an-undo-restores-only-the-offered-change.cfg`
  (`Checked = FALSE`: a restore with no check at the write, the defect the shipped witness ports
  from the undo before SPEC-371) kills `AnUndoRestoresOnlyTheOfferedChange`;
  `witness/a-restore-runs-only-on-a-confirm.cfg` (`OnConfirm = FALSE`: the write on the press, as
  the shipped witness ports it) kills `ARestoreRunsOnlyOnAConfirm`. Each must read VIOLATION before
  the model reads clean: the control runs forward.
- **Covers:** the shipped five (`undo_answer.rs` `judge`, `dispatch.rs` `run_undo`, `wasm.rs`
  `undo`, `undo_offer`, `rate`), re-stamped where their spans moved, plus `undo_change.rs`
  `judge_change`, `dispatch.rs` `run_restore`, and `wasm.rs` `bury`, `flag` and `current_card`.
- **Cites:** #714 and this delivery's issue.
- **Stamping:** every new `digest=` is committed as 64 zeros, the entry is checked, and each `STALE`
  line's current digest is copied in, after the last edit to the entry and to the covered code.

**Lean is not applicable.** `judge_change` is seven comparisons in a fixed order, which A6 to A14
and rows `S38302` to `S38310` hold one operand at a time, as SPEC-371 section 8 decided for `judge`.

A builder may raise the Lean decision, and says why.

## 9. Mutation rows

Each row names a behaviour of R1 to R6, its mutant and the one test that kills it. Rows live in
`scripts/mutation-rows.d/S38300-S38399.json`, table `MUTATIONS`, and each killer is in the row's
own crate.

| row | file | mutant | killer |
|---|---|---|---|
| `S38301-THE-MARK-READ-IS-BY-CARD` | `engine-core` `src/dispatch.rs` | `CARD_MARK_SQL`'s `where id = ?` becomes `where nid = ?` (a literal in a `const`: a hand row) | `undo_change::a_confirmed_undo_of_a_bury_restores_the_card_whole` |
| `S38302-A-GONE-CARD-IS-REFUSED` | `engine-core` `src/undo_change.rs` | `judge_change`'s no-mark arm admits | `undo_change::a_change_whose_card_is_gone_is_refused` |
| `S38303-A-CHANGE-NAMES-ITS-CARD` | `engine-core` `src/undo_change.rs` | the record's card comparison compares the target with itself | `undo_change::a_change_recorded_for_another_card_is_refused` |
| `S38304-A-BURY-IS-STILL-BURIED` | `engine-core` `src/undo_change.rs` | the queue comparison compares the mark's queue with itself | `undo_change::a_bury_no_longer_buried_is_refused` |
| `S38305-A-FLAG-IS-STILL-SET` | `engine-core` `src/undo_change.rs` | the flag comparison compares the recorded flag with itself | `undo_change::a_flag_since_changed_is_refused` |
| `S38306-THE-USER-FLAG-IS-THREE-BITS` | `engine-core` `src/undo_change.rs` | the user flag mask's `7` becomes `3` (a literal in a `const`: a hand row) | `undo_change::a_flag_is_judged_by_the_users_three_bits` |
| `S38307-A-SYNCED-CHANGE-IS-REFUSED` | `engine-core` `src/undo_change.rs` | `mark.usn != -1` compares the mark's `usn` with itself | `undo_change::a_synced_change_is_refused` |
| `S38308-A-LATER-STEP-REFUSES-A-CHANGE` | `engine-core` `src/undo_change.rs` | the `last_step` comparison compares the current step with itself | `undo_change::a_later_step_refuses_a_change` |
| `S38309-ANOTHER-LABEL-REFUSES-A-CHANGE` | `engine-core` `src/undo_change.rs` | the label comparison compares the current label with itself | `undo_change::another_undo_label_refuses_a_change` |
| `S38310-AN-EMPTY-QUEUE-REFUSES-A-CHANGE` | `engine-core` `src/undo_change.rs` | `now.undo.is_empty()` becomes `false` | `undo_change::an_empty_undo_queue_refuses_a_change` |
| `S38311-THE-RESTORE-WAITS-FOR-THE-CHECK` | `engine-core` `src/dispatch.rs` | `run_restore` discards `judge_change`'s refusal and runs (3,8) | `undo_change::an_undo_of_a_bury_after_a_later_change_is_refused_and_changes_nothing` |
| `S38312-AN-ANSWER-IS-KIND-ZERO` | `engine-core` `src/undo_answer.rs` | `Kind`'s `Answer = 0` becomes `Answer = 3` (a literal: a hand row) | `undo_change::an_old_record_decodes_as_an_answer` |
| `S38313-AN-UNKNOWN-KIND-IS-REFUSED` | `engine-core` `src/dispatch.rs` | `run_undo`'s kind decode admits an unknown kind as an answer | `undo_change::a_record_of_an_unknown_kind_is_refused_and_leaves_the_answer` |
| `S38314-A-MARK-TAKES-THE-RESTORE` | `engine-core` `src/dispatch.rs` | `run_undo` sends a bury's record down the answer path | `undo_change::a_confirmed_undo_of_a_bury_restores_the_card_whole` |
| `S38315-A-BURY-RECORDS-ITS-CHANGE` | `web-engine` `src/wasm.rs` | `bury` leaves `LAST_MARK` unset | `boundary::the_review_records_one_change_at_a_time` |
| `S38316-A-FLAG-RECORDS-ITS-CHANGE` | `web-engine` `src/wasm.rs` | `flag` leaves `LAST_MARK` unset | `boundary::the_review_records_one_change_at_a_time` |
| `S38317-A-CHANGE-CLEARS-THE-ANSWER` | `web-engine` `src/wasm.rs` | `bury` leaves `LAST_ANSWER` set | `boundary::the_review_records_one_change_at_a_time` |
| `S38318-AN-ANSWER-CLEARS-THE-CHANGE` | `web-engine` `src/wasm.rs` | `rate` leaves `LAST_MARK` set | `boundary::the_review_records_one_change_at_a_time` |
| `S38319-AN-UNDO-CLEARS-THE-CHANGE` | `web-engine` `src/wasm.rs` | a successful undo of a change leaves `LAST_MARK` set | `boundary::the_review_records_one_change_at_a_time` |
| `S38320-A-MARK-VIEW-NAMES-ITS-KIND` | `web-engine` `src/study.rs` | `mark_view`'s admitted bury answers `"flag"` | `study::a_mark_view_names_its_kind` |
| `S38321-A-REPLACED-FLAG-IS-NAMED` | `web-engine` `src/study.rs` | `flag_change`'s any-other-flag arm answers `added` | `study::a_flag_offer_says_what_the_undo_puts_back` |

Rows 01, 06 and 12 change a literal the tool never mutates, so each is a hand row planted by the
build. Rows 15 to 19 sit in code only the browser target compiles: the boundary census is their
native killer, and their equivalence records, if the browser target's verdict needs any, go in
`scripts/mutation-equivalent.d/deck-streak-web-engine.json`. Rows 04 and 05 are the two arms of
one kind match, so each has a test in which it alone decides (A8, A9). The page's code is mutated
by StrykerJS, which mutates each changed `web/app` production file whole, at a break of 100; a
mutant it shows equivalent is recorded in `scripts/mutation-equivalent.d/miniapp.json`.

The killers of every row are tests this delivery adds, so none exists at the merge-base. CI checks
out the merge ref, selects every row whose id the base's rows lack, and proves each in the merge
ref's tree, where its killer exists, as SPEC-371 section 9 records.

## 10. What only CI or a device proves

| id | criterion | who, and when |
|---|---|---|
| C1 | The browser build of the web engine buries a card and undoes the bury, and flags a card that carried another flag and undoes the flag, each through the offer and the confirmation, over OPFS (`engine.spec.ts`) | the browser engine suite in CI, on the train; `wasm.rs` is built only there |
| C2 | The browser target's mutation verdict for `wasm.rs`'s `bury`, `flag`, `rate`, `undo`, `undo_offer` and `current_card` | CI's mutation job for the web engine, read by row stem and record |
| C3 | StrykerJS's verdict over `review.ts`, `ReviewScreen.svelte`, `session.ts` and `protocol.ts` | CI's `mutation-web` job, on the train |
| C4 | The native harness and its tests are unchanged and green | the native workflow's tests, on the train; the native code is built only there |
| V1 | On a device: bury a card, press Undo, read the card and the state it returns to, confirm, and see the card again; flag a card that carried another flag, press Undo, confirm, and see that flag again | the owner, in the acceptance session (#714) |
| V2 | The sixty strings of the six non-English locales say what R10, R11 and R13 say | a fluent reader of each locale, before release (#714) |
