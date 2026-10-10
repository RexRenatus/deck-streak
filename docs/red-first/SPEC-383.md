# Red-first record: SPEC-383

The SPEC, ADR-397 with its three amendment sections and the two schematics were committed first
(91165025), and the censuses alone after them. Each red below is quoted from the run of that
criterion's fence line at its red commit, and each green is measured at the commit named.

## The fence, line by line

Each of the 28 lines of SPEC-383 section 3's fence resolves to one test, named by its exact name or
its `-t` filter.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/engine-core/tests/undo_change.rs` `a_confirmed_undo_of_a_bury_restores_the_card_whole` | added |
| 2 | A2 | `crates/engine-core/tests/undo_change.rs` `a_confirmed_undo_of_a_flag_puts_back_the_flag_the_card_had` | added |
| 3 | A3 | `crates/engine-core/tests/undo_change.rs` `an_undo_of_a_bury_after_an_answer_leaves_the_answer` | added |
| 4 | A4 | `crates/engine-core/tests/undo_change.rs` `an_undo_of_a_bury_after_a_later_change_is_refused_and_changes_nothing` | added |
| 5 | A5 | `crates/engine-core/tests/undo_change.rs` `an_undo_of_a_change_aimed_at_another_card_is_refused` | added |
| 6 | A6 | `crates/engine-core/tests/undo_change.rs` `a_change_whose_card_is_gone_is_refused` | added |
| 7 | A7 | `crates/engine-core/tests/undo_change.rs` `a_change_recorded_for_another_card_is_refused` | added |
| 8 | A8 | `crates/engine-core/tests/undo_change.rs` `a_bury_no_longer_buried_is_refused` | added |
| 9 | A9 | `crates/engine-core/tests/undo_change.rs` `a_flag_since_changed_is_refused` | added |
| 10 | A10 | `crates/engine-core/tests/undo_change.rs` `a_flag_is_judged_by_the_users_three_bits` | added |
| 11 | A11 | `crates/engine-core/tests/undo_change.rs` `a_synced_change_is_refused` | added |
| 12 | A12 | `crates/engine-core/tests/undo_change.rs` `an_empty_undo_queue_refuses_a_change` | added |
| 13 | A13 | `crates/engine-core/tests/undo_change.rs` `a_later_step_refuses_a_change` | added |
| 14 | A14 | `crates/engine-core/tests/undo_change.rs` `another_undo_label_refuses_a_change` | added |
| 15 | A15 | `crates/engine-core/tests/undo_change.rs` `an_old_record_decodes_as_an_answer` | added |
| 16 | A16 | `crates/engine-core/tests/undo_change.rs` `a_record_of_an_unknown_kind_is_refused_and_leaves_the_answer` | added |
| 17 | A17 | `crates/web-engine/tests/study.rs` `a_mark_view_names_its_kind` | added |
| 18 | A18 | `crates/web-engine/tests/study.rs` `a_mark_record_matches_only_its_card_and_step` | added |
| 19 | A19 | `crates/web-engine/tests/study.rs` `a_flag_offer_says_what_the_undo_puts_back` | added |
| 20 | A20 | `crates/web-engine/tests/boundary.rs` `the_review_records_one_change_at_a_time` | added |
| 21 | A21 | `web/app/src/lib/study/review.test.ts` "the undo control names the change it would undo" | added |
| 22 | A22 | `web/app/src/lib/study/review.test.ts` "after a flag the undo control offers the flag" | added |
| 23 | A23 | `web/app/src/lib/study/review.test.ts` "a pressed undo of a bury or a flag asks before it writes" | added |
| 24 | A24 | `web/app/src/lib/study/review.test.ts` "a refused undo of a change reads its own notice" | added |
| 25 | A25 | `web/app/src/lib/study/review-screen.test.ts` "the confirmation of a bury names the card and the state it returns to" | added |
| 26 | A26 | `web/app/src/lib/study/review-screen.test.ts` "the confirmation of a flag says what the undo puts back" | added |
| 27 | A27 | `web/app/src/lib/engine/session.test.ts` "an offer of a bury or a flag reads its kind" | added |
| 28 | A28 | `web/app/src/lib/study/undo-reach.test.ts` "every locale holds the bury and flag undo messages" | added |

## The reds and greens

Each line's command is the criterion's line in SPEC-383 section 3's fence, run at the commit named.

```red-first
A20: red at 46bcfc23: assertion `left == right` failed: left holds 10 problems, the first "bury records the bury it made as the slot's one change, after reading the card's mark (SPEC-383 R1, R2), and its body lacks `Read::CardMark(`", right: []
A28: red at 46bcfc23: messages/en.json lacks a bury or flag undo message: expected [ 'study_undo_bury', …(9) ] to deeply equal []
A1: red at 403af6a9: assertion `left == right` failed: left (Err(NotTheTarget { write: Undo, target: Card(B) }), CardMark [-3, 0, -1]), right (Ok(()), the rows before the bury): the answer path refuses and the card stays buried
A2: red at 403af6a9: assertion `left == right` failed: left (Err(NotTheTarget { write: Undo, target: Card(B) }), CardMark [0, 1, -1]), right (Ok(()), CardMark [0, 4, -1]): refused, and the flag stays red
A3: red at 403af6a9: assertion `left == right` failed: left (Err(NotTheTarget { write: Undo, target: Card(B) }), B's CardMark [-3, 0, -1], A's rows, A's review row), right (Ok(()), B's rows before the bury, A's rows, A's review row): refused, and B stays buried
A6: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(())), right (Err(Gone), Ok(())): the stub admits
A7: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(())), right (Err(NotTheCard), Ok(())): the stub admits
A8: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(())), right (Err(Changed), Ok(())): the stub admits
A9: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(())), right (Err(Changed), Ok(())): the stub admits
A10: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(()), Ok(())), right (Ok(()), Ok(()), Err(Changed)): the stub admits flags 12 against a recorded none
A11: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(())), right (Err(Synced), Ok(())): the stub admits
A12: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(())), right (Err(Gone), Ok(())): the stub admits
A13: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(())), right (Err(Changed), Ok(())): the stub admits
A14: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Ok(())), right (Err(Changed), Ok(())): the stub admits
A15: not red: it pins the meaning of the shipped two-field bytes, which the new fields already keep (a record without tags 3 to 5 decodes as kind 0, an answer, with flag 0 and card 0); row S38312 holds it
A16: red at 403af6a9: assertion `left == right` failed: left (Ok(()), Some(0), true), right (Err(NotTheTarget { write: Undo, target: Card(A) }), Some(1), false): the kind is not read, the answer path admits and the answer is undone
A1: green at 676a3189
A2: green at 676a3189
A3: green at 676a3189
A16: green at 676a3189
A4: red at 676a3189: assertion `left == right` failed: left (Ok(()), B's CardMark [-3, 0, -1], C's CardSnapshot reps 0, C's review row Null), right (Err(NotTheTarget { write: Undo, target: Card(B) }), B's rows, C's answered rows, C's review row): the unchecked restore reverts the front, and C's answer is gone
A5: red at 676a3189: assertion `left == right` failed: left (Ok(()), B's CardMark [0, 0, -1], C's rows), right (Err(NotTheTarget { write: Undo, target: Card(C) }), B's buried rows, C's rows): the unchecked restore runs, and B is no longer buried
A4: green at 9ac5d4af
A5: green at 9ac5d4af
A6: green at 9ac5d4af
A7: green at 9ac5d4af
A8: green at 9ac5d4af
A9: green at 9ac5d4af
A10: green at 9ac5d4af
A11: green at 9ac5d4af
A12: green at 9ac5d4af
A13: green at 9ac5d4af
A14: green at 9ac5d4af
A17: red at 6aca7cf4: assertion `left == right` failed: Some((Bury(Review), Ok(()))) left: None right: Some("bury"): the stub answers nothing
A18: red at 6aca7cf4: assertion `left == right` failed: left: Err(NotUndoable(Changed)) right: Ok(LastMark { card: 42, step: 7, change: Bury(New), .. }): the stub answers nothing
A19: red at 6aca7cf4: assertion `left == right` failed: from flag 1 to flag 0 left: "added" right: "removed": the stub answers added
```
