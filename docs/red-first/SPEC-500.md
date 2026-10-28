# Red-first record: SPEC-500

SPEC-500 (R1 to R5, A1 to A5). The SPEC, ADR-518 and the insert-only notes in SPEC-039, ADR-057,
SPEC-290, SPEC-362, SPEC-422 and the two mutation schematics were committed first, at
a113ec4963eadbd16cea79246901cbc97ab4daee. The tests of A1 and A2, A3's release context, the
bounded cut of R5 and the census's listed sites of both new tests were committed next and alone,
at c70e9aab7de1ced0889bed8f0a461293375a27b6, over the base's `ci.yml`. Each red is therefore its
criterion's assertion over today's workflow and never a missing name. The workflow change is
bf69f0364509572c25bcb2e3f76c35f35e6b74ef.

The runs below were read locally, with Python 3.12.3, from the repository root, one
`python3 -m unittest discover -s scripts/tests -p <file>` line per file as `scripts/check.sh` runs
them; the pull request's checks read them again.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_release_output_is_release_run
A2: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_every_mutation_result_on_the_release_run_and_no_gating_one
A3: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_a_not_started_leg_and_no_other_skip
A3: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_a_skip_from_the_two_legs_and_from_no_other_need
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k OnlyThisRepositorysDevReachesMain
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_aggregate_check_needs_every_job_and_always_runs
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read
```

Over the tests' commit, `test_ci_workflows.py` ran 69 tests with 9 failures and
`test_not_started_legs.py` ran 9 with 1: A1's and A2's tests, each red for its criterion's reason,
and the 8 failures the base already shows by the same names (below). Over the workflow change,
`test_ci_workflows.py` ran 69 with those 8 failures alone, `test_not_started_legs.py` ran 9 with
none, and `test_verdict_folds.py`, which renders a step through `rendered_env`, ran 7 with none.
A1 examined 108 release contexts and A2 408 need results.

```red-first
A1: red at c70e9aab7de1ced0889bed8f0a461293375a27b6: AssertionError: base-is-dev holds 0 steps with `id: release`, so no step writes the release output
A1: green at bf69f0364509572c25bcb2e3f76c35f35e6b74ef
A2: red at c70e9aab7de1ced0889bed8f0a461293375a27b6: AssertionError: 1 != 0 : mutation-plan failure, release 'true', alone, a step summary set:
A2: green at bf69f0364509572c25bcb2e3f76c35f35e6b74ef
A3: not red: today's scenarios and population run with the release output `false` and keep their expectations, green before and after the change
A4: not red: the bounded cut reads the base's one-step job as the old cut did; over the two-step job the old cut reads the admit case's exit as 127 (below)
A5: not red: `ci` needs every job and always runs before and after the change, and the census of reads lists the new tests' sites
```

A4 was measured on the workflow change with the old cut, `(?ms)^        run: \|\n(.*?)(?=^\S|\Z)`,
put back in `run_base_is_dev` for that run alone: `test_base_is_dev_admits_this_repositorys_dev_into_main`
read `AssertionError: 127 != 0 : base-is-dev: ok (pull_request into 'main')`, because the release
step's YAML ran as part of the first step's script, while the fork and other-branch cases kept
their exit of 1. With the bounded cut, the three cases pass.

The 8 failures the base shows, before and after the change, by name, none of them this
delivery's: `test_the_app_is_generated_tested_and_archived_in_the_harness_job`,
`test_one_job_body_serves_the_change_and_the_tag`, `test_the_card_probe_suite_runs_on_both_simulators`,
`test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger`,
`test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read`, and three
controls of `test_the_census_is_red_on_every_planted_site`. The census's failures name 3 problems,
each in the formal ledger's test or module and none in a site this delivery adds, so A5's census
test reads the base's red there and no new problem.

## The job output's wiring (R1)

A1's scenario tests set `needs.base-is-dev.outputs.release` as a value, so no test read the line
of `ci.yml` that wires the job output to the release step: a mutant that reads another output of
the step, `steps.release.outputs.released`, and one that reads another step id,
`steps.release-run.outputs.release`, each passed every test that reads `ci.yml`.
`TheReleaseRunIsNamedInBaseIsDev.test_the_job_output_reads_the_step_that_writes_release` now reads
the job through the tests' own loader and holds the wiring closed: the job's `outputs` map holds
exactly the key `release`, its value is exactly `${{ steps.<id>.outputs.release }}`, exactly one
step of the job carries that id, and that step's one `GITHUB_OUTPUT` line writes `release=`. It was
committed alone, with `ci.yml` unchanged, at 3721a581fe5954f3e70b79d6ce316a73d815c0dd, and read
`examined 1 base-is-dev outputs` and `examined 2 base-is-dev steps`.

```text
python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_job_output_reads_the_step_that_writes_release
```

Each mutant was applied to a scratch copy of the tree at that commit and the test run alone:

```text
steps.release.outputs.released: red: AssertionError: unexpectedly None : base-is-dev's release output reads '${{ steps.release.outputs.released }}', not a step's release output
steps.release-run.outputs.release: red: AssertionError: 0 != 1 : base-is-dev's release output reads step 'release-run', and 0 of its steps carry that id
unmutated: green: Ran 1 test, OK
```

The 37 hand-written mutants of this delivery's `ci.yml` lines were then run over
`test_not_started_legs.py` and `test_verdict_folds.py` whole and `test_ci_workflows.py`'s two
base-is-dev classes, after an unmutated control that read green: 37 examined, 35 killed, 2
equivalent, 0 escapes. The two equivalent mutants change the release step's predicate without
changing its value on any event the workflow runs on: `!= push` for `= pull_request`, where `on:`
holds only those two events, and `-n "$HEAD_REPO"` for `-n "$REPO"`, beside
`[ "$HEAD_REPO" = "$REPO" ]`. The new test also kills the two mutants that rename the release
step's id or its output key. Over the commit, `test_ci_workflows.py` ran 70 tests with the same 8
failures by name as above, `test_not_started_legs.py` ran 9 with none and `test_verdict_folds.py`
ran 7 with none; the census of reads names the same 3 problems, as the new test reads `ci.yml`
only through the loader.
