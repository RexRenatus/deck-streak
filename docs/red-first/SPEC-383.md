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
