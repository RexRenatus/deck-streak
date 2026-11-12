# Red-first record: SPEC-657

SPEC-657 (R1 to R12, A1 to A8). The SPEC, ADR-679, the planted fixtures and the tests of A1 to A8 in
`scripts/tests/test_process_waiter_census.py` were committed first and alone, over stubs that keep
every name and input but the missing behaviour: the population takes `.sh` files only, the shell
and Python readers find no lookup, the member reader reads nothing, the census refuses nothing,
and the verdict prints only `STALE` lines. `LEVEL`, `LISTED` and every expected value were already
there, so each red below is its criterion's assertion and never a missing name.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_tree_counts_each_script_step_and_python_file
A2: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_lookup_that_can_match_its_own_command_line_is_a_site
A3: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_lookup_that_cannot_match_itself_is_clean
A4: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_the_planted_python_lookups_are_read_by_their_argument_list_and_command_text
A5: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_the_planted_level_warns_on_an_unlisted_site_and_fails_on_a_stale_entry
A6: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_empty_population_is_void
A7: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_member_the_census_cannot_read_is_an_unread_site
A8: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_the_repository_holds_exactly_the_listed_sites_at_warn
```

Seven reds were read over planted text: the module run with its repository test skipped failed
exactly its seven planted tests, each on its first assertion, and no other test. A8 reads the tree, so its red
was read in CI at the first push: the `hygiene` job 130465679036 failed A8 on its `STALE` assertion,
the three listed entries matching no site, and no test outside this module.

Test edits after the red commit: none. The green commit a7707a28 replaces the stub bodies with the readers;
it changes no test function, planted text or expected value, and `git diff --quiet a7707a2 HEAD -- scripts/tests/test_process_waiter_census.py scripts/tests/fixtures/process-waiters` reads rc 0.

A8's green is read at the second push's head, which carries the green commit, the rows and this
record, in the `hygiene` job of the run at that head, which prints each unlisted site's `WARN` line, or none.

```red-first
A1: red at 09b85d42: AssertionError: Lists differ (the workflow, the shebang script and the Python file not counted)
A1: green at a7707a28
A2: red at 09b85d42: AssertionError: Lists differ: [] != [('.github/workflows/waits.yml', 'step 1 l[206 chars]rue)]
A2: green at a7707a28
A3: red at 09b85d42: AssertionError: Lists differ, first: [] (no clean lookup found)
A3: green at a7707a28
A4: red at 09b85d42: AssertionError: Lists differ, first: [] (no Python lookup found)
A4: green at a7707a28
A5: red at 09b85d42: AssertionError: Tuples differ, first: ([], False) (no WARN line printed)
A5: green at a7707a28
A6: red at 09b85d42: AssertionError: AssertionError not raised (an empty population not refused)
A6: green at a7707a28
A7: red at 09b85d42: AssertionError: Lists differ, first: [] (no unread site)
A7: green at a7707a28
A8: red at 09b85d42: AssertionError: Lists differ: ['STALE process-waiter scripts/tests/fixtu[298 chars] #1'] != []
A8: green at a7707a28
```
