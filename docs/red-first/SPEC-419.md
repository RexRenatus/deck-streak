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

The same `hygiene` job 114893256663 at `0b34613d` also failed the read census
`test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read` and three
controls of `test_the_census_is_red_on_every_planted_site`, all on one cause, the raw read of the
script in the test of A1, `(REPO / WEB_SHARDS_SCRIPT).read_text(encoding='utf-8')`, which the
register `NOT_WORKFLOW_READS` in `scripts/tests/test_ci_workflows.py` did not list, and the green
commit cures all four by adding that one entry to the register.

The reds were read in job 114893256663, `hygiene` over the red commit `0b34613d`: the thirteen
tests the first fence names failed, each by its own `AssertionError` and none by an error, A8
with 8 failures, A12 with 17 and A13 with 24, and the only other failures were the census and its
three controls named above. Each red quotes its test's first `AssertionError` line as CI printed it,
unittest's own `[N chars]` cuts included. CI runs the pull request's tip, whose workflow, script
and tests are byte-identical to the green commit's, so each green is read in the checks over the
tip.

The `hygiene` job 114938699081 at `4145fb2b` failed one test of the suite, `test_dispatch_shards.TheReleaseFitsOneRun.test_the_ceiling_is_the_runs_job_budget`, reading `{'ci': 193, 'battery': 234} != {'ci': 190, 'battery': 234}`, on one cause: the three shard legs raise the other `ci.yml` jobs from 63 to 66, and the commit that follows moves `LEG_CEILING["ci"]` and its pins to 190.

```red-first
A1: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Lists differ: ['ci.yml has no mutation-web-shard job', "[696 chars]dge'] != [] (job 114893256663)
A1: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A2: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Lists differ: ['ci.yml has no mutation-web-shard job', "[696 chars]dge'] != [] (job 114893256663)
A2: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A3: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Lists differ: ['bas[150 chars]b', 'release', 'rust', 'web', 'web-engine', 'workflow-lint'] != ['bas[150 chars]b', 'mutation-web-shard', 'release', 'rust', '[31 chars]int'] (job 114893256663)
A3: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A4: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Lists differ: [] != ['${{ always() }}'] (job 114893256663)
A4: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A5: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Lists differ: [] != ['${{ always() }}'] (job 114893256663)
A5: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A6: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Lists differ: [[], [], []] != [['src/a.ts', 'src/f.ts'], ['src/b.ts', 'sr[46 chars]ts']] (job 114893256663)
A6: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A7: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Lists differ: [[], [], []] != [['src/w.ts', 'src/z.ts'], ['src/x.ts'], ['src/y.ts']] (job 114893256663)
A7: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A8: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Lists differ: [] != ['src/f00.ts', 'src/f01.ts', 'src/f02.ts',[513 chars].ts'] (job 114893256663)
A8: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A9: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Tuples differ: (0, '') != (0, 'web shards: shard 2 of 3 holds 0 file(s), 0 mutant(s)\n') (job 114893256663)
A9: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A10: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Tuples differ: (0, '') != (0, 'web shards: shard 0 of 3 holds 2 file(s), 45 mutant(s)\n') (job 114893256663)
A10: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A11: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Tuples differ: (0, '') != (0, 'web shards: counted 3 of 3 reports whole\[127 chars]n\n') (job 114893256663)
A11: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A12: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Tuples differ: (0, '') != (3, 'web shards: MISSING: mutation-web-sha[231 chars]n\n') (job 114893256663)
A12: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
A13: red at 0b34613df1cb531c2a0ec52df3080f94c5785986: AssertionError: Tuples differ: (0, '') != (2, '') (job 114893256663)
A13: green at 8cbc25caa1d22593e66bb93d7433a55db3b4733b
```
