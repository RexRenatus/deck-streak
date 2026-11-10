# SPEC-669: Each test suite's run time is held to a recorded bound, at warn

Follows SPEC-436 and ADR-450: a census that starts at warn, reads its whole population, prints what
it examined and refuses an empty one. Decided by ADR-692. Tracked by #1387.

## 1. The problem, measured

- No check bounds a test suite's run time. `git grep -c -E 'slow-timeout|slow_timeout|junit|JUnit'`
  over the tree matches 0 lines, and no workflow step compares a duration with a limit.
- CI already records a time per suite, in the stage logs every gate job uploads. In run
  42582016940, the `rust` job 129090178681's `test` stage log holds at least 2358 nextest result lines in
  at least 478 binaries, each with the test's own duration; the `hygiene` job 129090178646's `python` stage
  log holds at least 148 `module:` lines, each after its module's `Ran N tests in Xs` line (at least 2220 tests); the
  `web` job 129090236405's `web` stage log holds 129 Vitest file lines with a duration each (859
  tests). The job logs of the same run (jobs 129090178681, 129090178646, 129090236405 and
  129090178740) hold 0 such lines. The same `rust` job's `doctest` stage log holds each crate's
  `Doc-tests` line, followed by its result lines with their durations.
- The browser suites print a run total only, and the Swift test targets run outside `ci`, so no
  stage log of `ci` holds a time for either.
- The figures move between runs. Over the eight runs read (42582016940, 42557050660,
  42552326063, 42546777601, 42541868033, 42541861672, 42538321688, 42534443970), taking one run's
  figure as the bound and the larger of 100 % of the bound and 30 s as the margin, 19 of 68912
  pairwise suite comparisons flag a suite, all on two suites; the larger of 50 % and 10 s flags
  216.

## 2. Requirements

R1. The census lists every suite CI runs by its runner's own unit: each Rust test binary under the
workspace's test runner (each crate's library, binaries and `tests/*.rs` targets, in the test and
release stages), each Python test module of the python stage, each web unit test file, each browser
test file and each Swift test target. It prints `examined N suite(s)` and refuses 0.

R2. It takes each suite's time from the stage logs: a Rust binary's figure is the sum of its tests'
own durations, a Python module's is the `Ran N tests in Xs` line its own `module:` line follows
with the same count, a web unit file's is the duration on its file line, and a crate's doc tests'
is the sum of the `finished in Xs` of the result lines after its `Doc-tests` line, or the
`all doctests ran in Xs` line where the runner prints its merged form.

R3. A suite whose figure is over its listed bound by more than its margin, the larger of the
margin's ratio of the bound and its floor in seconds, is a site, reported at warn with its reason.

R4. A suite with no time recorded is reported, never passed; a browser or Swift suite is reported
as having no time recorded in `ci`.

R5. A suite whose figure would list a bound lower than its listed one is reported for the listing;
only a delivery changes an entry.

R6. A listing entry that names no current suite fails, at warn as at fail.

R7. A CI job reads the stage logs of the same run and runs the census over them at warn, after the
gate jobs it reads.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a suite under its bound is clean | `test_a_suite_under_its_bound_is_clean` |
| A2 | a suite over its bound past its margin is a site | `test_a_suite_over_its_bound_past_its_margin_is_a_site` |
| A3 | a suite over its bound within its margin is clean | `test_a_suite_over_its_bound_within_its_margin_is_clean` |
| A4 | a faster suite is reported for the listing | `test_a_faster_suite_is_reported_for_the_listing` |
| A5 | a suite with no time recorded is reported, never passed | `test_a_suite_with_no_time_recorded_is_reported_never_passed` |
| A6 | a listing entry naming no current suite fails | `test_a_listing_entry_naming_no_current_suite_fails` |
| A7 | an empty population is refused | `test_an_empty_population_is_void` |
| A8 | each runner's time is read from its own form | `test_each_runner_s_time_is_read_from_its_own_form` |
| A9 | the aggregate check needs the suite-time job | `test_the_gate_runs_in_four_parallel_jobs` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_suite_time_ratchet.py -k test_a_suite_under_its_bound_is_clean
A2: python3 -m unittest discover -s scripts/tests -p test_suite_time_ratchet.py -k test_a_suite_over_its_bound_past_its_margin_is_a_site
A3: python3 -m unittest discover -s scripts/tests -p test_suite_time_ratchet.py -k test_a_suite_over_its_bound_within_its_margin_is_clean
A4: python3 -m unittest discover -s scripts/tests -p test_suite_time_ratchet.py -k test_a_faster_suite_is_reported_for_the_listing
A5: python3 -m unittest discover -s scripts/tests -p test_suite_time_ratchet.py -k test_a_suite_with_no_time_recorded_is_reported_never_passed
A6: python3 -m unittest discover -s scripts/tests -p test_suite_time_ratchet.py -k test_a_listing_entry_naming_no_current_suite_fails
A7: python3 -m unittest discover -s scripts/tests -p test_suite_time_ratchet.py -k test_an_empty_population_is_void
A8: python3 -m unittest discover -s scripts/tests -p test_suite_time_ratchet.py -k test_each_runner_s_time_is_read_from_its_own_form
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_gate_runs_in_four_parallel_jobs
```

## 4. File manifest

- `scripts/tests/test_suite_time_ratchet.py`: the census, its planted tests and its CI reader.
- `scripts/tests/fixtures/suite-times/tree.json.fixture`: the planted tree.
- `scripts/tests/fixtures/suite-times/test.log.fixture`: planted Rust result lines.
- `scripts/tests/fixtures/suite-times/python.log.fixture`: planted Python module lines.
- `scripts/tests/fixtures/suite-times/web.log.fixture`: planted web unit file lines.
- `scripts/tests/fixtures/suite-times/doctest.log.fixture`: planted doc-test lines.
- `scripts/tests/fixtures/suite-times/listing.json.fixture`: the planted listing.
- `scripts/tests/fixtures/suite-times/empty-tree.json.fixture`: a planted tree with no suite.
- `scripts/suite-times.json`: the listing of today's suites, each with its bound, margin and reason.
- `.github/workflows/ci.yml`: the job of R7.
- `scripts/tests/test_ci_workflows.py`: the registers the job of R7 needs, insert-only.
- `docs/specs/SPEC-669-each-test-suite-s-run-time-is-held-to-a-recorded-bound-at-warn.md`: this
  SPEC.
- `docs/decisions/ADR-692-a-suite-s-time-is-read-from-the-stage-logs-and-held-to-a-listed-bound-at-warn.md`:
  the decision.
- `docs/red-first/SPEC-669.md`: the red-first record.
- `changelog.d/suite-time-ratchet-669.md`: the changelog fragment.
- `scripts/mutation-rows.d/S66900-S66999.json`: the rows on the census's arms.

## 5. What this does NOT do

- It fails no run on a suite's time: a site is reported at warn, and promotion to fail is its own
  delivery under ADR-450 D6 (#1387).
- It records no per-file time for the browser suites, and reads no time for the Swift test targets
  (#1387).
- It changes no test, runner configuration or gate stage, and lowers no bound by itself (#1387).

## 6. Risks

- A runner-wide slowdown moves every suite at once and may report many sites together; the report
  names each, and the listing changes only by a delivery.
- A suite whose figure moves widely between runs needs its own margin, with its reason, in its
  listing entry; the measurement above names two such suites.
- A runner's output form may change; a suite whose time is no longer read is reported as having no
  time recorded, never passed, so the change is seen.
