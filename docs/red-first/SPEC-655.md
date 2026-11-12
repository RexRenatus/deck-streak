# Red-first record: SPEC-655

SPEC-655 (R1 to R9, A1 to A6). The SPEC, ADR-677, the tests of A1 to A5 in
`scripts/tests/test_shell_script_census.py` with their planted fixtures, and the test of A6 in
`scripts/tests/test_ci_workflows.py` were committed first and alone, over stubs that keep every
input but the missing behaviour: `population` returns no script, `judge` makes no `examined`
count, `saved` refuses nothing and returns an empty list and no finding, and `verdict` returns no
line and no problem. `LEVEL` was already `warn` and `LISTED` already empty, and the hygiene job had
no ShellCheck step, so each red below is its criterion's assertion and never a missing name.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_shell_script_census.py -k test_a_script_is_counted_by_its_extension_or_its_shell_shebang_and_nothing_else
A2: python3 -m unittest discover -s scripts/tests -p test_shell_script_census.py -k test_an_unlisted_finding_prints_a_warn_line_at_warn_and_fails_at_fail
A3: python3 -m unittest discover -s scripts/tests -p test_shell_script_census.py -k test_a_listed_finding_passes_and_one_no_longer_found_is_stale_at_both_levels
A4: python3 -m unittest discover -s scripts/tests -p test_shell_script_census.py -k test_an_empty_population_and_a_saved_output_off_its_pin_or_its_list_are_refused
A5: python3 -m unittest discover -s scripts/tests -p test_shell_script_census.py -k test_every_tracked_shell_script_is_judged_from_the_pinned_shellchecks_saved_output
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_hygiene_runs_the_pinned_shellcheck_once_before_its_stages
```

Four reds were read locally over planted trees (A1 to A4), each on an assertion of its criterion.
A5 and A6 read the tree and the workflow, so their reds were read in CI: the hygiene job 130525614159
failed the six tests of A1 to A6 and no other. 5e37402b replaced the four stubs with the census
bodies and restored the two imports only those bodies use; no test line changed between 689cb5a7
and 5e37402b. The entries of `LISTED` were written from the saved output of the hygiene job
130609594872, the run of the second push at 5e37402b. The coverage workflow's Python job also runs
this module outside the hygiene job, where A5 reads VOID; that job prints a failing suite and does
not fail on it, so the VOID there changes no verdict.

```red-first
A1: red at 689cb5a7: AssertionError: Lists differ: [] != ['clean.sh', 'direct-tool', 'fake-tool']
A1: green at 5e37402b
A2: red at 689cb5a7: AssertionError: Tuples differ: ([], []) != (['WARN finding.sh SC2086 found 1, listed 0, at line(s) 3'], [])
A2: green at 5e37402b
A3: red at 689cb5a7: AssertionError: Tuples differ: ([], []) != ([], ['STALE finding.sh SC2086 found 0, listed 1'])
A3: green at 5e37402b
A4: red at 689cb5a7: AssertionError: AssertionError not raised
A4: green at 5e37402b
A5: red at 689cb5a7: AssertionError: 'scripts/check.sh' not found in []
A5: green at 5e37402b
A6: red at 689cb5a7: AssertionError: 0 != 1 : hygiene runs ShellCheck in no single step
A6: green at 5e37402b
```
