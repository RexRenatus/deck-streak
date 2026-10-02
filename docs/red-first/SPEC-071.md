# Red-first record: SPEC-071

The order of work: the SPEC promoted and ADR-071 and ADR-087 accepted (c726ea8); the goldens
(ad6bb6a); the kernel's, ingest's and analytics' acceptance tests beside stubs, with the migrations,
the example courses file and the registrations of the two new tables (97948b9); their
implementation (83f3a78); the fold's tests beside a stubbed fold and step (08c5515); the fold
(ee98ad9); the surfaces' tests and the sync cycle's call of the fold, beside routes that answered
anyone and rendered an absent card state and retention as numbers, a /score that read nothing, a
breakdown that drew an absent retention as 0, and a cycle that held its fold without running it
(5c8ab5d); their implementation (2dd74ce); and SPEC-071's mutation rows, each proved (915920a).

The goldens were generated from the predecessor's own functions at `27ee2bc`, with a scratch
`--registry` holding only `registry/spec_071.py` and a scratch `--out`, under
`PYTHONDONTWRITEBYTECODE=1`. The registry module's sha256 read the same before and after the run,
and the predecessor's checkout was left as it was: no change and no bytecode file added.

Each criterion was run at its red commit with the SPEC's own fenced command, selecting one test, and
failed by assertion for its own criterion, not by a compile error, a missing fixture or an empty
selection. The stubs compiled and returned nothing: empty metrics, rows and snapshots, zero
constants and pillars, no course for any deck, no digest, a port that declared no table, and a fold
that evaluated and settled no day.

```red-first
A1: red at 97948b9: assertion `left == right` failed: answered of the tie case; left: 0
A1: green at 83f3a78
A2: red at 97948b9: assertion `left == right` failed: the snapshot of the due-boundary case; left: every count 0
A2: green at 83f3a78
A3: red at 97948b9: assertion `left == right` failed: the rows of the prefix-only-deck case; left: 0, right: 2
A3: green at 83f3a78
A4: red at 97948b9: assertion `left == right` failed: "Qaa"; left: None
A4: green at 83f3a78
A5: red at 97948b9: a new row is a change
A5: green at 83f3a78
A6: red at 97948b9: assertion failed: roll(&db, DAY, &reviews(DAY, 4), 1_000).await
A6: green at 83f3a78
A7: red at 97948b9: assertion `left == right` failed: constants.ANSWER_TIME_CAP_SECONDS; left: Number(0.0)
A7: green at 83f3a78
A8: red at 97948b9: assertion `left == right` failed: mastery: the port's 0 against the predecessor's 85.0
A8: green at 83f3a78
A9: red at 97948b9: assertion `left == right` failed: consistency of {"reviews":0,"streak_days":1}: the port's 0 against the predecessor's 1.2
A9: green at 83f3a78
A10: red at 97948b9: assertion `left == right` failed: the band of {"score":-1}; left: Array [String(""), String("")]
A10: green at 83f3a78
A11: red at 97948b9: assertion `left == right` failed: the day number of the first rollover case; left: Number(0)
A11: green at 83f3a78
A12: red at 97948b9: assertion `left == right` failed: the course of "Qaa"; left: None
A12: green at 83f3a78
A13: red at 97948b9: assertion `left == right` failed: the example's courses; left: []
A13: green at 83f3a78
A14: red at 97948b9: loaded courses have a digest
A14: green at 83f3a78
A15: red at 08c5515: two codes for one deck: ()
A15: green at ee98ad9
A16: red at 08c5515: assertion `left == right` failed: the first recompute settles the closing day; left: [], right: [StudyDay(20001)]
A16: green at ee98ad9
A17: red at 08c5515: assertion `left == right` failed; left: [], right: [StudyDay(20001)]
A17: green at ee98ad9
A18: red at 08c5515: assertion `left == right` failed; left: [], right: [StudyDay(20000)]
A18: green at ee98ad9
A19: red at 08c5515: assertion `left == right` failed: every past study day of the window; left: [], right: [StudyDay(19990), StudyDay(19995), StudyDay(19997), StudyDay(19999)]
A19: green at ee98ad9
A20: red at 08c5515: assertion `left == right` failed: the steps run in the declared phase order; left: the registration order, right: PHASES
A20: green at ee98ad9
A21: red at 08c5515: assertion `left == right` failed; left: [], right: [StudyDay(20000), StudyDay(20001)]
A21: green at ee98ad9
A22: red at 5c8ab5d: assertion `left == right` failed: the card state of a day never recorded; left: Object {every count 0}, right: Null
A22: green at 2dd74ce
A23: red at 5c8ab5d: assertion `left == right` failed: /api/analytics/days with no session; left: 200, right: 401
A23: green at 2dd74ce
A24: red at 5c8ab5d: assertion `left == right` failed: score-no-retention; left: the answer that today has no score, right: the day's numbers
A24: green at 2dd74ce
A25: red at 5c8ab5d: AssertionError: expected 'Retention 0' to be 'Retention No review answered yet'
A25: green at 2dd74ce
A26: red at 97948b9: assertion `left == right` failed: the owner's rollups are the owner's data; left: []
A26: green at 83f3a78
```

Two measurements shaped the green commit. The predecessor's `sum` of floats is the one CPython 3.12
computes, Neumaier's compensated summation, so the seconds port it (`metrics.rs`, `python_sum`);
and serde_json's default float parser is best-effort, so analytics' tests parse the goldens with
its `float_roundtrip` feature, which lets the floats be compared bit for bit.

R15's wiring has no criterion of its own, and no test of A16 to A21 runs a sync cycle, so the
cycle's test, `settle_fold::cycle::a_sync_cycle_runs_the_fold_and_settles_only_after_a_successful_sync`,
is recorded under A18, whose rule it proves through the cycle (SPEC-071 §10). It was red at 5c8ab5d,
where the cycle held its fold and never ran it: assertion `left == right` failed: the fold ran for
the current day, and settled nothing; left: [], right: [(StudyDay(20000), false)]. It is green at
2dd74ce, where the cycle runs the fold after its read with the study day of the latest successful
sync. A22 and A23 were run at 5c8ab5d with their test target whole
(`--test rollup_routes`, its two tests) and A24 with its target's one test, each failing by its own
assertion; A25 was run with its fenced command.

## Fix round 1

The first review planted six defects, S1 to S6, that every test passed. The code is right in all
six places, so the round adds tests and changes no production code. The order of work: `dev` at
c96c8eb absorbed by a merge commit that resolved nothing, since no file of #292 is one of this
delivery's (b3c71ce); A13's own test widened (b41cef0, and one more malformed file at 504eaff); five
tests of the fold and the sync cycle (892d904); rows S07111 and S07112 (ed31396); SPEC-071's §3,
§9 and §10 (c9920c8); and this record.

None of the six can be red first, because the code each one observes was already right. Each was
run green on the real code instead, and red under its defect, planted on an export: the committed
head, written out with `git archive` into a scratch directory, the defect applied at an anchor that
occurs exactly once, the test run alone with `--exact`, and the file restored, its sha256 the same
before the plant and after the restore. The defects are the review's own, verbatim. Under each one
the whole test target ran as well, and the new test was the only one to fail, which is the gap the
review measured. None of the six is a criterion of §3, so their record stands outside the
`red-first` fence:

```text
S1, R9, recorded under A6: settle_fold::a_late_review_rerolls_a_settled_day_and_keeps_what_it_closed_with
  green at 892d904
  red under P3a-step (a revisit that re-rolls a day records its card state), export of ed31396:
  assertion `left == right` failed: the card state it closed with
  left: Some(CardState { mature_count: 2, young_count: 1, leech_active: 1, backlog: 0, due_today: 0 })
  right: Some(CardState { mature_count: 2, young_count: 1, leech_active: 1, backlog: 1, due_today: 1 })
S2, R16, recorded under A16: settle_fold::a_settle_whose_cursor_is_refused_commits_none_of_its_steps_work
  green at 892d904
  red under P3c (the steps' write commits before the cursor's write opens), export of ed31396:
  assertion `left == right` failed: the refused settle committed none of its steps' work
  left: 1, right: 0
S3, R15, recorded under A18: settle_fold::cycle::a_sync_across_the_rollover_leaves_the_day_it_started_in_owed
  green at 892d904
  red under P3d (the fold is handed the day the sync finished in), export of ed31396:
  assertion `left == right` failed: the fold rolled the current day up, and the day that closed stays owed
  left: [(StudyDay(19999), true), (StudyDay(20000), false)], right: [(StudyDay(20000), false)]
S4, R18, recorded under A21: settle_fold::a_change_to_any_review_field_or_to_the_courses_rerolls_the_day
  green at 892d904
  red under P3f (the fingerprint omits `ease`), export of ed31396:
  assertion `left != right` failed: the fingerprint digests a review's ease
  left: "9353b43f58d25e3d", right: "9353b43f58d25e3d"
S5, R1, A13's own test: courses_config::the_courses_file_refuses_a_duplicate_or_overlapping_course
  green at b41cef0 and at 504eaff
  red under P5a (a malformed code's refusal quotes the code), exports of ed31396 and 504eaff:
  an upper-case code: the setting DECKSTREAK_COURSES_FILE is malformed: it must be courses each
  with a code of 1 to 8 lowercase letters, digits or hyphens, a name, a flag, a deck root, a
  one-letter alias, a writing flag and unit bands, not "QAA" quotes QAA
S6, R4, recorded under A14: settle_fold::cycle::cycles_with_an_unchanged_courses_file_leave_the_settings_generation_alone
  green at 892d904
  red under P5c (each recompute records another digest, then the file's own), export of ed31396:
  assertion `left == right` failed: run 0: the generation stays where the first start put it
  left: 3, right: 1
```

Each planted file read the same sha256 after its restore as before its plant:
`recompute/analytics_step.rs` 35fe311db55052bb, `recompute/mod.rs` 132b02cd2202c5db,
`sync_cycle.rs` b52b99ce74d72341 (twice), `rollup.rs` 581053c8f09564d5 and `courses.rs`
08a5ce966bddd3ca (twice), each the first sixteen hex digits. They are the digests the review read
before its own plants, so no production file changed between its round and this one.

DISCLOSURE, A13 (`the_courses_file_refuses_a_duplicate_or_overlapping_course`): its body changed at
b41cef0 and 504eaff, after its green commit 83f3a78. The loop over the malformed files asserted
each refusal's variant and setting. It now also asserts that the refusal's text names the setting
and quotes none of the file's string values longer than one character, as the loop over the
contradictions already did. A text that is no JSON may not be quoted whole, and the schema's
published name, `COURSES_SCHEMA`, is the one value a refusal may name, as the shape it expects.
`malformed_files()` grew from 11 files to 19, so that every place the reader refuses a file's shape
is reached by one of them. The eight added are a list, a course that is no object, unit bands that
are no object, focus subjects that are no list, a focus subject that is no object, a focus
subject's upper-case code, a focus subject's two-letter alias, and a course with no alias. The
refusals of a missing file and of a relative path are held to naming the setting and not the path.
Why: R1 refuses start "with an error naming the setting and never a value" at every refusal, and
the test held only the contradictions to it, so a malformed code's refusal that quoted the code
passed it (the review's S5). Nothing it asserted before was removed. A13's text in SPEC-071 §3 grew
with it, from "refuses a duplicate code, alias or deck root and overlapping or unordered unit
bands, naming the setting and never a value" to "refuses a file it cannot read, a malformed file, a
duplicate code, alias or deck root and overlapping or unordered unit bands, each naming the setting
and never a value".

No other criterion's test changed in this round. The five other tests are new functions, and the
tests of A6, A14, A16, A18 and A21 read as they did.

Two of the six hold a settlement invariant that cargo-mutants cannot produce, and they take rows
in SPEC-071's band. S07111-SETTLE-ONE-WRITE inserts a commit between a settle's steps and its
cursor, and S2's test kills it. S07112-SYNC-START-DAY hands the fold the day the sync finished in,
and S3's test kills it. `python3 scripts/mutation_rows.py prove --band S07100-S07199` at c9920c8 reads
`rows: examined 12: killed 12, survived 0, void 0`: each killer selected its one test with
and without the mutant, and each target was restored byte for byte. The other four take no row. R9's (S1) and R4's (S6) defects
each add a call, which no constant, string, attribute or `new` method holds, and their tests hold
them. A row for R18 (S4) would run its killer in analytics' own package, where no fold test runs.
R1's (S5) defect leaks a value into a `&'static str` through `Box::leak`, which is contrived, and
A13 itself now kills it.

## Continuation

The first review's private observation, ruled a defect: after the owner's sync that started before
the rollover and finished after it, the next scheduled sync starts after the close, but nothing in
the collection changed since that sync's anchor, so the change gate skipped the recompute and the
closed day stayed owed. R15 stays as written. The order of work: the test of the scheduled path and
the negative test, alone (83cceb7); the fix, with `first_success_in`'s own test in ingest's package
(2d76848); row S07113 (54da2ba); SPEC-071's §4, §9 and §10, ADR-071's "The fold" and this record.

The new test is red first, and it was the only failure of its whole target (`cargo test -p
deck-streak-coordination --test settle_fold`: 17 passed, 1 failed at 83cceb7; 18 passed at
2d76848). The negative test passed at 83cceb7, since the code before the fix never recomputed for
a day with no owed close, so it cannot be red first; it and `first_success_in`'s test, which came
with the function it observes, were run red under a plant instead. The plants were applied in the
committed worktree at 2d76848, at an anchor that occurs exactly once, the whole test target run,
and the file restored, its sha256 the same before the plant and after the restore, with the tree
clean again before the next plant. None of the three is a criterion of §3, so their record stands
outside the `red-first` fence:

```text
R15, recorded under A18: settle_fold::cycle::the_scheduled_sync_after_a_sync_across_the_rollover_settles_the_owed_day
  red at 83cceb7: assertion `left == right` failed: the scheduled sync, which started after the close, settled the day
  left: [(StudyDay(20000), false)], right: [(StudyDay(19999), true), (StudyDay(20000), false)]
  green at 2d76848
R15, recorded under A18: settle_fold::cycle::the_scheduled_sync_after_a_settle_still_skips_the_recompute
  green at 83cceb7 and at 2d76848
  red under the latest successful sync as the deadline (last_success_at for first_success_in):
  assertion `left == right` failed: no day is owed and nothing changed, so the gate skips the recompute
  left: Ran { reason: DeadlineDue { label: "owed_settle" }, reviews: 0, cards: 0 }, right: Skipped
  the only failure of its target: 17 passed, 1 failed
R15, first_success_in's own test: gate::a_study_days_first_success_is_its_earliest_successful_start
  green at 2d76848
  red under first_success_in answering none: assertion `left == right` failed: each day's first
  success is its own earliest successful start
  left: (None, None), right: (Some(UtcMillis(1728018000000)), Some(UtcMillis(1728105300000)))
  the only failure of its target: 11 passed, 1 failed; under the same plant the fold's target
  failed only the_scheduled_sync_after_a_sync_across_the_rollover_settles_the_owed_day
```

Each planted file read the same sha256 after its restore as before its plant: `sync_cycle.rs`
428705d0742e66de and `sync_runs.rs` 9723db156eda77a9 (twice), each the first sixteen hex digits.

The S3 test of fix round 1, `a_sync_across_the_rollover_leaves_the_day_it_started_in_owed`, reads
as it did: its second cycle is the owner's sync, which marks a rescore, and it stays S07112's
killer. The new test adds the scheduled path, and does not subsume it. No other test changed.

S07113-OWED-SETTLE-DUE drops the obligation where `with_fold` registers it, which no
function-level mutant does, and the new test kills it. `python3 scripts/mutation_rows.py prove
--band S07100-S07199` at 54da2ba reads `rows: examined 13: killed 13, survived 0, void 0`: each
killer selected its one test with and without the mutant, and each target was restored byte for
byte.

## Amendment of 2026-10-02: two folds that overlap, and a day before the first settle (#311)

The order of work: the FoldSettlesOnce model and its covers (417a407, a29203c); ADR-313 and
SPEC-071's §11 and §12 (c5e943b); A27 and A28 alone, beside the fold as it was (beb465a); the fix,
each owed day's write reading the settle cursor again inside its own `BEGIN IMMEDIATE` (9118c8b);
the model's re-read arm, its property SettleInTurn with its witness, and the run covers re-stamped
(15f680f, 4d52eab); row S07114 (b8b9f69); and this record.

A27 was run at beb465a with its fenced command and with its whole target, and was that target's
only failure: `cargo test -p deck-streak-coordination --test settle_fold` read 19 passed, 1 failed,
after `examined 36 overlapping runs`. It failed by its own assertion, not by a compile error, a
missing fixture or an empty selection. At 9118c8b the package read 148 passed, 0 failed, A27 among
them. A28 pins what the base already did, so it cannot be red first; ADR-313 states that rule as
its (b).

```red-first
A27: red at beb465a: assertion `left == right` failed: every day from the first settled one to the last closed one, once, oldest first: member (None, 20002, 20002), the owner's fold joined first: false; left: [20001, 20001], right: [20001]
A27: green at 9118c8b
A28: not red: pins the base's behaviour, which ADR-313 states as its (b): before the first settled day a day has a row only as a study day of the window or as some recompute's current day; green at beb465a, and no code arm exists to mutate
```

S07114-OWED-DAY-RE-READS-THE-CURSOR replaces the in-write re-read with the day the run already
holds, which is the fold before the fix. `python3 scripts/mutation_rows.py --row
S07114-OWED-DAY-RE-READS-THE-CURSOR prove`, run in a clean clone at b8b9f69, reads `KILLED: its
killer passed without the mutant and failed with it` and `rows: examined 1: killed 1, survived 0,
void 0`: A27 selected its one test with and without the mutant, and the target was restored byte
for byte.

## Addendum of 2026-10-02: the settle-once test counts the fold's offers (#311)

Row S07108-SETTLE-ONCE turns the first owed day after the cursor, `next(cursor)`, into the cursor
itself. Before 9118c8b that made a second recompute settle the cursor's day again, and A16 failed.
Since 9118c8b each owed day's write reads the cursor again inside its own `BEGIN IMMEDIATE` and
moves to the day the cursor owes, so the mutant's stale start day is corrected inside the write:
the run passes over the cursor's day, rolls that write back, offers once more, and still settles
every closed day once, oldest first. With the mutant installed at a266377 the package read 148
passed, 0 failed, and the rows command CI runs, `python3 scripts/mutation_rows.py prove
--rows-from plan.json` over the plan from dev's tip, read `rows: examined 8: killed 7, survived 1,
void 0`.

75375f2 strengthens A16's test, `the_fold_settles_each_closed_day_once_oldest_first`, with an
offers port that counts the fold's calls: each recompute offers once before each day it settles,
once before the current day's write and once after its last write, so its settled count plus two,
and one more for every day at or before the cursor it passes over. The test is MUTATION COVERAGE,
not red-first evidence: the commit changes no production file, and the test passes over the
base's fold. S07108's row, its killer and SPEC-071's table line are unchanged.

```text
A16, MUTATION COVERAGE: not red: the_fold_settles_each_closed_day_once_oldest_first passes at 75375f2 with crates/coordination/src/recompute/mod.rs as dev's tip 1da1d05 holds it (the fold before 9118c8b starts at the day after the cursor and never passes over one), so the count has no base arm to be red for; at 75375f2 with S07108's mutant installed it fails: the second recompute offers once before each of the 3 day(s) it settles, once before the current day's write and once after its last write: it passes over no day at or before the cursor; left: 6, right: 5 (KILLED)
A16 at 75375f2: 1 passed, after `examined 3 runs' offers` and `examined 4 settles`
rows at 75375f2, the same command over the plan from dev's tip: rows: examined 8: killed 8, survived 0, void 0
```

cargo-mutants over the diff from dev's tip, run in a clean clone as CI runs it, listed two mutants
of `Fold::run`. Its body replaced by `Ok(Default::default())` was caught. `owed != day` turned
into `owed == day` is a TIMEOUT at the run's `--timeout 300`, not a miss: every write whose day is
still owed then rolls back and reads the cursor again, so the loop never ends. `outcomes.json`
read 2 mutants: caught 1, missed 0, timeout 1, unviable 0.
