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

## Amendments: the signed rows legs (`docs/rulings/OWNER-RULING-2026-10-18-rows-legs-822.md`)

SPEC-417's section 11 moves A2, A10 and A12 and adds A13. The amendment sections of SPEC-417 and
ADR-431 were committed first, and the red commit holds the tests and their pins at 18 legs and a
`ci` ceiling of 189, over a `scripts/mutation_rows.py` whose `ROWS_MAX_LEGS` is still 17 and a
`scripts/mutation-verdict.py` whose `LEG_CEILING["ci"]` is still 190. The test modules run only in
CI, so every red is quoted from the pull request's checks over the red commit, from the job that
runs `scripts/tests`. A2, A10 and A12 are changed criteria, so their new reds are recorded in this
section's prose when seen, and their lines above stand. A13 is new: its red and green lines join
this section in a `red-first` fence, in the commit after the constants.

```text
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_release_runs_2688_rows_are_dealt_into_18_legs
```

The reds were read in job 117803082972, the job that runs `scripts/tests` over the red commit: 17 failures, each a test this
section or section 11 names, and no other. A2 read AssertionError: Regex didn't match: '(?m)^legs: REFUSED: 2845 row\\(s\\) need more than 18 legs\\b' not found in 'legs: REFUSED: 2845 row(s) need more than 17 legs of at most 158 row(s), each projected within 2700 s at 17 s a row\n'; A10 read AssertionError: {'ci': 190, 'battery': 234} != {'ci': 189, 'battery': 234}; A12's tests
read test_dispatch_shards.TheReleaseFitsOneRun.test_a_listing_beyond_the_ceiling_is_refused_whole: AssertionError: 0 != 1 : mutation: shards: 190 shard(s) for 12853 listed mutant(s), projected at 1619478 s serially; the slowest at 10709 s of its 10800 s bound; legs 190 of ceiling 190; test_dispatch_shards.TheReleaseFitsOneRun.test_every_sizing_prints_its_headroom: AssertionError: 'legs 2 of ceiling 189' not found in 'mutation: shards: 2 shard(s) for 113 listed mutant(s), projected at 9040 s serially; the slowest at 6701 s of its 10800 s bound; legs 2 of ceiling 190\n'; test_dispatch_shards.TheReleaseFitsOneRun.test_the_release_listing_fits_one_run: AssertionError: 'need more than 189 legs' not found in "mutation: shards: REFUSED: 8367 mutant(s), projected at 1253460 s serially, need more than 190 legs within 10800 s each, the pull request run's ceiling: split the change, since a run is never capped\n"; test_mutation_verdict.TheLegsAreSizedToTheRun.test_the_sizing_constants_are_the_measured_ones: AssertionError: {'ci': 190, 'battery': 234} != {'ci': 189, 'battery': 234}; test_mutation_verdict.TheLegsAreSizedToTheRun.test_the_whole_tree_is_sized_at_the_batterys_ceiling: AssertionError: 'legs 2 of ceiling 189' not found in 'mutation: shards: 2 shard(s) for 113 listed mutant(s), projected at 9040 s serially; the slowest at 6701 s of its 10800 s bound; legs 2 of ceiling 190\n'; test_mutation_verdict.TheShardsFitTheirBound.test_a_listing_that_needs_exactly_the_shard_cap_is_sized_at_the_cap: AssertionError: 190 is not None : ci; test_release_python_ceiling.TheReleaseRunDealsUpToItsCeiling.test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run: AssertionError: Tuples differ: (0, '[365 chars]ing 190\nmutation: shards: run legs 4 of 222: [69 chars]0\n') != (0, '[365 chars]ing 189\nmutation: shards: run legs 4 of 221: [69 chars]0\n'); test_release_rust_ceiling.TheReleaseRunSizesItsRustLegsToItsCeiling.test_a_run_that_is_not_the_release_keeps_the_pull_request_ceiling_at_any_rust_ceiling: AssertionError: Tuples differ: (0, 'mutation: shards: 190 shard(s) for 12[1014 chars]53'}) != (1, "mutation: shards: REFUSED: 12853 muta[165 chars], {}); test_release_rust_ceiling.TheReleaseRunSizesItsRustLegsToItsCeiling.test_a_rust_ceiling_outside_its_range_is_refused_by_name: AssertionError: Tuples differ: (1, '[71 chars]00 is outside 190..256; a ceiling outside it is never dealt\n') != (1, '[71 chars]00 is outside 189..256; a ceiling outside it is never dealt\n'); test_release_rust_ceiling.TheReleaseRunSizesItsRustLegsToItsCeiling.test_only_a_true_release_mark_holds_the_rust_ceiling: AssertionError: None != 190 : a true mark one past the placeholder; test_release_rust_ceiling.TheReleaseRunSizesItsRustLegsToItsCeiling.test_the_placeholder_ceiling_sizes_the_release_run_as_a_pull_request_run: AssertionError: Tuples differ: (1, 'mutation: shards: REFUSED: the releas[92 chars]t\n') != (0, 'mutation: shards: 3 python shard(s) f[448 chars]0\n'); test_release_rust_ceiling.TheReleaseRunSizesItsRustLegsToItsCeiling.test_the_rust_lines_read_the_release_ceiling_on_the_release_run_alone: AssertionError: Tuples differ: (0, '[370 chars]ing 190\nmutation: shards: run legs 5 of 222: [69 chars]0\n') != (0, '[370 chars]ing 189\nmutation: shards: run legs 5 of 221: [69 chars]0\n'); test_release_rust_ceiling.TheReleaseRunSizesItsRustLegsToItsCeiling.test_the_rust_release_line_stands_beside_the_python_release_line: AssertionError: '; legs 2 of ceiling 189\n' not found in 'mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\nmutation: shards: python legs dealt by cost: 3 priced leg(s), the slowest at 1800 s of its 1800 s bound; 0 unpriced leg(s) of at most 20\nmutation: shards: 2 shard(s) for 113 listed mutant(s), projected at 9040 s serially; the slowest at 6701 s of its 10800 s bound; legs 2 of ceiling 190\nmutation: shards: run legs 5 of 222: python 3 of 32, rust 2 of 190; projected 313 runner-minute(s) of 35160\n' : a plan that is not the release; test_release_rust_ceiling.WhatTheReleaseRunKeeps.test_the_python_ceiling_tests_read_the_rust_ceiling_at_its_placeholder: AssertionError: Tuples differ: (0, '[365 chars]ing 190\nmutation: shards: run legs 4 of 222: [69 chars]0\n') != (0, '[365 chars]ing 189\nmutation: shards: run legs 4 of 221: [69 chars]0\n'). The greens are read in the checks over the constants commit.
CI runs only the head of the last push, so the greens are read over that push, and `git diff --quiet 7e56182b HEAD -- scripts/tests/test_mutation_rows.py scripts/tests/test_dispatch_shards.py scripts/tests/test_mutation_verdict.py scripts/tests/test_release_rust_ceiling.py scripts/tests/test_release_python_ceiling.py scripts/tests/test_ci_workflows.py scripts/mutation_rows.py scripts/mutation-verdict.py` reads rc 0 at that push's tip.
No commit after the red commit changes a file under `scripts/tests`: `7e56182b` changes the two scripts, `c752cb79` two rows files, `0dc01f70` the fragment, and the record commit only this record.

```red-first
A13: red at 816c980cb4a68dc0ad5d8b633ed0d2a35004a3f3: AssertionError: 2 != 0 : legs: REFUSED: 2688 row(s) need more than 17 legs of at most 158 row(s), each projected within 2700 s at 17 s a row
A13: green at 7e56182ba985eb0df7aad025842f2cc8ea77df88
```
