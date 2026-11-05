# SPEC-608: the harness report lists each planted card test with its result on each simulator

- **Issue:** #841 item 1, and #843 item 1, which asks for the same change: when a planted card test failed on one of
  the two simulators, the `harness` job's results table held no row for the planted suite, so the job did not say
  which simulator failed (jobs 115343048693 and 115407471652). **Context(s):** CI
  (`.github/workflows/xcframework.yml`) and its tests (`scripts/tests/`); no crate, Swift file or caller workflow
  changes.
- **Decided by:** ADR-630 (the report reads the planted suite bundle like every other test bundle). It keeps ADR-612
  (one report body in both jobs, naming the other job for the rows it does not measure).
- **Status:** delivered by the pull request that adds this file, with its tests and `docs/red-first/SPEC-608.md`.
  **Mutation band:** S60800-S60899. It amends SPEC-591 R6's owners map by one entry, as section 8 states; no earlier
  byte of that file changes.

## 1. The problem, measured

The planted suite step (`the card view's planted suite, Debug, on the iPhone and then the iPad`) writes its result
bundle to `$RESULTS/card-probe.xcresult`. The report step reads five bundles by `cases_in(...)`: `debug`, `release`,
`app`, `review` and `review-actions`. It never reads `card-probe`, so its results table holds no row for a planted
card test. In jobs 115343048693 and 115407471652 a planted card test failed on one of the two simulators, and the
report did not say which one: only the step's console did, and a fresh run could not be compared with it simulator
by simulator.

## 2. Requirements

- **R1.** The report body reads the planted suite bundle as it reads every other bundle:
  `card_probe_cases = cases_in("card-probe")` beside the five existing `cases_in` lines, so `card-probe-tests.json`
  and its `details-card-probe-*.json` files are written beside the report.
- **R2.** The results table lists each planted case under the label `Card probe`:
  `("Card probe", card_probe_cases)` follows `("Review actions", review_actions_cases)` in the table loop, so each
  row reads ``| Card probe `<test id>` | <simulator>: <result>; <simulator>: <result> |`` through the existing
  `per_simulator`, which names each simulator a case ran on.
- **R3.** The owners map gains `"card-probe": "harness"`, so the report of `release-and-app`, which runs no planted
  suite, writes ``| Card probe: no test result was read | in `harness` |`` and never raises on the new label.
- **R4.** Nothing else changes: the planted suite step, its tests, waits, bounds and assertions, every other report
  row, every job bound and trigger. The two report bodies stay one text (SPEC-591 R6): each edit is applied to both,
  and `split_report_problems` keeps reading `[]`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | each report body, in `harness` and in `release-and-app`, holds `cases_in("card-probe")`, `("Card probe", card_probe_cases)` and `"card-probe": "harness"`, and each of the three removed from either body is refused by its own line (R1 to R3) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_report_reads_the_planted_suite_bundle_in_both_jobs` |
| A2 | each job's live report step, run under bash with a planted `xcrun` that answers for the planted suite's bundle with one case that fails on one simulator and passes on the other, writes in `harness` the one row naming each simulator with its result, and in `release-and-app` the row that names `harness` (R1 to R3) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_planted_card_test_that_fails_on_one_simulator_is_named_by_its_simulator` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_report_reads_the_planted_suite_bundle_in_both_jobs
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_planted_card_test_that_fails_on_one_simulator_is_named_by_its_simulator
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/xcframework.yml` | CI | changed: the report body, one text in both jobs, takes in the planted suite bundle and lists its cases (R1 to R3) |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: one new class with the A1 and A2 tests, their constants and plants |
| `scripts/mutation-rows.d/S60800-S60899.json` | CI tests | added: rows S60800 to S60802 |
| `docs/specs/SPEC-608-the-harness-report-lists-each-planted-card-test-on-each-simulator.md` | docs | added: this delivery's SPEC |
| `docs/decisions/ADR-630-the-harness-report-reads-the-planted-suite-bundle-like-every-other-test-bundle.md` | docs | added: the decision |
| `docs/schematics/harness-report-card-probe.md` | docs | added: the report's data flow |
| `docs/red-first/SPEC-608.md` | docs | added: A1 and A2, red and green |
| `changelog.d/harness-report-card-probe.md` | docs | added: the fragment |

## 5. What this does NOT do

- It does not measure the planted suite per simulator across runs, and it does not change the load wait: that is
  item 2 of #841 and #843, which the load-wait cure (SPEC-349 R11) carries. The rows this adds are the instrument
  its readings use.
- It retries, skips or loosens no test, and changes no wait, bound, trigger, upload or step other than the report.
- It closes no issue: #841 and #843 close at the release.

## 6. Risks

- **A bundle with one destination.** A case with no Device children prints its one result, as every other row does
  through `per_simulator`.
- **Long finds.** The two report bodies are one text, so S60800's and S60801's finds run from a line only `harness`
  holds to the line they mutate, and S60802's from a line only `release-and-app` holds. A later edit inside one moves
  its anchor; the row census names it.
- **A2 runs the live report body.** It needs `bash` and `python3` on the test's path, as `run_output_check` does. Its
  planted `xcrun` exits non-zero for every call but the one it answers, so every other bundle reads as absent.

## 7. Mutation rows

| row | file | mutant | killer |
|---|---|---|---|
| `S60800` | `xcframework.yml` | `harness`'s report sets `card_probe_cases` to an empty list in place of reading the bundle | A1 |
| `S60801` | `xcframework.yml` | `harness`'s report drops `("Card probe", card_probe_cases)` from the table loop | A2 |
| `S60802` | `xcframework.yml` | `release-and-app`'s report drops `"card-probe": "harness"` from the owners map | A1 |

## 8. What this keeps, supersedes and amends

| clause | reading |
|---|---|
| SPEC-591 R6 | amended: the owners map names `harness` for the planted suite's rows; the two bodies stay one text |
| SPEC-349 R10 and R11 | kept: the planted suite step, its waits and its assertions are unchanged |
| ADR-612 | kept: the sibling report names the job that measures a row |
