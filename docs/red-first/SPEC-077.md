# Red-first record: SPEC-077

Each criterion's test was committed before the rule it tests, red on a compiling stub, in four
steps: the curriculum goldens of A1 to A6 and A11 at a26ed3ad, made green by 58c946eb; A8 and A10,
with migration 007701 and the store's stubs, at 9a3da777, made green by 782a2c1a; A7 at f015827a,
made green by 683d1a89; and the law block's A12 to A15 and the milestone's A21 at 6fd5308f, made
green by 4b713129. For this record every line was replayed from an export of its commit's tree,
running the criterion's own fence command. Each red line quotes where the replay's run panicked
and what it printed; each green line's run printed `1 passed`.

```red-first
A1: red at a26ed3ad: panicked at crates/ingest/tests/progress_memory_state.rs:49:23: the state of Some("{\"s\": 4.5, \"d\": NaN, \"decay\": Infinity, \"lrt\": -Infinity}"): ours None, theirs Some(MemoryState { stability: 4.5, difficulty: 0.0, decay: 0.2, desired_retention: None, last_review_sec: None })
A1: green at 58c946eb
A2: red at a26ed3ad: panicked at crates/curriculum/tests/progress_goldens.rs:77:9: the mastery of {"card":{"ctype":2,"ivl":21,"queue":-1},"mature_ivl":21,"now_sec":1700000000}: ours -1, theirs 0
A2: green at 58c946eb
A3: red at a26ed3ad: panicked at crates/curriculum/tests/progress_goldens.rs:91:9: assertion `left == right` failed: the unit of "Alpha"; left: Some(4294967295), right: None
A3: green at 58c946eb
A4: red at a26ed3ad: panicked at crates/curriculum/tests/progress_goldens.rs:156:9: assertion `left == right` failed: the courses of the first case; left: 0, right: 1
A4: green at 58c946eb
A5: red at a26ed3ad: panicked at crates/curriculum/tests/progress_goldens.rs:229:9: constants.XP_BONUS_BAND_UP: ours 0, theirs 500
A5: green at 58c946eb
A6: red at a26ed3ad: panicked at crates/curriculum/tests/progress_unit_bands.rs:41:5: assertion `left == right` failed; left: Some("C2"), right: Some("A1")
A6: green at 58c946eb
A7: red at f015827a: panicked at crates/coordination/tests/progress_band_up.rs:429:9: assertion `left == right` failed: case 3 (band_up): each band-up is offered to the router; left: [], right: [("band_up", "bandup:al:a2")]
A7: green at 683d1a89
A8: red at 9a3da777: panicked at crates/coordination/tests/progress_band_up.rs:101:5: assertion `left == right` failed: the first sighting records the current band as a silent baseline, owing nothing; left: [], right: [("be", "B1", 20000, 1, true)]
A8: green at 782a2c1a
A10: red at 9a3da777: panicked at crates/curriculum/tests/progress_store.rs:82:5: assertion `left == right` failed: the export carries both courses' progress; left: [], right: [["al", "A2"], ["be", "A1"]]
A10: green at 782a2c1a
A11: red at a26ed3ad: panicked at crates/curriculum/tests/law_goldens.rs:19:9: the pillar at 0: ours -1, theirs 100
A11: green at 58c946eb
A12: red at 6fd5308f: panicked at crates/coordination/tests/law_block.rs:175:9: assertion `left == right` failed: case 0: the level of the lifetime law XP; left: Some(0), right: Some(1)
A12: green at 4b713129
A13: red at 6fd5308f: panicked at crates/coordination/tests/law_block.rs:239:5: assertion `left == right` failed: no law streak, law XP or leech: the block is omitted, dues and language XP or not; left: Some(LawLines { lines: [], level_shown: false }), right: None
A13: green at 4b713129
A14: red at 6fd5308f: panicked at crates/coordination/tests/law_block.rs:281:5: assertion `left == right` failed: before the first recompute the dues are pending; left: Some(0), right: None
A14: green at 4b713129
A15: red at 6fd5308f: panicked at crates/coordination/tests/law_block.rs:310:5: assertion `left == right` failed: the leeches are pending, never 0; left: Some(0), right: None
A15: green at 4b713129
A21: red at 6fd5308f: panicked at crates/coordination/tests/progress_milestone.rs:67:5: assertion `left == right` failed: the configured courses' stored mature cards, summed; an unconfigured course is not; left: None, right: Some(115)
A21: green at 4b713129
A9: red at 2943b041: panicked at crates/daemon/tests/progress_live_band.rs:117:5: assertion `left == right` failed: the stored current band of the configured course the subject names
A9: green at 4d29499f
A16: red at 95f3c98e: panicked at crates/api/tests/progress_routes.rs:228:5: assertion `left == right` failed: the configured course's stored progress, whole; the stale row is absent
A16: green at bcde9feb
A17: red at 95563f6a: panicked at crates/api/tests/law_routes.rs:190:5: assertion `left == right` failed: the law block over an empty database, whole
A17: green at 8a878792
A18: red at 25207b0c: panicked at crates/bot/tests/progress_commands.rs:77:5: assertion `left == right` failed
A18: green at 42666e15
A22: red at e1fe64f7: panicked at crates/daemon/src/wiring.rs:750:9: assertion `left == right` failed: the progress step is registered in phase 4, right after the streaks step
A22: green at 23641fae
```

## Disclosures

- **How A1, A2, A5 and A11 fail.** Their tests compare each case with its golden and panic with
  the case, ours and theirs on the first that differs, rather than through `assert_eq!`. A1 fails
  at its mismatch arm (`(a, b) => panic!(...)` at progress_memory_state.rs:49): the stub parses
  no memory state where the golden has one. A2 fails at its `assert!` that the two masteries are
  near, A5 at its `assert!` that the constant is the same, and A11 at its `assert!` that the two
  pillars agree. Each red is the behaviour's mismatch on a compiling stub, quoted as the run
  printed it.
- **A4's quoted case is shortened.** The run prints the whole first case, its cards and its
  courses, before `left: 0, right: 1`; the line above names it "the first case".
- **A changed criterion, A3 and A4 (the seat's ruling 12).** 782a2c1a changed what A3 and A4
  decide: a unit beyond 32 bits reads as no unit, as the predecessor counts it, pinned by one new
  golden case, and changed `parse_unit` to match in the same commit. Its red is recorded here and
  not as a second fence line. Replayed on 782a2c1a's tree with 9a3da777's `parse_unit` body put
  back, both tests fail, each with exit 101. A3 panicked at progress_goldens.rs:96:5,
  `assertion left == right failed: a unit beyond u32 is no unit`, left `Some(0)`, right `None`.
  A4 panicked at progress_goldens.rs:168:13, the comparison of a course's total cards, left `3`,
  right `1`: the case's card with a unit beyond 32 bits was counted. On 782a2c1a's own tree both
  pass. 734a85a0 edits only `crates/coordination/tests/relight_order.rs`'s census of statics and
  changes no criterion.
- **The mutation-coverage tests are not criteria.**
  `progress_band_up::the_progress_step_runs_for_the_current_day_only` and
  `progress_store::a_band_up_is_marked_once` were green when committed; each pins a rule that
  already held, and each is the killer of a row the SPEC's T18 names (S07715 and S07722), proved
  by its mutant and not recorded here as red-first evidence.
- **A5's comparison was changed and the change went undisclosed.** 58c946eb changed the A5 test,
  `the_curriculum_constants_equal_the_predecessors`, from `a == b` at its red commit a26ed3ad to
  `near(a, b)`, a 1e-9 tolerance, because clippy's `float_cmp` refuses `a == b`. That loosened the
  criterion and the record did not say so. 21f7465b27acce0010850ae3854c1a1fa2117109 restores exact
  equality with `a.to_bits() == b.to_bits()`; A5's red line at a26ed3ad is unchanged.

## Addendum, 2026-10-03: the Mini App part

The second part of the second pull request delivers A19 and A20, each committed before the rule it
tests, red on a compiling stub: A19's course ladder at 6ab922f1, a stub that draws no band cell, made
green by edc6c555; A20's law block at d7bd6425, a stub that renders each count `?? 0`, made green by
f25690b7. Each red line was replayed at fe1ff5dd with the red commit's three files put back
(`CourseLadder.svelte`, `CourseLadder.test.ts` and `progress.ts`; `LawBlock.svelte`,
`LawBlock.test.ts` and `law.ts`), running the criterion's own fence command: each exited 1 with
`Tests  1 failed (1)`, at `CourseLadder.test.ts:49:19` and `LawBlock.test.ts:44:19`. Each green
line's run printed `1 passed`, at fe1ff5dd, whose `web/app/src/lib/progress/` and
`web/app/src/lib/law/` are byte-equal to the green commits'.

```red-first
A19: red at 6ab922f1: AssertionError: examined 0 band cells: the population is empty, so nothing was judged: expected 0 to be greater than 0
A19: green at edc6c555
A20: red at d7bd6425: AssertionError: examined 0 pending counts: the population is empty, so nothing was judged: expected 0 to be greater than 0
A20: green at f25690b7
```

## Addendum, 2026-10-03: the test edits inside A16's, A17's and A18's green commits

Each green commit below also changed test files beside its criterion's test. None of them changed
that test: the bodies of `the_progress_route_answers_only_the_owner` (A16),
`the_law_route_answers_only_the_owner` (A17) and `progress_shows_each_course_band_and_mastery`
(A18) are byte-equal at their red and green commits. What else each changed:

- **bcde9feb (A16's green).** `crates/api/tests/progress_routes.rs` moves the app's builder into
  `app_with(scratch, open)`, which marks the database opened only when `open` is true; `app` calls
  it with `true`, so every existing test builds the same app as before. Its one removed line,
  `readiness.database_opened(db.clone());`, is that move. It adds
  `the_progress_route_names_why_it_cannot_answer`, mutation coverage beside A16. It adds
  `crates/coordination/tests/progress_view.rs` whole, the tests of the progress view (T28) added
  with the view they test; neither is red-first evidence of a criterion.
- **8a878792 (A17's green).** `crates/api/tests/law_routes.rs` makes the same `app_with` move, with
  the same one removed line, and adds `seed_law`,
  `the_law_route_answers_the_stored_block_with_its_line_keys` and
  `the_law_route_names_why_it_cannot_answer`, mutation coverage beside A17.
- **42666e15 (A18's green).** `crates/bot/tests/commands.rs`: the menu's test lists `progress`
  beside the eleven commands it listed and names twelve; the rendered messages gain `progress`,
  `progress-none` and `progress-failed`, with the new goldens `progress-none.msg.json` and
  `progress-failed.msg.json`; the `help` and `start` goldens each gain the `/progress` line and keep
  every line they had, in order. `crates/bot/tests/progress_commands.rs` adds three tests beside
  A18's. `crates/daemon/tests/role_bot.rs` adds `start_role_with`, which passes further settings to
  the role, creates its state directory with `create_dir_all` in place of `create_dir`, so a state
  directory the progress test has already seeded is kept, and adds
  `the_bot_role_answers_progress_from_the_courses_its_settings_name`. No assertion narrowed.

After the last green, the reader tests' positive controls in `law.test.ts` and `progress.test.ts`
assert a value the reader returns where they asserted only that it was not null, and
`the_view_is_empty_before_the_first_recompute` asserts that two courses are configured beside its
empty view. Each tightens a test that is not a criterion's.

## Addendum, 2026-10-03: the test edits inside A19's and A20's green commits

- **edc6c555 (A19's green).** The commit changed A19's own test body: the meter's `value` string
  assertion became the numbers `[min, max, value]` plus the `aria-label` "<band> mastery". The
  green body also fails on the red stub, at `:49:19`, so the recorded red line stands. The commit
  added the test 'names the course as its region, and says so when no unit is current yet' and
  `progress.test.ts` as a whole file.
- **f25690b7 (A20's green).** A20's body is byte-equal. The commit added the `LawTiersView` import,
  five tests beside A20, and `law.test.ts` as a whole file. No assertion narrowed.
