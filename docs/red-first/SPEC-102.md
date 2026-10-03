# Red-first record: SPEC-102

The goldens came first (47da2b32): `tools/parity-oracle/registry/spec_102.py` registered the
landmarks, their texts and their constants, and the generator wrote each golden from the
predecessor's own functions at `27ee2bc` over synthetic inputs.

The tests were committed beside inert stubs: `landmarks.rs` answered no landmark, no due day, an
empty text, a step of 0 and empty event names and templates; `CollectionReader::study_days`
answered no day. Each whole test file was run and every test failed by assertion, never by a
compile error, a missing fixture or an empty selection.

Part a, A1 to A4, `cargo test -p deck-streak-notifications --test landmarks` (4 failed, 0 passed):

- A1 `the_landmarks_match_the_parity_golden`: `left: []`, `right: [("landmark:anniv:1",
  "landmark_anniversary", 1, StudyDay(11688))]` for the case `{"study_days":[11323],"today":11688}`.
- A2 `only_the_landmarks_dated_the_day_are_due`: `left: []`, `right: ["landmark:anniv:1"]` for the
  same case.
- A3 `the_landmark_text_matches_the_parity_golden`: `left: ""`, `right: "<calendar> <b>1st
  anniversary</b> on 2024-05-17"` for `{"day":19860,"event":"landmark_anniversary","ordinal":1}`.
- A4 `the_landmark_constants_equal_the_predecessors`: `left: 0`, `right: 25` for
  `landmarks.LANDMARK_DAY_STEP`.

Part a, A24, `cargo test -p deck-streak-ingest --test study_days` (1 failed, 0 passed):

- A24 `the_study_days_are_every_scoped_study_event_day_once`: `left: []`, `right:
  [StudyDay(10000), StudyDay(19001), StudyDay(19005), StudyDay(19020)]`.

The reading, one line per criterion: red at the test commit, green at the implementation commit.

```red-first
A1: red at be50d156: assertion `left == right` failed: the case {"study_days":[11323],"today":11688}: left [], right [("landmark:anniv:1", "landmark_anniversary", 1, StudyDay(11688))]
A1: green at a8ffb874
A2: red at be50d156: assertion `left == right` failed: the case {"study_days":[11323],"today":11688}: left [], right ["landmark:anniv:1"]
A2: green at a8ffb874
A3: red at be50d156: assertion `left == right` failed: the case {"day":19860,"event":"landmark_anniversary","ordinal":1}: left "", right a text of the 1st anniversary
A3: green at a8ffb874
A4: red at be50d156: assertion `left == right` failed: landmarks.LANDMARK_DAY_STEP: left 0, right 25
A4: green at a8ffb874
A24: red at be50d156: assertion `left == right` failed: left [], right [StudyDay(10000), StudyDay(19001), StudyDay(19005), StudyDay(19020)]
A24: green at a8ffb874
```

The green run reads `test result: ok. 4 passed` for `tests/landmarks.rs` and `test result: ok. 1
passed` for `tests/study_days.rs`.

Disclosure: the green commit a8ffb874 also touches the two test files `crates/ingest/tests/study_days.rs` and
`crates/notifications/tests/landmarks.rs`. It is `cargo fmt` re-wrapping the `assert_eq!` calls, one array
constant and one iterator chain, plus the file attribute `#![allow(clippy::expect_used)]` that the crate's other
test files carry. No expected value, no case and no comparison changed.

The anniversary walk's cap (ruling 66, SPEC-102 10.7): the test `the_anniversary_walk_stops_at_its_cap` in `crates/notifications/tests/landmarks.rs` was red at f92a24b5, run as `cargo test -p deck-streak-notifications --test landmarks the_anniversary_walk_stops_at_its_cap`: assertion `left == right` failed: left 10005, right 10000. It is green at 141f1d2f, the commit that adds the constant `ANNIVERSARY_WALK_CAP`.

Part b, A25 to A27 and A35 to A39 (SPEC-102 section 11), `cargo test -p deck-streak-coordination
--test landmarks_step -- --test-threads=1`, run in a detached tree of the test commit e1b1442d (8
failed, 0 passed). The test commit carries the tests, the `landmarks_run` adapter and its golden;
no offer raised a landmark there and nothing stored the mark or the cursor, so every test failed by
assertion, never by a compile error, a missing fixture or an empty selection:

- A25 `each_evaluated_day_raises_its_due_landmarks_once`: `left: []`, `right: ["landmark:day:25",
  "landmark:anniv:1"]`.
- A26 `the_first_run_seeds_the_mark_and_raises_at_most_one`: `left: []`, `right:
  ["landmark:anniv:1"]` for the case `{"mark":null,"study_days":[19634,20000],"today":20000}`.
- A27 `the_anniversary_is_honest_on_a_day_without_study`: `left: []`, `right:` the honest text of
  the 1st anniversary on 2024-10-04.
- A35 `an_unanswered_landmark_keeps_the_cursor_and_is_offered_again`: the cursor, `left: None`,
  `right: Some("19999")`.
- A36 `the_first_run_never_moves_the_cursor_past_a_landmark_it_did_not_raise`: `left: []`, `right:
  ["landmark:anniv:1"]`.
- A37 `an_imported_mark_raises_todays_landmarks_and_no_history`: the cursor, `left: None`, `right:
  Some("19999")`.
- A38 `the_high_water_mark_equals_the_predecessors_bytes`: `left: None`, `right: Some("{\"anniversary\":
  0, \"seeded\": true, \"study_day\": 0}")` for `{"mark":null,"study_days":[19634,19637],"today":19999}`.
- A39 `the_sync_cycle_offers_the_landmarks_after_the_awards`: `left:` the badge's line alone,
  `right:` the badge's line, then the 1st anniversary's.

```red-first
A25: red at e1b1442d: assertion `left == right` failed: the day evaluated's landmark first, then the settled day's: left [], right ["landmark:day:25", "landmark:anniv:1"]
A25: green at 96f6fe31
A26: red at e1b1442d: assertion `left == right` failed: the keys of {"mark":null,"study_days":[19634,20000],"today":20000}: left [], right ["landmark:anniv:1"]
A26: green at 96f6fe31
A27: red at e1b1442d: assertion `left == right` failed: the honest variant on a day without study: left [], right the honest text of the 1st anniversary
A27: green at 96f6fe31
A35: red at e1b1442d: assertion `left == right` failed: the cursor stops the day before the landmark the router did not answer: left None, right Some("19999")
A35: green at 96f6fe31
A36: red at e1b1442d: assertion `left == right` failed: the first run raises the first due landmark only: left [], right ["landmark:anniv:1"]
A36: green at 96f6fe31
A37: red at e1b1442d: assertion `left == right` failed: the cursor is stored at yesterday: left None, right Some("19999")
A37: green at 96f6fe31
A38: red at e1b1442d: assertion `left == right` failed: the mark of {"mark":null,"study_days":[19634,19637],"today":19999}: left None, right the predecessor's mark
A38: green at 96f6fe31
A39: red at e1b1442d: assertion `left == right` failed: the owed badge first, then the landmark due: left [the badge's line], right [the badge's line, the 1st anniversary's line]
A39: green at 96f6fe31
```

The green run reads `test result: ok. 8 passed` for `tests/landmarks_step.rs`, and the notifications
package's own killers, `tests/landmark_settings.rs` and
`the_mark_is_the_predecessors_json_for_every_golden_run` in `tests/landmarks.rs`, pass beside it.

Disclosure: the green commit 96f6fe31 also touches `crates/coordination/tests/landmarks_step.rs` in
two places, neither an expected value, a case or a comparison. The copy's connection registers the
`unicase` collation, as the daemon's own connection does, because A39's failed-read arm renames the
review table, and the rename re-reads an index of the copy collated `unicase`; at e1b1442d that arm
was never reached, since A39 failed first at the badge-then-landmark assertion. And it drops an
`#[allow(dead_code)]` on the golden module, which the module already carries.
