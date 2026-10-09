# SPEC-382: every Apple CI run records its own timing, and uploads it whatever the outcome

- **Issue:** none. No open issue holds exactly this work, so none closes and none is referenced.
  **Context(s):** none (the workflows and their census, not a bounded context).
- **Decided by:** ADR-393 (each run records and uploads its own timing), ADR-394 (the repository
  keeps no timing history, baseline or budget), ADR-395 (the first simulator boots in a timed step
  of its own) and ADR-396 (the framework job pins its Xcode as the other jobs do). It keeps
  ADR-355 D1 (one job body, called by a change caller and a tag caller) and D3 (the Apple build is
  advisory).
- **Schematic:** none. No job is added, renamed or re-ordered, so the job graph
  `docs/schematics/apple-build-on-change-and-on-tag.md` draws is unchanged; the delivery adds steps
  inside two jobs and lines inside existing steps.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-382.md` (ADR-016). **Mutation band:**
  `S38200-S38299` (section 7). **Model:** none (section 9).

## 1. The problem, measured

Each statement below is read from `.github/workflows/xcframework.yml` at dev `3ca06142`, by the
command beside it. This text states the mechanism; the durations are on the runs' own pages.

```
git grep -n -e xcodebuild 3ca06142 -- .github/workflows/xcframework.yml
git grep -c -e -showBuildTimingSummary 3ca06142 -- .github/workflows/xcframework.yml
git grep -c -e card-probe-seconds -e 'cases_in("card-probe")' 3ca06142 -- .github/workflows/xcframework.yml
git grep -n -e 'DEVELOPER_DIR:' 3ca06142 -- .github/workflows/xcframework.yml
git grep -c -e simctl 3ca06142 -- .github/workflows/xcframework.yml
```

1.1 **A run does not record all of its own timing.** The file holds nine `xcodebuild` lines: seven
builds, tests or the archive, the planted suite's test, and the framework assembly
(`xcodebuild -create-xcframework`). None of them prints its build timing summary: the second
command reads no match. The planted-suite step ("the card view's planted suite, Debug, on the
iPhone and then the iPad") has no timer, and the report never reads its result bundle's tests:
the third command reads no match, while the report calls `cases_in` for the Debug, Release, app,
review and review-actions bundles. The static-library step has one timer for both targets. Per-test
durations exist only inside the result bundles. The fourth command names three lines, one in each of
`harness-wire`, `card-isolation` and `harness`, so the framework job builds with the image's
default Xcode while the other three jobs set the developer directory.

1.2 **The first simulator boots silently inside the first test step.** The last command reads no
match: no step boots a simulator, so the first test step's boot cannot be told apart from
installing and launching the tests.

## 2. Requirements

R1. The planted-suite step ("the card view's planted suite, Debug, on the iPhone and then the
    iPad") writes its elapsed seconds to the report directory, timed as the other test steps are,
    and the report gains its row and reads its result bundle's tests as it reads the others'. The
    step's `xcodebuild` command is byte-identical to the base's: only the timer lines around it
    change.
R2. Every other `xcodebuild` build, test or archive invocation in the body passes
    `-showBuildTimingSummary`. The framework assembly (`xcodebuild -create-xcframework`) and version
    reads are not builds and take no flag.
R3. The static-library step records each target's seconds in a file of its own as well as the
    total. Its cargo command changes only by an optional `--timings` placed after the target
    argument, and that timings report is uploaded with the framework job's timing record.
R4. In `harness`, after "the project, generated", a separate step boots the first simulator of the
    first test step, waits until it has booted and writes its seconds. It fails unless exactly one
    available device matches the pinned iPhone name and OS. The iPad is booted by `xcodebuild`, as
    before, and never early.
R5. The framework job's environment sets the developer directory to the same Xcode path the other
    jobs set.
R6. The framework and `harness` jobs each write `timing.json`, schema `ci-timing/1`: every seconds
    file the job wrote, each test's duration per destination, `xcodebuild -version` and `sw_vers`.
    Each job uploads it as an artifact of its own, `timing-xcframework` and `timing-harness`, with
    `if: ${{ always() }}`, placed after the job's existing upload and using the same pinned upload
    action.
R7. No test, destination, configuration, pass, wait, timeout, bound, skip or retry changes. No job
    is added or renamed, and no context, secret or permission is added.
R8. The census gains a class, `TheRunRecordsItsOwnTiming`, that refuses, each by a planted
    negative: an `xcodebuild` other than the planted suite's without the timing summary; a test
    step without its timer; an uploaded result bundle the report does not read. The same class
    holds the planted suite's command equal to its base text.
R9. This SPEC amends SPEC-361 R14 ("no workflow change but R15's") and the freeze words of SPEC-361
    R15 to admit exactly R1 to R6's lines. SPEC-361 R15's bound and R16's load wait are kept
    unchanged.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the planted-suite step writes its seconds and the report reads its bundle's tests; its command equals the base text, and a planted change to it is refused | `test_ci_workflows.py` `TheRunRecordsItsOwnTiming.test_the_planted_suite_is_timed_and_its_command_is_unchanged` |
| A2 | every `xcodebuild` build, test or archive other than the planted suite's passes the timing summary; a planted invocation without it is refused by its step name | `test_ci_workflows.py` `TheRunRecordsItsOwnTiming.test_every_other_xcodebuild_prints_its_build_timing_summary` |
| A3 | each static-library target writes its seconds and the total is kept; the cargo command keeps `--locked` | `test_ci_workflows.py` `TheRunRecordsItsOwnTiming.test_each_static_library_target_is_timed` |
| A4 | one boot step follows "the project, generated", boots the first simulator only, waits for it and writes its seconds | `test_ci_workflows.py` `TheRunRecordsItsOwnTiming.test_the_first_simulator_boots_in_its_own_timed_step` |
| A5 | the framework job sets the developer directory to the path the other jobs set | `test_ci_workflows.py` `TheRunRecordsItsOwnTiming.test_the_framework_job_pins_the_developer_directory` |
| A6 | each of the two jobs uploads its timing record under `always()`, after its existing upload, with the pinned upload action | `test_ci_workflows.py` `TheRunRecordsItsOwnTiming.test_each_job_uploads_its_timing_record_whatever_the_outcome` |
| A7 | every result bundle a job uploads is read by that job's report; a planted unread bundle is refused | `test_ci_workflows.py` `TheRunRecordsItsOwnTiming.test_every_uploaded_bundle_is_read_by_the_report` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_planted_suite_is_timed_and_its_command_is_unchanged
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_other_xcodebuild_prints_its_build_timing_summary
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_static_library_target_is_timed
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_first_simulator_boots_in_its_own_timed_step
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_framework_job_pins_the_developer_directory
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_job_uploads_its_timing_record_whatever_the_outcome
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_uploaded_bundle_is_read_by_the_report
```

The red each must show first, at the base:

- A1: `AssertionError` naming the missing planted-suite timer (the timer assertion runs before the
  command comparison).
- A2: `AssertionError` naming the first step whose `xcodebuild` lacks the timing summary.
- A3: `AssertionError` naming the missing per-target seconds file.
- A4: `AssertionError`: no step boots a simulator after "the project, generated".
- A5: `AssertionError`: the framework job's environment does not set the developer directory.
- A6: `AssertionError` naming the missing `timing-harness` upload.
- A7: `AssertionError` naming the planted suite's result bundle, uploaded and not read by the
  report.

## 3a. Read on the pull request's own run

No criterion in the fence; each is read on the run by its check name. The run is green; both timing
artifacts exist; every seconds file the record names is present; the per-target files sum to the
total; the planted-suite timer agrees with the step's own duration; the identity names the pinned
Xcode; the Release size and imports equal the previous run's at the same engine tree.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/xcframework.yml` | none (CI) | changed: R1 to R6 |
| `scripts/tests/test_ci_workflows.py` | none (tests) | changed: the class `TheRunRecordsItsOwnTiming` (A1 to A7); `APP_REPORT_NEEDS` grows to a superset that holds the report's new rows |
| `scripts/mutation-rows.d/S38200-S38299.json` | none (rows) | added: section 7's rows |
| `docs/specs/SPEC-382-every-apple-ci-run-records-its-own-timing-and-uploads-it-whatever-the-outcome.md` | docs | added: this SPEC |
| `docs/decisions/ADR-393-each-apple-ci-run-records-and-uploads-its-own-timing.md` | docs | added: the timing record |
| `docs/decisions/ADR-394-the-repository-keeps-no-timing-history-baseline-or-budget.md` | docs | added: no timing history in the repository |
| `docs/decisions/ADR-395-the-first-simulator-boots-in-a-timed-step-of-its-own.md` | docs | added: the booted first simulator |
| `docs/decisions/ADR-396-the-framework-job-pins-its-xcode-as-the-other-jobs-do.md` | docs | added: the framework job's Xcode pin |
| `docs/specs/SPEC-361-the-containment-layer-for-card-scripts-on-iphone-and-ipad.md` | docs | changed: an appended, insert-only amendment naming R9 |
| `docs/red-first/SPEC-382.md` | docs | added |
| `changelog.d/ios-timing-record-382.md` | docs | added |

Unchanged: `ci.yml`, `release.yml`, the four caller workflows (`apple-on-change.yml`,
`apple-on-tag.yml`, `testflight-internal.yml`, `testflight-release.yml`), the repository's ruleset
files, `deny.toml`, `Cargo.lock`, `Cargo.toml` and `deploy/**`.

## 5. What this does NOT do

- It moves neither the Debug tests nor the planted suite out of `harness` (#616).
- It does not shard the planted suite, split it below the method, or run both simulators at once in
  any step (#651).
- It changes no wait, bound or load-wait rule; SPEC-361 R15's bound and R16's load wait stand as
  written (#651).
- It caches nothing: no built framework, no derived data, no Swift compilation and no Rust artifact
  (#622).
- It adds no context to any required set and makes no Apple check required (ADR-355 D3; #622).
- It keeps no timing history, baseline or budget in the repository, and adds no step that fails on
  elapsed time (ADR-394; #622).
- It changes no runner label, image or pinned toolchain; the framework job's developer directory
  names the Xcode the other jobs already pin (#622).
- It changes neither release lane's own jobs (`testflight-internal.yml`, `testflight-release.yml`),
  their secrets nor how many bodies a tag starts (#634).
- It changes no test body; making a slow test faster inside itself is test work, not CI work
  (#616).

## 6. Risks

- **The timing artifacts are public, as the run's logs are.** They hold only the hosted runner's
  durations of that run and the versions already printed in its log. Detected by reading the first
  run's two artifacts against its log.
- **The timing summary adds output to a measured step.** The Release measurements' step gains the
  summary; the Release size and imports come from the no-test Release build and are compared with
  the previous run's at the same engine tree (section 3a). A difference is read before merge.
- **The boot step selects the wrong device, or none.** It fails unless exactly one available device
  matches the pinned iPhone name and OS, so a missing or doubled device fails its own step by name
  rather than the first test step.
- **A result bundle is missing when the record is written.** The record step runs whatever the
  outcome, so after a failed test step a bundle may be absent; the record then holds no durations
  for it and the upload still runs.

## 7. The mutation rows

One band, `scripts/mutation-rows.d/S38200-S38299.json`, table of script mutations, each row
`[id, target, find, replace, why, killer]` and each `find` occurring exactly once in its target.
Rows that already anchor on a touched line keep their finds, or are re-anchored in the same commit
with their ids kept.

| row | target | mutant | killer |
|---|---|---|---|
| S38200 | `.github/workflows/xcframework.yml` | the timing summary removed from the Debug tests' `xcodebuild` | A2 |
| S38201 | `.github/workflows/xcframework.yml` | the timing summary removed from the archive's `xcodebuild` | A2 |
| S38202 | `.github/workflows/xcframework.yml` | the planted-suite timer's write removed | A1 |
| S38203 | `.github/workflows/xcframework.yml` | the planted suite's command gains the timing summary | A1 |
| S38204 | `.github/workflows/xcframework.yml` | the per-target seconds write removed | A3 |
| S38205 | `.github/workflows/xcframework.yml` | the boot step's wait removed | A4 |
| S38206 | `.github/workflows/xcframework.yml` | the framework job's developer directory removed | A5 |
| S38207 | `.github/workflows/xcframework.yml` | the `timing-harness` upload's `always()` removed | A6 |
| S38208 | `.github/workflows/xcframework.yml` | the report's read of the planted suite's bundle removed | A7 |

## 8. What this keeps and what it amends

| clause | reading |
|---|---|
| SPEC-344 R1, R4, R5 and R6 | kept: one body for the change and the tag, the same steps on both, no concurrency block, advisory, pinned, `--locked` |
| ADR-355 D1 and D3 | kept: one body called by two callers; the Apple build is advisory |
| SPEC-190 R1, R4 and R9 to R12 | kept: the pull-request group on the change caller, no cancelled push or tag run, no job-level concurrency, tag runs queue |
| SPEC-336 R6 and R8 | kept: the static libraries are built in release and clocked, now per target as well as in total; the framework artifact carries the build time |
| SPEC-339 R11 | kept: the synthetic collection is written by the same commit's engine |
| SPEC-339 R12 and R14 | kept: `harness` measures and reports as before; its report gains rows and keeps every row it had |
| SPEC-347 R13 and A14 | kept: the app's tests step and its place are unchanged; its command gains only the timing summary |
| SPEC-348 R20 | kept: the review screen's tests step is unchanged but for the timing summary on its command |
| SPEC-352 R6, R16, A17 and A20 | kept: the lanes call the body unchanged, the lane files cache nothing, the app job's prefix equals the harness's (every new `harness` step sits after "the project, generated") |
| SPEC-355 R13 | kept: no new job; the planted step still runs the `CardProbe` scheme whole |
| SPEC-361 R14 and R15 | the freeze words are amended by R9 to admit exactly R1 to R6's lines; R15's bound is kept |
| SPEC-361 R16 | kept: the load wait |

## 9. Formal model

None. The delivery changes workflow text and the census that reads it: it adds no actor, store or
state, so there is no model to write or check.
