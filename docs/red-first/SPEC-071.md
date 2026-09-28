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
