# Red-first record: SPEC-419

Every criterion's test is committed before the code it judges, beside a stub of
`scripts/web_mutation_shards.py` that holds the names the tests call and parses, deals, counts and
reads nothing, so each test fails by its own assertion rather than by an import error. With the
tests come the stub's entry in `scripts/mutation-python.json` and the census register's one entry
for the new module's import of the script. A1 to A5 read `ci.yml` and are red at the base, which
has no shard job; A6 to A13 run the stub. All thirteen run in CI's `hygiene` job, which runs
`scripts/tests`, and their reds are read there, by name, at the commit that adds them. Each red and
green line below is recorded with the fix.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_web_sweep_runs_in_three_shards_read_by_name
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_web_shard_defect_is_refused_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_gate_runs_in_four_parallel_jobs
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_mutation_jobs_are_needs_of_ci_with_pinned_tools_and_no_saved_cache
A5: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_each_legs_job_condition_reads_the_plans_listing_alone
A6: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_the_deal_puts_the_largest_file_first_onto_the_lightest_shard
A7: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_equal_counts_go_by_path_and_equal_shards_by_the_lowest_index
A8: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_every_file_is_dealt_to_exactly_one_shard_and_the_total_is_kept
A9: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_a_shard_dealt_nothing_writes_an_explicit_empty_record
A10: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_the_count_is_strykers_own_discover_over_the_planned_files
A11: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_the_union_reads_every_shard_by_name_and_merges_their_files
A12: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_a_missing_cut_or_partial_shard_reads_void_and_writes_nothing
A13: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_the_command_line_refuses_what_it_cannot_read
```
