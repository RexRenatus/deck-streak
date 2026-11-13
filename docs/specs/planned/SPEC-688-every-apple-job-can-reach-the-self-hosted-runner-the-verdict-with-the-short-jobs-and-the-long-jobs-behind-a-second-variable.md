# SPEC-688: every Apple job can reach the self-hosted runner, the verdict with the short jobs and the long jobs behind a second variable

- **Issue:** none. **Context(s):** the repository's continuous integration: the Apple job body
  (`.github/workflows/xcframework.yml`) and the workflow hardening test
  (`scripts/tests/test_ci_workflows.py`).
- **Decided by:** ADR-708 (the long Apple jobs reach the self-hosted runner for this repository's
  own pull requests behind a second variable).
- **Status:** planned
- **Schematic:** none: no component, data flow or state changes shape; three values of one
  workflow become expressions.

## 1. The problem, measured

`.github/workflows/xcframework.yml` holds six jobs. The three short jobs, `xcframework`,
`harness-wire` and `card-isolation`, hold SPEC-621's expression: the labels `self-hosted`,
`macOS`, `ARM64` and `ds-mini` when the repository variable `DS_MINI` is `on`, the actor is not
`dependabot[bot]` and the event is a push, a dispatch or a pull request from this repository,
`xcode-27` otherwise. `harness`, `release-and-app` and `verdict` hold `runs-on: xcode-27` on every
event (SPEC-621 R3). Four callers run the file: `apple-on-change.yml` on a pull request into dev,
`apple-on-tag.yml` on a release tag, and the two upload lanes, `testflight-internal.yml` and
`testflight-release.yml`, on a push or a dispatch. A called workflow reads its caller's event, so
one expression for every job would also move the upload lanes' and the tag's long jobs.

The long jobs' hosted wall times, read from the forge's records of completed runs on `xcode-27`
(durations only):

| job | wall time | run | job id |
|---|---|---|---|
| `harness` | 117m07s | 43146170780 | 130948690666 |
| `harness` | 115m09s | 43191874176 | 131098502329 |
| `harness` | 110m32s | 43290095817 | 131441786878 |
| `harness` | 85m55s | 42541868085 | 128941158772 |
| `release-and-app` | 85m02s | 42497754697 | 128811876431 |
| `release-and-app` | 80m29s | 43191874176 | 131098502470 |
| `release-and-app` | 50m47s | 43290095817 | 131441786904 |
| `release-and-app` | 48m27s | 43121315660 | 130872110099 |

Hosted, the two long jobs of a run run side by side. Run one after the other, the same two jobs
would take 195m38s in run 43191874176 (jobs 131098502329 and 131098502470) and 161m19s in run
43290095817 (jobs 131441786878 and 131441786904).

## 2. Requirements

R1. **The verdict routes with the short jobs.** The `runs-on` of `verdict` is the short jobs'
    expression byte for byte, so it reaches the self-hosted labels exactly when they do; it needs
    no Apple tool, only the results of the jobs it needs.

R2. **The long jobs take one expression of their own.** The `runs-on` of `harness` and of
    `release-and-app` is one expression, the same in both: the labels `self-hosted`, `macOS`, `ARM64`, `ds-mini` and `ds-mini-sim` when `DS_MINI` and a
    second repository variable, `DS_MINI_LONG`, both read `on`, the actor is not
    `dependabot[bot]` and the event is a pull request whose head repository is this repository;
    `xcode-27` otherwise.

R3. **Every other run keeps the long jobs hosted.** A push, a release tag push, a dispatch and a
    fork's pull request run both long jobs on `xcode-27` whatever the variables read, so every
    run of the upload lanes and of the tag keeps them hosted; with `DS_MINI_LONG` unset or off,
    every event does.

R4. **Each expression is admitted to its own jobs alone.** The hardening test admits the short
    expression to `xcframework`, `harness-wire`, `card-isolation` and `verdict`, and the long
    expression to `harness` and `release-and-app`, each by file and job, and refuses either under
    any other job or file.

R5. **Nothing else of the workflow changes.** Its environment lines (`DEVELOPER_DIR`,
    `IPHONE_SIM`, `IPAD_SIM` and `SIM_OS`), its steps and its comments stay byte for byte on both
    routes, and no caller changes: the change is the three `runs-on` values alone.

This SPEC amends SPEC-621 insert-only: its R3 now reads as R1 to R3 here, and its criterion that
the long jobs and the verdict resolve to `xcode-27` in every scenario is replaced by this SPEC's
criteria, which SPEC-621 records in an appended section. SPEC-591 R3's and R8's admitted runner
for `release-and-app` and `verdict` now reads as each job's route here; SPEC-591 itself is not
edited. The two exceptions SPEC-621 inserted in other SPECs, SPEC-002's section 9 (the one
expression admitted to the three short jobs alone, R10 standing for every other job) and the
amendment under SPEC-336's section 8 (that expression admitted to those three jobs alone and
refused under any other job), now read with R4 here: the short expression admitted to four jobs
and the long expression to two. Neither SPEC is edited.

## 3. Acceptance criteria of SPEC-688

Each criterion is one test of the class `EveryAppleJobReachesTheRunnerOnlyBehindItsVariables` in
`scripts/tests/test_ci_workflows.py`, named in the block below. The scenarios are every event a
caller can hand the file (a push, a release tag push, a dispatch, this repository's pull request,
a fork's pull request and a scheduled run), every value of `DS_MINI` and of `DS_MINI_LONG` (`on`,
`ON`, unset and `off`) and both actors (a person and `dependabot[bot]`).

| criterion | what it decides | test |
|---|---|---|
| A1 | the `runs-on` of `verdict` equals the short jobs' expression byte for byte and resolves as theirs in every scenario (R1) | `test_ci_workflows.py` |
| A2 | `harness` and `release-and-app` hold the long expression, the same in both, and over every scenario, with the scenario count and the routed counts asserted, reach the labels `self-hosted`, `macOS`, `ARM64`, `ds-mini` and `ds-mini-sim` only for this repository's pull request with both variables on and a person as actor, `xcode-27` otherwise (R2, R3) | `test_ci_workflows.py` |
| A3 | planted copies of the long expression without the `DS_MINI` clause, without the `DS_MINI_LONG` clause, without the Dependabot clause, without the same-repository clause, with another label list, and with the short expression in its place, are each refused by a scenario (R2, R3) | `test_ci_workflows.py` |
| A4 | the jobs that hold each expression are exactly its own, and the hardening test admits each expression to its own jobs alone and refuses it under any other job or file (R4) | `test_ci_workflows.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_verdict_holds_the_short_expression_and_routes_with_the_short_jobs
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_long_jobs_reach_the_runner_only_on_own_pull_requests_with_both_variables_on
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_planted_long_route_is_refused_by_its_scenario
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_apple_expression_is_admitted_to_its_own_jobs_alone
```

The pins that read a long job's or the verdict's `runs-on` move with them: `admitted_runner` and
`apple_job_runner` take the long expression for the two long jobs, the verdict joins the short
jobs by file and job, and `test_one_job_body_serves_the_change_and_the_tag`, `app_steps_problems`,
`split_copy_problems` and `split_verdict_problems` read each job's route. SPEC-621's test that held
the long jobs and the verdict hosted on every event is replaced by this SPEC's criteria. R5 is
held by the diff: the three `runs-on` lines are the change's only workflow lines.

## 4. File manifest

| path | change |
|---|---|
| `docs/specs/SPEC-688-every-apple-job-can-reach-the-self-hosted-runner-the-verdict-with-the-short-jobs-and-the-long-jobs-behind-a-second-variable.md` | new |
| `docs/decisions/ADR-708-the-long-apple-jobs-reach-the-self-hosted-runner-for-this-repositorys-own-pull-requests-behind-a-second-variable.md` | new |
| `docs/specs/SPEC-621-the-short-apple-jobs-run-on-the-self-hosted-runner-only-when-the-repository-variable-is-on.md` | amended insert-only: one appended section |
| `docs/decisions/ADR-643-the-short-apple-jobs-move-to-a-self-hosted-runner-behind-a-repository-variable-and-the-long-jobs-stay-hosted.md` | amended insert-only: one appended section |
| `scripts/tests/test_ci_workflows.py` | changed: the routing pins, the runner admission by file and job, and the new class |
| `.github/workflows/xcframework.yml` | changed: the `runs-on` of `verdict`, `harness` and `release-and-app` |
| `docs/red-first/SPEC-688.md` | new |
| `changelog.d/apple-long-jobs-mini-688.md` | new |

## 5. What this does NOT cover

- Setting `DS_MINI` or `DS_MINI_LONG`: both are set outside the tree, and until `DS_MINI_LONG`
  reads `on` both long jobs run where they run today, so the change lands inert for them.
- Registering or keeping the self-hosted runner, its toolchain, its simulator runtime or its
  devices: that is done outside the tree, and no file here names a machine.
- The header comment of `xcframework.yml` and the comments over its simulator names, which word
  the hosted route: they stay byte for byte here (R5), and a later change rewords them.

## 6. Risks

- **The routing is not a boundary by itself.** A pull request can edit the workflow file, so the
  same-repository clause and the repository's approval rule for outside contributors together keep
  a fork's pull request off the self-hosted runner, as SPEC-621 states.
- **A runner image that lacks what the long jobs name.** The environment lines stay as they are,
  so a red set-up step on the self-hosted runner is a runner-image fault, never a test red; the
  trial reads the set-up steps first (section 7).
- **Long jobs one after the other.** On the self-hosted runner both long jobs may run one after
  the other, so a pull request's Apple run can take their sum (section 1) or more; the trial
  measures it.
- **A Dependabot pull request.** It comes from this repository, so the actor clause keeps it
  hosted.

## 7. The trial's read plan

`DS_MINI_LONG` is the trial switch, set outside the tree, so this change lands inert for the long
jobs. The trial is this change's own pull request run with both variables on, read in this order:

1. Each long job's set-up steps first, by name, from `this run's XCFramework, bindings and
   synthetic collection` to `each simulator booted and the harness launched once, before its
   tests`: the toolchain path, both simulator names and the simulator OS the environment lines
   name must exist on the self-hosted runner, and a red before any test is a runner-image fault,
   never a test red.
2. Every job by job name and by destination (`IPHONE_SIM` and `IPAD_SIM`), and the card view's
   planted suite in `harness` by its two passes by name, the iPhone pass and then the iPad pass.
3. Each route's Xcode build, read from the run's own logs or result bundles, since the
   self-hosted runner's Xcode may run ahead of the hosted image's.
4. Both long jobs' wall times on the self-hosted runner beside the hosted ones of section 1, and
   their sum with both long jobs running one after the other on the self-hosted runner.
5. A red simulator pass there blocks no land: the change lands on its hosted reads,
   `DS_MINI_LONG` stays off so both long jobs stay hosted with no commit, and that red is the
   trigger of the parked follow-up for that pass. A green trial leaves `DS_MINI_LONG` on.
6. The steady state, `DS_MINI` on and `DS_MINI_LONG` off, in which the hosted long jobs link a
   framework the self-hosted runner built, gets its own read by job name before `DS_MINI` stays
   on.
