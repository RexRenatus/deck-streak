# Red-first record: SPEC-421

SPEC-421 (R1 to R16, A1 to A16). The SPEC, ADR-435, the amendments to SPEC-418 and ADR-434, the
Python schematic's section 8, and the tests of A1 to A15 were committed first and alone, over stubs
that keep every input but the missing behaviour: the new flags are parsed and ignored, the scoped
table is empty and read by nothing, and the scoped verb examines its records and prices nothing.
So each red below is its criterion's assertion and never a missing name. The test modules run only
in CI, so every red is quoted from the pull request's checks over that commit, in the job
`hygiene`, the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_scoped_file_run_scopes_each_mutant_and_records_its_sentinel
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_scoped_is_refused_beside_a_verdict_run
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_sample_is_refused_outside_a_scoped_file_run
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_sample_runs_every_ceil_m_over_s_th_mutant_in_order
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_setup_record_names_its_job
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_whole_run_writes_no_sentinel_record
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_plan_prices_scoped_exactly_when_the_legs_trace
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_prints_one_triple_per_file
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_over_no_record
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_a_record_with_no_run
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_a_record_with_no_job
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_a_whole_run_mutant_record
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_one_path_across_two_jobs
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_one_path_across_two_attempts
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_table_is_the_verbs_output_over_the_recorded_price_run
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_triple_deals_by_its_setup_and_its_mutant_figure
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_file_is_priced_scoped_then_whole_then_by_the_count_rule
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_an_empty_or_absent_scoped_table_leaves_every_whole_deal_byte_identical
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_mixed_prices_keep_the_partition_and_the_count
A14: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_plan_prints_how_many_files_it_priced_scoped
A15: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_scoped_price_table_its_verb_and_its_workflow
A16: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_price_workflow_measures_scoped_on_the_python_legs_runner_class
```

The reds were read in job 115151063286, `hygiene` over the red commit, a job of 962 s:
`Ran 1246 tests` and `FAILED (failures=26)`. Twenty tests failed, four of them in more
than one subtest. Eighteen are the tests the fence above names for A1 to A4, A6 to A11, A14 and
A15, every one of them, while A5, A12 and A13 passed there; A16's test lands with the workflow.
Each failing line is quoted as CI printed it, unittest's own `[N chars]` cuts included, and the one
line cut here is marked `[cut]` (AS is `test_mutation_python.AScopedFileRunPricesItsFile`, EL is
`test_mutation_python.EachLegRunsItsPlannedNames`, SB is
`test_mutation_python_shard_binding.AScopedLegIsPricedByItsTriple`):

```text
AS.test_a_sample_runs_every_ceil_m_over_s_th_mutant_in_order :: AssertionError: Lists differ: ['scr[77 chars]t.py:13:12: replace return value with return N[918 chars]ual'] != ['scr[77 chars]t.py:21:12: replace return value with return N[83 chars]ass']
AS.test_a_scoped_file_run_scopes_each_mutant_and_records_its_sentinel :: AssertionError: Lists differ: [] != ['scope_checks.Lines.test_in_process']
AS.test_each_setup_record_names_its_job :: AssertionError: Lists differ: [('co[23 chars]y', 'no job'), ('mutant', 'scripts/calc.py', '[131 chars]ob')] != [('co[23 chars]y', '7373'), ('mutant', 'scripts/calc.py', 'no[168 chars]73')]
AS.test_sample_is_refused_outside_a_scoped_file_run (args=['--plan', 'PLAN', '--sample', '2']) :: AssertionError: Tuples differ: (0, 'mutation-python: killed 3, survived 0[2405 chars]']}]) != (2, '', 'mutation-python: usage: --sample [82 chars], [])
AS.test_sample_is_refused_outside_a_scoped_file_run (args=['--all', '--sample', '2']) :: AssertionError: Tuples differ: (0, 'mutation-python: killed 3, survived 0[2275 chars]l'}]) != (2, '', 'mutation-python: usage: --sample [81 chars], [])
AS.test_sample_is_refused_outside_a_scoped_file_run (args=['--file', 'scripts/calc.py', '--sample', '2']) :: AssertionError: Tuples differ: (0, 'mutation-python: killed 3, survived 0[2277 chars]l'}]) != (2, '', 'mutation-python: usage: --sample [85 chars], [])
AS.test_sample_is_refused_outside_a_scoped_file_run (args=['--file', 'scripts/calc.py', '--scoped', '--sample', '0']) :: AssertionError: Tuples differ: (0, 'mutation-python: killed 3, survived 0[2274 chars]l'}]) != (2, '', 'mutation-python: usage: --sample [65 chars], [])
AS.test_scoped_is_refused_beside_a_verdict_run (args=['--plan', 'PLAN', '--scoped']) :: AssertionError: Tuples differ: (0, 'mutation-python: killed 3, survived 0[2407 chars]']}]) != (2, '', 'mutation-python: usage: --scoped [73 chars], [])
AS.test_scoped_is_refused_beside_a_verdict_run (args=['--all', '--scoped']) :: AssertionError: Tuples differ: (0, 'mutation-python: killed 3, survived 0[2277 chars]l'}]) != (2, '', 'mutation-python: usage: --scoped [72 chars], [])
EL.test_a_sharded_legs_cost_records_name_their_shard_one_record_a_line :: AssertionError: 4 != 5 : ['{"kind": "control", "path": "scripts/calc.py", "ran": 1, "failures": 0, "seconds": 20.0, "bound": 100.0, "shard": "0/2", "run": "5151", "attempt": "3"}', '{"kind": "mutant", [cut]
EL.test_each_files_control_cost_is_recorded_beside_the_report_before_its_mutants :: AssertionError: {'kin[95 chars]0.0, 'shard': None, 'run': '4242', 'attempt': '2'} != {'kin[95 chars]0.0, 'shard': None, 'run': '4242', 'attempt': '2', 'job': None}
SB.test_a_file_is_priced_scoped_then_whole_then_by_the_count_rule (arguments=('a.py', {'a.py': 100}, {'a.py': (5, 6, 7)})) :: AssertionError: Tuples differ: (100, 100, 100) != (5, 6, 7)
SB.test_a_file_is_priced_scoped_then_whole_then_by_the_count_rule (arguments=('a.py', {}, {'a.py': (5, 6, 7)})) :: AssertionError: None != (5, 6, 7)
SB.test_a_triple_deals_by_its_setup_and_its_mutant_figure :: AssertionError: None != {'count': 2, 'shards': [{'shard': 0, 'mut[1660 chars]68}]}
SB.test_the_plan_prices_scoped_exactly_when_the_legs_trace (selections=['plan']) :: AssertionError: Lists differ: [(['a[521 chars]in f'], 1700), (['a.py:15:5: replace + with - [513 chars]700)] != [(['a[521 chars]in f', 'a.py:15:5: replace + with - in f', 'a.[502 chars]800)]
SB.test_the_plan_prices_scoped_exactly_when_the_legs_trace (selections=['all', 'plan']) :: AssertionError: Lists differ: [(['a[521 chars]in f'], 1700), (['a.py:15:5: replace + with - [513 chars]700)] != [(['a[521 chars]in f', 'a.py:15:5: replace + with - in f', 'a.[502 chars]800)]
SB.test_the_plan_prints_how_many_files_it_priced_scoped :: AssertionError: 'mutation: shards: 1 shard(s) for 0 listed mutant(s), projec[82 chars] 193' != 'mutation: shards: python legs priced scoped: 2 file(s) at t[55 chars]iced'
SB.test_the_scoped_prices_verb_is_void_on_a_record_with_no_job :: AssertionError: Tuples differ: (0, 'mutation: prices: examined 1 record(s[21 chars]0\n') != (1, 'mutation: prices: VOID scripts/create[103 chars]0\n')
SB.test_the_scoped_prices_verb_is_void_on_a_record_with_no_run :: AssertionError: Tuples differ: (0, 'mutation: prices: examined 1 record(s[21 chars]0\n') != (1, 'mutation: prices: VOID scripts/create[105 chars]0\n')
SB.test_the_scoped_prices_verb_is_void_on_a_whole_run_mutant_record :: AssertionError: Tuples differ: (0, 'mutation: prices: examined 3 record(s[21 chars]0\n') != (1, "mutation: prices: VOID scripts/create[152 chars]0\n")
SB.test_the_scoped_prices_verb_is_void_on_one_path_across_two_attempts :: AssertionError: Tuples differ: (0, 'mutation: prices: examined 2 record(s[21 chars]0\n') != (1, 'mutation: prices: VOID scripts/create[99 chars]0\n')
SB.test_the_scoped_prices_verb_is_void_on_one_path_across_two_jobs :: AssertionError: Tuples differ: (0, 'mutation: prices: examined 2 record(s[21 chars]0\n') != (1, 'mutation: prices: VOID scripts/create[95 chars]0\n')
SB.test_the_scoped_prices_verb_is_void_over_no_record :: AssertionError: Tuples differ: (0, 'mutation: prices: examined 0 record(s[21 chars]1\n') != (1, 'mutation: prices: VOID <costs> holds [74 chars]1\n')
SB.test_the_scoped_prices_verb_prints_one_triple_per_file :: AssertionError: 'mutation: prices: examined 9 record(s) in[16 chars] 1\n' != '    "scripts/create-issues.py": (4, 8, 3)[1247 chars] 1\n'
SB.test_the_scoped_table_is_the_verbs_output_over_the_recorded_price_run :: AssertionError: 0 != 1 : []
test_rulesets.TheReleaseRunbookNamesTheScopedPrices.test_the_runbook_names_the_scoped_price_table_its_verb_and_its_workflow :: AssertionError: Lists differ: ['PYTHON_SCOPED_SECONDS', 'prices --scoped', 'mutation-prices.yml'] != []
```

Three of the twenty are red for another reason, and none of them is its criterion's red:
- A9's test failed on its absent fixture (`AssertionError: 0 != 1 : []`), which is not its
  criterion's reason. Its red is read at the fixture commit, below.
- `EL.test_each_files_control_cost_is_recorded_beside_the_report_before_its_mutants` pins the
  control record whole, and the red commit gave its pinned record the key `"job": None`, which the
  stubs do not write. The green commit writes it (R5).
- `EL.test_a_sharded_legs_cost_records_name_their_shard_one_record_a_line` now reads the traced
  sentinel record a leg writes after its control: the red commit made it count five records where
  it counted four, and assert the sentinel record whole beside the three mutants' names. Every
  assertion it held is kept.

Test edits after the red commit, each named here:
- The green commit, 1a66978ac9bba3c94085ae398e52a5c0a881ee65, adds A16's test to `test_ci_workflows.py`, with the census
  entries the new workflow's reads need. It edits no other test module.
- 5a73f251731514df44419b93d6a216e9dcc0074b adds two tests to `test_mutation_python.py`, each pinning R2's sample at an
  edge: a sample of one runs only the first mutant, and a sample at or over the listing runs every
  mutant in order. They kill two generated mutants that survived the second push, and change no
  criterion's test.
- The rows commit, the fixture commit, the table commit and this record's commit edit no test
  module.

## The fixture commit's red

A9 is red at the fixture commit, cc11ecdcd697574dd96465978bcbbc608956ae92, which adds the price run's records and leaves
the table empty. It was read in job 115242787212, `hygiene` over that commit, a job of 1218 s:
`Ran 1249 tests` and `FAILED (failures=1)`. Its one failure is A9's test, on its
assertion that the table is not the verb's output over the committed records:

```text
SB.test_the_scoped_table_is_the_verbs_output_over_the_recorded_price_run :: AssertionError: Lists differ: ['    "scripts/audit-web-verdict.py": (3, [1794 chars](s)'] != []
```

The price run priced fifteen files and left `scripts/mutation-verdict.py` unpriced, so the two
rows SPEC-421 section 9 plans on that file's table line are not written.

The green commit was read at the second push's head, whose other commit adds rows, and again at
the third push's head, which adds two tests. The table commit is read at the fifth push's head,
whose other commit adds this record and the changelog fragment. No CI run read either code commit
alone.

```red-first
A1: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: Lists differ: [] != ['scope_checks.Lines.test_in_process']
A1: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A2: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: Tuples differ: (0, 'mutation-python: killed 3, survived 0[2407 chars]']}]) != (2, '', 'mutation-python: usage: --scoped [73 chars], [])
A2: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A3: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: Lists differ: ['scr[77 chars]t.py:13:12: replace return value with return N[918 chars]ual'] != ['scr[77 chars]t.py:21:12: replace return value with return N[83 chars]ass']
A3: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A4: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: Lists differ: [('co[23 chars]y', 'no job'), ('mutant', 'scripts/calc.py', '[131 chars]ob')] != [('co[23 chars]y', '7373'), ('mutant', 'scripts/calc.py', 'no[168 chars]73')]
A4: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A5: not red: the base writes no sentinel record, so a whole run already writes none
A6: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: Lists differ: [(['a[521 chars]in f'], 1700), (['a.py:15:5: replace + with - [513 chars]700)] != [(['a[521 chars]in f', 'a.py:15:5: replace + with - in f', 'a.[502 chars]800)]
A6: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A7: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: 'mutation: prices: examined 9 record(s) in[16 chars] 1\n' != '    "scripts/create-issues.py": (4, 8, 3)[1247 chars] 1\n'
A7: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A8: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: Tuples differ: (0, 'mutation: prices: examined 0 record(s[21 chars]1\n') != (1, 'mutation: prices: VOID <costs> holds [74 chars]1\n')
A8: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A9: red at cc11ecdcd697574dd96465978bcbbc608956ae92: AssertionError: Lists differ: ['    "scripts/audit-web-verdict.py": (3, [1794 chars](s)'] != []
A9: green at 1766097fdf8b74c1800be95c97ae11abef35bbed
A10: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: None != {'count': 2, 'shards': [{'shard': 0, 'mut[1660 chars]68}]}
A10: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A11: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: Tuples differ: (100, 100, 100) != (5, 6, 7)
A11: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A12: not red: the red commit's stubs accept and ignore the scoped table, so every whole deal and its printed lines already hold
A13: not red: the red commit's stubs accept and ignore the scoped table, so the partition and the count already hold
A14: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: 'mutation: shards: 1 shard(s) for 0 listed mutant(s), projec[82 chars] 193' != 'mutation: shards: python legs priced scoped: 2 file(s) at t[55 chars]iced'
A14: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A15: red at a0f9c8f24527d53f5aaf166d9c2824c4a4816dab: AssertionError: Lists differ: ['PYTHON_SCOPED_SECONDS', 'prices --scoped', 'mutation-prices.yml'] != []
A15: green at 1a66978ac9bba3c94085ae398e52a5c0a881ee65
A16: not red: the workflow is a new file, whose absence is not its behaviour
```

A5, A12, A13 and A16 are not red first, because the base writes no sentinel record, the red
commit's stubs ignore the scoped table, and the workflow is a new file. Each is recorded `not red`,
with that reason, and holds its behaviour against a change.
