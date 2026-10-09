# Red-first record: SPEC-382

A1 to A7 are the seven tests of `TheRunRecordsItsOwnTiming` in `scripts/tests/test_ci_workflows.py`,
committed alone at `2f55005f`, before the workflow lines they judge. They run in CI's `hygiene` job,
which runs `scripts/tests`. At `2f55005f`, run 37956483933, job 113908188479 ran 1099 tests and
failed 7, the seven new tests and no other, each by its own assertion, with no error. Each test's
problem list, as its failure printed it:

- A1: ``harness: the planted suite's step writes no `card-probe-seconds` timer``, then
  `harness: the report lacks cases_in("card-probe")` and
  `harness: the report lacks minutes("card-probe-seconds")`.
- A2: seven problems, the first ``harness: the step 'the tests, Debug, on the iPhone and then the
  iPad' runs `xcodebuild test` without -showBuildTimingSummary``.
- A3: ``xcframework: the static-library step writes no per-target seconds file,
  `build-seconds-$target` ``.
- A4: `harness: no step boots a simulator after 'the project, generated'`.
- A5: `xcframework: the job's environment does not set the developer directory`.
- A6: `harness: 0 uploads named 'timing-harness', not one`, then
  `xcframework: 0 uploads named 'timing-xcframework', not one`.
- A7: `harness: the result bundle card-probe.xcresult is uploaded and not read by the report`.

`6f131d5a` changes the workflow alone (SPEC-382 R1 to R6) and turns all seven green; their green is
read by CI on the pull request's head, which runs the same job. After the green, `e87e7ad9` grows
`APP_REPORT_NEEDS` in the same test file to a superset that also holds the report's new rows, the
planted suite's cases and seconds and the boot seconds. It changes no assertion of A1 to A7, and
the one plant built on that tuple, the app steps census's good report joined from it, keeps its
verdict, because a superset keeps every need the plant removes once. Rows S38200 to S38208
(`fcc649fe`) read each criterion red against its planted defect, their kills read in CI.

```red-first
A1: red at 2f55005f: run 37956483933, job `hygiene`: "harness: the planted suite's step writes no `card-probe-seconds` timer"
A1: green at 6f131d5a
A2: red at 2f55005f: run 37956483933, job `hygiene`: "harness: the step 'the tests, Debug, on the iPhone and then the iPad' runs `xcodebuild test` without -showBuildTimingSummary"
A2: green at 6f131d5a
A3: red at 2f55005f: run 37956483933, job `hygiene`: "xcframework: the static-library step writes no per-target seconds file, `build-seconds-$target`"
A3: green at 6f131d5a
A4: red at 2f55005f: run 37956483933, job `hygiene`: "harness: no step boots a simulator after 'the project, generated'"
A4: green at 6f131d5a
A5: red at 2f55005f: run 37956483933, job `hygiene`: "xcframework: the job's environment does not set the developer directory"
A5: green at 6f131d5a
A6: red at 2f55005f: run 37956483933, job `hygiene`: "harness: 0 uploads named 'timing-harness', not one"
A6: green at 6f131d5a
A7: red at 2f55005f: run 37956483933, job `hygiene`: "harness: the result bundle card-probe.xcresult is uploaded and not read by the report"
A7: green at 6f131d5a
```
