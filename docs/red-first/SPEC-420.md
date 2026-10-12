# Red-first record: SPEC-420

SPEC-420 (R1 to R11, A1 to A18). The SPEC, ADR-434, the amendment to ADR-432, the Python
schematic, the leg control's and the scope fixtures, the recorded run's costs, and the tests of A1
to A18 were committed first and alone, over stubs that keep the base behaviour or return None, so
each red below is its criterion's assertion and never a missing name. The test modules run only in
CI, so every red is quoted from the pull request's checks over that commit, in the job `hygiene`,
the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_share_is_one_value_capped_at_128_in_the_tree
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_share_outside_the_cap_refuses_the_plan_whole_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_every_printed_share_reads_the_one_value
A4: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_python_share_by_its_name_never_its_number
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_price_table_is_the_prices_verbs_output_over_the_recorded_run
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_file_the_filled_table_does_not_price_keeps_the_count_rule
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_every_files_control_is_recorded_before_any_mutant_of_the_leg
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_mutant_runs_only_the_tests_that_execute_its_line
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_line_run_at_import_or_outside_a_test_runs_the_whole_suite
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_line_no_traced_test_executes_runs_the_whole_suite
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_line_only_a_child_process_executes_keeps_that_test
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_test_whose_process_left_no_record_joins_every_scoped_set
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_test_whose_child_execs_joins_every_scoped_set
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_test_that_leaves_a_thread_running_joins_every_scoped_set
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_failed_or_partial_trace_runs_every_mutant_of_its_file_whole
A14: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_scoped_run_that_ran_fewer_tests_than_its_set_reruns_whole
A15: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_scoped_survivor_is_confirmed_by_the_whole_suite
A16: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_the_battery_runs_every_mutant_against_the_whole_suite
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_leg_whose_control_reads_wrong_voids_every_file_by_name
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_the_repository_holds_the_leg_control_and_its_three_mutants
A18: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_mutant_record_names_its_scope_and_its_tests
```

The reds were read in job 114965437284, `hygiene` over the red commit: `Ran 1199 tests` and
`FAILED (failures=20)`. Eighteen of the twenty are the tests the fence above names for A1, A2, A4,
A5 and A7 to A18, every one of them, while A3, A6 and A16 passed there.
Each failing line is quoted as CI printed it, unittest's own `[N chars]` cuts included (EMR is
`test_mutation_python.EachMutantRunsTheTestsItsLineTraced`, SB is
`test_mutation_python_shard_binding.ThePythonShareIsOneCappedValue`):

```text
EMR.test_a_failed_or_partial_trace_runs_every_mutant_of_its_file_whole :: AssertionError: Tuples differ: (None, None) != ('whole', 'trace')
EMR.test_a_leg_whose_control_reads_wrong_voids_every_file_by_name :: AssertionError: Lists differ: [None] != ['control: scripts/tests/fixtures/mutation[95 chars]led']
EMR.test_a_line_no_traced_test_executes_runs_the_whole_suite :: AssertionError: Lists differ: [(None, None, 'survived')] != [('whole', 'unreached', 'survived')]
EMR.test_a_line_only_a_child_process_executes_keeps_that_test :: AssertionError: Lists differ: [(None, None, 'killed'), (None, None, 'killed')] != [('lines', ['scope_checks.Lines.test_in_chi[70 chars]ed')]
EMR.test_a_line_run_at_import_or_outside_a_test_runs_the_whole_suite :: AssertionError: Lists differ: [(9, None, None), (25, None, None), (25, None,[38 chars]one)] != [(9, 'whole', 'module'), (25, 'whole', 'import[73 chars]rt')]
EMR.test_a_scoped_run_that_ran_fewer_tests_than_its_set_reruns_whole :: AssertionError: Lists differ: [(None, None, 'survived')] != [('whole', 'ran', 'survived')]
EMR.test_a_scoped_survivor_is_confirmed_by_the_whole_suite :: AssertionError: Lists differ: [(None, None, 'killed'), (None, None, 'killed')] != [('whole', 'survivor', 'killed'), ('whole', 'survivor', 'killed')]
EMR.test_a_test_that_leaves_a_thread_running_joins_every_scoped_set :: AssertionError: Lists differ: [('in_process', None), ('in_process', None), ('shared', N[18 chars]one)] != [('in_process', ['scope_checks.Lines.test_case', 'scope_c[358 chars]o'])]
EMR.test_a_test_whose_child_execs_joins_every_scoped_set :: AssertionError: Lists differ: [('in_process', None), ('in_process', None), ('shared', N[18 chars]one)] != [('in_process', ['scope_checks.Lines.test_case', 'scope_c[358 chars]o'])]
EMR.test_a_test_whose_process_left_no_record_joins_every_scoped_set :: AssertionError: Lists differ: [('in_process', None), ('in_process', None), ('shared', N[18 chars]one)] != [('in_process', ['scope_checks.Lines.test_case', 'scope_c[358 chars]o'])]
EMR.test_each_mutant_record_names_its_scope_and_its_tests :: AssertionError: None not found in ('lines', 'whole') : scripts/scope_target.py:9:9: replace 2 with 3 in <module>
EMR.test_each_mutant_runs_only_the_tests_that_execute_its_line :: AssertionError: Lists differ: [] != ['scope_checks.Lines.test_in_process']
EMR.test_every_files_control_is_recorded_before_any_mutant_of_the_leg :: AssertionError: Lists differ: [('co[25 chars]), ('mutant', 'scripts/calc.py'), ('mutant', '[173 chars]py')] != [('co[25 chars]), ('control', 'scripts/loose.py'), ('mutant',[173 chars]py')]
EMR.test_the_repository_holds_the_leg_control_and_its_three_mutants :: AssertionError: Lists differ: [('sc[115 chars]d', []), ('scripts/tests/fixtures/mutation-pyt[223 chars] [])] != [('sc[115 chars]d', ['control_checks.Control.test_direct']), ([297 chars] [])]
test_mutation_python_lister_kills.TheChildReportsWhatFailed.test_the_child_function_returns_the_ok_exit_and_not_none :: AssertionError: 'RETURNED 0\n' not found in 'Traceback (most recent call last):\n  File "<string>", line 5, in <module>\n  File "/home/runner/work/deck-streak/deck-streak/scripts/mutation_python.py", line 680, 
SB.test_a_share_outside_the_cap_refuses_the_plan_whole_by_name :: AssertionError: 'mutation: shards: 2 python shard(s) for 30 listed mutant(s)[422 chars]20\n' != 'mutation: shards: REFUSED: the python share PYTHON_MAX_SHAR[81 chars]lt\n'
SB.test_the_price_table_is_the_prices_verbs_output_over_the_recorded_run :: AssertionError: '    "scripts/audit-web-verdict.py": 3,  #[3521 chars] 0\n' != 'mutation: prices: examined 1017 record(s)[20 chars] 0\n'
SB.test_the_share_is_one_value_capped_at_128_in_the_tree :: AssertionError: None != 128
test_mutation_rows.EveryRowIsCensused.test_every_committed_row_is_anchored_once_and_names_one_test :: AssertionError: 1 != 0 : census: S08704-A-BYTE-READER-IS-NEVER-A-KILLER: its anchor occurs 0 times in scripts/mutation_python.py
test_rulesets.TheReleaseRunbookNamesThePythonShare.test_the_runbook_names_the_python_share_by_its_name_never_its_number :: AssertionError: Lists differ: [] != ['PYTHON_MAX_SHARDS', 'PYTHON_SHARE_CAP']
```

Two of the twenty are red for another reason, and neither is a criterion of SPEC-420:
- `test_mutation_rows.EveryRowIsCensused`: the red commit's runner rewrote the line that
  `S08704-A-BYTE-READER-IS-NEVER-A-KILLER` was anchored on. The green commit re-anchors that row on
  the line where `Judge.judge` reads the byte readers, and re-anchors
  `S41506-THE-RUNBOOK-NAMES-THE-LEG-SIZE-AS-THE-CEILING` on the runbook phrase the green rewrites,
  each with its killer unchanged.
- `test_mutation_python_lister_kills.TheChildReportsWhatFailed`: the red commit's `child()` read
  `args.trace` and `args.only`, which the lister test's namespace does not carry
  (`AttributeError: 'Namespace' object has no attribute 'trace'`). The green commit reads both with
  `getattr` and a default, and changes no test of the lister.

The green commit, d01a8dfc, also edits three test modules. None of them holds a criterion's
test, and each edit is named here:
- `test_mutation_python.py`: `EachLegRunsItsPlannedNames` pins each mutant record's keys, which
  now include `scope`, `tests` and `why`.
- `test_mutation_python_shard_binding.py`: SPEC-418's A3 test asserts that the shipped price table
  is the `prices` verb's output over the recorded run, where it asserted an empty table, and its
  docstring says so (SPEC-420 R4). Two tests that pin SPEC-418's plan lines patch the share to 32
  and the leg ceiling, so the lines they pin stay SPEC-418's whatever the share's value.
- `test_rulesets.py`: SPEC-415's runbook phrases name the share `PYTHON_MAX_SHARDS` where they
  named its number (SPEC-420 R3).

```red-first
A1: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: None != 128
A1: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A2: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: 'mutation: shards: 2 python shard(s) for 30 listed mutant(s)[422 chars]20\n' != 'mutation: shards: REFUSED: the python share PYTHON_MAX_SHAR[81 chars]lt\n'
A2: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A3: not red: the base already reads the share at call time, so every printed share reads it
A4: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [] != ['PYTHON_MAX_SHARDS', 'PYTHON_SHARE_CAP']
A4: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A5: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: '    "scripts/audit-web-verdict.py": 3,  #[3521 chars] 0\n' != 'mutation: prices: examined 1017 record(s)[20 chars] 0\n'
A5: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A6: not red: the base already keeps the count rule for a file no table prices
A7: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [('co[25 chars]), ('mutant', 'scripts/calc.py'), ('mutant', '[173 chars]py')] != [('co[25 chars]), ('control', 'scripts/loose.py'), ('mutant',[173 chars]py')]
A7: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A8: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [] != ['scope_checks.Lines.test_in_process']
A8: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A9: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [(9, None, None), (25, None, None), (25, None,[38 chars]one)] != [(9, 'whole', 'module'), (25, 'whole', 'import[73 chars]rt')]
A9: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A10: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [(None, None, 'survived')] != [('whole', 'unreached', 'survived')]
A10: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A11: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [(None, None, 'killed'), (None, None, 'killed')] != [('lines', ['scope_checks.Lines.test_in_chi[70 chars]ed')]
A11: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A12: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [('in_process', None), ('in_process', None), ('shared', N[18 chars]one)] != [('in_process', ['scope_checks.Lines.test_case', 'scope_c[358 chars]o'])]
A12: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A13: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Tuples differ: (None, None) != ('whole', 'trace')
A13: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A14: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [(None, None, 'survived')] != [('whole', 'ran', 'survived')]
A14: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A15: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [(None, None, 'killed'), (None, None, 'killed')] != [('whole', 'survivor', 'killed'), ('whole', 'survivor', 'killed')]
A15: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A16: not red: the base already runs every mutant against the whole suite
A17: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: Lists differ: [None] != ['control: scripts/tests/fixtures/mutation[95 chars]led']
A17: green at d01a8dfc216981b9f2305b8acb920e35da12418b
A18: red at d887c36c07d26954f3534c51721ea789a76b4591: AssertionError: None not found in ('lines', 'whole') : scripts/scope_target.py:9:9: replace 2 with 3 in <module>
A18: green at d01a8dfc216981b9f2305b8acb920e35da12418b
```

A3, A6 and A16 are not red first, because the base already reads the share at call time, keeps
the count rule, and runs every mutant whole. Each is recorded `not red`, with that reason, and
holds its behaviour against a change.

## The traced sentinel's reds

The second push's `mutation-verdict` job, 115022243114, read nine tests of
`test_mutation_python.py` as byte readers of `scripts/mutation_python.py`: each failed in the
traced sentinel run, so every mutant run of that file skipped it, and the mutants only they
observe survived. The same job read one survivor of A2's share check. Each line is quoted once,
as CI printed it, without CI's timestamp:

```text
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_line_no_traced_test_executes_runs_the_whole_suite
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_line_only_a_child_process_executes_keeps_that_test
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_line_run_at_import_or_outside_a_test_runs_the_whole_suite
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_scoped_run_that_ran_fewer_tests_than_its_set_reruns_whole
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_scoped_survivor_is_confirmed_by_the_whole_suite
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_test_that_leaves_a_thread_running_joins_every_scoped_set
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_test_whose_child_execs_joins_every_scoped_set
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_test_whose_process_left_no_record_joins_every_scoped_set
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: byte reader: scripts/mutation_python.py: test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_each_mutant_runs_only_the_tests_that_execute_its_line
job 115022243114 red at af64c424dc53784411d195d0e789058bc45f6cab: mutation: scripts: SURVIVED scripts/mutation-verdict.py:1014:42: replace 1 with 2 in python_share_refused
```

The nine are the tests that start a `run --plan` leg of the scope fixture as a subprocess. That
leg inherited the traced test process's environment: the `MUTATION_PYTHON_TRACE_` names, and the
`PYTHONPATH` that puts the trace's `sitecustomize` first. So the leg's own tests process held the
monitoring tool from its start, the leg could take no trace of its own, and every mutant of the
leg ran whole, which is not what the nine assert. The cure is in the test module: each scope leg
starts from an environment without those names and without that `PYTHONPATH` (`untraced`), and no
assertion of the nine changes. A2's test now also reads that a share of 1, the lowest whole share,
is not refused, which is the observation the survivor lacked.

The same commit edits one more thing, and it holds no criterion's test. Merging the base left two
of SPEC-418's plan-line tests in `test_mutation_python_shard_binding.py` patching the leg ceiling
to 193, as this branch wrote them, while pinning the base's lines for a ceiling of 190. The commit
puts back this branch's lines under that patch, so the lines they pin stay SPEC-418's, as the
green commit's note above says. The shipped ceiling stays the base's 190, and no assertion is
removed.

## The traced sentinel's greens

Each red line above is green at the cure commit: the nine pass under the traced sentinel's trace,
so the sentinel lists none of them as a byte reader, and A2's test fails on the survivor's mutant.
CI reads both again on the next push.

```text
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_line_no_traced_test_executes_runs_the_whole_suite: green at e782d9624acd456ae3031e05273cc299bd230ffe
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_line_only_a_child_process_executes_keeps_that_test: green at e782d9624acd456ae3031e05273cc299bd230ffe
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_line_run_at_import_or_outside_a_test_runs_the_whole_suite: green at e782d9624acd456ae3031e05273cc299bd230ffe
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_scoped_run_that_ran_fewer_tests_than_its_set_reruns_whole: green at e782d9624acd456ae3031e05273cc299bd230ffe
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_scoped_survivor_is_confirmed_by_the_whole_suite: green at e782d9624acd456ae3031e05273cc299bd230ffe
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_test_that_leaves_a_thread_running_joins_every_scoped_set: green at e782d9624acd456ae3031e05273cc299bd230ffe
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_test_whose_child_execs_joins_every_scoped_set: green at e782d9624acd456ae3031e05273cc299bd230ffe
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_a_test_whose_process_left_no_record_joins_every_scoped_set: green at e782d9624acd456ae3031e05273cc299bd230ffe
test_mutation_python.EachMutantRunsTheTestsItsLineTraced.test_each_mutant_runs_only_the_tests_that_execute_its_line: green at e782d9624acd456ae3031e05273cc299bd230ffe
scripts/mutation-verdict.py:1014:42: replace 1 with 2 in python_share_refused: green at e782d9624acd456ae3031e05273cc299bd230ffe
```
