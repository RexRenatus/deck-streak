# SPEC-379: the Mini App's mutation sweep fits the bound of both web legs

- **Issue:** #693, which tracks the web sweep's growth and asks for it to be sharded; this delivery
  refers to it and closes nothing. **Context(s):** CI (`.github/workflows/`) and its tests
  (`scripts/tests/`); no crate, no Mini App source and no StrykerJS setting changes.
- **Decided by:** ADR-390 (both web legs' bound is raised by a stated rule from measured whole-app
  sweeps and held by a band; sharding, more StrykerJS workers and a smaller sweep are rejected).
- **Schematic:** none. No data flow, state or component changes: two jobs keep every step and
  only their wall-clock bound moves.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-379.md`. **Mutation band:** `S37900-S37999` (section 9). **Model:** none
  (section 8). Amends SPEC-039 (R18's bound of `mutation-web` and section 8's reading of it) and
  SPEC-362 (section 5's reading of the release's StrykerJS job), each by an amendment section in
  its own file.

## 1. The problem, measured

Two jobs run StrykerJS over the Mini App. The weekly battery's `web` job
(`.github/workflows/mutation-weekly.yml:187`) sweeps every production file, and `ci.yml`'s
`mutation-web` job (`.github/workflows/ci.yml:702`) mutates every web production file a pull
request changes, whole (SPEC-039 R3, ADR-057 D3), which for a release pull request is most of the
app. Each declares `timeout-minutes: 60` (`mutation-weekly.yml:190`, `ci.yml:704`). Every figure
below is read from the job's own log: the job's start, StrykerJS's `Initial test run succeeded`
line (where mutation testing begins), its last `Mutation testing` progress line, and the job's end.

- **The weekly sweep was cut at its bound.** Run 37924809374, attempt 2, `web` job 113826350088:
  StrykerJS instrumented 92 files with 6446 mutants; set-up to the initial test run's end was
  1.63 minutes (job 113826350088); the last progress line read 6425 of 6446 mutants tested, 58.51
  minutes after mutation testing began (job 113826350088), so 0.5464 s a mutant (job 113826350088).
  The job was cancelled for exceeding its maximum execution time, uploaded no report, and the
  battery's `survivors` job 113850137962 read `table: verdict: VOID`.
- **The larger sweep completed.** Run 37924800926, `web` job 113801075582: 97 files, 6886 mutants;
  set-up 1.44 minutes (job 113801075582); 6885 of 6886 tested 49.00 minutes after mutation testing
  began (job 113801075582), so 0.4271 s a mutant; from that line to the job's end, which writes and
  uploads the report, 4.4 s (job 113801075582).
- **The pull request's leg sweeps most of the app on a release.** The release pull request's run
  37410215011, `mutation-web` job 112096749167: 73 files, 5542 mutants; set-up 1.18 minutes (job
  112096749167); 5533 tested 38.00 minutes after mutation testing began (job 112096749167), so
  0.4121 s a mutant; the tail to the job's end, its verdict step included, 11.1 s (job
  112096749167).
- **A cut leg judges nothing.** The verdict reads StrykerJS's own report; a job cancelled at its
  bound writes none, so the leg reads VOID by name, never a pass, and no mutant of it is judged.
  At the worst measured cost the whole app's 6886 mutants need 3762 s of mutation testing alone
  (0.5464 s from job 113826350088), past the 60-minute bound before any set-up.

## 2. Requirements

R1. **The bound is the rule's.** Both legs' `timeout-minutes` is 100: the worst measured cost a
    mutant (0.5464 s, job 113826350088) times the larger measured count (6886, job 113801075582),
    one and a half times (SPEC-327 R2's margin), is 5643 s, 94.05 minutes; plus the longest
    measured set-up (1.63 minutes, job 113826350088) and the longest measured tail (0.18 minutes,
    job 112096749167), 95.87 minutes, rounded up to the five.
R2. **A band holds both legs.** `WEB_MUTATION_TIMEOUT_MINUTES` in
    `scripts/tests/test_ci_workflows.py` is 100 to 120, beside the release job's band. Its floor is R1's bound, so a leg set back to 60
    is refused; its ceiling is twice the projected whole sweep at the worst cost (64.52 minutes,
    job 113826350088's cost over job 113801075582's count), rounded down to the ten, so a hung
    test still ends within two hours and a raise past it needs a new measurement. A checker names
    each leg by its workflow file and its job, and a fixture test plants each defect into both.
R3. **Nothing that decides a verdict changes.** `web/app/stryker.config.json` (its `concurrency`,
    `timeoutMS`, `mutate` and `thresholds`, with `timeoutFactor` and `ignoreStatic` still absent),
    `scripts/mutation-verdict.py`, the weekly battery's `size`, `rust`, `python`, `rows`, `listing`,
    `survivors` and `rehearsal` jobs, every Rust leg's `timeout-minutes`, `SHARD_BOUND_SECONDS`, and
    every `--timeout` and `--build-timeout` are byte-identical to the base. Each workflow keeps its
    line count, and in each exactly one line changes.
R4. **Rows pin both bounds.** Rows `S37900` to `S37905` set each leg's bound back to 60, one past
    the ceiling, and remove it; A1 kills each. Row `S36701` keeps its mutant and its killer, and
    its anchor names the `release` job, so it still occurs once in `ci.yml` beside a second
    `timeout-minutes: 100`.
R5. **The records agree.** SPEC-039 and SPEC-362 each gain an amendment section naming this SPEC;
    no earlier byte of either changes.

## 3. Acceptance criteria of SPEC-379

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | the weekly battery's `web` job and `ci.yml`'s `mutation-web` job each declare a digit-string `timeout-minutes` inside 100 to 120 (R1, R2) | red at the base, where both are 60 | `test_both_web_mutation_legs_hold_the_whole_mini_app_sweep` |
| A2 | the checker accepts both legs at 100 and at 120, and refuses each leg at 60, at 121 and with no bound, naming that leg alone (R2) | not red: it plants its own bounds into copies of both files, so it pins the checker; rows S37900 to S37905 read A1 red | `test_a_web_leg_bound_outside_the_band_is_refused_by_its_leg` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_both_web_mutation_legs_hold_the_whole_mini_app_sweep
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_web_leg_bound_outside_the_band_is_refused_by_its_leg
```

Both are read red and green from CI's `hygiene` job, which runs `scripts/tests` through
`scripts/check.sh python`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/mutation-weekly.yml` | CI | changed: the `web` job's `timeout-minutes`, 60 to 100, one line |
| `.github/workflows/ci.yml` | CI | changed: the `mutation-web` job's `timeout-minutes`, 60 to 100, one line |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: `WEB_MUTATION_TIMEOUT_MINUTES`, `WEB_MUTATION_LEGS`, the checker and its two tests |
| `scripts/mutation-rows.d/S37900-S37999.json` | CI tests | added: rows `S37900` to `S37905` |
| `scripts/mutation-rows.d/S36700-S36799.json` | CI tests | changed: row `S36701`'s anchor and replacement name the `release` job |
| `docs/specs/SPEC-379-the-mini-app-mutation-sweep-fits-the-bound-of-both-web-legs.md` | docs | added: this file |
| `docs/decisions/ADR-390-the-web-legs-bound-is-raised-by-measurement-and-the-sweep-stays-whole.md` | docs | added: the decision |
| `docs/red-first/SPEC-379.md` | docs | added: A1's red and green, A2's not-red |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | docs | changed: an amendment section |
| `docs/specs/SPEC-362-a-releases-mutants-are-judged-in-one-run-whose-legs-fit-the-run.md` | docs | changed: an amendment section |
| `changelog.d/web-mutation-bound-379.md` | docs | added: the fragment |

## 5. What this does NOT cover

- It does not shard the web sweep. Sharding moves the verdict's reading of StrykerJS's one report
  to several, through the `survivors` job and `scripts/mutation-verdict.py`'s `judge`, `battery`
  and `table`, none of which this delivery edits; it stays #693.
- It changes no StrykerJS setting: more workers, a narrower `mutate` set and `ignoreStatic` are
  each rejected in ADR-390, and the sweep's growth stays tracked by #693.
- It changes no Rust leg's bound, `SHARD_BOUND_SECONDS`, or any per-mutant budget; making the
  census faster, which could lower that budget, is #692.
- It does not size for a later cut: a leg cut at 100 is a new measurement for #693, not a second
  raise by this rule.

## 6. Risks

- **The sweep outgrows 100 minutes.** The app grows and its mutant count with it. Detected by each
  leg's own duration in every run: a completed web leg past two thirds of the bound (66:40) is the
  signal to re-measure, and a cut at 100 is a measurement this rule did not hold (#693).
- **A hung test holds a runner longer.** A test that never ends now runs to 100 minutes before the
  job is cancelled, not 60. StrykerJS's own per-test timeout still ends a mutant's hung test long
  before that, and the band's ceiling of 120 keeps the job's wait bounded.
- **A row's anchor moves.** Two lines of `ci.yml` now read `timeout-minutes: 100`, so every row on
  either names its job; a span that no longer occurs exactly once is refused by the rows census by
  the row's stem, which detects it.
- Mutation rows: six, on the two changed lines that carry behaviour, and `S36701`'s anchor widened;
  the tests read workflow files, so they have no function of their own to mutate.

## 7. What only CI proves

A1's red and green, A2's pass, the rows' census and proof, and both legs' next whole sweep are
CI's, read by name. No test module runs off CI for this delivery.

## 8. Formal model

None. A job's wall-clock bound has no ordering, concurrency or state of its own: it decides only
whether a leg's report is written, and the verdict already reads a missing report as VOID.

## 9. Mutation rows

| stem | file | mutant | killer |
|---|---|---|---|
| `S37900-THE-WEEKLY-WEB-LEG-IS-CUT-AT-60` | `.github/workflows/mutation-weekly.yml` | the `web` job's bound is 60 again | `test_ci_workflows.TheWebMutationLegsHoldTheWholeSweep.test_both_web_mutation_legs_hold_the_whole_mini_app_sweep` |
| `S37901-THE-PULL-REQUEST-WEB-LEG-IS-CUT-AT-60` | `.github/workflows/ci.yml` | the `mutation-web` job's bound is 60 again | `test_ci_workflows.TheWebMutationLegsHoldTheWholeSweep.test_both_web_mutation_legs_hold_the_whole_mini_app_sweep` |
| `S37902-THE-WEEKLY-WEB-LEG-PASSES-THE-CEILING` | `.github/workflows/mutation-weekly.yml` | the `web` job's bound is 121 | `test_ci_workflows.TheWebMutationLegsHoldTheWholeSweep.test_both_web_mutation_legs_hold_the_whole_mini_app_sweep` |
| `S37903-THE-PULL-REQUEST-WEB-LEG-PASSES-THE-CEILING` | `.github/workflows/ci.yml` | the `mutation-web` job's bound is 121 | `test_ci_workflows.TheWebMutationLegsHoldTheWholeSweep.test_both_web_mutation_legs_hold_the_whole_mini_app_sweep` |
| `S37904-THE-WEEKLY-WEB-LEG-HAS-NO-BOUND` | `.github/workflows/mutation-weekly.yml` | the `web` job's `timeout-minutes` key is removed | `test_ci_workflows.TheWebMutationLegsHoldTheWholeSweep.test_both_web_mutation_legs_hold_the_whole_mini_app_sweep` |
| `S37905-THE-PULL-REQUEST-WEB-LEG-HAS-NO-BOUND` | `.github/workflows/ci.yml` | the `mutation-web` job's `timeout-minutes` key is removed | `test_ci_workflows.TheWebMutationLegsHoldTheWholeSweep.test_both_web_mutation_legs_hold_the_whole_mini_app_sweep` |

The rows' kills are read in CI's rows job; the test module does not run off CI for this delivery.
