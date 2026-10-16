# Red-first record: SPEC-429

SPEC-429 (R1 to R13, A1 to A11). The SPEC, ADR-443, the amendment to SPEC-421, the Python
schematic's section 10, the changelog fragment and the tests of A1 to A11 were committed first and
alone, over a stub that keeps every input but the missing behaviour: the `stale` verb is parsed and
routed, its readers read the tables, the mapped modules and the record files and hold no row, no
test and no suite then, and the verb prints only `examined 0` and exits 0. So each red below is
its criterion's assertion and never a missing name. The test modules run only in CI, so every red
is quoted from the pull request's checks over that commit,
a5f510b20d68af65779fe1e73521ec116449bbc1, in the job `hygiene`, the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_row_whose_suite_grew_since_its_run_is_named_with_its_fix
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_suite_no_larger_than_its_run_ran_reads_fresh
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_file_is_judged_by_the_row_the_plan_prices_it_by
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_suite_then_is_the_largest_ran_its_runs_records_hold
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_syntax_tree_counts_what_the_runners_loader_loads
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_each_unread_reason_is_named_and_not_examined
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_no_row_read_prints_void_and_examined_zero
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_every_reading_exits_zero_and_leaves_the_root_unchanged
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_trees_rows_are_every_row_of_both_tables
A10: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_reads_the_stale_lines_before_the_release
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_python_workflows.py -k test_the_plan_job_runs_the_stale_verb_in_a_step_that_cannot_fail
```

The reds were read in job 116887092743, `hygiene` over the red commit, a job of 1241 s:
`Ran 1296 tests` and `FAILED (failures=11)`. Eleven tests failed, and they are the eleven tests the
fence above names for A1 to A11, every one of them, each at its own assertion; no other test
failed and none errored. Each failing line is quoted as CI printed it, unittest's own `[N chars]`
cuts included (SR is `test_mutation_python_shard_binding.AStalePriceRowIsNamedAndNeverBlocks`):

```text
SR.test_a_file_is_judged_by_the_row_the_plan_prices_it_by :: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: scripts/b.py: its suite [248 chars]d 3']
SR.test_a_row_whose_suite_grew_since_its_run_is_named_with_its_fix :: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: scripts/a.py: its suite [247 chars]d 1']
SR.test_a_suite_no_larger_than_its_run_ran_reads_fresh :: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: 0 file(s) stale of 1 pri[74 chars]d 1']
SR.test_each_unread_reason_is_named_and_not_examined :: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: unread: scripts/odd.py ([164 chars]d 1']
SR.test_every_reading_exits_zero_and_leaves_the_root_unchanged :: AssertionError: Lists differ: ['examined 0'] != ['examined 1']
SR.test_no_row_read_prints_void_and_examined_zero :: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: 0 file(s) stale of 0 pri[156 chars]d 0']
SR.test_the_suite_then_is_the_largest_ran_its_runs_records_hold :: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: scripts/a.py: its suite [593 chars]d 3']
SR.test_the_syntax_tree_counts_what_the_runners_loader_loads :: AssertionError: Lists differ: [] != ['test_stale_shape_async.Awaited.test_awaited']
SR.test_the_trees_rows_are_every_row_of_both_tables :: AssertionError: Lists differ: [] != [('PYTHON_CONTROL_SECONDS', 'scripts/audit[1253 chars]py')]
test_mutation_python_workflows.ThePlanJobNamesTheStalePriceRows.test_the_plan_job_runs_the_stale_verb_in_a_step_that_cannot_fail :: AssertionError: Lists differ: [] != [11]
test_rulesets.TheReleaseRunbookReadsTheStalePriceRows.test_the_runbook_reads_the_stale_lines_before_the_release :: AssertionError: Lists differ: ['`mutation-verdict.py stale`', "Before th[129 chars]429'] != []
```

None of the eleven is red for another reason.

Test edits after the red commit: none. The green commit,
97f482454f1da67d5a0f68db2cd676aea36b7795, the rows commit and this record's commit edit no test
module.

The green commit was read at the second push's head, whose other two commits add the rows and this
record. No CI run read the green commit alone.

```red-first
A1: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: scripts/a.py: its suite [247 chars]d 1']
A1: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A2: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: 0 file(s) stale of 1 pri[74 chars]d 1']
A2: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A3: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: scripts/b.py: its suite [248 chars]d 3']
A3: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A4: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: scripts/a.py: its suite [593 chars]d 3']
A4: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A5: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: [] != ['test_stale_shape_async.Awaited.test_awaited']
A5: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A6: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: unread: scripts/odd.py ([164 chars]d 1']
A6: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A7: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: ['examined 0'] != ['mutation: stale: 0 file(s) stale of 0 pri[156 chars]d 0']
A7: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A8: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: ['examined 0'] != ['examined 1']
A8: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A9: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: [] != [('PYTHON_CONTROL_SECONDS', 'scripts/audit[1253 chars]py')]
A9: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A10: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: ['`mutation-verdict.py stale`', "Before th[129 chars]429'] != []
A10: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
A11: red at a5f510b20d68af65779fe1e73521ec116449bbc1: AssertionError: Lists differ: [] != [11]
A11: green at 97f482454f1da67d5a0f68db2cd676aea36b7795
```
