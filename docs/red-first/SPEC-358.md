# Red-first record: SPEC-358

The SPEC, its schematic, ADR-369 and the changelog fragment were committed first (2594b548). Each
criterion's test was then committed before the code that turns it green, over a stub that compiles
and does nothing, and each red below is quoted from the run at the red commit. This record covers
part a: A1 to A6, A11 to A14, A37 and A40.

```red-first
A1: red at e37d0817: the_review_pairs_are_ordinary_on_the_web panicked at crates/engine-core/tests/table.rs:268:9: assertion `left == right` failed: left: NotAllowed, right: Admit
A1: green at ef18b5bc
A2: red at ef18b5bc: panicked at crates/engine-core/tests/parity.rs:60:5: crates/ffi/src/allow_list.rs: in the adapter's table only: []; in the core's column only: [(5, 4, "CardsService.SetFlag"), (13, 14, "SchedulerService.BuryOrSuspendCards")]
A2: green at 8a3d6859
A3: red at d0b3e470: not run on the box; red by construction (the census input: the five items defined only at `crates/web-engine/src/study.rs`, the git grep in the dot-positive form, quoted below), and row S35808 read KILLED in CI: run 37830797611, job `mutation-rows`: S35808-RED-LIVES-ONCE-IN-THE-CORE KILLED, its killer passed without the mutant and failed with it
A3: green at e4c5ebab
A4: not red: the rules move unchanged and their behaviour was pinned on the web (SPEC-350 A4, A5); the moved tests guard the move
A5: red at 2cfe4b92: panicked at crates/ffi/tests/review_actions.rs:168:5: after the bury the queue moves on to the next card, left: 1000007 right: 1000008
A6: red at 2cfe4b92: panicked at crates/ffi/tests/review_actions.rs:193:5: a card with no flag turns red in the engine, left: (1000007, 0) right: (1000007, 1)
A5: green at 63fd21f3
A6: green at 63fd21f3
A11: red at d6a6bf96: run 37830797813, job `apple / harness`, its step "the bury and flag tests, Debug, on the iPhone and then the iPad", on the iPhone and on the iPad: test_a11_bury_moves_on_and_flag_toggles_red: ReviewActionsFlowTests.swift:45: XCTAssertEqual failed: ("[["(not shown)", "shown"], ["shown", "(not shown)"], ["(not shown)"], ["(not shown)"]]") is not equal to ("[["shown", "(not shown)"], ["shown", "Flag"], ["Flagged red"], ["Flag"]]") - A11: Bury shows the image card in the text card's place; the text card's Flag reads "Flagged red" after one press and "Flag" again after the next
A12: not red: SPEC-348 R16 holds the state in the model; this pins it for the layout
A13: red at d6a6bf96: run 37830797611, job `hygiene`: test_every_swift_file_keeps_its_role_its_doors_and_its_budget: AssertionError: Lists differ: ['ios/App/Sources/ReviewPlayback.swift: the register: not listed'] != []: examined 61 files, 198 doors, 18 decisions, 3 admissions
A14: red at d6a6bf96: run 37830797611, job `hygiene`: test_the_review_actions_step_runs_on_the_iphone_and_then_the_ipad: AssertionError: Lists differ: ['harness: the report lacks cases_in("review-actions")', 'harness: the report lacks ("Review actions", review_actions_cases)', 'harness: the report lacks minutes("review-actions-seconds")'] != []
A37: not red: an unchanged constraint (R27); the census already holds it, and this pins it for part a's files
A40: red at d6a6bf96: run 37830797813, job `apple / harness-wire`: ResponseDecodingTests.swift:165: XCTAssertEqual failed: ("[0, 0]") is not equal to ("[1, 0]") - A40: the red card carries Card.flags (17) as 1, and the card with none carries 0
A11: green at 92556acb
A13: green at 92556acb
A14: green at 92556acb
A40: green at 92556acb
```

A11, A13, A14 and A40 are decided in CI alone, and their greens at 92556acb are unread here: CI reads
them at this pull request's second push, A11 in the `apple / harness` job's own step on the iPhone
and on the iPad, A13 and A14 in `hygiene`, and A40 in `apple / harness-wire`.

A3's census was not run on the box. Its input at d0b3e470, read with
`git grep -n -E 'pub (const RED|fn toggled_red|const BURY_USER|struct BuryOf|fn bury_of)'` over the
tree with `.` as its positive pathspec: `crates/web-engine/src/study.rs:264` `pub const RED`,
`:269` `toggled_red`, `:274` `BURY_USER`, `:278` `BuryOf` and `:289` `bury_of`, and none under
`crates/engine-core/src`. Its green was read in CI at d6a6bf96 (run 37830797611, job `hygiene`):
each of the five reads `1 definition(s): crates/engine-core/src/review.rs` over 717 Rust files.

Test files changed between a criterion's red and its green, each insert-only or a move:

- ef18b5bc, A1's green and A2's red, grew `crates/engine-core/tests/review_pairs.rs`'s `NATIVE`
  list by the two review pairs, insert-only: that test pins the native column, so admitting the
  pairs meant growing it.
- 8a3d6859, A2's green, grew `crates/ffi/tests/review_pairs.rs`'s `EXPECTED` list by the same two
  pairs, insert-only.
- e4c5ebab, A3's green, moved the two flag and bury tests from `crates/web-engine/tests/study.rs`
  to `crates/engine-core/tests/review.rs`, names kept, and re-pointed SPEC-350's two rows and two
  fence lines that named them.

`crates/engine-core/tests/review.rs`'s `the_bury_and_flag_requests_are_the_engines_bytes`
(bae1dfbf) is mutation coverage, written after the request encoders: it has no criterion and no
fence line, and it is not red-first evidence.

CI read A11, A13, A14 and A40 green by name at 0412f956: A11 in the `apple / harness` job's own step on the iPhone and on the iPad, A13 and A14 in `hygiene`, and A40 in `apple / harness-wire`.

From b5ef7dbe, A3's census drops the parked Rust files by name before any file is opened and prints how many it dropped (the drill surface is parked, #158); its count is read in CI's `hygiene` at this pull request's next push. That commit changes A3's test after A3's green, and it is a narrowing, recorded in the pull request's weakening table.
