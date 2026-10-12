# Red-first record: SPEC-417

SPEC-417 (R1 to R14, A1 to A12). The SPEC, ADR-431, schematic Part B, the amendments to SPEC-039
and SPEC-362, the tests of A1 to A12 with the ceiling's pins at 193, and stubs in
`scripts/mutation_rows.py` that parse every new verb and flag, carry the three constants and answer
without the behaviour, are committed first, in the red commit, over a `ci.yml` whose rows job is one
job and a `scripts/mutation-verdict.py` whose `LEG_CEILING["ci"]` is still 209.
The test modules run only in CI, so every red is quoted from the pull request's checks over the red
commit, job `hygiene`, the job that runs `scripts/tests`, and joins this record with its green line
in the commit that carries the rows.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_fewest_legs_whose_largest_leg_fits_the_bound
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_population_past_the_most_legs_is_refused_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_legs_reach_the_workflow_as_a_count_and_a_matrix
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_each_leg_proves_its_dealt_rows_and_the_legs_cover_the_plan_once
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_malformed_leg_is_refused_before_any_row_runs
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_merge_writes_every_legs_rows_and_names_a_missing_leg
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_merge_refuses_a_row_the_deal_did_not_give_its_leg
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_rows_legs_run_as_planned_and_the_verdict_merges_them_first
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_rows_bound_is_half_of_the_rows_jobs_timeout
A10: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_ceiling_is_the_runs_job_budget
A11: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k test_the_layout_holds_for_every_count_of_artifacts
A11: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k test_a_producer_that_uploaded_nothing_leaves_the_reading_unchanged
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_release_listing_fits_one_run
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_listing_beyond_the_ceiling_is_refused_whole
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_sizing_prints_its_headroom
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_listing_that_needs_exactly_the_shard_cap_is_sized_at_the_cap
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_sizing_constants_are_the_measured_ones
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_whole_tree_is_sized_at_the_batterys_ceiling
```

The reds were read in job 114621932441, `hygiene` over the red commit: 17 failures, each a
test the fence above names for A1 to A8 and A10 to A12, and no other, while A9 passed there.
A criterion with several tests quotes one of them, and `<the job's text>` stands for the
`mutation-rows` job's text, which A8's message prints whole. The greens are read in the checks
over the green commit.

```red-first
A1: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: 1 != 2 : 159 rows read 1, want 2
A1: green at f579a71396392635321ea469ae3327b9fa235147
A2: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: 0 != 2 : legs: 1 leg(s) for 2687 row(s)
A2: green at f579a71396392635321ea469ae3327b9fa235147
A3: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: None != '16' : legs: 1 leg(s) for 2512 row(s)
A3: green at f579a71396392635321ea469ae3327b9fa235147
A4: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: Lists differ: ['S00001-FIRST', 'S00002-SECOND', 'S00003-THIRD'] != ['S00001-FIRST', 'S00003-THIRD']
A4: green at f579a71396392635321ea469ae3327b9fa235147
A5: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: 3 != 2 : --leg 2/2: S00001-FIRST: VOID: its anchor occurs 0 times in scripts/fixmod.py
A5: green at f579a71396392635321ea469ae3327b9fa235147
A6: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: 0 != 1 : one line names leg 1 absent: examined 4
A6: green at f579a71396392635321ea469ae3327b9fa235147
A7: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: 0 != 2 : twice: examined 7
A7: green at f579a71396392635321ea469ae3327b9fa235147
A8: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: '      matrix:\n        leg: ${{ fromJSON(needs.mutation-plan.outputs.rows_matrix) }}\n' not found in '<the job's text>' : mutation-rows has no matrix over rows_matrix
A8: green at f579a71396392635321ea469ae3327b9fa235147
A9: not red: the red commit's stub carries ROWS_BOUND_SECONDS = 2700 and the base keeps 90 minutes, so it is green there; it guards the kept bound as the killer of S41701 and S41712
A10: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: {'ci': 209, 'battery': 234} != {'ci': 193, 'battery': 234}
A10: green at f579a71396392635321ea469ae3327b9fa235147
A11: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: 'mutation-rows' unexpectedly found in ['mutation-plan', 'mutation-rows', None, None] : the verdict asks for mutation-rows by name, which no leg uploads
A11: green at f579a71396392635321ea469ae3327b9fa235147
A12: red at 7cd75c03fa926259b80a9d2384c549613fce2a54: AssertionError: 0 != 1 : mutation: shards: 200 shard(s) for 8367 listed mutant(s), projected at 1253460 s serially; the slowest at 10175 s of its 10800 s bound; legs 200 of ceiling 209
A12: green at f579a71396392635321ea469ae3327b9fa235147
```
