# Red-first record: SPEC-422

SPEC-422 (R1 to R17, A1 to A21). The SPEC, ADR-436, the amendments to ADR-373, ADR-434, SPEC-362
and SPEC-290, the Python schematic's section 9, and the tests of A1 to A17, A20 and A21 were
committed first and alone, over stubs that keep every input but the missing behaviour:
`release_run` returns False, the `plan` verb takes `--head-ref`, `--head-repo` and `--repo` and
the `legs` verb takes `--python-release-leg`, each dropping them, `PYTHON_MATRIX_LEGS` and
`PYTHON_MATRICES` exist with their `#:` lines, `python_by_cost` takes a ceiling and ignores it, and
no `PYTHON_RELEASE_CEILING` exists, which a test that sets one patches in. Each red below is
therefore its criterion's assertion and never a missing name. The test modules run only in CI, so
every red is quoted from the pull request's checks over that commit, in the job `hygiene`, the job
that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_release_run_holds_only_for_this_repositorys_dev_into_main
A2: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_plan_records_release_only_when_the_predicate_holds
A3: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_a_release_mark_that_is_not_true_is_not_the_release_run
A4: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_plan_step_passes_head_ref_head_repo_and_repo_by_env
A5: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run
A6: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_a_ceiling_outside_its_range_is_refused_by_name
A7: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_release_run_deals_up_to_the_ceiling_and_refuses_one_leg_past_it
A8: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_a_run_that_is_not_the_release_keeps_the_share_at_any_ceiling
A9: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_count_rule_keeps_the_share_on_the_release_run
A10: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_release_legs_split_into_matrices_each_shard_once
A11: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_release_job_is_the_python_job_on_the_release_axis
A12: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_legs_judges_the_release_leg_against_the_plan
A13: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_a_not_started_leg_and_no_other_skip
A13: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_a_skip_from_the_two_legs_and_from_no_other_need
A14: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_verdict_counts_every_shard_of_both_matrices
A15: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_each_python_matrix_of_the_release_run_fits_the_run_jobs
A16: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_ceiling_cites_the_signed_ruling_only_above_the_cap
A17: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_python_matrices_equals_the_workflows_python_matrix_jobs
A18: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_matrix_limit_workflow_measures_two_matrices_in_one_run
A19: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_recorded_matrix_limit_run_ran_each_matrix_whole
A20: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_runbook_names_the_release_ceiling_its_job_and_its_refusal
A21: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_release_run_assigns_no_share_cap_bound_or_ceiling_of_its_own
```

The reds were read in job 115678147390, `hygiene` over the red commit, a job of 1185 s:
`Ran 1269 tests` and `FAILED (failures=22)`. Nineteen of the twenty-two are the tests the fence
above names for A1 to A17 and A20, every one of them and A13's two among them, while A21 passed
there; A18's and A19's tests land later. Each failing line is quoted as CI printed it, unittest's
own `[N chars]` cuts included. The three lines cut here, and A5's red line in the fence below, are
marked `[cut]` after the first line of the text they searched (FP is
`test_release_python_ceiling.TheReleaseRunIsNamedByAFailClosedPredicate`, DC is
`test_release_python_ceiling.TheReleaseRunDealsUpToItsCeiling`, EL is
`test_release_python_ceiling.EveryLegOfBothMatricesIsJudged`, CS is
`test_release_python_ceiling.TheCeilingIsSignedAndTheRunbookSaysTheMethod`, NS is
`test_not_started_legs.ALegWithNothingToExamineIsNotStarted`):

```text
test_dispatch_shards.TheReleaseFitsOneRun.test_each_python_matrix_of_the_release_run_fits_the_run_jobs :: AssertionError: 1 != 0 : mutation: shards: REFUSED: 10221 python mutant(s) need 1 leg(s) priced at most 1800 s each and 511 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
test_mutation_python_workflows.ThePythonJobsRunInCi.test_the_python_job_runs_the_plans_shards_and_the_verdict_reads_them :: AssertionError: Regex didn't match: '(?m)^    needs: \\[mutation-plan, mutation-rust, mutation-python, mutation-python-release, mutation-rows\\]$' not found in '    needs: [mutation-plan, mutation-rust, mutation-python, mutation-rows]\n[cut]
test_mutation_workflows.TheShardsAreThePlans.test_the_sharded_job_runs_the_plans_matrix_and_the_verdict_counts_every_shard :: AssertionError: Regex didn't match: '(?m)^    needs: \\[mutation-plan, mutation-rust, mutation-python, mutation-python-release, mutation-rows\\]$' not found in '    needs: [mutation-plan, mutation-rust, mutation-python, mutation-rows]\n[cut]
NS.test_ci_admits_a_not_started_leg_and_no_other_skip :: AssertionError: 'mutation-python-release' not found in ['rust', 'engine', 'release', 'web', 'card-sandbox', 'hygiene', 'mutation-plan', 'mutation-rust', 'mutation-python', 'mutation-rows', 'mutation-verdict', 'mutation-web-shard', 'mutation-web', 'web-engine', 'workflow-lint', 'base-is-dev']
NS.test_ci_admits_a_skip_from_the_two_legs_and_from_no_other_need :: AssertionError: Lists differ: ['mutation-python-release'] != []
NS.test_each_legs_job_condition_reads_the_plans_listing_alone :: AssertionError: 0 != 1 : mutation-python-release has no job-level condition
EL.test_legs_judges_the_release_leg_against_the_plan :: AssertionError: 'mutation: legs: mutation-python-release: started, success\n' not found in 'mutation: legs: mutation-rust: started, success\nmutation: legs: mutation-rows: started, success\nmutation: legs: verdict: ok\nexamined 2\n' : started
EL.test_python_matrices_equals_the_workflows_python_matrix_jobs :: AssertionError: 1 != 2
EL.test_the_release_job_is_the_python_job_on_the_release_axis :: AssertionError: 0 != 1 : the release job's job-level condition
EL.test_the_verdict_counts_every_shard_of_both_matrices :: AssertionError: 'mutation-python-release' not found in ['mutation-plan', 'mutation-rust', 'mutation-python', 'mutation-rows']
CS.test_the_ceiling_cites_the_signed_ruling_only_above_the_cap :: AssertionError: Lists differ: ['0 ceiling and 1 cap assignment(s), not one of each'] != []
CS.test_the_runbook_names_the_release_ceiling_its_job_and_its_refusal :: AssertionError: '`PYTHON_RELEASE_CEILING`' not found in ''
DC.test_a_ceiling_outside_its_range_is_refused_by_name :: AssertionError: Tuples differ: (0, 'mutation: shards: 3 python shard(s) f[443 chars]0\n') != (1, 'mutation: shards: REFUSED: the releas[94 chars]t\n')
DC.test_a_run_that_is_not_the_release_keeps_the_share_at_any_ceiling :: AssertionError: 1 != 0 : mutation: shards: REFUSED: 40 python mutant(s) need 40 leg(s) priced at most 1800 s each and 0 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
DC.test_the_count_rule_keeps_the_share_on_the_release_run :: AssertionError: 1 != 0 : mutation: shards: REFUSED: 660 python mutant(s) need 165 leg(s) priced at most 1800 s each and 0 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
DC.test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run :: AssertionError: 'mutation: shards: release run: 3 python leg(s) of ceiling 200 (PYTHON_RELEASE_CEILING): 3 in mutation-python and 0 in mutation-python-release, at most 256 a matrix\n' not found in 'mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\n[cut]
DC.test_the_release_legs_split_into_matrices_each_shard_once :: AssertionError: None != '0' : 1
DC.test_the_release_run_deals_up_to_the_ceiling_and_refuses_one_leg_past_it :: AssertionError: 1 != 0 : mutation: shards: REFUSED: 5981 python mutant(s) need 1 leg(s) priced at most 1800 s each and 299 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
FP.test_a_release_mark_that_is_not_true_is_not_the_release_run :: AssertionError: 1 != 0 : mutation: shards: REFUSED: 40 python mutant(s) need 40 leg(s) priced at most 1800 s each and 0 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
FP.test_release_run_holds_only_for_this_repositorys_dev_into_main :: AssertionError: False is not True : the release pull request
FP.test_the_plan_records_release_only_when_the_predicate_holds :: AssertionError: None is not True : the release pull request
FP.test_the_plan_step_passes_head_ref_head_repo_and_repo_by_env :: AssertionError: None != '${{ github.head_ref }}' : HEAD_REF
```

Three of the twenty-two are not a criterion's test. Each is a pin the red commit widened to name
the job `mutation-python-release`, every old member kept, so each is red for the missing job:
- `test_mutation_python_workflows.py` and `test_mutation_workflows.py` pin the needs of
  `mutation-verdict`, which do not yet name the job.
- `NS.test_each_legs_job_condition_reads_the_plans_listing_alone` ranges over the widened `LEGS`,
  so it reads the job's condition, which does not yet exist.

Each is green at the second push's head, with the job.

The greens were read in job 116416307351, `hygiene` at the second push's head, a job of 1185 s:
`Ran 1270 tests` and `FAILED (failures=6)`. Each criterion's test it held passed but A16's. A16's
test failed on its own fixture: the fixture's ruling line put a period after the figure, and the
helper reads a figure only where no word character or period touches it, so that it fails closed
on a decimal. That red is not its criterion's. The other five failures are pins outside the
criteria, each reading the needs of `ci` or `mutation-verdict`, which the green commit widened
to name `mutation-python-release`: the gate test's `mutation` list in `test_ci_workflows.py`, and
four tests of `test_verdict_folds.py`, whose driver modelled no result for the new need and whose
`JOB_AT` named the old needs:

```text
test_ci_workflows.TheGateRunsInParallelJobs.test_the_gate_runs_in_four_parallel_jobs :: AssertionError: Lists differ: ['bas[89 chars]tion-python-release', 'mutation-rows', 'mutati[119 chars]int'] != ['bas[89 chars]tion-rows', 'mutation-rust', 'mutation-verdict[92 chars]int']
CS.test_the_ceiling_cites_the_signed_ruling_only_above_the_cap :: AssertionError: Lists differ: ["docs/rulings/signed.md's first line does not hold 200 and the word release"] != []
test_verdict_folds.TheCensusJudgesEveryCommandByWhereItSits.test_every_command_form_is_driven_by_the_harness_or_refused_by_name :: AssertionError: the step reads needs.mutation-python-release.result, which no scenario models
test_verdict_folds.TheResolverReadsWhatGitHubRunsOrRefusesByName.test_every_placement_of_the_shell_is_read_correctly_or_refused_by_name :: AssertionError:   mutation-verdict:
test_verdict_folds.TheVerdictStepFailsOnEachJudgeAlone.test_the_harness_sees_a_pipe_that_loses_a_recorded_exit :: AssertionError: the step reads needs.mutation-python-release.result, which no scenario models
test_verdict_folds.TheVerdictStepFailsOnEachJudgeAlone.test_the_step_fails_when_any_one_command_alone_fails_with_any_of_its_exits :: AssertionError: the step reads needs.mutation-python-release.result, which no scenario models
```

Test edits after the red commit, each named here:
- The green commit, 73e28079dcf7bc1b9fc1358779c79ab0862ca4cd, widens `generated()` in
  `test_dispatch_shards.py` to count the release run's second Python axis, the helper A15's test
  reads, and adds A18's test to `test_release_python_ceiling.py`. It changes no assertion of a
  criterion's test.
- e68b83e2ea66b663428fd0f2d6d6a6872a3c416f re-words A16's fixture so that no period follows the
  figure, and keeps the helper's pattern. It adds assertions to A2's test (the plan file's bytes,
  each repository flag given alone, and `plan_diff` called with no release), which observe
  mutants that survived the second push, and a case to A12's test (no count and a skipped release
  leg). It widens the three pins above (the driver's results and `JOB_AT` in
  `test_verdict_folds.py`, and the gate test's `mutation` list in `test_ci_workflows.py`), every
  old member kept, and adds the census entries the new reads need. It removes no assertion.
- 44324263679920058492c2282ce43e14f5ce221a, the matrix-limit run's record, adds A19's test, its
  helper and the census entries its reads need, with the run's record under
  `scripts/tests/fixtures/`, and widens its class's docstring to name A19. It comes after every
  green below, and changes no other test.
- The rows commit, the section 4 commit, the commit that drops two sites no release test can tell
  apart, and this record's commit edit no test module.

```red-first
A1: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: False is not True : the release pull request
A1: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A2: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: None is not True : the release pull request
A2: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A3: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 1 != 0 : mutation: shards: REFUSED: 40 python mutant(s) need 40 leg(s) priced at most 1800 s each and 0 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
A3: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A4: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: None != '${{ github.head_ref }}' : HEAD_REF
A4: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A5: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 'mutation: shards: release run: 3 python leg(s) of ceiling 200 (PYTHON_RELEASE_CEILING): 3 in mutation-python and 0 in mutation-python-release, at most 256 a matrix\n' not found in 'mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\n[cut]
A5: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A6: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: Tuples differ: (0, 'mutation: shards: 3 python shard(s) f[443 chars]0\n') != (1, 'mutation: shards: REFUSED: the releas[94 chars]t\n')
A6: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A7: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 1 != 0 : mutation: shards: REFUSED: 5981 python mutant(s) need 1 leg(s) priced at most 1800 s each and 299 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
A7: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A8: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 1 != 0 : mutation: shards: REFUSED: 40 python mutant(s) need 40 leg(s) priced at most 1800 s each and 0 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
A8: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A9: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 1 != 0 : mutation: shards: REFUSED: 660 python mutant(s) need 165 leg(s) priced at most 1800 s each and 0 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
A9: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A10: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: None != '0' : 1
A10: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A11: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 0 != 1 : the release job's job-level condition
A11: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A12: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 'mutation: legs: mutation-python-release: started, success\n' not found in 'mutation: legs: mutation-rust: started, success\nmutation: legs: mutation-rows: started, success\nmutation: legs: verdict: ok\nexamined 2\n' : started
A12: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A13: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 'mutation-python-release' not found in ['rust', 'engine', 'release', 'web', 'card-sandbox', 'hygiene', 'mutation-plan', 'mutation-rust', 'mutation-python', 'mutation-rows', 'mutation-verdict', 'mutation-web-shard', 'mutation-web', 'web-engine', 'workflow-lint', 'base-is-dev']
A13: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A14: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 'mutation-python-release' not found in ['mutation-plan', 'mutation-rust', 'mutation-python', 'mutation-rows']
A14: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A15: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 1 != 0 : mutation: shards: REFUSED: 10221 python mutant(s) need 1 leg(s) priced at most 1800 s each and 511 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
A15: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A16: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: Lists differ: ['0 ceiling and 1 cap assignment(s), not one of each'] != []
A16: green at e68b83e2ea66b663428fd0f2d6d6a6872a3c416f
A17: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: 1 != 2
A17: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A18: not red: the workflow is a new file, whose absence is not its behaviour
A19: not red: the record is the measurement itself, committed after the dispatched run; its planted copy less one leg is its control, and the record reads the control refused
A20: red at 0694a4252d87b46c70e7b9a3ca5fb377909422d4: AssertionError: '`PYTHON_RELEASE_CEILING`' not found in ''
A20: green at 34f11a78d45b66785c34934701fdb2eeba5d8644
A21: not red: the base already assigns each of the four values once and reads it by name
```

No run reads the green commit alone, so each green is read at the second push's head, which
holds the green and the rows commits. A16's green is read at the third push's head, which holds
the commit that re-words its fixture and this record.

A18, A19 and A21 are not red first, because the workflow is a new file, the record is the
measurement itself, committed after the dispatched run, and the base already assigns each of the
four values once and reads it by name. Each is recorded `not red`, with that reason, and holds its
behaviour against a change.
