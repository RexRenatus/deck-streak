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
