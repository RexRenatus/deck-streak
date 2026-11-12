# Red-first record: SPEC-409

The SPEC, ADR-423 and the schematic's section were committed first (3c8a74e6), before any test
or code. The two censuses, A13 and A14, were then committed alone (a7c27a80), and the criteria's
tests after them, each before the code that turns it green. Each red below is quoted from the run
at the red commit.

## The fence, line by line

Each of the 17 lines of SPEC-409 section 3's fence runs the test it names, in `web/app` by the pinned tool route.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `web/app/src/lib/study/update-now.test.ts` "a late review raises the offer below the controls" | added (step 3) |
| 2 | A2 | `web/app/src/lib/study/update-now.test.ts` "an on-time review and an unanswered late card raise no offer" | added (step 3) |
| 3 | A3 | `web/app/src/lib/study/update-now.test.ts` "not now hides the offer for the screen and syncs nothing" | added (step 3) |
| 4 | A4 | `web/app/src/lib/study/update-now.test.ts` "one tap syncs once and shows the result" | added (step 3) |
| 5 | A5 | `web/app/src/lib/study/update-now.test.ts` "an offline answer says so and lets the learner retry" | added (step 3) |
| 6 | A6 | `web/app/src/lib/study/update-now.test.ts` "a sync with no sign-in links to the sync page" | added (step 3) |
| 7 | A7 | `web/app/src/lib/study/update-now.test.ts` "a full sync required sends the learner to the sync page" | added (step 3) |
| 8 | A8 | `web/app/src/lib/study/update-now.test.ts` "a refused sync says the engine stopped and lets the learner retry" | added (step 3) |
| 9 | A9 | `web/app/src/lib/study/update-now.test.ts` "the review goes on while a sync is pending" | added (step 3) |
| 10 | A10 | `web/app/src/lib/study/update-now.test.ts` "the offer is reachable by touch keyboard and screen reader" | added (step 3) |
| 11 | A11 | `web/app/src/lib/study/update-now.test.ts` "the review route gives the screen the engine sync" | added (step 3) |
| 12 | A12 | `web/app/src/lib/study/update-now.test.ts` "the review counts only settled late answers" | added (step 3) |
| 13 | A13 | `web/app/src/lib/study/update-now.test.ts` "every locale holds the offer words" | added (step 2) |
| 14 | A14 | `web/app/src/lib/study/update-now.test.ts` "the tap is the one new sync call in the study code" | added (step 2) |
| 15 | A15 | `web/app/src/lib/study/engine.test.ts` "a session syncs at its start and end" | shipped, kept |
| 16 | A16 | `crates/coordination/tests/job_table.rs` `the_job_table_holds_sync_to_one_daily_slot_claimed_per_study_day` | shipped, kept |
| 17 | A16 | `crates/coordination/tests/job_table.rs` `the_daily_sync_slot_keeps_off_the_predecessors_ticks_and_every_other_slot` | shipped, kept |

## Disclosure

Commit 64c98a3 changed one assertion of A1 between its red and its green: the test first read the
focus before the answer and asserted it had not moved, and now asserts R8's stronger invariant,
that focus stays on the screen's own region and never on the offer. The change strengthens the
check (it names where focus must be), and SPEC-409's A1 row carries R8's wording.

## The reds and greens

Each line's command is the criterion's line in SPEC-409 section 3's fence, run at the commit named.

```red-first
A1: red at a7c27a80: TestingLibraryElementError: Unable to find an element with the text: Your reviews stay on this device until they sync. You can sync them now. (update-now.test.ts:339)
A2: red at a7c27a80: AssertionError: expected null not to be null: the late card answered raises no offer (update-now.test.ts:364)
A3: red at a7c27a80: AssertionError: expected null not to be null: no offer is raised to dismiss (update-now.test.ts:372)
A4: red at a7c27a80: TestingLibraryElementError: Unable to find an accessible element with the role "button" and name "Sync now" (update-now.test.ts:326)
A5: red at a7c27a80: TestingLibraryElementError: Unable to find an accessible element with the role "button" and name "Sync now" (update-now.test.ts:326)
A6: red at a7c27a80: TestingLibraryElementError: Unable to find an accessible element with the role "button" and name "Sync now" (update-now.test.ts:326)
A7: red at a7c27a80: TestingLibraryElementError: Unable to find an accessible element with the role "button" and name "Sync now" (update-now.test.ts:326)
A8: red at a7c27a80: TestingLibraryElementError: Unable to find an accessible element with the role "button" and name "Sync now" (update-now.test.ts:326)
A9: red at a7c27a80: TestingLibraryElementError: Unable to find an accessible element with the role "button" and name "Sync now" (update-now.test.ts:326)
A10: red at a7c27a80: AssertionError: expected 1 to be 2: the page holds one status region, not the offer's too (update-now.test.ts:524)
A11: red at a7c27a80: AssertionError: expected null not to be null: the route's screen raises no offer (update-now.test.ts:552)
A12: red at a7c27a80: AssertionError: expected undefined to be +0: the machine has no lateReviews count (update-now.test.ts:577)
A13: red at 3c8a74e6: AssertionError: expected { en: [ …(4) ], es: [ …(4) ], …(5) } to deeply equal { Object (en, es, ...) }: en, es, fr, ja, ko, zh-Hans and zh-Hant each read "lacks study_update_prompt" (update-now.test.ts:76)
A14: red at 3c8a74e6: AssertionError: expected { 'src/lib/study/engine.ts': 1 } to deeply equal { 'src/lib/study/engine.ts': 1, …(1) }: the walk finds engine.ts alone (update-now.test.ts:163)
A15: not red: the base holds a session's two syncs and this delivery changes no line of engine.ts; it guards them
A16: not red: the base holds the daily sync slot and this delivery changes no server file; it guards them
A1: green at 64c98a3d
A2: green at 64c98a3d
A3: green at 64c98a3d
A4: green at 64c98a3d
A5: green at 64c98a3d
A6: green at 64c98a3d
A7: green at 64c98a3d
A8: green at 64c98a3d
A9: green at 64c98a3d
A10: green at 64c98a3d
A11: green at 64c98a3d
A12: green at 64c98a3d
A13: green at 64c98a3d
A14: green at 64c98a3d
```
