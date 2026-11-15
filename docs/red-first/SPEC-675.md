# Red-first record: SPEC-675

SPEC-675 (R1 to R16). The SPEC, ADR-698, the `Planted*` tests and `TheRunsRange` in
`scripts/tests/test_commit_messages.py` and the planted fixtures under
`scripts/tests/fixtures/commit-messages/` were committed first and alone, over stubs that keep
every input but the missing behaviour: the fixture reader keeps its shape, the log reader returns
no commit, the range refusal refuses nothing, the trailer reader reads no block, each arm finds no
site and every subject is judged, the `read` line counts nothing, and the verdict passes everything
without reading its population. `LEVEL` and the empty `LISTED` were already there, so each red
below is its criterion's assertion and never a missing name.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_commit_messages.py -k test_a_house_type_subject_is_clean_and_every_other_subject_is_a_site
A2: python3 -m unittest discover -s scripts/tests -p test_commit_messages.py -k test_an_attribution_key_in_gits_block_is_a_site_and_every_other_key_is_counted
A3: python3 -m unittest discover -s scripts/tests -p test_commit_messages.py -k test_merges_and_reverts_a_tool_wrote_are_counted_and_every_other_subject_is_judged
A4: python3 -m unittest discover -s scripts/tests -p test_commit_messages.py -k test_the_read_line_counts_each_kind_and_each_counted_key
A5: python3 -m unittest discover -s scripts/tests -p test_commit_messages.py -k test_a_bots_commit_is_read_by_the_same_forms
A6: python3 -m unittest discover -s scripts/tests -p test_commit_messages.py -k test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does
A7: python3 -m unittest discover -s scripts/tests -p test_commit_messages.py -k test_an_empty_population_is_void
A8: python3 -m unittest discover -s scripts/tests -p test_commit_messages.py -k test_the_log_reader_splits_each_record_and_an_unreadable_range_is_void
A9: read in CI only, by the hygiene job's python stage
```

Eight reds were read by name over planted text, each on its first assertion against the literal
sites, lines and counts. `TheRunsRange` reads the run's range, so its red was read in CI: the hygiene job
130645384461 of the first push failed it, with no commit read against `HEAD`, beside the eight planted
reds and one test of another module, the red commit's own house-census red: that job also failed
`test_mutation_workflows`' `test_the_finder_is_defined_once_and_a_planted_copy_is_caught`, which
read the stub of `range_void` (a docstring and `return None`) as a copy of `_mutants_finder`'s
`no_wrapper`; the green body of `range_void` is no copy, and the second push read that test green.

Test edits after the red commit: none. The green commit replaces the stub bodies with the readers
and the arms; it changes no test function, fixture or expected value.

The hygiene job 130888449784 of the second push first measured the run's range: it printed
`examined N commit(s)`, the `read` line and each `WARN` line, listed here; a site of a commit
reachable from dev's tip is listed in `LISTED` with its reason.

```text
examined 6 commit(s)
read 6 commit(s): 5 judged, 1 merge(s) and 0 revert(s) a tool wrote, 0 by a bot
WARN commit-message 675c828073e6 no-type -
```

It printed 1 `WARN` line(s); none is of a commit reachable from dev's tip, so `LISTED` stays empty.

```red-first
A1: red at 6cc189e: AssertionError: Lists differ: [('cl[218 chars]d', []), ('no-desc', 'judged', []), ('empty-sc[75 chars] [])] != [('cl[218 chars]d', [('no-type', '-')]), ('no-desc', 'judged',[166 chars]')])]
A1: green at 02bdc24
A2: red at 6cc189e: AssertionError: Lists differ: [('co-author', [], []), ('sign-off', [], []), ('caps-key'[197 chars] [])] != [('co-author', [('attribution-trailer', 'co-authored-by')[463 chars] [])]
A2: green at 02bdc24
A3: red at 6cc189e: AssertionError: Lists differ: [('gh-merge', 'judged', []), ('gh-long', 'judged', []), [265 chars] [])] != [('gh-merge', 'merge', []), ('gh-long', 'merge', []), ('[348 chars]')])]
A3: green at 02bdc24
A4: red at 6cc189e: AssertionError: Lists differ: [] != ['read 11 commit(s): 3 judged, 6 merge(s) [80 chars] ci']
A4: green at 02bdc24
A5: red at 6cc189e: AssertionError: Lists differ: [('bo[15 chars]d', []), ('bot-build', 'judged', []), ('person', 'judged', [])] != [('bo[15 chars]d', [('no-type', '-'), ('attribution-trailer',[70 chars] [])]
A5: green at 02bdc24
A6: red at 6cc189e: AssertionError: Tuples differ: ([], []) != (['WARN commit-message no-desc no-type -', [309 chars] -'])
A6: green at 02bdc24
A7: red at 6cc189e: AssertionError: AssertionError not raised
A7: green at 02bdc24
A8: red at 6cc189e: AssertionError: Lists differ: [] != [('planted-one', 2, True, 'Merge pull requ[85 chars]es')]
A8: green at 02bdc24
A9: red at 6cc189e: AssertionError: 'HEAD' not found in []
A9: green at 02bdc24
```

The green commit `02bdc24` carries the census, and `git diff --quiet 02bdc24 HEAD` over the module and its planted files reads rc 0.
