# SPEC-500: On the release run, ci reads every mutation result as advisory and still judges every other need

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-518, beside ADR-057 D11 and ADR-290 (neither amended in kind: section 7) |
| measured at | this branch's base, `d3b09d5e` (the line numbers below are read there; section 4 names the files the change edits) |
| schematic | `docs/schematics/mutation-testing.md` section 13 (new), and a note in `docs/schematics/mutation-release-carry.md` |
| mutation band | S50000-S50099 (no hand row: section 9) |

## 1. The problem, measured

The release pull request, `dev` into `main`, is the only way a change reaches `main` (`base-is-dev`,
`.github/workflows/ci.yml:1102-1120`, refuses every other head). On that run every mutation job's result decides `ci`,
so the release cannot merge while any mutation job of its merge diff fails or is cancelled, though every change it
carries was judged on its own pull request into `dev`. What was read, at `d3b09d5e`:

- **`ci` needs 17 jobs, 8 of them mutation jobs.** The `needs:` line of `ci` (`ci.yml:1125`) names `mutation-plan`
  (`:376`), `mutation-rust` (`:522`), `mutation-python` (`:626`), `mutation-python-release` (`:673`), `mutation-rows`
  (`:721`), `mutation-verdict` (`:799`), `mutation-web-shard` (`:891`) and `mutation-web` (`:966`), and 9 other jobs:
  `rust`, `engine`, `release`, `web`, `card-sandbox`, `hygiene`, `web-engine`, `workflow-lint` and `base-is-dev`
  (`awk` over the line: 17 entries, 8 starting `mutation-`).
- **`ci`'s one step admits a skip from three legs and nothing else.** Its script (`ci.yml:1141-1154`) counts one
  admission for each of `mutation-rust`, `mutation-rows` and `mutation-python-release` whose result is `skipped`
  (SPEC-290 R8, SPEC-422 R11), then requires `success` of every other result of `RESULTS`, `join(needs.*.result, ' ')`.
  A failed or cancelled mutation job fails `ci` on every run, the release run's included.
- **The release run already has one fail-closed definition.** `release_run(event, base_ref, head_ref, head_repo, repo)`
  (`scripts/mutation-verdict.py:232-243`, SPEC-422 R1) holds exactly for a pull request from this repository's `dev`
  into `main`. `base-is-dev` reads the same five inputs from the same five expressions (`ci.yml:1107-1112`), and no job
  output names the release run to `ci`.
- **The mutation jobs report outside `ci`.** Each job keeps its log, the verdict uploads its report, and the verdict
  job uploads the run's Python outcomes as `mutation-outcomes` under `if: ${{ always() }}` (`ci.yml:866-875`), which a
  later release run's carry reads (SPEC-493 R12).
- **The weekly sweep never runs.** `mutation-weekly.yml` holds `schedule: cron "17 3 * * 1"`
  (`mutation-weekly.yml:27-28`), and GitHub runs `schedule` only from the default branch, `main` (SPEC-039 section 1).
  `main` holds two workflow files, `changelog.yml` and `ci.yml` (`git ls-tree --name-only origin/main
  .github/workflows/` at `3d77726e`), so the sweep reaches `main` only through a release pull request that can merge.
- **The test harness reads past the step it runs.** `run_base_is_dev` (`scripts/tests/test_ci_workflows.py:1175-1189`)
  cuts its script with `(?ms)^        run: \|\n(.*?)(?=^\S|\Z)` (`:1180`) inside the job's text, where `^\S` never
  matches, so it reads from the first `run: |` to the end of `base-is-dev`. A second step there would run as part of
  the first step's script, and `test_base_is_dev_admits_this_repositorys_dev_into_main` would read an exit of 127.
- **Rows pin the step's text.** Three hand rows hold anchors in `ci`'s script: S29009 (`if [ "$result" = skipped ] &&
  [ "$admitted" -gt 0 ]; then`), S29016 (`admitted=$((admitted + 1))`) and S42204 (`"mutation-rows:$ROWS_LEG"
  "mutation-python-release:$PYTHON_RELEASE_LEG"; do`), each killed by `test_ci_admits_a_not_started_leg_and_no_other_skip`.
  An anchor must occur exactly once in its file (`scripts/mutation_rows.py:950-952`).
- **Not measured here:** any hosted run. The release pull request's run and the first `dev` pull request after the
  merge are read on CI (section 7).

## 2. Requirements

R1. **`base-is-dev` names the release run.** The job gains `outputs: release: ${{ steps.release.outputs.release }}`
and a second step, after its first, with `id: release`. The first step stays byte-identical. The second reads the same
five env names from the same five expressions (`EVENT`, `BASE`, `HEAD`, `HEAD_REPO`, `REPO`) and writes
`release=true` to `$GITHUB_OUTPUT` exactly when `EVENT` is `pull_request`, `BASE` is `main`, `HEAD` is `dev`, `REPO`
is not empty and `HEAD_REPO` equals `REPO`, which is `release_run`'s predicate, and `release=false` otherwise. A
refused first step fails the job before the second runs, so the output is empty and the run is not the release run.

R2. **On the release run every mutation result is advisory.** `ci`'s step gains `RELEASE: ${{
needs.base-is-dev.outputs.release }}` and one env per mutation job it did not name yet: `PLAN_JOB`, `PYTHON_LEG`,
`VERDICT_JOB`, `WEB_SHARD_JOB` and `WEB_JOB`, beside `RUST_LEG`, `ROWS_LEG` and `PYTHON_RELEASE_LEG`. When `RELEASE`
is `true` it prints `ci: <job> reads <result>; advisory on the release run (SPEC-500)` for each of the 8 mutation
jobs, in `ci`'s needs' order, appends the same lines to `$GITHUB_STEP_SUMMARY` when that variable is set, and admits,
by value, one non-success result of `RESULTS` for each mutation job whose result is not `success`. Every other result
must be `success`: exactly the 9 other needs', and any need added later, since `RESULTS` stays
`join(needs.*.result, ' ')`.

R3. **Every other run keeps today's rule.** When `RELEASE` is not `true` (`false`, or empty because `base-is-dev`
refused), the step admits a `skipped` from `mutation-rust`, `mutation-rows` and `mutation-python-release` once each,
prints each as today, and fails on every other result that is not `success`. Its lines that the rows S29009, S29016 and
S42204 anchor stay in the script, each once in `ci.yml`, so those rows and their killer stand.

R4. **The mutation jobs are unchanged.** No mutation job, condition, matrix, report, upload or the rows job changes:
on the release run they run and report through their logs, `mutation-outcomes` and the legs' report artifacts, and on
every pull request into `dev` they gate as before. The comments over the mutation jobs and over `ci`'s step say the
release run's rule.

R5. **The harness reads one step.** `run_base_is_dev` cuts the first `run: |` block up to the next step,
`(?ms)^        run: \|\n(.*?)(?=^      - |\Z)`, so its three callers keep running the first step alone, and the
release test cuts the `id: release` step the same way.

## 3. Acceptance criteria of SPEC-500

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1: the release step, run under `bash -e` over event {pull_request, push} x base {main, dev, ""} x head {dev, feature/x, ""} x head repository {this one, a fork, ""} x repository {this one, ""}, 108 contexts, writes `release=true` exactly where `release_run` holds and `release=false` elsewhere | base-is-dev holds no step with `id: release`, so no step writes the output | ADR-518 D1 |
| A2 | R2, R3: `ci`'s step, run under its resolved shell over every need, every result {success, failure, cancelled, skipped} and the release output {true, false, ""}, alone and beside every mutation job reading the same result: on the release run it passes exactly when every need that is not a mutation job succeeded and prints the 8 advisory lines, to the step summary too when one is set; otherwise today's rule, with no advisory line | a failed mutation job on the release run exits 1 | ADR-518 D2, D3 |
| A3 | R3: today's scenarios and today's population, each with the release output `false`, keep their expectations | none: green before and after, and a planted rule change turns them red (section 6) | ADR-290, ADR-518 D2 |
| A4 | R5: the three base-is-dev cases still run the first step alone | the admit case reads 127 with the old cut over the two-step job (section 6) | ADR-518 D1 |
| A5 | R4: `ci` needs every job and always runs; the census of reads stays as it was | none: green before and after | ADR-518 D4 |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_release_output_is_release_run
A2: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_every_mutation_result_on_the_release_run_and_no_gating_one
A3: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_a_not_started_leg_and_no_other_skip
A3: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_a_skip_from_the_two_legs_and_from_no_other_need
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k OnlyThisRepositorysDevReachesMain
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_aggregate_check_needs_every_job_and_always_runs
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read
```

Every test that parses `ci.yml`'s text runs again after the change: at least `test_ci_workflows.py` and
`test_not_started_legs.py`, and `test_verdict_folds.py`, which renders the verdict step through `rendered_env`.

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/ci.yml` | CI | changed: `base-is-dev`'s output and release step; `ci`'s step env and script; the two comments |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: A1 and its step runner, the bounded cut (R5), the census's listed sites of both |
| `scripts/tests/test_not_started_legs.py` | CI tests | changed: `rendered_env` reads the scenario's release output; A2; A3's release context |
| `docs/specs/SPEC-500-on-the-release-run-ci-reads-every-mutation-result-as-advisory-and-still-judges-every-other-need.md` | docs | new: this file |
| `docs/decisions/ADR-518-the-release-run-is-named-in-base-is-dev-and-ci-admits-its-mutation-results-by-value.md` | docs | new: the decisions |
| `docs/red-first/SPEC-500.md` | docs | new: the red-first record |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | docs | changed: one insert-only amendment |
| `docs/decisions/ADR-057-mutation-testing-runs-on-the-diff-in-ci-and-weekly-on-dev.md` | docs | changed: one note |
| `docs/specs/SPEC-290-a-leg-with-nothing-to-examine-is-not-started.md` | docs | changed: one insert-only amendment |
| `docs/specs/SPEC-362-a-releases-mutants-are-judged-in-one-run-whose-legs-fit-the-run.md` | docs | changed: one insert-only amendment |
| `docs/specs/SPEC-422-the-release-pull-requests-python-legs-may-pass-the-share-up-to-one-signed-ceiling-dealt-into-matrices-of-at-most-256.md` | docs | changed: one insert-only amendment |
| `docs/schematics/mutation-testing.md` | docs | changed: section 13, `ci` before and after |
| `docs/schematics/mutation-release-carry.md` | docs | changed: one note on how `ci` reads the release run's jobs |
| `changelog.d/release-mutation-advisory-500.md` | changelog | new: the fragment |

## 5. What this does NOT cover

- Any mutation job, its condition, matrix, bound, report or upload; the rows job; the verdict; the plan's release
  mark; the weekly sweep and its legs' sizes.
- The ruleset: `ci` and `fragment` stay the required checks, and no branch rule changes.
- A pull request into `dev`, a push, or a release pull request from a fork or from any branch but `dev`: each keeps
  today's rule, and `base-is-dev` refuses the last two.
- Moving `mutation-weekly.yml` to `main`: that is the next release pull request's merge, which this change lets pass.

## 6. Risks

- **A run that is not the release reads as one.** Detected by A1 over all 108 contexts against `release_run`, and by
  A2, where an empty or `false` output keeps today's rule. The release step's output is a literal `true` or `false`
  written by the step itself; no input is written to `$GITHUB_OUTPUT`.
- **A gating need's failure hides behind a mutation job's.** Detected by A2's second context: every mutation job reads
  the same result as the need under test, so an admission that is not one for one by value lets a ninth non-success
  through and reads green where A2 expects red.
- **A change to today's rule.** A3's scenarios run unchanged with the output `false`; the three rows anchored in the
  script keep their killer.
- **The harness runs the second step as part of the first.** A4: measured on the two-step job with the old cut, the
  admit case reads 127; with the bounded cut, 0.
- **A summary write breaks the step.** A2 runs half its members with no `GITHUB_STEP_SUMMARY` set, where the step must
  still decide by the rule, and the other half with one set, where it must hold exactly the 8 lines.

## 7. What only CI proves

- The release pull request's next run: `gh run view <id> --json jobs` lists all 8 mutation jobs with their
  conclusions; `ci`'s log and its step summary carry the 8 advisory lines; `mutation-outcomes` is among the run's
  artifacts; and `ci` reads `success` when the 9 other needs succeed.
- The first pull request into `dev` after the merge: a failed mutation leg still fails `ci`.
- `workflow-lint`'s zizmor audit of the changed workflow.

## 8. Formal model

- **Lean: NOT APPLICABLE.** The admission is a multiset difference over at most 17 results, held whole by A2's
  population.
- **TLA+: NOT APPLICABLE.** Nothing interleaves: `ci` reads its needs' results once, after they finish.

## 9. Mutation rows

None. The change is workflow shell and tests, which are not production code (SPEC-039 R2). The three rows anchored in
`ci`'s script (S29009, S29016, S42204) keep their anchors once each and their killer, A3's first test. The band
S50000-S50099 holds no row.
