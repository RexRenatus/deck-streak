# SPEC-591: the harness work runs in two jobs and a verdict, each job inside its own bound, with no step dropped

- **Issue:** the pull request #1116, whose `harness` job was cancelled at its own bound in the bury and flag step
  (job 125641992874). **Context(s):** CI (`.github/workflows/xcframework.yml`) and its tests (`scripts/tests/`); no
  crate, Swift file or caller workflow changes.
- **Decided by:** ADR-610 (the Release, app, review, bury and flag work and the archive run in a sibling job,
  `release-and-app`, that repeats the prefix and boot of `harness`), ADR-611 (a `verdict` job reads every other job
  of the body and fails unless each reads success) and ADR-612 (one report body in both jobs, naming the other job
  for the rows it does not measure). It keeps ADR-355 D1 (one job body for every caller) and ADR-378 (the bound
  rule).
- **Status:** delivered by the pull request that adds this file, with its tests and `docs/red-first/SPEC-591.md`.
  **Mutation band:** S59100-S59199. It supersedes the "no job is added" words of SPEC-347 R13, SPEC-348 R20 and
  SPEC-358 R8 and amends SPEC-339 R12 and R14 and SPEC-347 A14, as section 8 states; no earlier byte of those files
  changes.

## 1. The problem, measured

The `harness` job of `xcframework.yml` runs 22 steps in series under `timeout-minutes: 150` (line 365 at the base).
Its per-step times were read from the jobs API (`actions/jobs/<id>`, its `steps`, each `completed_at` less
`started_at`) and summed by a program, on three jobs:

- job 125484649336 (#1060): 150.37 minutes, cancelled after its last step had finished;
- job 125641992874 (#1116): 153.28 minutes, cancelled at its bound in the bury and flag step, the archive skipped;
- job 125710788160 (#1116): 147.95 minutes, every step success.

The worst time of each step over those three jobs sums to 166.53 minutes (jobs 125484649336, 125641992874 and
125710788160), past the bound, so a green run fits only on a fast runner. The largest steps, each at its worst:

| step | worst minutes | job |
|---|---|---|
| the card view's planted suite, Debug | 74.47 | 125484649336 |
| the review screen's tests, Debug | 15.68 | 125710788160 |
| each simulator booted and the harness launched once, before its tests | 14.92 | 125641992874 |
| the measurements, Release | 13.20 | 125484649336 |
| the bury and flag tests, Debug | 11.25 | 125484649336 |
| the app's tests, Debug | 10.90 | 125710788160 |
| the tests, Debug | 10.37 | 125641992874 |

Split at the ruled line, the worst sums are:

| part | worst minutes | jobs |
|---|---|---|
| `harness`: its set-up, the prefix, the Debug build, boot and tests, the planted suite, report, upload and close | 103.65 | 125484649336, 125641992874, 125710788160 |
| the moved steps: the Release build, boot and measurements through the unsigned archive | 62.88 | 125484649336, 125641992874, 125710788160 |
| what the sibling repeats for itself: the prefix, the Debug build and the boot before the Debug tests | 16.30 | 125484649336, 125641992874, 125710788160 |
| `release-and-app` whole: its set-up, the repeated steps, the moved steps, report, upload and close | 81.70 | 125484649336, 125641992874, 125710788160 |

The Release boot step runs the warm script that the Debug boot step writes to the runner, and it installs nothing
it did not build, so a sibling on a fresh runner must repeat that Debug boot, and the Debug build it installs, before
its Release work (lines 412-463 and 501-504 at the base).

## 2. Requirements

R1. **No step is dropped.** Every step `harness` holds at the base runs exactly once across `harness` and
    `release-and-app`, in its order at the base. Each job's report and upload is its own tail, and the steps the
    sibling repeats (R3) are its own prefix.
R2. **`harness` keeps its first ten steps.** It keeps, byte for byte and in order: the checkout, the framework
    download, placing it, the app icon, XcodeGen, "the project, generated", the Debug build for testing, the boot
    and launch before the Debug tests, the Debug tests and the planted suite; then its report and its `harness`
    upload, and no other step. `needs: xcframework` and `timeout-minutes: 150` are unchanged.
R3. **The sibling repeats the prefix and the boot.** A job `release-and-app` needs `xcframework`, runs on the
    admitted runner with the environment of `harness` byte for byte, and repeats the first eight steps of `harness`
    byte for byte, through the boot and launch before the Debug tests, so its simulators are booted and the
    harness app launched once before its Release work, as at the base.
R4. **The sibling runs the moved steps.** Then `release-and-app` runs, moved byte for byte and in their order at
    the base: the Release build for testing, the boot and launch before the measurements, the Release
    measurements, the Release app built alone, its size, the required-reason symbols, the app's tests, the
    review screen's tests, the bury and flag tests and the unsigned archive, each condition literal kept. Its
    one download is the framework of its own run, by SPEC-339 R12's rule. The one exception to "byte for
    byte" is R13's measurement lines in the run text of the app's, the review screen's and the bury and flag
    tests; every line those steps held at the base stays in their run text, byte for byte and in its order.
R5. **No gate is weakened.** No test, planted card, assertion, destination, configuration, selection, wait,
    timeout or retry changes; no step is skipped, filtered or made conditional; the planted suite stays whole in
    `harness`, one simulator at a time.
R6. **One report body.** Both jobs run the same report text. A row whose source the other job measures reads
    "in `<that job>`"; a row of its own job reads its measurement, or "not measured" when its own step left none.
    The body names its job from the runner's job id and fails on any id that is not a job of the split.
R7. **Each new job has its own bound.** `release-and-app` has `timeout-minutes: 130`: one and a half times its
    worst sum, 81.70 minutes (jobs 125484649336, 125641992874 and 125710788160), rounded up to the ten, by
    ADR-378's rule. A band test holds it inside 123 to 149: 123 is the rule's product rounded up to the minute,
    and 149 keeps it under the bound of `harness`, so the sibling never takes that bound and S36111's line stays
    the harness's alone. `verdict` has `timeout-minutes: 10`, inside 1 to 10. No other bound moves.
R8. **One verdict.** A job `verdict` needs every other job of the body, runs under `if: ${{ always() }}` on the
    admitted runner, checks out and downloads nothing, reads the result of each need from the needs context
    through its step's environment, one entry per need, and exits non-zero unless every result is `success`: a
    failure, a cancellation or a skip fails it.
R9. **Two uploads, two names.** Each split job uploads its report and result bundles whatever the outcome, under
    its own artifact name: `harness` and `release-and-app`.
R10. **The census reads each step where it runs.** The four readers of the moved steps
    (`harness_warm_problems`, `app_steps_problems`, `review_step_problems` and `review_actions_step_problems`) take
    the job they read as an argument whose default is `harness`, so every planted control keeps its job and its
    message, and each live assertion reads the job that holds its steps. No assertion is removed.
R11. **Rows follow their lines.** The six rows of S33900-S33999 that anchor inside the boot and launch step
    (S33901, S33902, S33903, S33904, S33907 and S33908), whose finds the sibling's copy doubles, are re-anchored
    in the same commit, ids, mutants and killers kept: each find runs on to the next step of `harness`, so it occurs
    once. Band S59100-S59199 adds thirteen rows for the new tests (section 7).
R12. **The callers do not change.** Each caller keeps its one call job; no context joins a required set.
R13. **The runner's state is recorded at each DeckStreak test step.** The app's, the review screen's and the
    bury and flag tests of `release-and-app` each record, as the step starts (the app's tests after their
    generate line, which stays first) and at its end whatever its outcome, the runner's memory (`vm_stat`), its
    swap (`sysctl vm.swapusage`), its load (`uptime`) and its booted simulators (`xcrun simctl list devices
    booted`); and, only when the step's `xcodebuild` fails, the iPhone simulator's unified log from the step's
    `started` time to the failure, narrowed by a predicate to the test manager, RunningBoard, the DeckStreak
    app and its UI test runner, the accessibility subsystem and any message that names the app. Each reading
    is a file of its own under `runner-state/` in the report folder, ending with its own status line, and the
    job's upload carries it (R9); the report body reads none of them. A reading that fails writes its status
    into its own file and changes no step's outcome: each runs only as an `if` condition, and an EXIT trap
    that never exits takes the end reading, so the step's status stays its own commands'. This requirement
    adds no criterion of its own: a later change keeps the measurement under a criterion or removes it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every step of `harness` at the base runs exactly once across `harness` and `release-and-app`, in its order at the base (R1) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_step_of_the_harness_at_the_split_runs_once_in_its_order` |
| A2 | `harness` holds its first ten steps at the base, then its report and upload, and no moved step (R2) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_harness_ends_at_its_report_and_upload_after_the_planted_suite` |
| A3 | `release-and-app` repeats the environment and first eight steps of `harness` byte for byte, then the moved steps in their order at the base (R3, R4) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_sibling_repeats_the_harness_prefix_and_boot_then_runs_the_moved_steps` |
| A4 | `verdict` needs every other job of the body, runs under `always()`, reads one result per need and fails unless each is `success` (R8) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_verdict_needs_every_other_job_and_fails_unless_each_reads_success` |
| A5 | the line of row S36111 occurs once in the file, inside `harness`, and each new job states a bound line of its own (R2, R7) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_harness_bound_line_occurs_once_and_each_new_job_has_its_own` |
| A6 | the bound of `release-and-app` is a digit string inside 123 to 149, and that of `verdict` inside 1 to 10 (R7) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_new_job_bound_sits_in_its_own_band` |
| A7 | the two report bodies are one text, which names the other job for each row it does not measure (R6) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_two_report_bodies_are_one_text_that_names_the_other_job` |
| A8 | the jobs that download the framework are exactly `harness` and `release-and-app`, each needs `xcframework` and downloads that one artifact of its own run (R4) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_job_that_downloads_the_framework_needs_its_job_and_reads_one_artifact` |
| A9 | each split job uploads under its own artifact name, under `always()`, its result folder included (R9) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_split_job_uploads_under_its_own_name_whatever_the_outcome` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_step_of_the_harness_at_the_split_runs_once_in_its_order
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_harness_ends_at_its_report_and_upload_after_the_planted_suite
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_sibling_repeats_the_harness_prefix_and_boot_then_runs_the_moved_steps
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_verdict_needs_every_other_job_and_fails_unless_each_reads_success
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_harness_bound_line_occurs_once_and_each_new_job_has_its_own
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_new_job_bound_sits_in_its_own_band
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_two_report_bodies_are_one_text_that_names_the_other_job
A8: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_job_that_downloads_the_framework_needs_its_job_and_reads_one_artifact
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_split_job_uploads_under_its_own_name_whatever_the_outcome
```

Each test is red at the base, read from CI's `hygiene` job, which runs `scripts/tests`: A1, A3, A6, A8 and A9
because there is no `release-and-app` job (the population rule runs first); A4 because there is no `verdict` job; A5
because neither new job states a bound line; A2 because `harness` holds the Release measurements; A7 because the
report names no other job. Each names its plants, and each plant is refused by its rule's name.

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/xcframework.yml` | CI | changed: the moved steps leave `harness` for `release-and-app`; `release-and-app` and `verdict` added; the report body edited, one text in both jobs (R1 to R9) |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: the class `TheHarnessWorkRunsInTwoJobsAndAVerdict` and its constants (A1 to A9); four readers take their job (R10) |
| `scripts/mutation-rows.d/S33900-S33999.json` | CI tests | changed: six finds re-anchored (R11) |
| `scripts/mutation-rows.d/S59100-S59199.json` | CI tests | added: thirteen rows (section 7) |
| `docs/specs/SPEC-591-the-harness-work-runs-in-two-jobs-and-a-verdict-each-inside-its-bound.md` | docs | added: this file |
| `docs/decisions/ADR-610-the-release-and-app-work-runs-in-a-sibling-job-that-repeats-the-harness-prefix-and-boot.md` | docs | added: the sibling job |
| `docs/decisions/ADR-611-a-verdict-job-reads-the-whole-apple-body-under-one-name.md` | docs | added: the verdict job |
| `docs/decisions/ADR-612-one-report-body-in-both-jobs-names-the-other-job-for-the-rows-it-does-not-measure.md` | docs | added: the report body |
| `docs/red-first/SPEC-591.md` | docs | added: A1 to A9, red and green |
| `changelog.d/ci-harness-split.md` | docs | added: the fragment |

## 5. What this does NOT do

- It moves no bound: `harness` keeps 150, and a raise to 180 stays out of this change (#1022).
- It adds no timing record and no dependency cache to the body; those are the timing record's own parts (#752).
- It edits no caller, ruleset or required context, and no earlier SPEC or ADR text (#1116).
- It does not redraw the body's schematics; the new job graph is drawn by its own change (#1116).
- It does not re-run, shard or re-order any test inside a step (#1116).

## 6. Risks

- **Two jobs need two runners at once.** When only one is free, the second job queues and the run's wall time
  grows, but each bound counts from its own job's start. Detected by each job's start against the other's
  end in the run's job list.
- **The sibling pays its own prefix and boot,** 16.30 minutes at its worst (jobs 125484649336, 125641992874 and
  125710788160), on every run. Detected by the sibling's own step times.
- **`harness` keeps little margin.** Its worst sum, 103.65 minutes (jobs 125484649336, 125641992874 and
  125710788160), is past two thirds of its bound (100 minutes), so a slower runner meets the bound sooner than ADR-378
  wants; the bound stays as ruled. Detected by each completed `harness` job's time against 150; a time past four
  fifths of the bound is the signal to measure again.
- **The runner's job id is not a job of the split.** The report then fails its own step, by R6, and the pull
  request's first run shows it.
- **The Release measurements start on a runner that ran no Debug tests.** Their figures are compared from the merge
  commit on, not against earlier runs.
- **Long finds.** The six re-anchored finds run to the next step of `harness`, so any later edit inside them
  re-derives them by program from the file, never by hand.
- FORMAL: not applicable. The split adds jobs and one result check; it has no protocol, ordering or state of its
  own beyond the needs graph, which A4 reads.

## 7. Mutation rows

| row | file | mutant | killer |
|---|---|---|---|
| `S59100` | `xcframework.yml` | the verdict compares each result to anything but `failure` | A4 |
| `S59101` | `xcframework.yml` | the verdict runs under `!cancelled()` | A4 |
| `S59102` | `xcframework.yml` | the verdict no longer needs `harness` | A4 |
| `S59103` | `xcframework.yml` | the verdict reads another job's result for `harness` | A4 |
| `S59104` | `xcframework.yml` | the verdict exits 0 | A4 |
| `S59105` | `xcframework.yml` | the sibling takes the bound of `harness` | A6 |
| `S59106` | `xcframework.yml` | the sibling waits on `harness-wire`, not the framework job | A8 |
| `S59107` | `xcframework.yml` | the sibling uploads under the name `harness` | A9 |
| `S59108` | `xcframework.yml` | the sibling's environment drifts from that of `harness` | A3 |
| `S59109` | `xcframework.yml` | the sibling's copied checkout persists its credentials | A3 |
| `S59110` | `xcframework.yml` | one line of the sibling's report body changes | A7 |
| `S59111` | `xcframework.yml` | the report of `harness` is renamed | A2 |
| `S59112` | `xcframework.yml` | a moved step changes its name | A1 |

The six re-anchored rows of S33900-S33999 keep their ids, mutants and killers; only their finds and replacements
grow, each by the same lines.

## 8. What this keeps, supersedes and amends

| clause | reading |
|---|---|
| SPEC-339 R11 | kept: the collection is written by the same commit's engine |
| SPEC-339 R12, as #1060 amends it | amended: each job that runs a warm writes the warm's script once, by its Debug boot step |
| SPEC-339 R14 | amended: the Release figures are reported in the `release-and-app` artifact and summary |
| SPEC-347 R13 and A14 | the app's steps run in `release-and-app`, not in `harness`; its "no job is added" words are superseded |
| SPEC-348 R20 | its step runs in `release-and-app`; its "no job is added" words are superseded |
| SPEC-349 | kept: the planted suite runs whole in `harness` |
| SPEC-358 R8 and A14 | its step runs in `release-and-app`; its "no job is added" words are superseded |
| SPEC-361 R15 and A16 | kept: `harness` keeps 150 inside its band of 150 to 180 |
| ADR-355 D1 and ADR-378 | kept: one body for every caller; the bound rule, applied to the sibling |
