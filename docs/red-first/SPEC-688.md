# SPEC-688: the red-first record

The four criteria were written as one test class before the workflow change, and every other pin
that reads the `runs-on` of a long job or of the verdict moved in the same commit. Each criterion
is red at `e89e25b`, the commit that changes the test module alone, by an assertion failure on the
missing behaviour: `verdict`, `harness` and `release-and-app` still read `xcode-27`. The reds
were read by test name in this pull request's own `ci` run 43433922753, its `hygiene` job 131914070839,
from the job's `python.log`, at `9d368dd`, the head of the first push, which changes only
documents after the red commit: `git diff --quiet e89e25b 9d368dd --
.github/workflows/xcframework.yml scripts/tests/test_ci_workflows.py` reads rc 0. There
eleven tests of `test_ci_workflows.py` failed, each by an assertion and no error, and no other
test of the stage failed: the four criteria and the seven moved pins, `test_a_planted_route_is_refused_by_its_scenario`, `test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger`, `test_one_job_body_serves_the_change_and_the_tag`, `test_the_app_is_generated_tested_and_archived_in_the_harness_job`, `test_the_short_apple_jobs_reach_the_mini_only_on_this_repositorys_own_events_with_the_variable_on`, `test_the_sibling_repeats_the_harness_prefix_and_boot_then_runs_the_moved_steps`, `test_the_verdict_needs_every_other_job_and_fails_unless_each_reads_success`. Each red line
below quotes its test's first failing assertion from that log.

```red-first
A1: red at e89e25b: AssertionError: 'xcode-27' != '${{ (vars.DS_MINI == \'on\' && github.act[294 chars]' }}'
A2: red at e89e25b: AssertionError: 'xcode-27' != '${{ (vars.DS_MINI == \'on\' && vars.DS_MI[256 chars]' }}'
A3: red at e89e25b: AssertionError: Lists differ: ["harness: this repository's pull request [5537 chars]els"] != []
A4: red at e89e25b: AssertionError: Lists differ: [] != ['harness', 'release-and-app']
A1: green at 9bbe166
A2: green at 9bbe166
A3: green at 9bbe166
A4: green at 9bbe166
```

## The weakening table

Across `git diff 8773a597...HEAD`, one test is removed, SPEC-621's
`test_the_long_apple_jobs_and_the_verdict_stay_hosted_on_every_event`, and SPEC-688's four
criteria replace it; no test is renamed or skipped, and no filter, exclusion, skip list, sample or
bound is added on the test side. The pins that read the `runs-on` of a long job or of the verdict
move with it in the same commit, each named in the row below. The diff changes this test module,
one workflow and Markdown documents.

| entry | commit | ruling |
|---|---|---|
| `test_ci_workflows.py` `test_the_long_apple_jobs_and_the_verdict_stay_hosted_on_every_event` REPLACED: it held `harness`, `release-and-app` and `verdict` on `xcode-27` in every scenario; SPEC-688's criteria replace it, holding the verdict to the short jobs' expression and route, and the two long jobs to one expression that reaches the self-hosted labels only for this repository's own pull request with `DS_MINI` and `DS_MINI_LONG` both on and an actor that is not Dependabot, `xcode-27` otherwise. The pins that read those jobs' `runs-on` move with it: `test_one_job_body_serves_the_change_and_the_tag` (each job's own expression), `app_steps_problems` (the runner of `release-and-app`), `split_copy_problems` and `split_verdict_problems` (the runners of `release-and-app` and of the verdict, which SPEC-591 R3 and R8 admitted as `xcode-27` and which now read as each job's route), `admitted_runner` and `apple_job_runner` (each expression admitted to its own jobs alone), `test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger`, and SPEC-621's `test_the_short_apple_jobs_reach_the_mini_only_on_this_repositorys_own_events_with_the_variable_on` and `test_a_planted_route_is_refused_by_its_scenario` (the verdict joins the short jobs) | `e89e25b` | the owner's decision that every Apple job can reach the self-hosted runner, the verdict with the short jobs and the long jobs behind a second variable on this repository's own pull requests |

The green commit `9bbe166` carries the runs-on change, and `git diff --quiet 9bbe166 HEAD -- .github/workflows/xcframework.yml scripts/tests/test_ci_workflows.py` reads rc 0.
