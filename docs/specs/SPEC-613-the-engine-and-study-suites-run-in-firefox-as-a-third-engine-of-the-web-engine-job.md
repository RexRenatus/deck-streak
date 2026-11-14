# SPEC-613: the engine and study suites run in Firefox as a third engine of the web-engine job

- **Issue:** #764 (Firefox in the engine and study suites), the follow-up to #652. **Context(s):**
  `miniapp` (`web/app`) and CI (`.github/workflows`).
- **Decided by:** ADR-635 (this SPEC's own: the two configurations' third project, the one install
  line, a server user per project, the bound, FORMAL, the pushes), which amends ADR-412 D2 for these
  two suites only.
- **Schematic:** `docs/schematics/firefox-engine-study-613.md`: the two configurations, their three
  projects, the install line, the two suite steps and the three sync users.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-613.md`. **Mutation
  band:** S61300-S61399.

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `b065fce` by `git show b065fce:<path>` and the line range
named.

- The engine suite runs in two engines: `web/app/playwright.engine.config.ts:37-40` declares the
  projects `chromium` and `webkit`, and nothing else, with one worker and no parallelism (`:32-33`).
- The study suite runs in the same two: `web/app/playwright.study.config.ts:19-22`, one worker, not
  parallel (`:14-15`). Its coverage test pins the devices `'Desktop Chrome'` and `'Desktop Safari'`
  (`web/app/src/lib/study/study-coverage.test.ts:74-77`).
- The `web-engine` job installs those two engines and no other
  (`.github/workflows/ci.yml:1091-1092`), runs the engine suite (`:1093-1094`) and then the study
  suite over the staged build (`:1103-1104`), and is bounded by `timeout-minutes: 45` (`:1013`); the
  aggregate `ci` check needs it (`:1162`).
- The engine suite's sync server has one user per project: `SYNC_USER1` is the run's base user and
  `SYNC_USER2` is `<base>-webkit` (`playwright.engine.config.ts:58-59`). The sync suite's `account()`
  gives WebKit `<base>-webkit` and every other project the base user (`web/app/tests-engine/sync.spec.ts:64-71`),
  so a third project would sync as Chromium's user, into Chromium's server collection. The server
  reads `SYNC_USER1`, `SYNC_USER2` and on until the first missing index (the pinned engine's
  `rslib/src/sync/http_server/mod.rs`, `new_from_env`, lines 94-98).
- Each spec file declares eight tests, counted as its `^test\(` and `^  test\(` lines:
  `web/app/tests-engine/engine.spec.ts`, `web/app/tests-engine/sync.spec.ts` and
  `web/app/tests-study/study.spec.ts`. Two engine tests already run in one engine each:
  `engine.spec.ts:230` skips every engine but WebKit and `:250` every engine but Chromium, so in a
  third engine 22 of the 24 run.
- SPEC-398 and ADR-412, the planted card suite in Firefox, exclude the engine and study suites and
  cite #764 (SPEC-398 lines 114-118; ADR-412 D2). ADR-412 D2 kept them on Chromium and WebKit while
  the engine's sync suite was open work in #748, which has since merged.
- The five latest successful `dev` runs of the `web-engine` job, in two engines, from the jobs API
  (durations in seconds; the job, then its engine suite step and its study suite step):

| run | job | job s | engine suite s | study suite s |
|---|---|---|---|---|
| 41796046856 | 126460294844 | 563 | 203 | 31 |
| 41772806414 | 126386179876 | 448 | 140 | 26 |
| 41745076887 | 126295917993 | 434 | 141 | 27 |
| 41649243317 | 125962723847 | 488 | 146 | 30 |
| 41616970396 | 125852731159 | 647 | 222 | 37 |

## 2. Requirements

- R1. The engine configuration declares three projects, in order `chromium`, `webkit` and
  `firefox`; the `firefox` project spreads the runner's `'Desktop Firefox'` device. It keeps one
  worker and no parallelism, so the three engines run one after another.
- R2. The study configuration declares the same three projects in the same order, each spreading
  its desktop device, with one worker and no parallelism; its coverage test pins the three devices.
- R3. The `web-engine` job installs Firefox with its system libraries on its existing install line,
  before both suites; every project either configuration declares is installed by an install step
  before the suite step that runs it. The job gains no step, and no job is added.
- R4. The two suite steps keep their commands and name the three engines. The job keeps its
  check-run name `web-engine`, its bound `timeout-minutes: 45` inside the tested band of 20 to 60,
  and the aggregate `ci` check's need of it.
- R5. Each engine project syncs as a server user of its own: the engine configuration makes one
  `SYNC_USER<n>` entry per project, numbered from 1 without a gap, all distinct, `SYNC_USER3` naming
  `<base>-firefox`; `account()` gives Chromium the base user and any other project
  `<base>-<project>`, so WebKit keeps `<base>-webkit` and no project's uploads reach another's
  server collection (SPEC-377 A1 to A3).
- R6. In Firefox every engine, sync and study test passes, except the two engine tests whose
  existing skip conditions exclude it; no test is retried, newly skipped or loosened.
- R7. The second push's `web-engine` job completes within two-thirds of its bound, and its time is
  recorded with its job id.

## 3. Acceptance criteria of SPEC-613

| id | criterion | decided by |
|---|---|---|
| A1 | the engine and study configurations each declare exactly `chromium`, `webkit` and `firefox`, in that order, each spreading its desktop device | `scripts/tests/test_ci_workflows.py` `the_engine_and_study_configurations_declare_three_engines`; the study half also by `web/app/src/lib/study/study-coverage.test.ts` "the study suite covers the review loop in both engines" |
| A2 | the `web-engine` job installs each of `chromium`, `webkit` and `firefox`, and every project both configurations declare, on an install step before the suite step that runs it | `scripts/tests/test_ci_workflows.py` `the_web_engine_job_installs_every_engine_before_each_suite` |
| A3 | each of `chromium`, `webkit` and `firefox`, and every engine project, syncs as a server user of its own, and `account()` maps each to the user of its entry | `scripts/tests/test_ci_workflows.py` `each_engine_project_syncs_as_a_user_of_its_own` |
| A4 | at the second push's head, every engine, sync and study test that runs in Firefox passes under `[firefox]`, the count equal to the tests the three spec files declare less the two whose skip conditions exclude Firefox | the Playwright suites `test:engine` and `test:study` in their `firefox` projects, read in the `web-engine` job; their engine coverage by A1's tests |
| A5 | the second push's `web-engine` job completes within two-thirds of its bound, recorded with its job id | the `web-engine` job's time from the jobs API; its bound held by `scripts/tests/test_ci_workflows.py` `the_web_engine_job_builds_the_module_and_holds_it_to_its_budget` |

The tdd probe resolves no Playwright command and no job time, so A4 and A5 name their readings in
the table and their fence lines run the tests that hold the configurations and the bound, as
SPEC-398's A5 does.

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_engine_and_study_configurations_declare_three_engines
A1: pnpm exec vitest run web/app/src/lib/study/study-coverage.test.ts -t "the study suite covers the review loop in both engines"
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_web_engine_job_installs_every_engine_before_each_suite
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k each_engine_project_syncs_as_a_user_of_its_own
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_engine_and_study_configurations_declare_three_engines
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_web_engine_job_builds_the_module_and_holds_it_to_its_budget
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/ci.yml` | CI | changed: the `web-engine` job's comment, its install line with `firefox`, and its two suite step names |
| `web/app/playwright.engine.config.ts` | `miniapp` | changed: the `firefox` project, `SYNC_USER3`, and the comments naming three projects |
| `web/app/playwright.study.config.ts` | `miniapp` | changed: the `firefox` project and the header comment |
| `web/app/tests-engine/sync.spec.ts` | `miniapp` | changed: `account()` gives each project but Chromium its own user; its doc comment |
| `web/app/tests-engine/engine.spec.ts` | `miniapp` | changed: the header comment names Firefox |
| `web/app/tests-study/study.spec.ts` | `miniapp` | changed: the header comment names Firefox |
| `web/app/src/lib/study/study-coverage.test.ts` | `miniapp` | changed: the device pin names `'Desktop Firefox'` (A1); its comments |
| `scripts/tests/test_ci_workflows.py` | CI | changed: the class `TheEngineAndStudySuitesRunInFirefox` (A1 to A3) with its constants and judges, and one named entry in the census table of non-workflow files |
| `docs/specs/SPEC-613-the-engine-and-study-suites-run-in-firefox-as-a-third-engine-of-the-web-engine-job.md` | docs | added |
| `docs/decisions/ADR-635-firefox-joins-the-engine-and-study-suites-as-a-third-project-installed-on-the-existing-line-of-the-web-engine-job.md` | docs | added |
| `docs/schematics/firefox-engine-study-613.md` | docs | added |
| `docs/red-first/SPEC-613.md` | docs | added |
| `scripts/mutation-rows.d/S61300-S61399.json` | CI | added: S61300 to S61303 |
| `changelog.d/firefox-engine-study-613.md` | docs | added |

## 5. What this does NOT cover

- It does not run the e2e or accessibility suites (`web/app/playwright.config.ts`) in Firefox: they
  check layout, flow and accessibility in the runner's default engine, and stay there (#652).
- It does not change the card configuration or the `card-sandbox` job, which already run the
  planted card suite in three engines (#652).
- It does not make `web-engine` a required context of its own on the `dev` ruleset; the aggregate
  `ci` check already needs it (#764).
- It does not add a job, a matrix or a step: Firefox joins the existing install line and the
  existing suite steps (#764).
- It does not test a learner's installed Firefox with its own preferences, extensions or policies;
  the suites run the engine the test runner installs (#764).
- It does not change the two engine tests that already run in one engine each, or any product file
  under `web/app/src` (#764).

## 6. Risks

- **Firefox fails an engine, sync or study test.** Detected by the second push's `web-engine` run,
  read under `[firefox]` by name. A Firefox failure stops the build for a design question; it is
  never skipped, retried or declared away.
- **The bound is short for three engines.** Detected by the job's own timeout, and by the second
  push's measured time against two-thirds of the bound, which stops the build for a re-size from
  that measurement.
- **Firefox's projects share a server collection with another engine.** Detected by A3's test: each
  project's user is its own entry's, and the entries are numbered without a gap, since the server
  stops at the first missing index.
- **An install line is written twice.** The card job's own Firefox step is a row anchor
  (S39800), and the rows runner needs each anchor once in its target; Firefox joins the
  `web-engine` job's existing line, which no row anchors, and the anchor census over every row on
  each touched file runs before each push.
- **Another delivery edits the `web-engine` block or the test module.** Detected by the builder's
  census of open pull requests at the cut and before each push; an overlap stops the build.

## 7. Mutation rows

The band is `scripts/mutation-rows.d/S61300-S61399.json`, in its `SCRIPT_MUTATIONS` table.

| row | file | mutant | killer |
|---|---|---|---|
| `S61300-THE-ENGINE-CONFIG-RUNS-FIREFOX` | `web/app/playwright.engine.config.ts` | the `firefox` project is removed | A1 |
| `S61301-THE-STUDY-CONFIG-RUNS-FIREFOX` | `web/app/playwright.study.config.ts` | the `firefox` project is removed | A1 |
| `S61302-THE-ENGINE-JOB-INSTALLS-FIREFOX` | `.github/workflows/ci.yml` | the install line drops `firefox` | A2 |
| `S61303-FIREFOX-SYNCS-AS-A-USER-OF-ITS-OWN` | `web/app/playwright.engine.config.ts` | `SYNC_USER3` names the Chromium user | A3 |
