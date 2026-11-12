# SPEC-367: the release job's bound holds its slowest measured run

- **Issue:** the pull request #708, whose `release` job was cancelled at its own bound; the census
  test's speed is #692. **Context(s):** CI (`.github/workflows/ci.yml`) and its tests
  (`scripts/tests/`); no crate changes.
- **Decided by:** ADR-378 (the bound is one and a half times the slowest measured release job,
  rounded up to the ten, and a band test holds it), which supersedes ADR-330's bound of 60 minutes
  and its slice matrix.
- **Status:** delivered by the pull request that adds this file, with its test and
  `docs/red-first/SPEC-367.md`. **Mutation band:** S36700-S36799. Amends SPEC-330's risk on
  the job's bound and ADR-330's decision outcome, each by an amendment line in its own file.

## 1. The problem, measured

`ci.yml`'s `release` job runs `scripts/check.sh test-release`, the workspace's tests in the release
profile, and the aggregate `ci` job needs it (SPEC-330 R1 to R3). Its `timeout-minutes` is 60
(ADR-330). The figures below are the `release` jobs of the last 100 completed `ci` runs, every
attempt, read from each run's jobs and their steps, with each job's `check-stage-logs-release`
artifact for the split inside the step (cargo's `Finished` line and nextest's `Summary` line):

- **The job runs close to its bound.** 102 jobs started the test step: 89 passed, 2 failed on a red
  test before the bound, 10 were cancelled by a newer push, and 1 was cancelled at the bound. Of the
  91 that ran to an end, the median took 51:54, the 90th percentile 54:19 and the slowest 55:45 (run
  37529482554); 58 reached 50:00 or more, and 14 reached 54:00 or more.
- **The test step is the job.** It is 97.3% of the median job, and 93.8% at the least. The seven
  steps before it take 1:21 at the median and 2:13 at most, and none of them compiles.
- **Inside the step, a release build, then the tests.** cargo's release build of the workspace took
  17:25 to 29:24 (median 27:02): the job restores the dev build's cache and pays its own release
  compile (ADR-330). The tests then took 16:09 to 25:42 (median 23:27), and one test sets that
  floor: the progression census, which compiles the workspace again in four passes of its own, took
  15:27 to 24:16 (median 22:11).
- **The bound cancelled a pull request.** Run 37660032517's first attempt (#708, one ruling file and
  one fragment) was cancelled at 60:26, "exceeded the maximum execution time". Its release build
  alone took 41:12, against 27:46 for the same crates in dev's run 37659069742, 1.48 times as long,
  and the census was still running. The second attempt of the same tree passed in 54:42, so which
  attempt is cancelled is decided by the runner's speed, not by the change.
- **The cancelled attempt needed about 81 minutes.** Its 1:57 of set-up and 41:12 of build are
  measured; its tests, at 1.48 times the same tree's 25:28 in run 37659069742, project to 37:47.
  The build and the tests slow alike: between runs 37574884226 and 37659069742 the build took 1.55
  times as long and the tests 1.54 times.
- **The job grows with the tree.** The test count rose from 1531 to 1675 across the window. The
  median job took 51:04 over the earliest 20 runs, 52:18 over the next 55 and 49:10 over the latest
  16.

## 2. Requirements

R1. **The bound holds the slowest measured run.** The `release` job's `timeout-minutes` is 100: one
    and a half times the slowest measured release job (60:26, run 37660032517, cancelled at the old
    bound before it finished), rounded up to the ten. The factor is SPEC-327 R2's margin.
R2. **A band holds the bound.** `RELEASE_TIMEOUT_MINUTES` in `scripts/tests/test_ci_workflows.py` is
    91 to 120, beside the engine and harness jobs' bands. 91 is one and a half times the slowest
    measured run rounded up to the minute, so a bound under the rule's own floor fails; 120 is twice
    the slowest measured run rounded down to the ten, so a hung test still ends within two hours and
    a raise past it needs a new measurement. A test reads the job's `timeout-minutes` as a digit
    string inside the band.
R3. **Nothing else changes.** No other job, step or line of any workflow changes: `ci.yml` keeps its
    line count, and the three lines of it that hold the per-mutant budget of 2300 seconds equal to
    `mutation-weekly.yml` and `scripts/tests/_mutants_finder.py` keep their text and their line
    numbers. The test step runs every test it ran before (SPEC-330 R2).
R4. **A row pins the bound.** Row `S36701` sets the bound back to 60, and A1's test kills it.
R5. **The records agree.** SPEC-330's risk on the job's bound and ADR-330's decision outcome each
    gain an amendment line naming ADR-378; no earlier byte of either changes.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the `release` job's `timeout-minutes` is a digit string inside 91 to 120 (R1, R2) | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_release_job_timeout_holds_its_slowest_measured_run` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_release_job_timeout_holds_its_slowest_measured_run
```

A1 is red at the base, where the bound is 60, and is read red and green from CI's `hygiene` job,
which runs `scripts/tests`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/ci.yml` | CI | changed: the `release` job's `timeout-minutes`, 60 to 100, one line |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: `RELEASE_TIMEOUT_MINUTES` and the band test |
| `scripts/mutation-rows.d/S36700-S36799.json` | CI tests | added: row `S36701` |
| `docs/specs/SPEC-367-the-release-jobs-bound-holds-its-slowest-measured-run.md` | docs | added: this file |
| `docs/decisions/ADR-378-the-release-jobs-bound-is-one-and-a-half-times-its-slowest-measured-run.md` | docs | added: the decision |
| `docs/red-first/SPEC-367.md` | docs | added: A1's red and green |
| `docs/specs/SPEC-330-the-tests-prove-their-rules-in-the-build-the-product-ships.md` | docs | changed: an amendment line |
| `docs/decisions/ADR-330-the-gate-tests-the-shipped-profile-and-refuses-build-selected-code.md` | docs | changed: an amendment line |
| `changelog.d/release-bound-367.md` | docs | added: the fragment |

## 5. What this does NOT do

- It does not make the census test faster. The census is the longest test in the step, and making
  it cheaper without skipping it is its own work, #692.
- It does not skip, filter or move any test out of the step: SPEC-330 R2 stands, #708.
- It does not change any other job's bound, the per-mutant budget, or the runner the job uses, #708.

## 6. Risks

- **The job outgrows 100 minutes.** The step grows with the workspace and with the census. Detected
  by the job's own duration in every run: a completed release job past two thirds of the bound
  (66:40) is the signal to re-measure and re-derive the bound by R1's rule, as this SPEC did at 60.
- **A hung test holds a runner longer.** A test that never ends now runs to 100 minutes before the
  job is cancelled, not 60. The band's ceiling of 120 keeps that wait bounded, and a raise past it
  is a new decision.
- Mutation rows: one, `S36701`, on the only changed line that carries behaviour; the test is a
  read of the workflow file, so it has no function of its own to mutate.
- FORMAL: not applicable. A job's bound has no ordering, concurrency or state of its own.

## 7. Mutation rows

| row | file | mutant | killer |
|---|---|---|---|
| `S36701` | `.github/workflows/ci.yml` | the `release` job's bound is 60 again | `test_ci_workflows.TheReleaseJobOutlastsItsSlowestRun.test_the_release_job_timeout_holds_its_slowest_measured_run` |
